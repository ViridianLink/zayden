use std::fmt::Write as _;

use jiff::civil::Weekday;
use serenity::all::{
    Colour,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateMediaGallery,
    CreateMediaGalleryItem,
    CreateSection,
    CreateSectionAccessory,
    CreateSectionComponent,
    CreateSeparator,
    CreateTextDisplay,
    CreateThumbnail,
    CreateUnfurledMediaItem,
};

use crate::model::{
    Attachment,
    BuildRecipe,
    Cradle,
    Faction,
    MapStatus,
    MarathonMap,
    MetaEntry,
    NewsItem,
    RotationWindow,
    Runner,
    Schedule,
    Weapon,
};

const ACCENT: Colour = Colour::BLURPLE;

fn separator() -> CreateContainerComponent<'static> {
    CreateContainerComponent::Separator(CreateSeparator::new().divider(true))
}

fn text(content: impl Into<String>) -> CreateContainerComponent<'static> {
    CreateContainerComponent::TextDisplay(CreateTextDisplay::new(content.into()))
}

fn body_component(
    content: String,
    thumbnail_url: Option<&str>,
) -> CreateContainerComponent<'static> {
    match thumbnail_url {
        Some(url) => CreateContainerComponent::Section(CreateSection::new(
            vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(
                content,
            ))],
            CreateSectionAccessory::Thumbnail(CreateThumbnail::new(
                CreateUnfurledMediaItem::new(url.to_string()),
            )),
        )),
        None => text(content),
    }
}

fn labelled_list(
    title: &str,
    lines: &[String],
) -> CreateContainerComponent<'static> {
    if lines.is_empty() {
        return text(format!("### {title}\n*Data unavailable.*"));
    }
    text(format!("### {title}\n{}", lines.join("\n")))
}

fn named_line(name: &str, detail: Option<&str>) -> String {
    detail.map_or_else(
        || format!("**{name}**"),
        |detail| format!("**{name}** — {detail}"),
    )
}

const fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
        Weekday::Sunday => "Sunday",
    }
}

pub fn weapon_component(weapon: &Weapon) -> CreateComponent<'static> {
    let mut body = format!("# {}", weapon.name);

    let subtitle: Vec<&str> =
        [weapon.weapon_type.as_deref(), weapon.ammo_type.as_deref()]
            .into_iter()
            .flatten()
            .collect();
    if !subtitle.is_empty() {
        let _ = write!(body, "\n-# {}", subtitle.join(" • "));
    }

    for (label, value) in [
        ("Damage", &weapon.damage),
        ("Fire Rate", &weapon.fire_rate),
        ("Magazine", &weapon.magazine_size),
        ("Reload", &weapon.reload_speed),
        ("Range", &weapon.range),
    ] {
        if let Some(value) = value {
            let _ = write!(body, "\n**{label}:** {value}");
        }
    }
    for stat in &weapon.stats {
        let _ = write!(body, "\n**{}:** {}", stat.name, stat.value);
    }
    if let Some(description) = &weapon.description {
        let _ = write!(body, "\n\n{description}");
    }

    let mut components = vec![body_component(body, weapon.thumbnail_url.as_deref())];
    components.push(separator());

    if weapon.attachment_slots.is_empty() {
        components.push(text("### Attachments\n*Data unavailable.*"));
    } else {
        let lines = weapon
            .attachment_slots
            .iter()
            .map(|slot| {
                slot.attachment.as_ref().map_or_else(
                    || format!("**{}** — *unavailable*", slot.slot),
                    |attachment| {
                        named_line(
                            &format!("{}: {}", slot.slot, attachment.name),
                            attachment.effect.as_deref(),
                        )
                    },
                )
            })
            .collect::<Vec<_>>();
        components.push(labelled_list("Attachments", &lines));
    }

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

pub fn attachment_component(attachment: &Attachment) -> CreateComponent<'static> {
    let mut body = format!("# {}", attachment.name);
    if let Some(slot) = &attachment.slot {
        let _ = write!(body, "\n**Slot:** {slot}");
    }
    if let Some(effect) = &attachment.effect {
        let _ = write!(body, "\n\n{effect}");
    }

    let compatible = if attachment.compatible_weapons.is_empty() {
        "*Data unavailable.*".to_string()
    } else {
        attachment.compatible_weapons.join(", ")
    };

    let components = vec![
        text(body),
        separator(),
        text(format!("### Compatible Weapons\n{compatible}")),
    ];

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

