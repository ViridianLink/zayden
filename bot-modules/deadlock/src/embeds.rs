use std::fmt::Write as _;

use deadlock_api::model::{Hero, HeroBuild};
use jiff::Timestamp;
use serenity::all::{
    Colour,
    CreateAllowedMentions,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateMessage,
    MessageFlags,
};
use zayden_core::limits::message::{
    COMPONENTS as MAX_COMPONENTS,
    COMPONENTS_TEXT as MAX_TEXT,
};
use zayden_core::templates::{body_component, separator, text};

use crate::analysis::{AbilityPicks, Counter, Counters, LaneSummary, Phase};
use crate::db::patches::PatchRow;
use crate::diff::{Change, HeroDiff, ItemDiff, PatchDiff, Presence};
use crate::heroes::HeroIndex;
use crate::items::ItemIndex;
use crate::summary::MatchSummary;

pub(crate) const ACCENT: Colour = Colour::new(0x00e0_8a2e);

const ALL_RANKS: &str = "All ranks";
const UNAVAILABLE: &str = "*Data unavailable.*";
const NO_CHANGES: &str = "*No hero or item stat changes detected.*";
const MAX_NAME_CHARS: usize = 100;
const MAX_MODS_PER_CATEGORY: usize = 12;
const COMPACT_THRESHOLD: u64 = 10_000;
const TEXT_RESERVE: usize = 300;
const MAX_SECTIONS: usize = MAX_COMPONENTS - 6;
const SUMMARY_TEXT: usize = MAX_TEXT / 8;

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

struct Section {
    heading: String,
    lines: Vec<String>,
}

fn change_line(scope: &str, change: &Change) -> String {
    let label = &change.label;

    match (&change.old, &change.new) {
        (Some(old), Some(new)) => format!("- {scope}{label}: {old} → {new}"),
        (None, Some(new)) => format!("- {scope}{label}: {new} (new)"),
        (Some(_), None) => format!("- {scope}{label}: removed"),
        (None, None) => format!("- {scope}{label}: changed"),
    }
}

fn hero_section(hero: &HeroDiff) -> Section {
    let mut lines = Vec::new();

    match hero.presence {
        Presence::Added => lines.push("- New hero".to_owned()),
        Presence::Removed => {
            lines.push("- Removed from the hero pool".to_owned());
        },
        Presence::Changed => {},
    }

    lines.extend(hero.stats.iter().map(|change| change_line("", change)));
    lines.extend(hero.weapon.iter().map(|change| change_line("Weapon · ", change)));

    for ability in &hero.abilities {
        match ability.presence {
            Presence::Added => {
                lines.push(format!("- New ability: {}", ability.name));
            },
            Presence::Removed => {
                lines.push(format!("- Removed ability: {}", ability.name));
            },
            Presence::Changed => {
                let scope = format!("{} · ", ability.name);
                lines.extend(
                    ability.changes.iter().map(|change| change_line(&scope, change)),
                );
            },
        }
    }

    Section { heading: format!("### {}", hero.name), lines }
}

fn items_section(items: &[ItemDiff]) -> Section {
    let mut lines = Vec::new();

    for item in items {
        match item.presence {
            Presence::Added => lines.push(format!("- New item: {}", item.name)),
            Presence::Removed => {
                lines.push(format!("- Removed item: {}", item.name));
            },
            Presence::Changed => {
                let scope = format!("{}: ", item.name);
                lines.extend(
                    item.changes.iter().map(|change| change_line(&scope, change)),
                );
            },
        }
    }

    Section { heading: "### Items".to_owned(), lines }
}

fn patch_sections(diff: &PatchDiff) -> Vec<Section> {
    let mut sections: Vec<Section> = diff.heroes.iter().map(hero_section).collect();

    if !diff.items.is_empty() {
        sections.push(items_section(&diff.items));
    }

    sections
}

pub fn patch_component(
    patch: &PatchRow,
    text_budget: usize,
) -> CreateComponent<'static> {
    let header = format!(
        "## {}\n[Read the official patch notes]({})",
        patch.title, patch.link
    );
    let limit = text_budget.min(MAX_TEXT).saturating_sub(TEXT_RESERVE);
    let mut used = header.chars().count();
    let mut parts = vec![text(header)];

    let sections = patch.diff.as_ref().map_or_else(Vec::new, patch_sections);
    if sections.is_empty() {
        parts.push(text(NO_CHANGES));
        return CreateComponent::Container(
            CreateContainer::new(parts).accent_colour(ACCENT),
        );
    }

    let total: usize = sections.iter().map(|section| section.lines.len()).sum();
    let mut shown = 0;

    'sections: for section in sections.iter().take(MAX_SECTIONS) {
        let mut block = String::new();

        for line in &section.lines {
            let heading =
                if block.is_empty() { section.heading.chars().count() } else { 0 };
            let cost = heading + line.chars().count() + 1;

            if used + cost > limit {
                if !block.is_empty() {
                    parts.push(text(block));
                }
                break 'sections;
            }

            if block.is_empty() {
                block.push_str(&section.heading);
            }
            block.push('\n');
            block.push_str(line);
            used += cost;
            shown += 1;
        }

        if !block.is_empty() {
            parts.push(text(block));
        }
    }

    if shown < total {
        parts.push(text(format!(
            "…and {} more changes in the official notes.",
            total - shown
        )));
    }

    CreateComponent::Container(CreateContainer::new(parts).accent_colour(ACCENT))
}

