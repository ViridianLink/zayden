use serenity::all::{
    ComponentInteraction,
    Context,
    CreateEmbed,
    CreateEmbedFooter,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, GuildMembersCache, as_i64};

use crate::common::LeaderboardManager;
use crate::common::leaderboard::{get_row_number, get_rows};
use crate::{GamblingError, Leaderboard, Result};

impl Leaderboard {
    pub async fn run_component<
        Data: GuildMembersCache + EmojiCacheData,
        Db: Database,
        Manager: LeaderboardManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let custom_id = interaction
            .data
            .custom_id
            .strip_prefix("leaderboard_")
            .ok_or_else(|| {
                GamblingError::Internal("expected leaderboard_ prefix".to_string())
            })?;

        let embed = interaction.message.embeds.first().ok_or_else(|| {
            GamblingError::Internal("leaderboard message missing embed".to_string())
        })?;

        let title = embed.title.as_ref().ok_or_else(|| {
            GamblingError::Internal("leaderboard embed missing title".to_string())
        })?;

        let global = title.strip_prefix("🏁 Global Leaderboard (");

        let leaderboard = global.map_or_else(
            || -> Result<&str> {
                title
                    .strip_prefix("🏁 Leaderboard (")
                    .ok_or_else(|| {
                        GamblingError::Internal(
                            "leaderboard title missing prefix".to_string(),
                        )
                    })?
                    .strip_suffix(")")
                    .ok_or_else(|| {
                        GamblingError::Internal(
                            "leaderboard title missing suffix".to_string(),
                        )
                    })
            },
            |s| {
                s.strip_suffix(")").ok_or_else(|| {
                    GamblingError::Internal(
                        "global leaderboard title missing suffix".to_string(),
                    )
                })
            },
        )?;

        let mut page_number: i64 = embed
            .footer
            .as_ref()
            .ok_or_else(|| {
                GamblingError::Internal(
                    "leaderboard embed missing footer".to_string(),
                )
            })?
            .text
            .strip_prefix("Page ")
            .ok_or_else(|| {
                GamblingError::Internal("footer missing 'Page ' prefix".to_string())
            })?
            .parse()
            .map_err(|_e| {
                GamblingError::Internal("page number parse failed".to_string())
            })?;

        let users = if global.is_none() {
            let users = {
                let data = ctx.data::<RwLock<Data>>();
                let data = data.read().await;
                let guild_id = interaction.guild_id.ok_or_else(|| {
                    GamblingError::Internal(
                        "guild_id missing in leaderboard component".to_string(),
                    )
                })?;
                data.get()
                    .get(&guild_id)
                    .ok_or_else(|| {
                        GamblingError::Internal(
                            "guild not in members cache".to_string(),
                        )
                    })?
                    .iter()
                    .map(|id| as_i64(id.get()))
                    .collect::<Vec<_>>()
            };
            Some(users)
        } else {
            None
        };

        match custom_id {
            "previous" => {
                page_number = (page_number - 1).max(1);
            },
            "user" => {
                let row_num = get_row_number::<Db, Manager>(
                    leaderboard,
                    pool,
                    users.as_deref(),
                    interaction.user.id,
                )
                .await?
                .unwrap_or(0);
                page_number = row_num / 10 + 1;
            },
            "next" => {
                page_number += 1;
            },
            _ => {},
        }

        let rows = get_rows::<Db, Manager>(
            leaderboard,
            pool,
            users.as_deref(),
            page_number,
        )
        .await?;

        if rows.is_empty() {
            return Err(GamblingError::internal("No entries for this page"));
        }

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let desc = rows
            .into_iter()
            .enumerate()
            .map(|(i, row)| {
                row.as_desc(
                    &emojis,
                    i + (usize::try_from(page_number - 1).unwrap_or(0)) * 10,
                )
            })
            .collect::<Result<Vec<_>>>()?
            .join("\n\n");

        let embed = CreateEmbed::from(embed.clone())
            .footer(CreateEmbedFooter::new(format!("Page {page_number}")))
            .description(desc);

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().embed(embed),
                ),
            )
            .await?;

        Ok(())
    }
}
