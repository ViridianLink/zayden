use topcoat::context::Cx;
use twilight_model::id::Id;

use super::dto::{LeaderboardEntry, LeaderboardPage};
use super::error::EngagementError;
use crate::auth::{admin_guild_id, db_pool, discord_client};

pub const PAGE_SIZE: i64 = 10;

struct LevelRow {
    user_id: i64,
    xp: i32,
    level: i32,
    message_count: i64,
}

/// Needs guild admin access even for the global board.
pub async fn get_leaderboard(
    cx: &Cx,
    guild: &str,
    global: bool,
    page: i32,
) -> Result<LeaderboardPage, EngagementError> {
    let guild_id = admin_guild_id(cx, guild).await?;
    let pool = db_pool(cx)?;

    let page = i64::from(page).max(1);
    let offset = (page - 1) * PAGE_SIZE;
    let limit = PAGE_SIZE + 1;

    let rows = if global {
        sqlx::query_as!(
            LevelRow,
            "SELECT user_id, xp, level, message_count FROM levels ORDER BY level DESC, xp DESC LIMIT $1 OFFSET $2",
            limit,
            offset
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as!(
            LevelRow,
            "SELECT user_id, xp, level, message_count FROM guild_levels WHERE guild_id = $1 ORDER BY level DESC, xp DESC LIMIT $2 OFFSET $3",
            guild_id,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?
    };

    let http = discord_client(cx)?;
    let mut entries = Vec::with_capacity(rows.len());
    for (rank, row) in (offset + 1..=offset + PAGE_SIZE).zip(&rows) {
        let user_id = row.user_id.cast_unsigned();

        let user = match Id::new_checked(user_id) {
            Some(id) => match http.user(id).await {
                Ok(resp) => resp.model().await.ok(),
                Err(_) => None,
            },
            None => None,
        };
        let (name, avatar) = match user {
            Some(user) => {
                let avatar = user.avatar.map(|hash| {
                    format!(
                        "https://cdn.discordapp.com/avatars/{user_id}/{hash}.png"
                    )
                });
                (user.global_name.unwrap_or(user.name), avatar)
            },
            None => (format!("User {user_id}"), None),
        };

        entries.push(LeaderboardEntry {
            rank,
            user_id: user_id.to_string(),
            name,
            avatar,
            level: row.level,
            xp: row.xp,
            message_count: row.message_count,
        });
    }

    let has_next = rows.len() > entries.len();

    Ok(LeaderboardPage { entries, has_next })
}
