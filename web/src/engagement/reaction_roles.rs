use reaction_roles::{
    GenericChannelId,
    MessageId,
    ParsedEmoji,
    ReactionRole,
    RoleId,
};
use topcoat::context::Cx;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::channel::message::Embed;
use twilight_model::id::Id;

use super::dto::ReactionRoleInfo;
use super::error::EngagementError;
use super::form::form_args;
use super::parse::{guild_ref, parse_snowflake, usable_id};
use crate::auth::{
    ChannelInfo,
    GuildIds,
    RoleInfo,
    admin_guild_id,
    db_pool,
    discord_client,
    list_guild_channels,
    list_guild_roles,
};

form_args! {
    AddReactionRoleForm { guild, channel_id, message_id, role_id, emoji }
}

form_args! {
    RemoveReactionRoleForm { guild, channel_id, message_id, emoji }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionRolesPage {
    pub mappings: Vec<ReactionRoleInfo>,
    pub channels: Vec<ChannelInfo>,
    pub roles: Vec<RoleInfo>,
}

fn request_reaction(
    emoji: &ParsedEmoji,
) -> Result<RequestReactionType<'_>, EngagementError> {
    emoji.custom_id.map_or_else(
        || Ok(RequestReactionType::Unicode { name: &emoji.stored }),
        |id| {
            Id::new_checked(id)
                .map(|id| RequestReactionType::Custom {
                    id,
                    name: Some(&emoji.name),
                })
                .ok_or(EngagementError::Invalid("custom emoji id"))
        },
    )
}

fn panel_embed(emoji: &str, role: u64) -> Embed {
    Embed {
        author: None,
        color: None,
        description: Some(format!("{emoji} | <@&{role}>")),
        fields: Vec::new(),
        footer: None,
        image: None,
        kind: "rich".to_string(),
        provider: None,
        thumbnail: None,
        timestamp: None,
        title: None,
        url: None,
        video: None,
    }
}

pub async fn list_reaction_roles(
    cx: &Cx,
    guild: &str,
) -> Result<Vec<ReactionRoleInfo>, EngagementError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    let pool = db_pool(cx)?;

    let mut rows = ReactionRole::rows(pool, guild_ref(guild_id)?).await?;
    rows.sort_by(|a, b| {
        (a.channel_id, a.message_id, &a.emoji).cmp(&(
            b.channel_id,
            b.message_id,
            &b.emoji,
        ))
    });

    Ok(rows
        .into_iter()
        .map(|r| ReactionRoleInfo {
            channel_id: r.channel_id().to_string(),
            message_id: r.message_id().to_string(),
            role_id: r.role_id().to_string(),
            emoji: r.emoji,
        })
        .collect())
}

/// The mappings plus the guild's channels and roles for the pickers and the
/// table. A failed channel or role listing reads as an empty one.
pub async fn load_reaction_roles_page(
    cx: &Cx,
    guild: &str,
) -> Result<ReactionRolesPage, EngagementError> {
    let mappings = list_reaction_roles(cx, guild).await?;
    let channels = list_guild_channels(cx, guild).await.unwrap_or_default();
    let roles = list_guild_roles(cx, guild).await.unwrap_or_default();

    Ok(ReactionRolesPage { mappings, channels, roles })
}

/// Maps an emoji on a message to a role. A blank message id posts a new panel
/// message in the channel and maps the emoji on that. The bot then adds the
/// reaction.
pub async fn add_reaction_role(
    cx: &Cx,
    form: &AddReactionRoleForm,
) -> Result<(), EngagementError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let pool = db_pool(cx)?;
    let http = discord_client(cx)?;

    let channel = parse_snowflake("channel", &form.channel_id)?;
    let role = parse_snowflake("role", &form.role_id)?;
    let emoji = ParsedEmoji::parse(&form.emoji)?;
    let reaction = request_reaction(&emoji)?;

    GuildIds::default()
        .channel(Some(channel.cast_signed()))
        .role(Some(role.cast_signed()))
        .ensure_in(cx, guild_id)
        .await?;

    let role = usable_id("role", role)?.get();
    let channel =
        Id::new_checked(channel).ok_or(EngagementError::Invalid("channel"))?;

    let message = if form.message_id.trim().is_empty() {
        let embed = panel_embed(&emoji.stored, role);
        http.create_message(channel).embeds(&[embed]).await?.model().await?.id
    } else {
        let id = Id::new_checked(parse_snowflake("message id", &form.message_id)?)
            .ok_or(EngagementError::Invalid("message id"))?;
        http.message(channel, id).await?.model().await?.id
    };

    let existing =
        ReactionRole::row(pool, MessageId::new(message.get()), &emoji.stored)
            .await?;
    if existing.is_some() {
        return Err(EngagementError::EmojiAlreadyMapped);
    }

    ReactionRole::create(
        pool,
        guild_ref(guild_id)?,
        GenericChannelId::new(channel.get()),
        MessageId::new(message.get()),
        RoleId::new(role),
        &emoji.stored,
    )
    .await?;

    http.create_reaction(channel, message, &reaction).await?;

    Ok(())
}

/// Removes a mapping and, if it was this guild's, clears the emoji's
/// reactions from the message. A mapping that is not found is not an error.
pub async fn remove_reaction_role(
    cx: &Cx,
    form: &RemoveReactionRoleForm,
) -> Result<(), EngagementError> {
    let guild_id = admin_guild_id(cx, &form.guild).await?;
    let pool = db_pool(cx)?;
    let http = discord_client(cx)?;

    let channel = parse_snowflake("channel", &form.channel_id)?;
    let message = parse_snowflake("message id", &form.message_id)?;
    let parsed = ParsedEmoji::parse(&form.emoji)?;
    let reaction = request_reaction(&parsed)?;

    let channel = usable_id("channel", channel)?;
    let message = usable_id("message id", message)?;

    let deleted = ReactionRole::delete(
        pool,
        guild_ref(guild_id)?,
        GenericChannelId::new(channel.get()),
        MessageId::new(message.get()),
        &parsed.stored,
    )
    .await?;

    // The channel may be another guild's and its reactions are not ours to clear.
    if deleted.rows_affected() == 0 {
        return Ok(());
    }

    if let Err(e) = http
        .delete_all_reaction(Id::from(channel), Id::from(message), &reaction)
        .await
    {
        tracing::warn!(error = ?e, "failed to clear reaction-role reaction");
    }

    Ok(())
}