pub fn runner_component(runner: &Runner) -> CreateComponent<'static> {
    let mut body = format!("# {}", runner.name);
    if let Some(role) = &runner.role {
        let _ = write!(body, "\n-# {role}");
    }
    if let Some(description) = &runner.description {
        let _ = write!(body, "\n\n{description}");
    }
    for stat in &runner.stats {
        let _ = write!(body, "\n**{}:** {}", stat.name, stat.value);
    }

    let mut components = vec![body_component(body, runner.portrait_url.as_deref())];
    components.push(separator());

    if runner.abilities.is_empty() {
        components.push(text("### Abilities\n*Data unavailable.*"));
    } else {
        let lines = runner
            .abilities
            .iter()
            .map(|ability| {
                let mut line = format!("**{}**", ability.name);
                if let Some(ability_type) = &ability.ability_type {
                    let _ = write!(line, " ({ability_type})");
                }
                if let Some(cooldown) = ability.cooldown_seconds {
                    let _ = write!(line, " — {cooldown}s cooldown");
                }
                if let Some(description) = &ability.description {
                    let _ = write!(line, "\n{description}");
                }
                line
            })
            .collect::<Vec<_>>();
        components.push(labelled_list("Abilities", &lines));
    }

    if !runner.cores.is_empty() {
        components.push(separator());
        components.push(text(format!("### Cores\n{}", runner.cores.join(", "))));
    }

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

pub fn cradle_component(cradle: &Cradle) -> CreateComponent<'static> {
    let mut body = "# Cradle".to_string();
    if let Some(description) = &cradle.description {
        let _ = write!(body, "\n\n{description}");
    }

    let mut components = vec![text(body), separator()];

    let lines = cradle
        .nodes
        .iter()
        .map(|node| named_line(&node.name, node.description.as_deref()))
        .collect::<Vec<_>>();
    components.push(labelled_list("Nodes", &lines));

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

pub fn build_component(build: &BuildRecipe) -> CreateComponent<'static> {
    let mut body = format!("# {}", build.name);
    if let Some(shell) = &build.shell {
        let _ = write!(body, "\n**Shell:** {shell}");
    }
    if let Some(focus) = &build.cradle_focus {
        let _ = write!(body, "\n**Cradle Focus:** {focus}");
    }

    let mut components = vec![text(body), separator()];
    components.push(labelled_list("Gear", &build.gear));

    if let Some(notes) = &build.notes {
        components.push(separator());
        components.push(text(format!("### Notes\n{notes}")));
    }

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

pub fn map_component(map: &MarathonMap) -> CreateComponent<'static> {
    let status = match map.status {
        Some(MapStatus::Available) => "🟢 Available",
        Some(MapStatus::Locked) => "🔒 Locked",
        Some(MapStatus::Duo) => "👥 Duo",
        None => "❔ Unavailable",
    };

    let mut components =
        vec![text(format!("# {}\n**Status:** {status}", map.name)), separator()];

    if let Some(map_image_url) = &map.map_image_url {
        components.push(CreateContainerComponent::MediaGallery(
            CreateMediaGallery::new(vec![
                CreateMediaGalleryItem::new(CreateUnfurledMediaItem::new(
                    map_image_url.clone(),
                ))
                .description(format!("{} spawn and exfil map", map.name)),
            ]),
        ));
        components.push(text(
            "-# Marker counts below come from the source wiki's legend and \
             may be stale — the map above is the more reliable reference."
                .to_string(),
        ));
        components.push(separator());
    }

    let poi_lines = map
        .pois
        .iter()
        .map(|poi| named_line(&poi.name, poi.description.as_deref()))
        .collect::<Vec<_>>();
    components.push(labelled_list("Points of Interest", &poi_lines));
    components.push(separator());

    let extraction_lines = map
        .extractions
        .iter()
        .map(|location| named_line(&location.name, location.description.as_deref()))
        .collect::<Vec<_>>();
    components.push(labelled_list("Possible Extractions", &extraction_lines));
    components.push(separator());

    let event_lines = map
        .event_spawns
        .iter()
        .map(|location| named_line(&location.name, location.description.as_deref()))
        .collect::<Vec<_>>();
    components.push(labelled_list("Event Spawns", &event_lines));
    components.push(separator());

    let keycard_lines = map
        .keycard_rooms
        .iter()
        .map(|room| named_line(&room.name, room.location_hint.as_deref()))
        .collect::<Vec<_>>();
    components.push(labelled_list("Keycard / Loot Rooms", &keycard_lines));

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

