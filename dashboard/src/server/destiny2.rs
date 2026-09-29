use leptos::prelude::*;
use leptos::server_fn::codec::Json;
#[cfg(feature = "ssr")]
use {
    crate::dto::destiny2::{
        ArmourForm,
        AspectForm,
        CatalogWeaponInfo,
        LoadoutOptions,
        StatForm,
        WeaponForm,
    },
    crate::server::auth::{
        WebRole,
        app_state,
        db_pool,
        discord_client,
        require_role,
        server_err,
    },
    crate::server::error::LoadoutFormError,
    destiny2::db::loadout_catalog as catalog_db,
    destiny2::db::{loadout_writes, loadouts},
    destiny2::endgame_analysis::sheet::Affinity,
    destiny2::loadouts::{
        Archetype,
        ArmourSlot,
        Class,
        Element,
        LoadoutDraft,
        Mode,
        RawArmour,
        RawAspect,
        RawLoadout,
        RawWeapon,
        StatKind,
    },
    std::fmt::Display,
    std::str::FromStr,
    tracing::warn,
    twilight_model::id::Id,
};

use crate::dto::destiny2::{LoadoutCatalog, LoadoutForm, LoadoutSummary};

#[cfg(feature = "ssr")]
fn pick<T: FromStr<Err = ()>>(
    field: &'static str,
    value: &str,
) -> Result<T, LoadoutFormError> {
    value.parse().map_err(|()| LoadoutFormError::UnknownOption {
        field,
        value: value.to_owned(),
    })
}

#[cfg(feature = "ssr")]
fn strings<T: Display>(all: &[T]) -> Vec<String> {
    all.iter().map(ToString::to_string).collect()
}

#[cfg(feature = "ssr")]
pub fn raw_loadout(f: &LoadoutForm) -> Result<RawLoadout, LoadoutFormError> {
    let stats = f
        .stats
        .iter()
        .filter(|s| !s.stat.trim().is_empty() && !s.value.trim().is_empty())
        .map(|s| {
            let value =
                s.value.trim().parse::<i16>().map_err(|_parse_int_error| {
                    LoadoutFormError::StatValue(s.value.clone())
                })?;
            Ok((pick::<StatKind>("stat", &s.stat)?, value))
        })
        .collect::<Result<Vec<_>, LoadoutFormError>>()?;

    Ok(RawLoadout {
        name: f.name.clone(),
        class: pick::<Class>("class", &f.class)?,
        element: pick::<Element>("element", &f.element)?,
        mode: pick::<Mode>("mode", &f.mode)?,
        tags: f.tags.clone(),
        super_name: f.super_name.clone(),
        super_emoji: f.super_emoji.clone(),
        class_ability: f.class_ability.clone(),
        jump: f.jump.clone(),
        melee: f.melee.clone(),
        grenade: f.grenade.clone(),
        aspects: f
            .aspects
            .iter()
            .map(|a| RawAspect {
                aspect: a.aspect.clone(),
                fragments: a.fragments.clone(),
            })
            .collect(),
        weapons: f
            .weapons
            .iter()
            .map(|w| {
                Ok(RawWeapon {
                    name: w.name.clone(),
                    affinity: pick::<Affinity>("affinity", &w.affinity)?,
                    archetype: pick::<Archetype>("archetype", &w.archetype)?,
                    icon_url: w.icon_url.clone(),
                    perks: w.perks.clone(),
                })
            })
            .collect::<Result<_, LoadoutFormError>>()?,
        armour: f
            .armour
            .iter()
            .filter(|a| !a.name.trim().is_empty())
            .map(|a| {
                Ok(RawArmour {
                    slot: pick::<ArmourSlot>("armour slot", &a.slot)?,
                    name: a.name.clone(),
                    icon_url: a.icon_url.clone(),
                    mods: a.mods.clone(),
                })
            })
            .collect::<Result<_, LoadoutFormError>>()?,
        stats,
        artifact_name: f.artifact_name.clone(),
        artifact_perks: f.artifact_perks.clone(),
        author: f.author.clone(),
        dim_link: f.dim_link.clone(),
        video_url: f.video_url.clone(),
        how_it_works: f.how_it_works.clone(),
    })
}

#[cfg(feature = "ssr")]
pub fn loadout_form(id: Option<i32>, r: RawLoadout) -> LoadoutForm {
    LoadoutForm {
        id,
        name: r.name,
        class: r.class.to_string(),
        element: r.element.to_string(),
        mode: r.mode.to_string(),
        tags: r.tags,
        super_name: r.super_name,
        super_emoji: r.super_emoji,
        class_ability: r.class_ability,
        jump: r.jump,
        melee: r.melee,
        grenade: r.grenade,
        aspects: r
            .aspects
            .into_iter()
            .map(|a| AspectForm { aspect: a.aspect, fragments: a.fragments })
            .collect(),
        weapons: r
            .weapons
            .into_iter()
            .map(|w| WeaponForm {
                name: w.name,
                affinity: w.affinity.to_string(),
                archetype: w.archetype.to_string(),
                icon_url: w.icon_url,
                perks: w.perks,
            })
            .collect(),
        armour: r
            .armour
            .into_iter()
            .map(|a| ArmourForm {
                slot: a.slot.to_string(),
                name: a.name,
                icon_url: a.icon_url,
                mods: a.mods,
            })
            .collect(),
        stats: r
            .stats
            .into_iter()
            .map(|(s, v)| StatForm { stat: s.to_string(), value: v.to_string() })
            .collect(),
        artifact_name: r.artifact_name,
        artifact_perks: r.artifact_perks,
        author: r.author,
        dim_link: r.dim_link,
        video_url: r.video_url,
        how_it_works: r.how_it_works,
    }
}

