use serenity::all::Colour;
use sqlx::PgPool;

use crate::endgame_analysis::sheet::Affinity;
use crate::endgame_analysis::sheet::tier::{Tier, TierLabel};
use crate::endgame_analysis::sheet::weapon::Weapon;

struct EndgameWeaponRow {
    icon: Option<String>,
    name: String,
    archetype: String,
    affinity: Option<Affinity>,
    frame: Option<String>,
    enhanceable: bool,
    reserves: Option<i32>,
    barrel: String,
    magazine: String,
    perk_1: String,
    perk_2: String,
    origin_trait: String,
    rank: i16,
    tier: TierLabel,
    tier_colour: i32,
    notes: Option<String>,
}

impl From<EndgameWeaponRow> for Weapon {
    fn from(row: EndgameWeaponRow) -> Self {
        Self {
            icon: row.icon,
            name: row.name,
            archetype: row.archetype,
            affinity: row.affinity,
            frame: row.frame,
            enhanceable: row.enhanceable,
            reserves: row.reserves.and_then(|r| u16::try_from(r).ok()),
            barrel: row.barrel,
            magazine: row.magazine,
            perk_1: row.perk_1,
            perk_2: row.perk_2,
            origin_trait: row.origin_trait,
            rank: u8::try_from(row.rank).unwrap_or(0),
            tier: Tier {
                tier: row.tier,
                colour: Colour::new(u32::try_from(row.tier_colour).unwrap_or(0)),
            },
            notes: row.notes,
        }
    }
}

pub async fn all(pool: &PgPool) -> sqlx::Result<Vec<Weapon>> {
    let rows = sqlx::query_as!(
        EndgameWeaponRow,
        r#"SELECT
            icon,
            name,
            archetype,
            affinity AS "affinity?: Affinity",
            frame,
            enhanceable,
            reserves,
            barrel,
            magazine,
            perk_1,
            perk_2,
            origin_trait,
            rank,
            tier AS "tier!: TierLabel",
            tier_colour,
            notes
        FROM destiny2_endgame_weapons
        ORDER BY rank"#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(Weapon::from).collect())
}

pub async fn is_empty(pool: &PgPool) -> sqlx::Result<bool> {
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM destiny2_endgame_weapons)"#
    )
    .fetch_one(pool)
    .await?;

    Ok(!exists.unwrap_or(false))
}

pub async fn count(pool: &PgPool) -> sqlx::Result<i64> {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM destiny2_endgame_weapons"#
    )
    .fetch_one(pool)
    .await
}

#[must_use]
pub const fn is_safe_replace(existing: usize, incoming: usize) -> bool {
    if existing == 0 {
        return true;
    }

    incoming * 2 >= existing
}

pub async fn replace(pool: &PgPool, weapons: &[Weapon]) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;

    sqlx::query!("TRUNCATE destiny2_endgame_weapons RESTART IDENTITY")
        .execute(&mut *tx)
        .await?;

    for w in weapons {
        sqlx::query!(
            r#"INSERT INTO destiny2_endgame_weapons (
                name, archetype, affinity, frame, enhanceable, reserves,
                barrel, magazine, perk_1, perk_2, origin_trait, rank,
                tier, tier_colour, notes, icon
            )
            VALUES (
                $1, $2, $3, $4, $5, $6,
                $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16
            )"#,
            w.name,
            w.archetype,
            w.affinity as _,
            w.frame,
            w.enhanceable,
            w.reserves.map(i32::from),
            w.barrel,
            w.magazine,
            w.perk_1,
            w.perk_2,
            w.origin_trait,
            i16::from(w.rank),
            w.tier.tier as _,
            i32::try_from(w.tier.colour.0).unwrap_or(0),
            w.notes,
            w.icon,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}
