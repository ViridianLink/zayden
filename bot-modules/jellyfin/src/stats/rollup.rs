use std::sync::Arc;

use sqlx::PgPool;
use tracing::info;

use crate::error::Result;
use crate::runtime::JellyfinRuntime;

const WINDOW_DAYS: i64 = 45;

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because DATE has no built-in jiff mapping"
)]
pub async fn run(runtime: &Arc<JellyfinRuntime>, pool: &PgPool) -> Result<u64> {
    let rows = runtime.playback.daily_rollup(WINDOW_DAYS).await?;
    let mut written = 0;

    for row in &rows {
        let Ok(day) = row.day.parse::<jiff::civil::Date>() else {
            continue;
        };

        sqlx::query!(
            r#"
            INSERT INTO jellyfin_playback_daily
                (jellyfin_user_id, day, seconds, items)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (jellyfin_user_id, day) DO UPDATE SET
                seconds = EXCLUDED.seconds,
                items = EXCLUDED.items
            "#,
            row.jellyfin_user_id,
            jiff_sqlx::Date::from(day) as jiff_sqlx::Date,
            row.seconds,
            row.items,
        )
        .execute(pool)
        .await?;

        written += 1;
    }

    info!(days = written, "jellyfin playback rollup complete");
    Ok(written)
}
