use destiny2::db::{loadout_catalog as catalog_db, loadout_writes, loadouts};
use destiny2::loadouts::{LoadoutDraft, RawLoadout};
use sqlx::PgPool;
use topcoat::context::Cx;

use super::convert::{blank, editor_form, loadout_check, options, raw_loadout};
use super::dto::{
    ArmourPieceInfo,
    CatalogWeaponInfo,
    EmojiInfo,
    LoadoutCatalog,
    LoadoutCheck,
    LoadoutForm,
    LoadoutSummary,
    UsageInfo,
};
use super::emoji::zayden_emojis;
use super::error::AdminError;
use crate::auth::{WebRole, db_pool, require_role};

pub async fn list_loadouts(cx: &Cx) -> Result<Vec<LoadoutSummary>, AdminError> {
    require_role(cx, WebRole::Admin).await?;
    summaries(db_pool(cx)?).await
}

pub async fn summaries(pool: &PgPool) -> Result<Vec<LoadoutSummary>, AdminError> {
    let records = loadouts::all(pool).await?;

    Ok(records
        .into_iter()
        .map(|r| LoadoutSummary {
            id: r.id,
            name: r.name,
            class: r.class.to_string(),
            element: r.element.to_string(),
            mode: r.mode.to_string(),
            author: r.author,
            tags: r.tags,
        })
        .collect())
}

pub async fn get_loadout(cx: &Cx, id: i32) -> Result<LoadoutForm, AdminError> {
    require_role(cx, WebRole::Admin).await?;
    stored_form(db_pool(cx)?, id).await
}

pub async fn stored_form(pool: &PgPool, id: i32) -> Result<LoadoutForm, AdminError> {
    let record =
        loadouts::by_id(pool, id).await?.ok_or(AdminError::LoadoutNotFound(id))?;

    Ok(editor_form(Some(id), RawLoadout::from(record)))
}

pub async fn loadout_catalog(cx: &Cx) -> Result<LoadoutCatalog, AdminError> {
    require_role(cx, WebRole::Admin).await?;
    catalog(db_pool(cx)?, zayden_emojis(cx)).await
}

pub async fn catalog(
    pool: &PgPool,
    emojis: impl Future<Output = Vec<EmojiInfo>>,
) -> Result<LoadoutCatalog, AdminError> {
    let weapons = catalog_db::weapons(pool).await?;
    let perks = catalog_db::perks(pool).await?;
    let usage = catalog_db::usage(pool).await?;
    let super_names = catalog_db::super_names(pool).await?;
    let known = catalog_db::weapon_known_perks(pool).await?;
    let armour = catalog_db::armour_pieces(pool).await?;
    let emojis = emojis.await;

    Ok(LoadoutCatalog {
        weapons: weapons
            .into_iter()
            .map(|w| CatalogWeaponInfo {
                known_perks: known
                    .iter()
                    .filter(|(weapon, _)| *weapon == w.name)
                    .map(|(_, perk)| perk.clone())
                    .collect(),
                name: w.name,
                affinity: w.affinity.to_string(),
                archetype: w.archetype.to_string(),
                icon_url: w.icon_url,
            })
            .collect(),
        perks,
        emojis,
        usage: usage
            .into_iter()
            .map(|u| UsageInfo {
                field: u.field,
                key: u.key,
                class: u.class.to_string(),
                element: u.element.to_string(),
                uses: u32::try_from(u.uses).unwrap_or(u32::MAX),
            })
            .collect(),
        super_names,
        armour: armour
            .into_iter()
            .map(|a| ArmourPieceInfo {
                slot: a.slot.to_string(),
                class: a.class.to_string(),
                name: a.name,
                icon_url: a.icon_url,
            })
            .collect(),
        options: options(),
        blank: blank(),
    })
}

pub async fn check_loadout(
    cx: &Cx,
    form: &LoadoutForm,
) -> Result<LoadoutCheck, AdminError> {
    require_role(cx, WebRole::Admin).await?;
    Ok(loadout_check(form))
}

pub async fn save_loadout(cx: &Cx, form: &LoadoutForm) -> Result<i32, AdminError> {
    require_role(cx, WebRole::Admin).await?;

    let draft = draft(form)?;
    write_unchecked(db_pool(cx)?, form.id, &draft).await
}

pub fn draft(form: &LoadoutForm) -> Result<LoadoutDraft, AdminError> {
    let raw = raw_loadout(form)?;
    Ok(LoadoutDraft::try_from(raw)?)
}

pub async fn write_unchecked(
    pool: &PgPool,
    id: Option<i32>,
    draft: &LoadoutDraft,
) -> Result<i32, AdminError> {
    let mut tx = pool.begin().await?;
    let id = loadout_writes::save(&mut tx, id, draft).await?;
    tx.commit().await?;

    Ok(id)
}

pub async fn delete_loadout(cx: &Cx, id: i32) -> Result<(), AdminError> {
    require_role(cx, WebRole::Admin).await?;
    remove_unchecked(db_pool(cx)?, id).await
}

pub async fn remove_unchecked(pool: &PgPool, id: i32) -> Result<(), AdminError> {
    if loadout_writes::delete(pool, id).await? {
        Ok(())
    } else {
        Err(AdminError::LoadoutNotFound(id))
    }
}
