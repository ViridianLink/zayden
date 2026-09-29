use sqlx::PgPool;

use crate::endgame_analysis::sheet::Affinity;
use crate::loadouts::Archetype;

#[derive(Debug, Clone)]
pub struct CatalogWeapon {
    pub name: String,
    pub affinity: Affinity,
    pub archetype: Archetype,
    pub icon_url: String,
}

pub async fn weapons(pool: &PgPool) -> sqlx::Result<Vec<CatalogWeapon>> {
    sqlx::query_as!(
        CatalogWeapon,
        r#"SELECT name, affinity AS "affinity!: Affinity", archetype AS "archetype!: Archetype", icon_url
           FROM destiny2_weapons ORDER BY name"#
    )
    .fetch_all(pool)
    .await
}

pub async fn perks(pool: &PgPool) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!("SELECT name FROM destiny2_perks ORDER BY name")
        .fetch_all(pool)
        .await
}

pub async fn emoji_keys_in_use(pool: &PgPool) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!(
        r#"SELECT key AS "key!" FROM (
               SELECT super_emoji AS key FROM destiny2_loadouts
               UNION SELECT class_ability FROM destiny2_loadouts
               UNION SELECT jump FROM destiny2_loadouts
               UNION SELECT melee FROM destiny2_loadouts
               UNION SELECT grenade FROM destiny2_loadouts
               UNION SELECT aspect_emoji FROM destiny2_loadout_aspects
               UNION SELECT fragment_emoji FROM destiny2_loadout_aspect_fragments
               UNION SELECT mod_emoji FROM destiny2_loadout_armour_mods
               UNION SELECT perk_emoji FROM destiny2_loadout_artifact_perks
               UNION SELECT name FROM destiny2_perks
           ) keys ORDER BY key"#
    )
    .fetch_all(pool)
    .await
}
