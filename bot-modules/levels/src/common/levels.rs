use serenity::all::{Context, CreateEmbed, CreateEmbedFooter, GuildId, Mentionable};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{GuildMembersCache, as_i64};

use crate::{LeaderboardRow, LevelsManager, LevelsRow};

pub async fn create_embed<
    'a,
    Data: GuildMembersCache,
    Db: Database,
    Manager: LevelsManager<Db>,
>(
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
            .expect("guild should be in member cache")
            .iter()
            .map(|id| as_i64(id.get()))
            .collect::<Vec<_>>()
    };

    let rows =
        Manager::leaderboard(pool, &users, page_number).await.expect("DB query");

    let desc = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            row_as_desc(
                &row,
                i + (usize::try_from(page_number).expect("page_number is positive")
                    - 1)
                    * 10,
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    CreateEmbed::new()
        .title("Leaderboard")
        .description(desc)
        .footer(CreateEmbedFooter::new(format!("Page {page_number}")))
}

#[must_use]
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
