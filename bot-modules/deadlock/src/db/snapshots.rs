use deadlock_api::model::{Hero, Item};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::PgPool;

use super::watch_state::column_version;
use crate::error::{DeadlockError, Result};

fn encode<T: Serialize + ?Sized>(what: &'static str, value: &T) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|source| DeadlockError::Encode { what, source })
}

fn decode<T: DeserializeOwned>(key: String, body: &str) -> Result<T> {
    serde_json::from_str(body)
        .map_err(|source| DeadlockError::CacheDecode { key, source })
}

pub async fn store(
    pool: &PgPool,
    version: u32,
    heroes: &[Hero],
    items: &[Item],
) -> Result<()> {
    let version_id = column_version(version)?;
    let heroes = encode("hero snapshot", heroes)?;
    let items = encode("item snapshot", items)?;

    sqlx::query!(
        r#"
        INSERT INTO deadlock_asset_snapshots (client_version, heroes, items)
        VALUES ($1, $2, $3)
        ON CONFLICT (client_version) DO NOTHING
        "#,
        version_id,
        heroes,
        items
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn load(
    pool: &PgPool,
    version: u32,
) -> Result<Option<(Vec<Hero>, Vec<Item>)>> {
    let Ok(version_id) = i32::try_from(version) else {
        return Ok(None);
    };

    let row = sqlx::query!(
        "SELECT heroes, items FROM deadlock_asset_snapshots WHERE client_version = $1",
        version_id
    )
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else { return Ok(None) };

    let heroes = decode(format!("snapshot heroes#{version}"), &row.heroes)?;
    let items = decode(format!("snapshot items#{version}"), &row.items)?;

    Ok(Some((heroes, items)))
}
