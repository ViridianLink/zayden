use serenity::all::{Context, CreateEmbed, CreateEmbedFooter, GuildId, Mentionable};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::GuildMembersCache;

use crate::{LeaderboardRow, LevelsManager, LevelsRow};

pub async fn create_embed<'a, Data: GuildMembersCache, Db: Database, Manager: LevelsManager<Db>>(
    ctx: &Context,
    pool: &Pool<Db>,
    guild_id: GuildId,
    page_number: i64,
) -> CreateEmbed<'a> {
    let users = {
        let data = ctx.data::<RwLock<Data>>();
        let data = data.read().await;

        data.get()
            .get(&guild_id)
            .unwrap()
            .iter()
            .map(|id| id.get() as i64)
            .collect::<Vec<_>>()
    };

    let rows = Manager::leaderboard(pool, &users, page_number)
        .await
        .unwrap();

    let desc = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| row_as_desc(&row, i + (page_number as usize - 1) * 10))
        .collect::<Vec<_>>()
        .join("\n\n");

    CreateEmbed::new()
        .title("Leaderboard")
        .description(desc)
        .footer(CreateEmbedFooter::new(format!("Page {page_number}")))
}

pub fn row_as_desc(row: &LeaderboardRow, i: usize) -> String {
    let place = if i == 0 {
        "🥇".to_string()
    } else if i == 1 {
        "🥈".to_string()
    } else if i == 2 {
        "🥉".to_string()
    } else {
        format!("#{}", i + 1)
    };

    let data = format!(
        "{}\n(Messages: {} | Total XP: {})",
        row.level(),
        row.message_count(),
        row.xp(),
    );

    format!("{place} - {} - {data}", row.user_id().mention())
}
