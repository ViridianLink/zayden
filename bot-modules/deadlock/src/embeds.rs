use std::fmt::Write as _;

use deadlock_api::model::{Hero, HeroBuild};
use jiff::Timestamp;
use serenity::all::{
    Colour,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
};
use zayden_core::templates::{body_component, separator, text};

use crate::analysis::{AbilityPicks, Counter, Counters, LaneSummary, Phase};
use crate::heroes::HeroIndex;
use crate::items::ItemIndex;

const ACCENT: Colour = Colour::new(0x00e0_8a2e);

const ALL_RANKS: &str = "All ranks";
const UNAVAILABLE: &str = "*Data unavailable.*";
const MAX_NAME_CHARS: usize = 100;
const MAX_MODS_PER_CATEGORY: usize = 12;
const COMPACT_THRESHOLD: u64 = 10_000;

#[derive(Debug, Clone, Copy)]
pub struct Freshness {
    pub fetched_at: Timestamp,
    pub stale: bool,
}

impl Freshness {
    #[must_use]
    pub fn combine(items: &[Self]) -> Self {
        let fetched_at = items
            .iter()
            .map(|fresh| fresh.fetched_at)
            .min()
            .unwrap_or_else(Timestamp::now);
        let stale = items.iter().any(|fresh| fresh.stale);

        Self { fetched_at, stale }
    }
}

fn compact(value: u64) -> String {
    if value < COMPACT_THRESHOLD {
        return value.to_string();
    }

    format!("{}.{}k", value / 1_000, (value % 1_000) / 100)
}

fn percent(rate: f64) -> String {
    format!("{:.1}%", rate * 100.0)
}

fn clip(value: &str) -> String {
    value.chars().take(MAX_NAME_CHARS).collect()
}

fn signed_thousands(value: f64) -> String {
    let rounded = format!("{value:.0}");
    let (negative, digits) = rounded
        .strip_prefix('-')
        .map_or((false, rounded.as_str()), |digits| (true, digits));
    let negative = negative && digits.chars().any(|digit| digit != '0');

    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }

    format!("{}{grouped}", if negative { '-' } else { '+' })
}

fn header(
    hero: &Hero,
    title: &str,
    rank: Option<&str>,
) -> CreateContainerComponent<'static> {
    let subtitle = rank.unwrap_or(ALL_RANKS);

    body_component(
        format!("## {title}\n{subtitle}"),
        hero.images.icon_image_small.as_deref(),
    )
}

fn footer(fresh: &Freshness) -> CreateContainerComponent<'static> {
    let mut line = format!(
        "-# Data: deadlock-api.com · updated <t:{}:R>",
        fresh.fetched_at.as_second()
    );
    if fresh.stale {
        line.push_str(" · cached copy, deadlock-api is busy");
    }

    text(line)
}

fn container(
    mut parts: Vec<CreateContainerComponent<'static>>,
    fresh: &Freshness,
) -> CreateComponent<'static> {
    parts.push(footer(fresh));

    CreateComponent::Container(CreateContainer::new(parts).accent_colour(ACCENT))
}

fn stat_line(name: &str, win_rate: f64, matches: u64) -> String {
    format!("- {name} — {} win ({} games)", percent(win_rate), compact(matches))
}

