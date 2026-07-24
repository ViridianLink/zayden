use leptos::prelude::*;
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{
        db_pool,
        discord_client,
        guild_admin_context,
        server_err,
    },
    twilight_model::id::Id,
};

use crate::dto::LeaderboardEntry;

#[cfg(feature = "ssr")]
const PAGE_SIZE: i64 = 10;

#[cfg(feature = "ssr")]
struct LevelRow {
    user_id: i64,
    xp: i32,
    level: i32,
    message_count: i64,
}

#[server]
pub async fn get_leaderboard(
    guild: String,
    global: bool,
    page: i32,
) -> Result<Vec<LeaderboardEntry>, ServerFnError> {
    let (guild_id, _user, _token) = guild_admin_context(&guild).await?;
    let pool = db_pool()?;

    let page = i64::from(page).max(1);
    let offset = (page - 1) * PAGE_SIZE;

    let rows = if global {
        sqlx::query_as!(
            LevelRow,
            "SELECT user_id, xp, level, message_count FROM levels ORDER BY level DESC, xp DESC LIMIT 10 OFFSET $1",
            offset
        )
        .fetch_all(&pool)
        .await
        .map_err(server_err)?
    } else {
        sqlx::query_as!(
            LevelRow,
            "SELECT user_id, xp, level, message_count FROM guild_levels WHERE guild_id = $1 ORDER BY level DESC, xp DESC LIMIT 10 OFFSET $2",
            guild_id,
            offset
        )
        .fetch_all(&pool)
        .await
        .map_err(server_err)?
    };

    let http = discord_client()?;
    let mut entries = Vec::with_capacity(rows.len());
    for (rank, row) in (offset + 1..).zip(rows) {
        let user_id = row.user_id.cast_unsigned();

        let user = match http.user(Id::new(user_id)).await {
            Ok(resp) => resp.model().await.ok(),
            Err(_) => None,
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

    Ok(entries)
}
