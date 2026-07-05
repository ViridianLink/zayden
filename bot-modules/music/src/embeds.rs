use std::cmp::Ordering;
use std::fmt::Write as _;
use std::time::Duration;

use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};

use crate::player::NowPlaying;
use crate::queue::Queue;
use crate::track::{LoopMode, ResolvedTrack};

const QUEUE_PAGE_SIZE: usize = 10;
const PROGRESS_BAR_WIDTH: u32 = 20;

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
        let n: u64 = part.parse().ok()?;
        secs = secs.checked_mul(60)?.checked_add(n)?;
    }

    Some(Duration::from_secs(secs))
}

#[must_use]
pub fn progress_bar(elapsed: Duration, total: Option<Duration>) -> String {
    let Some(total) = total else {
        return format!("`{}` 🔴 PLAYING", format_duration(elapsed));
    };

    let ratio = if total.is_zero() {
        0.0
    } else {
        (elapsed.as_secs_f64() / total.as_secs_f64()).clamp(0.0, 1.0)
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ratio is clamped to [0.0, 1.0], so the product fits in PROGRESS_BAR_WIDTH"
    )]
    let filled = (ratio * f64::from(PROGRESS_BAR_WIDTH)).round() as u32;
    let filled = filled.min(PROGRESS_BAR_WIDTH);

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
        .field("Requested by", now.track.requested_by.display_name.clone(), true);

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

pub fn queued_embed(track: &ResolvedTrack, position: usize) -> CreateEmbed<'static> {
    CreateEmbed::new()
        .title("Queued")
        .description(format!("[{}]({})", track.title, track.url))
        .colour(Colour::BLURPLE)
        .field("Position", position.to_string(), true)
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
                "**{}.** [{}]({}) — {}",
                i + 1,
                track.title,
                track.url,
                track.requested_by.display_name
            );
        }
    }

    CreateEmbed::new()
        .title("Queue")
        .description(description)
        .colour(Colour::BLURPLE)
        .footer(CreateEmbedFooter::new(format!("Page {}/{total_pages}", page + 1)))
}
