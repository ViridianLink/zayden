use futures::{StreamExt, stream};
use serde::Deserialize;

use crate::transport::http::{ApiResult, fetch_json};
use crate::transport::jellyfin::JellyfinClient;
use crate::transport::jellyfin::model::{ItemsPage, ticks_to_seconds};

const CONCURRENCY: usize = 8;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaSegment {
    #[serde(rename = "Type")]
    pub segment_type: String,
    #[serde(default)]
    pub start_ticks: i64,
    #[serde(default)]
    pub end_ticks: i64,
}

impl MediaSegment {
    #[must_use]
    pub fn seconds(&self) -> i64 {
        ticks_to_seconds((self.end_ticks - self.start_ticks).max(0))
    }

    #[must_use]
    pub fn is_skippable(&self) -> bool {
        matches!(
            self.segment_type.as_str(),
            "Intro" | "Outro" | "Recap" | "Preview" | "Commercial"
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SegmentStats {
    pub episodes: usize,
    pub measured: usize,
    pub skippable_seconds: i64,
}

impl SegmentStats {
    #[must_use]
    pub const fn has_coverage(&self) -> bool {
        self.measured > 0
    }

    #[must_use]
    pub fn coverage_percent(&self) -> u32 {
        if self.episodes == 0 {
            return 0;
        }
        u32::try_from(self.measured * 100 / self.episodes).unwrap_or(100)
    }
}

impl JellyfinClient {
    pub async fn media_segments<'a>(
        &'a self,
        item_id: &'a str,
    ) -> ApiResult<Vec<MediaSegment>> {
        let page: ItemsPage<MediaSegment> =
            fetch_json(super::jellyfin::SERVICE, "media segments", || {
                self.get(&format!("MediaSegments/{item_id}"))
            })
            .await?;

        Ok(page.items)
    }
}

pub async fn segment_stats(
    client: JellyfinClient,
    item_ids: Vec<String>,
) -> SegmentStats {
    let episodes = item_ids.len();

    let requests = item_ids.into_iter().map(|id| {
        let client = client.clone();
        async move { client.media_segments(&id).await }
    });

    let per_item = stream::iter(requests)
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

    let mut stats = SegmentStats { episodes, ..Default::default() };

    for segments in per_item {
        let segments = segments.unwrap_or_default();
        let seconds: i64 = segments
            .iter()
            .filter(|s| s.is_skippable())
            .map(MediaSegment::seconds)
            .sum();

        if seconds > 0 {
            stats.measured += 1;
            stats.skippable_seconds += seconds;
        }
    }

    stats
}
