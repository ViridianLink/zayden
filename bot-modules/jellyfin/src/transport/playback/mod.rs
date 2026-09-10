pub mod model;
pub mod query;

use model::{DailyPlay, ItemPlayCount, QueryResponse, RecentPlay};
use reqwest::Client;
use serde_json::json;

use crate::transport::http::{ApiResult, fetch_json, trim_base_url};

pub const SERVICE: &str = "Playback Reporting";

#[derive(Debug, Clone)]
pub struct PlaybackClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl PlaybackClient {
    #[must_use]
    pub fn new(client: Client, base_url: String, api_key: String) -> Self {
        Self { client, base_url: trim_base_url(base_url), api_key }
    }

    async fn run(&self, sql: &str) -> ApiResult<QueryResponse> {
        fetch_json(SERVICE, "custom query", || {
            self.client
                .post(format!(
                    "{}/user_usage_stats/submit_custom_query",
                    self.base_url
                ))
                .header(
                    reqwest::header::AUTHORIZATION,
                    format!(r#"MediaBrowser Token="{}""#, self.api_key),
                )
                .header(reqwest::header::ACCEPT, "application/json")
                .json(&json!({
                    "CustomQueryString": sql,
                    "ReplaceUserId": false,
                }))
        })
        .await
    }

    pub async fn daily_rollup(&self, since_days: i64) -> ApiResult<Vec<DailyPlay>> {
        let resp = self.run(&query::daily_rollup(since_days)).await?;

        Ok(resp
            .results
            .iter()
            .filter_map(|row| {
                Some(DailyPlay {
                    jellyfin_user_id: row.first()?.clone(),
                    day: row.get(1)?.clone(),
                    seconds: row.get(2)?.parse().ok()?,
                    items: row.get(3)?.parse().ok()?,
                })
            })
            .collect())
    }

    pub async fn recent_plays(
        &self,
        jellyfin_user_id: &str,
        limit: i64,
    ) -> ApiResult<Vec<RecentPlay>> {
        let resp = self.run(&query::recent_plays(jellyfin_user_id, limit)).await?;
        Ok(decode_plays(&resp))
    }

    pub async fn server_recent_plays(
        &self,
        limit: i64,
    ) -> ApiResult<Vec<RecentPlay>> {
        let resp = self.run(&query::server_recent_plays(limit)).await?;
        Ok(decode_plays(&resp))
    }

    pub async fn item_play_counts(&self) -> ApiResult<Vec<ItemPlayCount>> {
        let resp = self.run(&query::item_play_counts()).await?;

        Ok(resp
            .results
            .iter()
            .filter_map(|row| {
                Some(ItemPlayCount {
                    item_id: row.first()?.clone(),
                    plays: row.get(1)?.parse().ok()?,
                    distinct_users: row.get(2)?.parse().ok()?,
                })
            })
            .collect())
    }
}

fn decode_plays(resp: &QueryResponse) -> Vec<RecentPlay> {
    resp.results
        .iter()
        .filter_map(|row| {
            Some(RecentPlay {
                item_id: row.first()?.clone(),
                item_name: row.get(1)?.clone(),
                item_type: row.get(2)?.clone(),
                played_at: row.get(3)?.clone(),
            })
        })
        .collect()
}
