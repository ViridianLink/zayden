use leptos::prelude::*;
use leptos::server_fn::codec::Json;
#[cfg(feature = "ssr")]
use {
    crate::dto::destiny2::{
        ArmourForm,
        ArmourPieceInfo,
        AspectForm,
        CatalogWeaponInfo,
        LoadoutOptions,
        StatForm,
        UsageInfo,
        WeaponForm,
    },
    crate::dto::destiny2_keys::{enum_key, is_valid_key},
    crate::server::auth::{
        WebRole,
        app_state,
        db_pool,
        discord_client,
        require_role,
        server_err,
    },
    crate::server::emoji_upload,
    crate::server::error::{EmojiUploadError, LoadoutFormError},
    destiny2::DraftError,
    destiny2::db::loadout_catalog as catalog_db,
    destiny2::db::{loadout_writes, loadouts},
    destiny2::endgame_analysis::sheet::Affinity,
    destiny2::loadouts::budget,
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
        limits,
    },
    std::fmt::Display,
    std::str::FromStr,
    tracing::warn,
    twilight_model::id::Id,
};

use crate::dto::destiny2::{
    EmojiInfo,
    EmojiSource,
    LoadoutCatalog,
    LoadoutCheck,
    LoadoutForm,
    LoadoutSummary,
};

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
            .filter(|a| !a.aspect.trim().is_empty())
            .map(|a| RawAspect {
                aspect: a.aspect.clone(),
                fragments: a.fragments.clone(),
            })
            .collect(),
        weapons: f
            .weapons
            .iter()
            .filter(|w| !w.name.trim().is_empty())
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

    form.aspects
        .resize_with(form.aspects.len().max(limits::ASPECTS), AspectForm::default);

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
        aspects: vec![AspectForm::default(); limits::ASPECTS],
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
pub(crate) async fn zayden_emojis() -> Vec<EmojiInfo> {
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
            Ok(list) => list
                .items
                .into_iter()
                .map(|e| EmojiInfo { name: e.name, id: e.id.to_string() })
                .collect(),
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

#[cfg(feature = "ssr")]
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(feature = "ssr")]
#[must_use]
pub fn loadout_check(form: &LoadoutForm) -> LoadoutCheck {
    let armour = form.armour.iter().filter(|a| !a.name.trim().is_empty()).count();
    let weapons = form.weapons.iter().filter(|w| !w.name.trim().is_empty()).count();
    let components = budget::components(form.tags.len(), weapons, armour);
    let (text, error) = match raw_loadout(form) {
        Err(e) => (None, Some(e.to_string())),
        Ok(raw) => match LoadoutDraft::try_from(raw) {
            Ok(draft) => (Some(budget::text(&draft)), None),
            Err(e @ DraftError::TooMuchText { estimate, .. }) => {
                (Some(estimate), Some(e.to_string()))
            },
            Err(e) => (None, Some(e.to_string())),
        },
    };
    let warning = (components > budget::MAX_COMPONENTS).then(|| {
        DraftError::TooManyComponents {
            needed: components,
            max: budget::MAX_COMPONENTS,
        }
        .to_string()
    });

    LoadoutCheck {
        components: count(components),
        max_components: count(budget::MAX_COMPONENTS),
        text: text.map(count),
        max_text: count(budget::MAX_TEXT),
        error,
        warning,
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
    let usage = catalog_db::usage(&pool).await.map_err(server_err)?;
    let super_names = catalog_db::super_names(&pool).await.map_err(server_err)?;
    let known = catalog_db::weapon_known_perks(&pool).await.map_err(server_err)?;
    let armour = catalog_db::armour_pieces(&pool).await.map_err(server_err)?;
    let emojis = zayden_emojis().await;

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
        options: LoadoutOptions {
            classes: strings(&Class::ALL),
            elements: strings(&Element::ALL),
            modes: strings(&Mode::ALL),
            affinities: strings(&Affinity::ALL),
            archetypes: strings(&Archetype::ALL),
            stats: strings(&StatKind::ALL),
            armour_slots: strings(&ArmourSlot::ALL),
        },
        blank: blank(),
    })
}

#[server(input = Json)]
pub async fn check_loadout(
    form: LoadoutForm,
) -> Result<LoadoutCheck, ServerFnError> {
    require_role(WebRole::Admin).await?;
    Ok(loadout_check(&form))
}

#[cfg(feature = "ssr")]
#[must_use]
pub fn reserved_keys() -> Vec<String> {
    [
        strings(&Class::ALL),
        strings(&Element::ALL),
        strings(&Mode::ALL),
        strings(&Affinity::ALL),
        strings(&Archetype::ALL),
        strings(&StatKind::ALL),
        strings(&ArmourSlot::ALL),
    ]
    .concat()
    .iter()
    .map(|label| enum_key(label))
    .collect()
}

#[cfg(feature = "ssr")]
async fn emoji_image(source: EmojiSource) -> Result<String, EmojiUploadError> {
    let bytes = match source {
        EmojiSource::Url(raw) => {
            emoji_upload::fetch(&emoji_upload::https_url(&raw)?).await?
        },
        EmojiSource::DataUri(uri) => emoji_upload::decode_data_uri(&uri)?,
    };
    emoji_upload::data_uri(&bytes)
}

#[server(input = Json)]
pub async fn create_zayden_emoji(
    name: String,
    source: EmojiSource,
) -> Result<EmojiInfo, ServerFnError> {
    require_role(WebRole::Admin).await?;

    if !is_valid_key(&name) {
        return Err(server_err(EmojiUploadError::InvalidName));
    }
    if reserved_keys().contains(&name) {
        return Err(server_err(EmojiUploadError::ReservedName(name)));
    }
    if zayden_emojis().await.iter().any(|e| e.name == name) {
        return Err(server_err(EmojiUploadError::NameTaken(name)));
    }
    let image = emoji_image(source).await.map_err(server_err)?;

    let http = discord_client()?;
    let application = Id::new_checked(app_state()?.zayden_id)
        .ok_or_else(|| ServerFnError::new("zayden_id is not configured"))?;
    let emoji = http
        .add_application_emoji(application, &name, &image)
        .await
        .map_err(|e| server_err(EmojiUploadError::Discord(e.to_string())))?
        .model()
        .await
        .map_err(|e| server_err(EmojiUploadError::Discord(e.to_string())))?;

    Ok(EmojiInfo { name: emoji.name, id: emoji.id.to_string() })
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
