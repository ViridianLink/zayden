use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use jellyfin::transport::jellyfin::model::ticks_to_seconds;
use jellyfin::transport::segments::{SegmentStats, segment_stats};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BingePlan {
    pub episodes: usize,
    pub total_seconds: i64,
    pub segments: SegmentStats,
}

impl BingePlan {
    #[must_use]
    pub fn measured_seconds(&self) -> i64 {
        (self.total_seconds - self.segments.skippable_seconds).max(0)
    }

    #[must_use]
    pub fn average_episode_seconds(&self) -> i64 {
        if self.episodes == 0 {
            return 0;
        }
        self.total_seconds / i64::try_from(self.episodes).unwrap_or(1)
    }

    #[must_use]
    pub const fn days_at(&self, per_day: usize) -> usize {
        if per_day == 0 {
            return 0;
        }
        self.episodes.div_ceil(per_day)
    }

    #[must_use]
    pub fn coverage_note(&self) -> String {
        if !self.segments.has_coverage() {
            return "No intro/outro data has been generated for this show yet, \
                    so nothing was subtracted."
                .to_owned();
        }

        format!(
            "Intro/outro data exists for {} of {} episodes ({}%). Only those \
             episodes had time subtracted — the rest are counted in full.",
            self.segments.measured,
            self.segments.episodes,
            self.segments.coverage_percent(),
        )
    }
}

pub async fn plan(
    runtime: &Arc<JellyfinRuntime>,
    series_id: &str,
    skip_intros: bool,
) -> Result<BingePlan, jellyfin::JellyfinError> {
    let episodes = runtime.jellyfin.episodes(series_id).await?;

    let total_seconds =
        episodes.iter().filter_map(|e| e.run_time_ticks).map(ticks_to_seconds).sum();

    let segments = if skip_intros {
        if let Some(cached) = runtime.caches.segment_stats.get(series_id).await {
            cached
        } else {
            let ids: Vec<String> = episodes.iter().map(|e| e.id.clone()).collect();
            let stats = segment_stats(runtime.jellyfin.clone(), ids).await;
            runtime.caches.segment_stats.insert(series_id.to_owned(), stats).await;
            stats
        }
    } else {
        SegmentStats::default()
    };

    Ok(BingePlan { episodes: episodes.len(), total_seconds, segments })
}

#[must_use]
pub fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours == 0 {
        format!("{minutes} minutes")
    } else if minutes == 0 {
        format!("{hours} hours")
    } else {
        format!("{hours} hours, {minutes} minutes")
    }
}
