use serenity::all::{
    ComponentInteraction, Context, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
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
            .unwrap();

        let embed = interaction.message.embeds.first().unwrap();

        let title = embed.title.as_ref().unwrap();

        let global = title.strip_prefix("🏁 Global Leaderboard (");

        let leaderboard = match global {
            Some(s) => s.strip_suffix(")").unwrap(),
            None => title
                .strip_prefix("🏁 Leaderboard (")
                .unwrap()
                .strip_suffix(")")
                .unwrap(),
        };

        let mut page_number: i64 = embed
            .footer
            .as_ref()
            .unwrap()
            .text
            .strip_prefix("Page ")
            .unwrap()
            .parse()
            .unwrap();

        let users = if global.is_none() {
            let data = ctx.data::<RwLock<Data>>();
            let data = data.read().await;
            let users = data
                .get()
                .get(&interaction.guild_id.unwrap())
                .unwrap()
                .iter()
                .map(|id| id.get() as i64)
                .collect::<Vec<_>>();
            Some(users)
        } else {
            None
        };

        match custom_id {
            "previous" => {
                page_number = (page_number - 1).max(1);
            }
            "user" => {
                let row_num = get_row_number::<Db, Manager>(
                    leaderboard,
                    pool,
                    users.as_deref(),
                    interaction.user.id,
                )
                .await
                .unwrap();
                page_number = row_num / 10 + 1;
            }
            "next" => {
                page_number += 1;
            }
            _ => unreachable!("Invalid custom id"),
        };

        let rows = get_rows::<Db, Manager>(leaderboard, pool, users.as_deref(), page_number).await;

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
            .map(|(i, row)| row.as_desc(&emojis, i + (page_number as usize - 1) * 10))
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
            .await
            .unwrap();

        Ok(())
    }
}
