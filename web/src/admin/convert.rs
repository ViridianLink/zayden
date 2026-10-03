use std::fmt::Display;
use std::str::FromStr;

use destiny2::DraftError;
use destiny2::endgame_analysis::sheet::Affinity;
use destiny2::loadouts::{
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
    budget,
    limits,
};

use super::dto::{
    ArmourForm,
    AspectForm,
    LoadoutCheck,
    LoadoutForm,
    LoadoutOptions,
    StatForm,
    WeaponForm,
};
use super::error::LoadoutFormError;
use super::keys::enum_key;

fn pick<T: FromStr<Err = ()>>(
    field: &'static str,
    value: &str,
) -> Result<T, LoadoutFormError> {
    value.parse().map_err(|()| LoadoutFormError::UnknownOption {
        field,
        value: value.to_owned(),
    })
}

pub(super) fn strings<T: Display>(all: &[T]) -> Vec<String> {
    all.iter().map(ToString::to_string).collect()
}

/// Parses the editor's strings into the destiny2 domain types. Blank stat,
/// aspect, weapon and armour rows are dropped.
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

/// The editor's string form of a stored loadout.
#[must_use]
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

/// [`loadout_form`] with one row per armour slot, at least
/// [`limits::ASPECTS`] aspect rows, and every missing stat appended blank.
#[must_use]
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

/// The new-loadout form.
#[must_use]
pub fn blank() -> LoadoutForm {
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

/// The option labels each enum selector offers.
#[must_use]
pub fn options() -> LoadoutOptions {
    LoadoutOptions {
        classes: strings(&Class::ALL),
        elements: strings(&Element::ALL),
        modes: strings(&Mode::ALL),
        affinities: strings(&Affinity::ALL),
        archetypes: strings(&Archetype::ALL),
        stats: strings(&StatKind::ALL),
        armour_slots: strings(&ArmourSlot::ALL),
    }
}

/// Emoji names that illustrate an enum option and so may not name a new emoji.
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

pub(super) fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The budget meter and the first error a save of `form` would report.
#[must_use]
pub fn loadout_check(form: &LoadoutForm) -> LoadoutCheck {
    let armour = form.armour.iter().filter(|a| !a.name.trim().is_empty()).count();
    let weapons = form.weapons.iter().filter(|w| !w.name.trim().is_empty()).count();
    let misc = form.stats.iter().any(|s| !s.value.trim().is_empty())
        || !form.artifact_perks.is_empty()
        || !form.how_it_works.trim().is_empty();
    let components = budget::components(form.tags.len(), weapons, armour, misc);
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

    LoadoutCheck {
        components: count(components),
        max_components: count(budget::MAX_COMPONENTS),
        text: text.map(count),
        max_text: count(budget::MAX_TEXT),
        error,
    }
}
