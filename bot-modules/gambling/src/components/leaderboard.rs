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
use zayden_core::{EmojiCacheData, GuildMembersCache};

use crate::common::LeaderboardManager;
use crate::common::leaderboard::{get_row_number, get_rows};
use crate::{Leaderboard, Result};

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
            .expect("registered with Prefix(\"leaderboard_\")");

        let embed = interaction
            .message
            .embeds
            .first()
            .expect("leaderboard message always has embed");

        let title =
            embed.title.as_ref().expect("leaderboard embed always has title");

        let global = title.strip_prefix("🏁 Global Leaderboard (");

        let leaderboard = global.map_or_else(
            || {
                title
                    .strip_prefix("🏁 Leaderboard (")
                    .expect("bot-set leaderboard title prefix")
                    .strip_suffix(")")
                    .expect("bot-set title ends with )")
            },
            |s| s.strip_suffix(")").expect("bot-set title ends with )"),
        );

        let mut page_number: i64 = embed
            .footer
            .as_ref()
            .expect("leaderboard embed always has footer")
            .text
            .strip_prefix("Page ")
            .expect("bot-set footer starts with Page")
            .parse()
            .expect("page number is always a valid integer");

        let users = if global.is_none() {
            let users = {
                let data = ctx.data::<RwLock<Data>>();
                let data = data.read().await;
                data.get()
                    .get(
                        &interaction
                            .guild_id
                            .expect("gambling command always used in guild"),
                    )
                    .expect("guild members cached when leaderboard command ran")
                    .iter()
                    .map(|id| id.get().cast_signed())
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
                .await
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
        .await;

        if rows.is_empty() {
            return Ok(());
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
            .collect::<Vec<_>>()
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