fn phases_text(phases: &[Phase], items: &ItemIndex) -> String {
    if phases.is_empty() {
        return UNAVAILABLE.to_string();
    }

    phases
        .iter()
        .map(|phase| {
            let mut block = format!("**Phase {}**", u16::from(phase.column) + 1);
            for item in &phase.items {
                let _ = write!(
                    block,
                    "\n{}",
                    stat_line(
                        &items.name(item.item_id),
                        item.win_rate,
                        item.matches
                    )
                );
            }
            block
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn build_text(build: &HeroBuild, items: &ItemIndex) -> String {
    let mut body = format!("**Top community build:** {}", clip(&build.name));

    for category in &build.details.mod_categories {
        if category.mods.is_empty() {
            continue;
        }

        let names = category
            .mods
            .iter()
            .take(MAX_MODS_PER_CATEGORY)
            .map(|item| items.name(item.ability_id))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(body, "\n**{}**: {names}", clip(&category.name));
    }

    body
}

fn sequence_text(sequence: &[u32], items: &ItemIndex) -> String {
    sequence
        .iter()
        .enumerate()
        .map(|(index, id)| format!("{}. {}", index + 1, items.name(*id)))
        .collect::<Vec<_>>()
        .join(" → ")
}

pub fn build_component(
    hero: &Hero,
    rank: Option<&str>,
    phases: &[Phase],
    build: Option<&HeroBuild>,
    items: &ItemIndex,
    fresh: &Freshness,
) -> CreateComponent<'static> {
    let mut parts = vec![
        header(hero, &format!("{} build", hero.name), rank),
        text(phases_text(phases, items)),
    ];

    if let Some(build) = build {
        parts.push(separator());
        parts.push(text(build_text(build, items)));
    }

    container(parts, fresh)
}

pub fn abilities_component(
    hero: &Hero,
    rank: Option<&str>,
    picks: Option<&AbilityPicks<'_>>,
    build: Option<(&str, &[u32])>,
    items: &ItemIndex,
    fresh: &Freshness,
) -> CreateComponent<'static> {
    let mut parts =
        vec![header(hero, &format!("{} ability order", hero.name), rank)];

    if let Some(picks) = picks {
        parts.push(text(format!(
            "**Most played** — {} win ({} games)\n{}",
            percent(picks.most_played.win_rate()),
            compact(picks.most_played.matches),
            sequence_text(&picks.most_played.abilities, items)
        )));

        if picks.best.abilities != picks.most_played.abilities {
            parts.push(text(format!(
                "**Highest win rate** — {} win ({} games)\n{}",
                percent(picks.best.win_rate()),
                compact(picks.best.matches),
                sequence_text(&picks.best.abilities, items)
            )));
        }
    }
    if picks.is_none() {
        parts.push(text(UNAVAILABLE));
    }

    if let Some((name, sequence)) = build
        && !sequence.is_empty()
    {
        parts.push(separator());
        parts.push(text(format!(
            "**Top community build ({})**\n{}",
            clip(name),
            sequence_text(sequence, items)
        )));
    }

    container(parts, fresh)
}

fn counter_list(
    title: &str,
    counters: &[Counter],
    heroes: &HeroIndex,
) -> CreateContainerComponent<'static> {
    if counters.is_empty() {
        return text(format!("**{title}**\n{UNAVAILABLE}"));
    }

    let lines = counters
        .iter()
        .map(|counter| {
            let name = heroes.get(counter.enemy_hero_id).map_or_else(
                || format!("Hero #{}", counter.enemy_hero_id),
                |enemy| enemy.name.clone(),
            );
            stat_line(&name, counter.win_rate, counter.matches)
        })
        .collect::<Vec<_>>()
        .join("\n");

    text(format!("**{title}**\n{lines}"))
}

pub fn counter_component(
    hero: &Hero,
    counters: &Counters,
    heroes: &HeroIndex,
    fresh: &Freshness,
) -> CreateComponent<'static> {
    let parts = vec![
        header(hero, &format!("{} matchups", hero.name), None),
        counter_list("Strong against", &counters.strong, heroes),
        counter_list("Weak against", &counters.weak, heroes),
    ];

    container(parts, fresh)
}

fn soul_table(lane: &LaneSummary) -> String {
    if lane.points.is_empty() {
        return UNAVAILABLE.to_string();
    }

    let mut table = String::from("```\n");
    for point in &lane.points {
        let _ = writeln!(
            table,
            "{:02}:{:02}  {}",
            point.time_s / 60,
            point.time_s % 60,
            signed_thousands(point.net_worth_diff)
        );
    }
    table.push_str("```");

    table
}

pub fn matchup_component(
    hero: &Hero,
    enemy: &Hero,
    rank: Option<&str>,
    lane: Option<&LaneSummary>,
    fresh: &Freshness,
) -> CreateComponent<'static> {
    let mut parts = vec![header(
        hero,
        &format!("{} vs {} in lane", hero.name, enemy.name),
        rank,
    )];

    if let Some(lane) = lane {
        parts.push(text(format!(
            "**Lane win rate:** {} ({} games)",
            percent(lane.win_rate()),
            compact(lane.matches)
        )));
        parts.push(text(format!("**Soul lead over time**\n{}", soul_table(lane))));
    }
    if lane.is_none() {
        parts.push(text(UNAVAILABLE));
    }

    container(parts, fresh)
}
