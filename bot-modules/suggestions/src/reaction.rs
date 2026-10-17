use futures::{StreamExt, TryStreamExt};
use serenity::all::{
    ButtonStyle,
    ChannelId,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateEmbed,
    CreateEmbedAuthor,
    CreateEmbedFooter,
    CreateMessage,
    EditMessage,
    EmbedField,
    GuildId,
    Http,
    Message,
    Reaction,
    ReactionType,
};
use sqlx::PgPool;
use tracing::debug;

use crate::layout::{Candidate, candidate, parse_link, thread_name};
use crate::{Result, Suggestions, SuggestionsGuildRow};

struct Suggestion {
    message: Message,
    title: String,
    link: String,
}

impl Suggestion {
    fn new(message: Message, title: String, guild_id: GuildId) -> Self {
        let link = message.id.link(message.channel_id, Some(guild_id)).to_string();

        Self { message, title, link }
    }

    fn is_reviewed_by(&self, review: &Message) -> bool {
        review
            .embeds
            .first()
            .and_then(|embed| embed.url.as_deref())
            .and_then(parse_link)
            .is_some_and(|(_, message_id)| message_id == self.message.id)
    }
}

impl Suggestions {
    pub async fn reaction(
        http: &Http,
        reaction: &Reaction,
        pool: &PgPool,
    ) -> Result<()> {
        let Some(guild_id) = reaction.guild_id else {
            debug!(channel_id = %reaction.channel_id, "reaction is not in a guild; ignoring");
            return Ok(());
        };

        if reaction.user_id.is_some_and(|user_id| {
            http.application_id().is_some_and(|app_id| app_id.get() == user_id.get())
        }) {
            return Ok(());
        }

        let row = match SuggestionsGuildRow::get(pool, guild_id).await {
            Ok(Some(row)) => row,
            Ok(None) | Err(sqlx::Error::RowNotFound) => {
                debug!(%guild_id, "no suggestions configuration found for guild; ignoring reaction");
                return Ok(());
            },
            Err(e) => return Err(e.into()),
        };

        let Some(configured) = row.channel_id() else {
            debug!(%guild_id, "guild has no suggestions channel configured; ignoring reaction");
            return Ok(());
        };

        let Some(review_channel_id) = row.review_channel_id() else {
            debug!(%guild_id, "guild has no review channel configured; ignoring reaction");
            return Ok(());
        };

        let Some(suggestion) = locate(http, reaction, guild_id, configured).await?
        else {
            debug!(channel_id = %reaction.channel_id, "reaction is not on a suggestion; ignoring");
            return Ok(());
        };

        let message = &suggestion.message;

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

        let thresholds = row.thresholds();

        let mut messages = review_channel_id.widen().messages_iter(http).boxed();

        match review_action(pos_count, neg_count, thresholds) {
            ReviewAction::Promote => {
                while let Some(mut msg) = messages.try_next().await? {
                    if suggestion.is_reviewed_by(&msg) {
                        let embed = create_embed(
                            &suggestion,
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
                                &suggestion,
                                &[],
                                pos_count,
                                neg_count,
                            ))
                            .components(vec![create_components()]),
                    )
                    .await?;
            },
            ReviewAction::Demote => {
                let reason = format!(
                    "Positive delta fell to or below {}",
                    thresholds.demote()
                );

                while let Some(msg) = messages.try_next().await? {
                    if suggestion.is_reviewed_by(&msg) {
                        msg.delete(http, Some(&reason)).await?;

                        return Ok(());
                    }
                }
            },
            ReviewAction::Unchanged => {},
        }

        Ok(())
    }
}

async fn locate(
    http: &Http,
    reaction: &Reaction,
    guild_id: GuildId,
    configured: ChannelId,
) -> Result<Option<Suggestion>> {
    match candidate(reaction.channel_id, reaction.message_id, configured) {
        Candidate::Ignore => Ok(None),
        Candidate::TextMessage => {
            let message = reaction.message(http).await?;

            if message.author.bot() {
                return Ok(None);
            }

            let title = message.thread.as_ref().map_or_else(
                || thread_name(message.author.display_name(), &message.content),
                |thread| thread.base.name.to_string(),
            );

            Ok(Some(Suggestion::new(message, title, guild_id)))
        },
        Candidate::ThreadStarter => {
            let Some(thread) = reaction.channel(http).await?.thread() else {
                return Ok(None);
            };

            if thread.parent_id != configured {
                return Ok(None);
            }

            let message = reaction.message(http).await?;

            Ok(Some(Suggestion::new(
                message,
                thread.base.name.to_string(),
                guild_id,
            )))
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewAction {
    Promote,
    Demote,
    Unchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewThresholds {
    promote: i32,
    demote: i32,
}

impl Default for ReviewThresholds {
    fn default() -> Self {
        Self { promote: Self::DEFAULT_PROMOTE, demote: Self::DEFAULT_DEMOTE }
    }
}

impl ReviewThresholds {
    pub const DEFAULT_DEMOTE: i32 = 15;
    pub const DEFAULT_PROMOTE: i32 = 20;

    #[must_use]
    pub const fn new(promote: i32, demote: i32) -> Self {
        let demote =
            if demote < promote { demote } else { promote.saturating_sub(1) };

        Self { promote, demote }
    }

    #[must_use]
    pub fn parse(promote: &str, demote: &str) -> Self {
        Self::new(
            promote.trim().parse().unwrap_or(Self::DEFAULT_PROMOTE),
            demote.trim().parse().unwrap_or(Self::DEFAULT_DEMOTE),
        )
    }

    #[must_use]
    pub const fn promote(self) -> i32 {
        self.promote
    }

    #[must_use]
    pub const fn demote(self) -> i32 {
        self.demote
    }
}

#[must_use]
pub const fn review_action(
    pos_count: i32,
    neg_count: i32,
    thresholds: ReviewThresholds,
) -> ReviewAction {
    let delta = pos_count - neg_count;

    if delta >= thresholds.promote {
        ReviewAction::Promote
    } else if delta <= thresholds.demote {
        ReviewAction::Demote
    } else {
        ReviewAction::Unchanged
    }
}

fn create_embed<'a>(
    suggestion: &'a Suggestion,
    embed_fields: &[EmbedField],
    pos_count: i32,
    neg_count: i32,
) -> CreateEmbed<'a> {
    let mut embed = CreateEmbed::new()
        .title(&suggestion.title)
        .url(&suggestion.link)
        .description(&suggestion.message.content)
        .author(CreateEmbedAuthor::new(&suggestion.message.author.name))
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
