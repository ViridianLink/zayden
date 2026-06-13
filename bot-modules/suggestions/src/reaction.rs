use futures::{StreamExt, TryStreamExt};
use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateEmbed,
    CreateEmbedAuthor,
    CreateEmbedFooter,
    CreateMessage,
    EditMessage,
    EmbedField,
    GuildChannel,
    Http,
    Message,
    Reaction,
    ReactionType,
};
use sqlx::{Database, Pool};
use tracing::debug;

use crate::{Result, Suggestions, SuggestionsError, SuggestionsGuildManager};

impl Suggestions {
    pub async fn reaction<Db: Database, Manager: SuggestionsGuildManager<Db>>(
        http: &Http,
        reaction: &Reaction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let Some(channel) = reaction.channel(http).await?.guild() else {
            debug!(channel_id = %reaction.channel_id, "reaction channel is not a guild channel; ignoring");
            return Ok(());
        };

        let guild_id = channel.base.guild_id;

        let row = match Manager::get(pool, guild_id).await {
            Ok(Some(row)) => row,
            Ok(None) | Err(sqlx::Error::RowNotFound) => {
                debug!(%guild_id, "no suggestions configuration found for guild; ignoring reaction");
                return Ok(());
            },
            Err(e) => return Err(e.into()),
        };

        if channel.parent_id.is_none()
            || row.channel_id().is_none()
            || channel.parent_id != row.channel_id()
        {
            return Err(SuggestionsError::Internal(format!(
                "reaction in channel {} is not in the suggestions forum (parent={:?}, configured={:?})",
                channel.id,
                channel.parent_id,
                row.channel_id()
            )));
        }

        let Some(review_channel_id) = row.review_channel_id() else {
            return Err(SuggestionsError::Internal(format!(
                "guild {guild_id} has no review channel configured"
            )));
        };

        let message = reaction.message(http).await?;

        let positive_reaction = ReactionType::from('👍');
        let negative_reaction = ReactionType::from('👎');

        let (pos_count, neg_count) =
            message.reactions.iter().fold((0, 0), |(pos, neg), r| {
                let count = i32::try_from(r.count).unwrap_or(i32::MAX);
                if r.reaction_type == positive_reaction {
                    (count, neg)
                } else if r.reaction_type == negative_reaction {
                    (pos, count)
                } else {
                    (pos, neg)
                }
            });

        let mut messages = review_channel_id.widen().messages_iter(http).boxed();

        if (pos_count - neg_count) >= 20 {
            while let Some(mut msg) = messages.try_next().await? {
                if let Some(embed) = msg.embeds.first()
                    && embed.url.as_deref()
                        == Some(message.link().to_string().as_str())
                {
                    let embed = create_embed(
                        &channel,
                        &message,
                        msg.embeds.first().map_or(&[], |e| e.fields.as_slice()),
                        pos_count,
                        neg_count,
                    );

                    msg.edit(
                        http,
                        EditMessage::new()
                            .embed(embed)
                            .components(vec![create_components()]),
                    )
                    .await?;

                    return Ok(());
                }
            }

            review_channel_id
                .widen()
                .send_message(
                    http,
                    CreateMessage::new()
                        .embed(create_embed(
                            &channel,
                            &message,
                            &Vec::new(),
                            pos_count,
                            neg_count,
                        ))
                        .components(vec![create_components()]),
                )
                .await?;
        } else if (neg_count - pos_count) <= 15 {
            while let Some(msg) = messages.try_next().await? {
                if msg.embeds.first().and_then(|e| e.url.as_deref())
                    == Some(message.link().to_string().as_str())
                {
                    msg.delete(http, Some("Positive delta fell below 15")).await?;

                    return Ok(());
                }
            }
        }

        Ok(())
    }
}

fn create_embed<'a>(
    channel: &'a GuildChannel,
    message: &'a Message,
    embed_fields: &[EmbedField],
    pos_count: i32,
    neg_count: i32,
) -> CreateEmbed<'a> {
    let mut embed = CreateEmbed::new()
        .title(&channel.base.name)
        .url(message.link().to_string())
        .description(&message.content)
        .author(CreateEmbedAuthor::new(&message.author.name))
        .footer(CreateEmbedFooter::new(format!("👍 {pos_count} · 👎 {neg_count}")));

    if let Some(team_response) = embed_fields.first() {
        embed = embed.field(
            team_response.name.clone(),
            team_response.value.clone(),
            team_response.inline,
        );
    }

    embed
}

fn create_components<'a>() -> CreateComponent<'a> {
    CreateComponent::ActionRow(CreateActionRow::buttons(vec![
        CreateButton::new("suggestions_accept")
            .label("Accept")
            .style(ButtonStyle::Success),
        CreateButton::new("suggestions_reject")
            .label("Reject")
            .style(ButtonStyle::Danger),
    ]))
}
