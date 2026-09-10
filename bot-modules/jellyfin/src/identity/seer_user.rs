use std::sync::Arc;

use jiff::{Span, Timestamp};
use sqlx::PgPool;

use crate::error::Result;
use crate::identity::link::JellyfinLinkRow;
use crate::runtime::JellyfinRuntime;

const STALE_AFTER_DAYS: i64 = 7;

pub async fn refresh(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    jellyfin_user_id: &str,
) -> Result<Option<i32>> {
    let seer_user = runtime.seer.user_for_jellyfin(jellyfin_user_id).await?;
    let seer_id = seer_user.map(|u| u.id);

    if let Some(row) =
        JellyfinLinkRow::by_jellyfin_id(pool, jellyfin_user_id).await?
    {
        JellyfinLinkRow::set_jellyseerr_user(pool, row.user_id, seer_id).await?;
    }

    Ok(seer_id)
}

pub async fn resolve(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    link: &JellyfinLinkRow,
) -> Result<Option<i32>> {
    let stale_before = Timestamp::now() - Span::new().days(STALE_AFTER_DAYS);
    let fresh = link
        .jellyseerr_checked_at
        .is_some_and(|checked| checked.to_jiff() > stale_before);

    if fresh && link.jellyseerr_user_id.is_some() {
        return Ok(link.jellyseerr_user_id);
    }

    refresh(runtime, pool, &link.jellyfin_user_id).await
}
