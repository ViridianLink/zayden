use std::collections::HashMap;

use itertools::Itertools;
use sqlx::PgPool;

use crate::endgame_analysis::sheet::Affinity;
use crate::loadouts::domain::{Archetype, ArmourSlot, Class, Element, StatKind};
use crate::loadouts::mode::Mode;
use crate::loadouts::record::{
    ArmourRecord,
    AspectRecord,
    LoadoutRecord,
    WeaponRecord,
};

struct LoadoutBase {
    id: i32,
    name: String,
    class: Class,
    element: Element,
    mode: Mode,
    super_name: String,
    super_emoji: String,
    class_ability: String,
    jump: String,
    melee: String,
    grenade: String,
    artifact_name: Option<String>,
    author: String,
    dim_link: String,
    video_url: Option<String>,
    how_it_works: Option<String>,
}

pub async fn all(pool: &PgPool) -> sqlx::Result<Vec<LoadoutRecord>> {
    let bases = sqlx::query_as!(
        LoadoutBase,
        r#"SELECT
            id,
            name,
            class AS "class!: Class",
            element AS "element!: Element",
            mode AS "mode!: Mode",
            super_name,
            super_emoji,
            class_ability,
            jump,
            melee,
            grenade,
            artifact_name,
            author,
            dim_link,
            video_url,
            how_it_works
        FROM destiny2_loadouts
        ORDER BY class, element, name"#
    )
    .fetch_all(pool)
    .await?;

    let fragment_rows = sqlx::query!(
        "SELECT aspect_id, fragment_emoji
        FROM destiny2_loadout_aspect_fragments
        ORDER BY aspect_id, ordinal"
    )
    .fetch_all(pool)
    .await?;
    let mut fragments_by_aspect: HashMap<i32, Vec<String>> = fragment_rows
        .into_iter()
        .map(|row| (row.aspect_id, row.fragment_emoji))
        .into_group_map();

    let aspect_rows = sqlx::query!(
        "SELECT id, loadout_id, aspect_emoji
        FROM destiny2_loadout_aspects
        ORDER BY loadout_id, ordinal"
    )
    .fetch_all(pool)
    .await?;
    let mut aspects_by_loadout: HashMap<i32, Vec<AspectRecord>> = aspect_rows
        .into_iter()
        .map(|row| {
            (row.loadout_id, AspectRecord {
                emoji: row.aspect_emoji,
                fragments: fragments_by_aspect.remove(&row.id).unwrap_or_default(),
            })
        })
        .into_group_map();

    let weapon_perk_rows = sqlx::query!(
        "SELECT lwp.loadout_weapon_id, p.name AS perk
        FROM destiny2_loadout_weapon_perks lwp
        JOIN destiny2_perks p ON p.id = lwp.perk_id
        ORDER BY lwp.loadout_weapon_id, lwp.ordinal"
    )
    .fetch_all(pool)
    .await?;
    let mut perks_by_weapon: HashMap<i32, Vec<String>> = weapon_perk_rows
        .into_iter()
        .map(|row| (row.loadout_weapon_id, row.perk))
        .into_group_map();

    let weapon_rows = sqlx::query!(
        r#"SELECT
            lw.id,
            lw.loadout_id,
            w.name,
            w.affinity AS "affinity!: Affinity",
            w.archetype AS "archetype!: Archetype",
            w.icon_url
        FROM destiny2_loadout_weapons lw
        JOIN destiny2_weapons w ON w.id = lw.weapon_id
        ORDER BY lw.loadout_id, lw.slot_ordinal"#
    )
    .fetch_all(pool)
    .await?;
    let mut weapons_by_loadout: HashMap<i32, Vec<WeaponRecord>> = weapon_rows
        .into_iter()
        .map(|row| {
            (row.loadout_id, WeaponRecord {
                name: row.name,
                affinity: row.affinity,
                archetype: row.archetype,
                icon_url: row.icon_url,
                perks: perks_by_weapon.remove(&row.id).unwrap_or_default(),
            })
        })
        .into_group_map();

    let armour_mod_rows = sqlx::query!(
        "SELECT armour_id, mod_emoji
        FROM destiny2_loadout_armour_mods
        ORDER BY armour_id, ordinal"
    )
    .fetch_all(pool)
    .await?;
    let mut mods_by_armour: HashMap<i32, Vec<String>> = armour_mod_rows
        .into_iter()
        .map(|row| (row.armour_id, row.mod_emoji))
        .into_group_map();

    let armour_rows = sqlx::query!(
        r#"SELECT
            id,
            loadout_id,
            slot AS "slot!: ArmourSlot",
            name,
            icon_url
        FROM destiny2_loadout_armour
        ORDER BY loadout_id, slot"#
    )
    .fetch_all(pool)
    .await?;
    let mut armour_by_loadout: HashMap<i32, Vec<ArmourRecord>> = armour_rows
        .into_iter()
        .map(|row| {
            (row.loadout_id, ArmourRecord {
                slot: row.slot,
                name: row.name,
                icon_url: row.icon_url,
                mods: mods_by_armour.remove(&row.id).unwrap_or_default(),
            })
        })
        .into_group_map();

    let stat_rows = sqlx::query!(
        r#"SELECT loadout_id, stat AS "stat!: StatKind", value
        FROM destiny2_loadout_stats
        ORDER BY loadout_id, ordinal"#
    )
    .fetch_all(pool)
    .await?;
    let mut stats_by_loadout: HashMap<i32, Vec<(StatKind, i16)>> = stat_rows
        .into_iter()
        .map(|row| (row.loadout_id, (row.stat, row.value)))
        .into_group_map();

    let tag_rows = sqlx::query!(
        "SELECT loadout_id, tag
        FROM destiny2_loadout_tags
        ORDER BY loadout_id, ordinal"
    )
    .fetch_all(pool)
    .await?;
    let mut tags_by_loadout: HashMap<i32, Vec<String>> =
        tag_rows.into_iter().map(|row| (row.loadout_id, row.tag)).into_group_map();

    let artifact_rows = sqlx::query!(
        "SELECT loadout_id, perk_emoji
        FROM destiny2_loadout_artifact_perks
        ORDER BY loadout_id, ordinal"
    )
    .fetch_all(pool)
    .await?;
    let mut artifact_by_loadout: HashMap<i32, Vec<String>> = artifact_rows
        .into_iter()
        .map(|row| (row.loadout_id, row.perk_emoji))
        .into_group_map();

    let records = bases
        .into_iter()
        .map(|base| LoadoutRecord {
            aspects: aspects_by_loadout.remove(&base.id).unwrap_or_default(),
            weapons: weapons_by_loadout.remove(&base.id).unwrap_or_default(),
            armour: armour_by_loadout.remove(&base.id).unwrap_or_default(),
            stats: stats_by_loadout.remove(&base.id).unwrap_or_default(),
            tags: tags_by_loadout.remove(&base.id).unwrap_or_default(),
            artifact_perks: artifact_by_loadout.remove(&base.id).unwrap_or_default(),
            id: base.id,
            name: base.name,
            class: base.class,
            element: base.element,
            mode: base.mode,
            super_name: base.super_name,
            super_emoji: base.super_emoji,
            class_ability: base.class_ability,
            jump: base.jump,
            melee: base.melee,
            grenade: base.grenade,
            artifact_name: base.artifact_name,
            author: base.author,
            dim_link: base.dim_link,
            video_url: base.video_url,
            how_it_works: base.how_it_works,
        })
        .collect();

    Ok(records)
}

pub async fn is_empty(pool: &PgPool) -> sqlx::Result<bool> {
    let exists =
        sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM destiny2_loadouts)")
            .fetch_one(pool)
            .await?;

    Ok(!exists.unwrap_or(false))
}
