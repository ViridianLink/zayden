use sqlx::PgPool;

use crate::endgame_analysis::sheet::Affinity;
use crate::loadouts::{Archetype, ArmourSlot, Class, Element};

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

#[derive(Debug, Clone)]
pub struct UsageRow {
    pub field: String,
    pub key: String,
    pub class: Class,
    pub element: Element,
    pub uses: i64,
}

pub async fn usage(pool: &PgPool) -> sqlx::Result<Vec<UsageRow>> {
    sqlx::query_as!(
        UsageRow,
        r#"SELECT u.field AS "field!", u.key AS "key!",
                  l.class AS "class!: Class", l.element AS "element!: Element",
                  COUNT(*) AS "uses!"
           FROM (
               SELECT id AS loadout_id, 'super' AS field, super_emoji AS key FROM destiny2_loadouts
               UNION ALL SELECT id, 'class_ability', class_ability FROM destiny2_loadouts
               UNION ALL SELECT id, 'jump', jump FROM destiny2_loadouts
               UNION ALL SELECT id, 'melee', melee FROM destiny2_loadouts
               UNION ALL SELECT id, 'grenade', grenade FROM destiny2_loadouts
               UNION ALL SELECT id, 'artifact', COALESCE(artifact_name, '') FROM destiny2_loadouts
               UNION ALL SELECT loadout_id, 'aspect', aspect_emoji FROM destiny2_loadout_aspects
               UNION ALL SELECT a.loadout_id, 'fragment', f.fragment_emoji
                   FROM destiny2_loadout_aspect_fragments f
                   JOIN destiny2_loadout_aspects a ON a.id = f.aspect_id
               UNION ALL SELECT lw.loadout_id, 'weapon_perk', p.name
                   FROM destiny2_loadout_weapon_perks wp
                   JOIN destiny2_loadout_weapons lw ON lw.id = wp.loadout_weapon_id
                   JOIN destiny2_perks p ON p.id = wp.perk_id
               UNION ALL SELECT lw.loadout_id, 'weapon', w.name
                   FROM destiny2_loadout_weapons lw
                   JOIN destiny2_weapons w ON w.id = lw.weapon_id
               UNION ALL SELECT ar.loadout_id, 'armour_mod', m.mod_emoji
                   FROM destiny2_loadout_armour_mods m
                   JOIN destiny2_loadout_armour ar ON ar.id = m.armour_id
               UNION ALL SELECT loadout_id, 'artifact_perk', perk_emoji
                   FROM destiny2_loadout_artifact_perks
           ) u
           JOIN destiny2_loadouts l ON l.id = u.loadout_id
           WHERE u.key <> ''
           GROUP BY u.field, u.key, l.class, l.element
           ORDER BY u.field, u.key"#
    )
    .fetch_all(pool)
    .await
}

pub async fn super_names(pool: &PgPool) -> sqlx::Result<Vec<(String, String)>> {
    let rows = sqlx::query!(
        "SELECT DISTINCT ON (super_emoji) super_emoji, super_name
         FROM destiny2_loadouts
         WHERE super_emoji <> ''
         ORDER BY super_emoji, id DESC"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| (r.super_emoji, r.super_name)).collect())
}

pub async fn weapon_known_perks(
    pool: &PgPool,
) -> sqlx::Result<Vec<(String, String)>> {
    let rows = sqlx::query!(
        r#"SELECT w.name AS "weapon!", p.name AS "perk!"
           FROM destiny2_weapon_perks wp
           JOIN destiny2_weapons w ON w.id = wp.weapon_id
           JOIN destiny2_perks p ON p.id = wp.perk_id
           UNION
           SELECT w.name, p.name
           FROM destiny2_loadout_weapon_perks lwp
           JOIN destiny2_loadout_weapons lw ON lw.id = lwp.loadout_weapon_id
           JOIN destiny2_weapons w ON w.id = lw.weapon_id
           JOIN destiny2_perks p ON p.id = lwp.perk_id
           ORDER BY 1, 2"#
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| (r.weapon, r.perk)).collect())
}

#[derive(Debug, Clone)]
pub struct ArmourPiece {
    pub slot: ArmourSlot,
    pub class: Class,
    pub name: String,
    pub icon_url: String,
}

pub async fn armour_pieces(pool: &PgPool) -> sqlx::Result<Vec<ArmourPiece>> {
    sqlx::query_as!(
        ArmourPiece,
        r#"SELECT DISTINCT ON (ar.slot, l.class, ar.name)
                  ar.slot AS "slot!: ArmourSlot", l.class AS "class!: Class",
                  ar.name, ar.icon_url
           FROM destiny2_loadout_armour ar
           JOIN destiny2_loadouts l ON l.id = ar.loadout_id
           WHERE ar.name <> ''
           ORDER BY ar.slot, l.class, ar.name, ar.id DESC"#
    )
    .fetch_all(pool)
    .await
}
