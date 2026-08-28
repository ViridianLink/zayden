use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{admin_guild_id, db_pool, discord_client, server_err},
    reaction_roles::{
        GenericChannelId,
        GuildId,
        MessageId,
        ParsedEmoji,
        ReactionRole,
        RoleId,
    },
    twilight_http::request::channel::reaction::RequestReactionType,
    twilight_model::channel::message::Embed,
    twilight_model::id::Id,
};

use crate::dto::ReactionRoleInfo;

#[cfg(feature = "ssr")]
fn invalid(what: &str) -> ServerFnError {
    ServerFnError::ServerError(format!("invalid {what}"))
}

#[cfg(feature = "ssr")]
fn parse_snowflake(label: &str, s: &str) -> Result<u64, ServerFnError> {
    s.trim().parse::<u64>().map_err(|_e| invalid(label))
}

#[cfg(feature = "ssr")]
fn request_reaction(
    emoji: &ParsedEmoji,
) -> Result<RequestReactionType<'_>, ServerFnError> {
    emoji.custom_id.map_or_else(
        || Ok(RequestReactionType::Unicode { name: &emoji.stored }),
        |id| {
            Id::new_checked(id)
                .map(|id| RequestReactionType::Custom {
                    id,
                    name: Some(&emoji.name),
                })
                .ok_or_else(|| invalid("custom emoji id"))
        },
    )
}

#[server]
pub async fn list_reaction_roles(
    guild: String,
) -> Result<Vec<ReactionRoleInfo>, ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let pool = db_pool()?;

    let mut rows = ReactionRole::rows(&pool, GuildId::new(guild_id.cast_unsigned()))
        .await
        .map_err(server_err)?;
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

#[server]
pub async fn add_reaction_role(
    guild: String,
    channel_id: String,
    message_id: String,
    role_id: String,
    emoji: String,
) -> Result<(), ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let pool = db_pool()?;
    let http = discord_client()?;

    let channel = parse_snowflake("channel", &channel_id)?;
    let role = parse_snowflake("role", &role_id)?;
    let emoji = ParsedEmoji::parse(&emoji).map_err(server_err)?;
    let reaction = request_reaction(&emoji)?;

    let channel = Id::new_checked(channel).ok_or_else(|| invalid("channel"))?;

    let message = if message_id.trim().is_empty() {
        let embed = panel_embed(&emoji.stored, role);
        http.create_message(channel)
            .embeds(&[embed])
            .await
            .map_err(server_err)?
            .model()
            .await
            .map_err(server_err)?
            .id
    } else {
        let id = Id::new_checked(parse_snowflake("message id", &message_id)?)
            .ok_or_else(|| invalid("message id"))?;
        http.message(channel, id)
            .await
            .map_err(server_err)?
            .model()
            .await
            .map_err(server_err)?
            .id
    };

    let existing =
        ReactionRole::row(&pool, MessageId::new(message.get()), &emoji.stored)
            .await
            .map_err(server_err)?;
    if existing.is_some() {
        return Err(ServerFnError::ServerError(
            "that emoji is already mapped on that message".to_string(),
        ));
    }

    ReactionRole::create(
        &pool,
        GuildId::new(guild_id.cast_unsigned()),
        GenericChannelId::new(channel.get()),
        MessageId::new(message.get()),
        RoleId::new(role),
        &emoji.stored,
    )
    .await
    .map_err(server_err)?;

    http.create_reaction(channel, message, &reaction).await.map_err(server_err)?;

    Ok(())
}

#[server]
pub async fn remove_reaction_role(
    guild: String,
    channel_id: String,
    message_id: String,
    emoji: String,
) -> Result<(), ServerFnError> {
    let guild_id = admin_guild_id(&guild).await?;
    let pool = db_pool()?;
    let http = discord_client()?;

    let channel = parse_snowflake("channel", &channel_id)?;
    let message = parse_snowflake("message id", &message_id)?;
    let parsed = ParsedEmoji::parse(&emoji).map_err(server_err)?;
    let reaction = request_reaction(&parsed)?;

    ReactionRole::delete(
        &pool,
        GuildId::new(guild_id.cast_unsigned()),
        GenericChannelId::new(channel),
        MessageId::new(message),
        &parsed.stored,
    )
    .await
    .map_err(server_err)?;

    let (Some(channel), Some(message)) =
        (Id::new_checked(channel), Id::new_checked(message))
    else {
        return Err(invalid("mapping"));
    };

    if let Err(e) = http.delete_all_reaction(channel, message, &reaction).await {
        tracing::warn!(error = ?e, "failed to clear reaction-role reaction");
    }

    Ok(())
}

#[cfg(feature = "ssr")]
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
