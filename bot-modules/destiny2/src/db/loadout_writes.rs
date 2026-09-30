use sqlx::{PgConnection, PgPool};

use crate::error::SaveError;
use crate::loadouts::{EmojiKey, LoadoutDraft};

fn names(keys: &[EmojiKey]) -> Vec<String> {
    keys.iter().map(|k| k.as_str().to_owned()).collect()
}

pub async fn save(
    conn: &mut PgConnection,
    id: Option<i32>,
    d: &LoadoutDraft,
) -> Result<i32, SaveError> {
    let id = match id {
        Some(id) => sqlx::query_scalar!(
            "UPDATE destiny2_loadouts SET
                name = $2, class = $3, element = $4, mode = $5, super_name = $6,
                super_emoji = $7, class_ability = $8, jump = $9, melee = $10,
                grenade = $11, artifact_name = $12, author = $13, dim_link = $14,
                video_url = $15, how_it_works = $16
            WHERE id = $1
            RETURNING id",
            id,
            d.name,
            d.class as _,
            d.element as _,
            d.mode as _,
            d.super_name,
            d.super_emoji.as_str(),
            d.class_ability.as_str(),
            d.jump.as_str(),
            d.melee.as_str(),
            d.grenade.as_str(),
            d.artifact_name,
            d.author,
            d.dim_link,
            d.video_url,
            d.how_it_works,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(SaveError::NotFound(id))?,
        None => sqlx::query_scalar!(
            "INSERT INTO destiny2_loadouts (
                name, class, element, mode, super_name, super_emoji, class_ability,
                jump, melee, grenade, artifact_name, author, dim_link, video_url,
                how_it_works
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING id",
            d.name,
            d.class as _,
            d.element as _,
            d.mode as _,
            d.super_name,
            d.super_emoji.as_str(),
            d.class_ability.as_str(),
            d.jump.as_str(),
            d.melee.as_str(),
            d.grenade.as_str(),
            d.artifact_name,
            d.author,
            d.dim_link,
            d.video_url,
            d.how_it_works,
        )
        .fetch_one(&mut *conn)
        .await?,
    };

    // Grandchild rows (fragments, weapon perks, armour mods) cascade.
    sqlx::query!("DELETE FROM destiny2_loadout_aspects WHERE loadout_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM destiny2_loadout_weapons WHERE loadout_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM destiny2_loadout_armour WHERE loadout_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM destiny2_loadout_stats WHERE loadout_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!("DELETE FROM destiny2_loadout_tags WHERE loadout_id = $1", id)
        .execute(&mut *conn)
        .await?;
    sqlx::query!(
        "DELETE FROM destiny2_loadout_artifact_perks WHERE loadout_id = $1",
        id
    )
    .execute(&mut *conn)
    .await?;

    for (ordinal, aspect) in (0_i16..).zip(&d.aspects) {
        let aspect_id = sqlx::query_scalar!(
            "INSERT INTO destiny2_loadout_aspects (loadout_id, ordinal, aspect_emoji)
             VALUES ($1, $2, $3) RETURNING id",
            id,
            ordinal,
            aspect.aspect.as_str(),
        )
        .fetch_one(&mut *conn)
        .await?;
        sqlx::query!(
            "INSERT INTO destiny2_loadout_aspect_fragments (aspect_id, ordinal, fragment_emoji)
             SELECT $1, (t.ord - 1)::smallint, t.key
             FROM UNNEST($2::text[]) WITH ORDINALITY AS t(key, ord)",
            aspect_id,
            &names(&aspect.fragments),
        )
        .execute(&mut *conn)
        .await?;
    }

    for (slot, weapon) in (0_i16..).zip(&d.weapons) {
        let weapon_id = sqlx::query_scalar!(
            "INSERT INTO destiny2_weapons (name, affinity, archetype, icon_url)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (name) DO UPDATE SET
                archetype = EXCLUDED.archetype,
                icon_url = EXCLUDED.icon_url
             RETURNING id",
            weapon.name,
            weapon.affinity as _,
            weapon.archetype as _,
            weapon.icon_url,
        )
        .fetch_one(&mut *conn)
        .await?;
        let loadout_weapon_id = sqlx::query_scalar!(
            "INSERT INTO destiny2_loadout_weapons (loadout_id, slot_ordinal, weapon_id, affinity)
             VALUES ($1, $2, $3, $4) RETURNING id",
            id,
            slot,
            weapon_id,
            weapon.affinity as _,
        )
        .fetch_one(&mut *conn)
        .await?;
        let perks = names(&weapon.perks);
        sqlx::query!(
            "INSERT INTO destiny2_perks (name) SELECT UNNEST($1::text[]) ON CONFLICT (name) DO NOTHING",
            &perks,
        )
        .execute(&mut *conn)
        .await?;
        sqlx::query!(
            "INSERT INTO destiny2_loadout_weapon_perks (loadout_weapon_id, ordinal, perk_id)
             SELECT $1, (t.ord - 1)::smallint, p.id
             FROM UNNEST($2::text[]) WITH ORDINALITY AS t(name, ord)
             JOIN destiny2_perks p ON p.name = t.name",
            loadout_weapon_id,
            &perks,
        )
        .execute(&mut *conn)
        .await?;
    }

    for armour in &d.armour {
        let armour_id = sqlx::query_scalar!(
            "INSERT INTO destiny2_loadout_armour (loadout_id, slot, name, icon_url)
             VALUES ($1, $2, $3, $4) RETURNING id",
            id,
            armour.slot as _,
            armour.name,
            armour.icon_url,
        )
        .fetch_one(&mut *conn)
        .await?;
        sqlx::query!(
            "INSERT INTO destiny2_loadout_armour_mods (armour_id, ordinal, mod_emoji)
             SELECT $1, (t.ord - 1)::smallint, t.key
             FROM UNNEST($2::text[]) WITH ORDINALITY AS t(key, ord)",
            armour_id,
            &names(&armour.mods),
        )
        .execute(&mut *conn)
        .await?;
    }

    for (ordinal, (stat, value)) in (0_i16..).zip(&d.stats) {
        sqlx::query!(
            "INSERT INTO destiny2_loadout_stats (loadout_id, ordinal, stat, value) VALUES ($1, $2, $3, $4)",
            id,
            ordinal,
            *stat as _,
            *value,
        )
        .execute(&mut *conn)
        .await?;
    }

    sqlx::query!(
        "INSERT INTO destiny2_loadout_tags (loadout_id, ordinal, tag)
         SELECT $1, (t.ord - 1)::smallint, t.tag FROM UNNEST($2::text[]) WITH ORDINALITY AS t(tag, ord)",
        id,
        &d.tags,
    )
    .execute(&mut *conn)
    .await?;

    sqlx::query!(
        "INSERT INTO destiny2_loadout_artifact_perks (loadout_id, ordinal, perk_emoji)
         SELECT $1, (t.ord - 1)::smallint, t.key FROM UNNEST($2::text[]) WITH ORDINALITY AS t(key, ord)",
        id,
        &names(&d.artifact_perks),
    )
    .execute(&mut *conn)
    .await?;

    Ok(id)
}

pub async fn delete(pool: &PgPool, id: i32) -> sqlx::Result<bool> {
    let result = sqlx::query!("DELETE FROM destiny2_loadouts WHERE id = $1", id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
