use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Duration;

use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};
use zayden_app::config::RadioStation;

use crate::player::NowPlaying;
use crate::queue::Queue;
use crate::track::{LoopMode, ResolvedTrack};

const QUEUE_PAGE_SIZE: usize = 10;
const PROGRESS_BAR_WIDTH: u32 = 20;

#[must_use]
pub fn requested_by_mention(track: &ResolvedTrack) -> String {
    format!("<@{}>", track.requested_by)
}

#[must_use]
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[must_use]
pub fn parse_timestamp(s: &str) -> Option<Duration> {
    let mut secs: u64 = 0;
    for part in s.split(':') {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: u64 = part.parse().ok()?;
        secs = secs.checked_mul(60)?.checked_add(n)?;
    }

    Some(Duration::from_secs(secs))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekTarget {
    Absolute(Duration),
    Forward(Duration),
    Backward(Duration),
}

#[must_use]
pub fn parse_seek(s: &str) -> Option<SeekTarget> {
    let s = s.trim();

    if let Some(rest) = s.strip_prefix('+') {
        return parse_timestamp(rest).map(SeekTarget::Forward);
    }
    if let Some(rest) = s.strip_prefix('-') {
        return parse_timestamp(rest).map(SeekTarget::Backward);
    }

    parse_timestamp(s).map(SeekTarget::Absolute)
}

#[must_use]
pub fn progress_bar(elapsed: Duration, total: Option<Duration>) -> String {
    let Some(total) = total else {
        return format!("`{}` 🔴 PLAYING", format_duration(elapsed));
    };

    let filled = if total.is_zero() {
        0
    } else {
        let total_nanos = total.as_nanos();
        let elapsed_nanos = elapsed.as_nanos().min(total_nanos);
        let scaled =
            2 * u128::from(PROGRESS_BAR_WIDTH) * elapsed_nanos + total_nanos;

        u32::try_from(scaled / (2 * total_nanos))
            .unwrap_or(PROGRESS_BAR_WIDTH)
            .min(PROGRESS_BAR_WIDTH)
    };

    let bar: String = (0..PROGRESS_BAR_WIDTH)
        .map(|i| match i.cmp(&filled) {
            Ordering::Equal => '🔘',
            Ordering::Less | Ordering::Greater => '▬',
        })
        .collect();

    format!("`{}` {bar} `{}`", format_duration(elapsed), format_duration(total))
}

pub fn now_playing_embed(
    now: &NowPlaying,
    loop_mode: LoopMode,
) -> CreateEmbed<'static> {
    let elapsed = now.started_at.elapsed();
    let embed = CreateEmbed::new()
        .title("Now Playing")
        .description(format!("[{}]({})", now.track.title, now.track.url))
        .colour(Colour::BLURPLE)
        .field("Progress", progress_bar(elapsed, now.track.duration), false)
        .field("Requested by", requested_by_mention(&now.track), true);

    let embed = if loop_mode == LoopMode::Off {
        embed
    } else {
        embed.field("Loop", format!("{loop_mode:?}"), true)
    };

    match &now.track.thumbnail_url {
        Some(url) => embed.thumbnail(url.clone(), None),
        None => embed,
    }
}

pub fn track_announcement_embed(track: &ResolvedTrack) -> CreateEmbed<'static> {
    let embed = CreateEmbed::new()
        .title("Now Playing")
        .description(format!(
            "Song finished, now playing [{}]({})",
            track.title, track.url
        ))
        .colour(Colour::BLURPLE)
        .field("Requested by", requested_by_mention(track), true);

    match &track.thumbnail_url {
        Some(url) => embed.thumbnail(url.clone(), None),
        None => embed,
    }
}

pub fn queued_embed(track: &ResolvedTrack, position: usize) -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title("Queued")
        .description(format!("[{}]({})", track.title, track.url))
        .colour(Colour::BLURPLE)
        .field("Position", position.to_string(), true)
}

pub fn radio_embed(station: &RadioStation) -> CreateEmbed<'static> {
    let embed = CreateEmbed::new()
        .title("📻 Now Streaming")
        .description(format!("[{}]({})", station.name, station.display_url()))
        .colour(Colour::BLURPLE)
        .field(
            "Genre",
            station.genre.clone().unwrap_or_else(|| "—".to_string()),
            true,
        )
        .field("Status", "🔴 LIVE", true);

    match &station.logo_url {
        Some(url) => embed.thumbnail(url.clone(), None),
        None => embed,
    }
}

pub fn radio_list_embed(stations: &[RadioStation]) -> CreateEmbed<'static> {
    if stations.is_empty() {
        return CreateEmbed::new()
            .title("📻 Radio Stations")
            .description("No radio stations are configured on this bot.")
            .colour(Colour::BLURPLE);
    }

    let mut by_genre: BTreeMap<&str, Vec<&RadioStation>> = BTreeMap::new();
    for station in stations {
        by_genre
            .entry(station.genre.as_deref().unwrap_or("Other"))
            .or_default()
            .push(station);
    }

    let mut description = String::new();
    for (genre, mut group) in by_genre {
        group.sort_by(|a, b| a.name.cmp(&b.name));
        let _ = writeln!(description, "**{genre}**");
        for station in group {
            let _ = writeln!(
                description,
                "· [{}]({}) — `{}`",
                station.name,
                station.display_url(),
                station.id
            );
        }
        description.push('\n');
    }

    CreateEmbed::new()
        .title("📻 Radio Stations")
        .description(description)
        .colour(Colour::BLURPLE)
        .footer(CreateEmbedFooter::new("Play one with /music radio play"))
}

#[must_use]
pub fn queue_page_count(queue_len: usize) -> usize {
    queue_len.div_ceil(QUEUE_PAGE_SIZE).max(1)
}

pub fn queue_embed(
    queue: &Queue,
    current: Option<&ResolvedTrack>,
    page: usize,
) -> CreateEmbed<'static> {
    let total_pages = queue_page_count(queue.len());
    let page = page.min(total_pages - 1);
    let start = page * QUEUE_PAGE_SIZE;

    let mut description = String::new();
    if let Some(current) = current {
        let _ = write!(
            description,
            "**Now Playing:** [{}]({})\n\n",
            current.title, current.url
        );
    }

    if queue.is_empty() {
        description.push_str("The queue is empty.");
    } else {
        for (i, track) in queue.iter().enumerate().skip(start).take(QUEUE_PAGE_SIZE)
        {
            let _ = writeln!(
                description,
                "**{}.** [{}]({}) - {}",
                i + 1,
                track.title,
                track.url,
                requested_by_mention(track)
            );
        }
    }

    CreateEmbed::new()
        .title("Queue")
        .description(description)
        .colour(Colour::BLURPLE)
        .footer(CreateEmbedFooter::new(format!("Page {}/{total_pages}", page + 1)))
}