pub fn faction_component(faction: &Faction) -> CreateComponent<'static> {
    let mut components = vec![text(format!("# {}", faction.name)), separator()];

    let contract_lines = faction
        .priority_contracts
        .iter()
        .map(|contract| {
            let mut line = format!("**{}**", contract.name);
            if let Some(difficulty) = &contract.difficulty {
                let _ = write!(line, " ({difficulty})");
            }
            if let Some(description) = &contract.description {
                let _ = write!(line, "\n{description}");
            }
            line
        })
        .collect::<Vec<_>>();
    components.push(labelled_list("Priority Contracts", &contract_lines));
    components.push(separator());

    let upgrade_lines = faction
        .upgrades
        .iter()
        .map(|upgrade| {
            let mut line = format!("**{}**", upgrade.name);
            if let Some(cost) = &upgrade.cost {
                let _ = write!(line, " — {cost}");
            }
            if let Some(requirements) = &upgrade.requirements {
                let _ = write!(line, "\n{requirements}");
            }
            line
        })
        .collect::<Vec<_>>();
    components.push(labelled_list("Upgrades", &upgrade_lines));

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

pub fn meta_component(entries: &[MetaEntry]) -> CreateComponent<'static> {
    let mut components = vec![text("# Current Meta".to_string()), separator()];

    let lines = entries
        .iter()
        .map(|entry| {
            let mut line = format!("**{}**", entry.item);
            if let Some(tier) = &entry.tier {
                let _ = write!(line, " — {tier}");
            }
            if let Some(note) = &entry.note {
                let _ = write!(line, "\n{note}");
            }
            line
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        components.push(text("*Data unavailable.*"));
    } else {
        components.push(text(lines.join("\n")));
    }

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}

pub fn news_item_component(item: &NewsItem) -> CreateComponent<'static> {
    let mut body = format!("### {}\n-# {}", item.title, item.source_label);
    if let Some(summary) = &item.summary {
        let _ = write!(body, "\n\n{summary}");
    }
    if let Some(url) = &item.url {
        let _ = write!(body, "\n\n{url}");
    }

    CreateComponent::Container(
        CreateContainer::new(vec![text(body)]).accent_colour(ACCENT),
    )
}

fn window_line(label: &str, window: RotationWindow) -> String {
    let status = if window.active { "🟢 Active now" } else { "⚪ Not active" };
    format!(
        "**{label}:** {} {:02}:00 PT → {} {:02}:00 PT — {status}",
        weekday_name(window.start_weekday),
        window.start_hour_pt,
        weekday_name(window.end_weekday),
        window.end_hour_pt
    )
}

pub fn schedule_component(schedule: &Schedule) -> CreateComponent<'static> {
    let mut components = vec![
        text("# Marathon Schedule".to_string()),
        separator(),
        text(window_line("Ranked", schedule.ranked_window)),
        text(window_line("Cryo Archive", schedule.cryo_window)),
        separator(),
    ];

    if schedule.duo_map_pool.is_empty() {
        components.push(text("### Daily Duo Map Pool\n*Data unavailable.*"));
    } else {
        components.push(text(format!(
            "### Daily Duo Map Pool\n{}\n-# Exact daily rotation order isn't confirmed by any source; this is the full pool.",
            schedule.duo_map_pool.join(", ")
        )));
    }

    if let Some(mode) = &schedule.weekly_game_mode {
        components.push(separator());
        components.push(text(format!("### Weekly Game Mode\n{mode}")));
    }

    CreateComponent::Container(
        CreateContainer::new(components).accent_colour(ACCENT),
    )
}