#[cfg(feature = "ssr")]
pub fn editor_form(id: Option<i32>, r: RawLoadout) -> LoadoutForm {
    let mut form = loadout_form(id, r);

    let mut saved = std::mem::take(&mut form.armour);
    form.armour = ArmourSlot::ALL
        .iter()
        .map(|slot| {
            let slot = slot.to_string();
            saved.iter().position(|a| a.slot == slot).map_or_else(
                || ArmourForm { slot, ..ArmourForm::default() },
                |i| saved.swap_remove(i),
            )
        })
        .collect();

    for stat in StatKind::ALL.iter().map(ToString::to_string) {
        if !form.stats.iter().any(|s| s.stat == stat) {
            form.stats.push(StatForm { stat, value: String::new() });
        }
    }

    form
}

#[cfg(feature = "ssr")]
fn blank() -> LoadoutForm {
    LoadoutForm {
        class: Class::Hunter.to_string(),
        element: Element::Arc.to_string(),
        mode: Mode::All.to_string(),
        aspects: vec![AspectForm::default()],
        weapons: vec![WeaponForm {
            affinity: Affinity::Kinetic.to_string(),
            archetype: Archetype::AutoRifle.to_string(),
            ..WeaponForm::default()
        }],
        armour: ArmourSlot::ALL
            .iter()
            .map(|s| ArmourForm { slot: s.to_string(), ..ArmourForm::default() })
            .collect(),
        stats: StatKind::ALL
            .iter()
            .map(|s| StatForm { stat: s.to_string(), value: String::new() })
            .collect(),
        ..LoadoutForm::default()
    }
}

#[cfg(feature = "ssr")]
async fn zayden_emoji_names() -> Vec<String> {
    let (http, app) = match (discord_client(), app_state()) {
        (Ok(http), Ok(app)) => (http, app),
        (Err(e), _) | (_, Err(e)) => {
            warn!(error = %e, "cannot check Zayden's application emojis");
            return Vec::new();
        },
    };
    let Some(application) = Id::new_checked(app.zayden_id) else {
        warn!("cannot check Zayden's application emojis: zayden_id is 0");
        return Vec::new();
    };
    match http.get_application_emojis(application).await {
        Ok(response) => match response.model().await {
            Ok(list) => list.items.into_iter().map(|e| e.name).collect(),
            Err(e) => {
                warn!(error = %e, "could not decode Zayden's application emojis");
                Vec::new()
            },
        },
        Err(e) => {
            warn!(error = %e, "could not list Zayden's application emojis");
            Vec::new()
        },
    }
}

#[server]
pub async fn list_loadouts() -> Result<Vec<LoadoutSummary>, ServerFnError> {
    require_role(WebRole::Admin).await?;
    let pool = db_pool()?;

    let records = loadouts::all(&pool).await.map_err(server_err)?;

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

#[server]
pub async fn get_loadout(id: i32) -> Result<LoadoutForm, ServerFnError> {
    require_role(WebRole::Admin).await?;
    let pool = db_pool()?;

    let record = loadouts::by_id(&pool, id)
        .await
        .map_err(server_err)?
        .ok_or_else(|| ServerFnError::new(format!("loadout {id} does not exist")))?;

    Ok(editor_form(Some(id), RawLoadout::from(record)))
}

#[server(name = LoadoutCatalogFn)]
pub async fn loadout_catalog() -> Result<LoadoutCatalog, ServerFnError> {
    require_role(WebRole::Admin).await?;
    let pool = db_pool()?;

    let weapons = catalog_db::weapons(&pool).await.map_err(server_err)?;
    let perks = catalog_db::perks(&pool).await.map_err(server_err)?;
    let emoji_keys =
        catalog_db::emoji_keys_in_use(&pool).await.map_err(server_err)?;
    let known_emoji = zayden_emoji_names().await;

    Ok(LoadoutCatalog {
        weapons: weapons
            .into_iter()
            .map(|w| CatalogWeaponInfo {
                name: w.name,
                affinity: w.affinity.to_string(),
                archetype: w.archetype.to_string(),
                icon_url: w.icon_url,
            })
            .collect(),
        perks,
        emoji_keys,
        known_emoji,
        options: LoadoutOptions {
            classes: strings(&Class::ALL),
            elements: strings(&Element::ALL),
            modes: strings(&Mode::ALL),
            affinities: strings(&Affinity::ALL),
            archetypes: strings(&Archetype::ALL),
            stats: strings(&StatKind::ALL),
        },
        blank: blank(),
    })
}

#[server(input = Json)]
pub async fn save_loadout(form: LoadoutForm) -> Result<i32, ServerFnError> {
    require_role(WebRole::Admin).await?;

    let raw = raw_loadout(&form).map_err(server_err)?;
    let draft = LoadoutDraft::try_from(raw).map_err(server_err)?;

    let pool = db_pool()?;
    let mut tx = pool.begin().await.map_err(server_err)?;
    let id =
        loadout_writes::save(&mut tx, form.id, &draft).await.map_err(server_err)?;
    tx.commit().await.map_err(server_err)?;

    Ok(id)
}

#[server]
pub async fn delete_loadout(id: i32) -> Result<(), ServerFnError> {
    require_role(WebRole::Admin).await?;
    let pool = db_pool()?;

    if loadout_writes::delete(&pool, id).await.map_err(server_err)? {
        Ok(())
    } else {
        Err(ServerFnError::new(format!("loadout {id} does not exist")))
    }
}