fn clock(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is made non-negative, rounded and clamped to the u32 range first"
)]
fn signed_clock(delta: f64) -> String {
    let sign = if delta < 0.0 { '-' } else { '+' };
    let seconds = delta.abs().round().min(f64::from(u32::MAX)) as u32;

    format!("{sign}{}", clock(seconds))
}

fn bounded(block: String) -> String {
    if block.chars().count() <= SUMMARY_TEXT {
        return block;
    }

    let mut clipped: String = block.chars().take(SUMMARY_TEXT - 1).collect();
    clipped.push('…');

    clipped
}

fn summary_blocks(
    summary: &MatchSummary,
    hero: Option<&Hero>,
    items: &ItemIndex,
    mention: Option<i64>,
) -> Vec<String> {
    let outcome = if summary.won { "Victory" } else { "Defeat" };
    let name = hero.map_or_else(
        || format!("Hero #{}", summary.hero_id),
        |hero| hero.name.clone(),
    );
    let prefix =
        mention.map_or_else(String::new, |id| format!("<@{}> ", id.cast_unsigned()));

    let mut blocks = vec![format!(
        "{prefix}**{outcome}** as {name}\nK/D/A {}/{}/{} · {} souls · {} LH / {} DN · {} · <t:{}:R>",
        summary.kills,
        summary.deaths,
        summary.assists,
        summary.net_worth,
        summary.last_hits,
        summary.denies,
        clock(summary.duration_s),
        summary.start_time.as_second()
    )];

    if !summary.souls.is_empty() {
        let lines = summary
            .souls
            .iter()
            .map(|soul| {
                let mut line = format!("{}m  {}", soul.time_s / 60, soul.net_worth);
                if let Some(average) = soul.average {
                    let _ = write!(
                        line,
                        "  ({} vs avg)",
                        signed_thousands(f64::from(soul.net_worth) - average)
                    );
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
        blocks.push(format!("**Souls**\n{lines}"));
    }

    if !summary.items.is_empty() {
        let lines = summary
            .items
            .iter()
            .map(|timing| {
                let mut line = format!(
                    "{} {}",
                    items.name(timing.item_id),
                    clock(timing.bought_at_s)
                );
                if let Some(average) = timing.average_s {
                    let _ = write!(
                        line,
                        " ({} vs avg)",
                        signed_clock(f64::from(timing.bought_at_s) - average)
                    );
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
        blocks.push(format!("**Items**\n{lines}"));
    }

    if let Some(abilities) = &summary.abilities {
        blocks.push(format!(
            "Followed the most-played ability order for {}/{} upgrades",
            abilities.followed, abilities.total
        ));
    }

    blocks.into_iter().map(bounded).collect()
}

fn summary_parts(
    summary: &MatchSummary,
    hero: Option<&Hero>,
    items: &ItemIndex,
    mention: Option<i64>,
) -> (Vec<CreateContainerComponent<'static>>, usize) {
    let blocks = summary_blocks(summary, hero, items, mention);
    let used = blocks.iter().map(|block| block.chars().count()).sum();

    let mut parts = Vec::new();
    for (index, block) in blocks.into_iter().enumerate() {
        if index > 0 {
            parts.push(separator());
        }
        parts.push(text(block));
    }

    (parts, used)
}

pub fn summary_component(
    summary: &MatchSummary,
    hero: Option<&Hero>,
    items: &ItemIndex,
    mention: Option<i64>,
) -> CreateComponent<'static> {
    let (parts, _) = summary_parts(summary, hero, items, mention);

    CreateComponent::Container(CreateContainer::new(parts).accent_colour(ACCENT))
}

pub fn summary_dm(
    summary: &MatchSummary,
    hero: Option<&Hero>,
    items: &ItemIndex,
) -> CreateMessage<'static> {
    CreateMessage::new()
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(vec![summary_component(summary, hero, items, None)])
        .allowed_mentions(CreateAllowedMentions::new())
}

pub fn summary_post(
    sections: &[(&MatchSummary, i64)],
    heroes: &HeroIndex,
    items: &ItemIndex,
) -> CreateMessage<'static> {
    let mut used = 0;
    let mut components = Vec::new();

    for (summary, discord_id) in sections {
        let (parts, chars) = summary_parts(
            summary,
            heroes.get(summary.hero_id),
            items,
            Some(*discord_id),
        );
        if !components.is_empty() && used + chars > MAX_TEXT {
            break;
        }

        used += chars;
        components.push(CreateComponent::Container(
            CreateContainer::new(parts).accent_colour(ACCENT),
        ));
    }

    CreateMessage::new()
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(components)
        .allowed_mentions(CreateAllowedMentions::new())
}
