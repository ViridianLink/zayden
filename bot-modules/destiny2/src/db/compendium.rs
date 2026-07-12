use sqlx::PgPool;

use crate::compendium::PerkInfo;

pub async fn is_empty(pool: &PgPool) -> sqlx::Result<bool> {
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM destiny2_compendium_perks)"#
    )
    .fetch_one(pool)
    .await?;

    Ok(!exists.unwrap_or(false))
}

pub async fn find(pool: &PgPool, key: &str) -> sqlx::Result<Option<PerkInfo>> {
    sqlx::query_as!(
        PerkInfo,
        "SELECT name, description FROM destiny2_compendium_perks WHERE key = $1",
        key
    )
    .fetch_optional(pool)
    .await
}

pub async fn search(pool: &PgPool, query: &str) -> sqlx::Result<Vec<PerkInfo>> {
    sqlx::query_as!(
        PerkInfo,
        "SELECT name, description FROM destiny2_compendium_perks
         WHERE key LIKE '%' || $1 || '%'
         ORDER BY name
         LIMIT 25",
        query
    )
    .fetch_all(pool)
    .await
}

pub async fn replace(
    pool: &PgPool,
    perks: &[(String, PerkInfo)],
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!("TRUNCATE destiny2_compendium_perks RESTART IDENTITY")
        .execute(&mut *tx)
        .await?;

    for (key, perk) in perks {
        sqlx::query!(
            "INSERT INTO destiny2_compendium_perks (key, name, description)
             VALUES ($1, $2, $3)
             ON CONFLICT (key) DO UPDATE
                SET name = EXCLUDED.name, description = EXCLUDED.description",
            key,
            perk.name,
            perk.description,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}
