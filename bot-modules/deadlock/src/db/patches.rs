use std::collections::HashSet;

use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::PgPool;

use super::watch_state::column_version;
use crate::diff::PatchDiff;
use crate::error::{DeadlockError, Result};

#[derive(Debug, Clone)]
pub struct PatchRow {
    pub guid: String,
    pub title: String,
    pub link: String,
    pub published_at: Timestamp,
    pub detected_at: Timestamp,
    pub diff: Option<PatchDiff>,
}

#[derive(Debug, Clone, Copy)]
pub struct NewPatch<'a> {
    pub guid: &'a str,
    pub title: &'a str,
    pub link: &'a str,
    pub published_at: Timestamp,
}

struct PatchRecord {
    guid: String,
    title: String,
    link: String,
    published_at: jiff_sqlx::Timestamp,
    detected_at: jiff_sqlx::Timestamp,
    diff: Option<String>,
}

impl PatchRecord {
    fn into_row(self) -> Result<PatchRow> {
        let diff =
            self.diff.as_deref().map(serde_json::from_str).transpose().map_err(
                |source| DeadlockError::CacheDecode {
                    key: format!("deadlock_patches.diff#{}", self.guid),
                    source,
                },
            )?;

        Ok(PatchRow {
            guid: self.guid,
            title: self.title,
            link: self.link,
            published_at: self.published_at.to_jiff(),
            detected_at: self.detected_at.to_jiff(),
            diff,
        })
    }
}

pub async fn known_guids(pool: &PgPool) -> sqlx::Result<HashSet<String>> {
    let guids = sqlx::query_scalar!("SELECT guid FROM deadlock_patches")
        .fetch_all(pool)
        .await?;

    Ok(guids.into_iter().collect())
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn insert(
    pool: &PgPool,
    note: &NewPatch<'_>,
    announced: bool,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO deadlock_patches (guid, title, link, published_at, announced_at)
        VALUES ($1, $2, $3, $4, CASE WHEN $5 THEN now() END)
        ON CONFLICT (guid) DO NOTHING
        "#,
        note.guid,
        note.title,
        note.link,
        note.published_at.to_sqlx() as jiff_sqlx::Timestamp,
        announced
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn awaiting_diff(pool: &PgPool) -> Result<Vec<PatchRow>> {
    let records = sqlx::query_as!(
        PatchRecord,
        r#"
        SELECT guid,
               title,
               link,
               published_at AS "published_at: jiff_sqlx::Timestamp",
               detected_at AS "detected_at: jiff_sqlx::Timestamp",
               diff
        FROM deadlock_patches
        WHERE announced_at IS NULL AND diff IS NULL
        ORDER BY published_at, guid
        "#
    )
    .fetch_all(pool)
    .await?;

    records.into_iter().map(PatchRecord::into_row).collect()
}

pub async fn set_diff(
    pool: &PgPool,
    guid: &str,
    baseline: u32,
    current: u32,
    diff: &PatchDiff,
) -> Result<()> {
    let baseline = column_version(baseline)?;
    let current = column_version(current)?;
    let diff = serde_json::to_string(diff)
        .map_err(|source| DeadlockError::Encode { what: "patch diff", source })?;

    sqlx::query!(
        r#"
        UPDATE deadlock_patches
        SET baseline_version = $2, client_version = $3, diff = $4
        WHERE guid = $1
        "#,
        guid,
        baseline,
        current,
        diff
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn ready(pool: &PgPool) -> Result<Vec<PatchRow>> {
    let records = sqlx::query_as!(
        PatchRecord,
        r#"
        SELECT guid,
               title,
               link,
               published_at AS "published_at: jiff_sqlx::Timestamp",
               detected_at AS "detected_at: jiff_sqlx::Timestamp",
               diff
        FROM deadlock_patches
        WHERE announced_at IS NULL AND diff IS NOT NULL
        ORDER BY published_at, guid
        "#
    )
    .fetch_all(pool)
    .await?;

    records.into_iter().map(PatchRecord::into_row).collect()
}

pub async fn mark_announced(pool: &PgPool, guid: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE deadlock_patches SET announced_at = now() WHERE guid = $1",
        guid
    )
    .execute(pool)
    .await?;

    Ok(())
}
