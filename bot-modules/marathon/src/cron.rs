use std::sync::Arc;

use reqwest::Client;
use serenity::all::{ChannelId, CreateMessage, MessageFlags};
use sqlx::{PgPool, Postgres};
use tracing::{debug, error};
use zayden_core::{CronJob, as_u64};

use crate::announce::{MarathonAnnounceRow, NewsSeenRow};
use crate::client::MarathonClient;
use crate::embeds;
use crate::model::NewsItem;
use crate::news::{self, BLUESKY_ACTORS, BlueskyFeed, BungieNewsFeed};

pub struct MarathonAnnounceCron;

impl MarathonAnnounceCron {
    pub fn cron_job(
        client: Arc<MarathonClient>,
    ) -> Result<CronJob<Postgres>, jiff_cron::error::Error> {
        CronJob::new("marathon_schedule_announce", "0 0 17,18 * * Sun,Thu *").map(|job| {
            job.set_action(move |ctx, pool| {
                let client = Arc::clone(&client);
                async move {
                    let schedule = match client.schedule() {
                        Ok(schedule) => schedule,
                        Err(e) => {
                            error!(error = ?e, "marathon: failed to compute schedule");
                            return;
                        }
                    };
                    let rotation_key = format!("{schedule:?}");

                    let rows = match MarathonAnnounceRow::all(&pool).await {
                        Ok(rows) => rows,
                        Err(e) => {
                            error!(error = ?e, "marathon: failed to load announce rows");
                            return;
                        }
                    };

                    let component = embeds::schedule_component(&schedule);

                    for row in rows {
                        if row.last_rotation.as_deref() == Some(rotation_key.as_str()) {
                            continue;
                        }

                        let channel_id = ChannelId::new(as_u64(row.channel_id));
                        if let Err(e) = channel_id
                            .widen()
                            .send_message(
                                &ctx.http,
                                CreateMessage::new()
                                    .flags(MessageFlags::IS_COMPONENTS_V2)
                                    .components(vec![component.clone()]),
                            )
                            .await
                        {
                            error!(
                                error = ?e,
                                guild_id = row.guild_id,
                                "marathon: failed to post schedule announcement"
                            );
                            continue;
                        }

                        if let Err(e) =
                            MarathonAnnounceRow::set_last_rotation(&pool, row.guild_id, &rotation_key)
                                .await
                        {
                            error!(
                                error = ?e,
                                guild_id = row.guild_id,
                                "marathon: failed to persist last_rotation"
                            );
                        }
                    }
                }
            })
        })
    }
}

async fn diff_and_seed(
    pool: &PgPool,
    feed_key: &str,
    items: &[NewsItem],
) -> crate::error::Result<Vec<NewsItem>> {
    let Some(newest) = items.first() else { return Ok(Vec::new()) };

    let last_id =
        NewsSeenRow::get(pool, feed_key).await?.and_then(|row| row.last_id);
    let new_items = news::new_since(items, last_id.as_deref()).to_vec();

    NewsSeenRow::set_last_id(pool, feed_key, &newest.id).await?;

    Ok(new_items)
}

pub struct MarathonNewsCron;

impl MarathonNewsCron {
    pub fn cron_job(
        client: Client,
        bungie_api_key: Option<String>,
    ) -> Result<CronJob<Postgres>, jiff_cron::error::Error> {
        CronJob::new("marathon_news_announce", "0 0,30 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let client = client.clone();
                let bungie_api_key = bungie_api_key.clone();
                async move {
                    let mut new_items: Vec<NewsItem> = Vec::new();

                    if let Some(api_key) = bungie_api_key.as_deref() {
                        match BungieNewsFeed::fetch(&client, api_key).await {
                            Ok(items) => match diff_and_seed(&pool, "bungie_news", &items).await {
                                Ok(mut new) => new_items.append(&mut new),
                                Err(e) => {
                                    error!(error = ?e, "marathon: failed to diff bungie news");
                                }
                            },
                            Err(e) => error!(error = ?e, "marathon: failed to fetch bungie news"),
                        }
                    } else {
                        debug!("marathon: BUNGIE_API_KEY unset, skipping Tier 1 news feed");
                    }

                    for actor in BLUESKY_ACTORS {
                        match BlueskyFeed::fetch_actor(&client, actor).await {
                            Ok(items) => {
                                let feed_key = format!("bluesky:{actor}");
                                match diff_and_seed(&pool, &feed_key, &items).await {
                                    Ok(mut new) => new_items.append(&mut new),
                                    Err(e) => {
                                        error!(error = ?e, actor, "marathon: failed to diff bluesky feed");
                                    }
                                }
                            }
                            Err(e) => {
                                error!(error = ?e, actor, "marathon: failed to fetch bluesky feed");
                            }
                        }
                    }

                    if new_items.is_empty() {
                        return;
                    }

                    let rows = match MarathonAnnounceRow::all(&pool).await {
                        Ok(rows) => rows,
                        Err(e) => {
                            error!(error = ?e, "marathon: failed to load announce rows");
                            return;
                        }
                    };

                    for item in new_items.iter().rev() {
                        let component = embeds::news_item_component(item);

                        for row in &rows {
                            let channel_id = ChannelId::new(as_u64(row.channel_id));
                            if let Err(e) = channel_id
                                .widen()
                                .send_message(
                                    &ctx.http,
                                    CreateMessage::new()
                                        .flags(MessageFlags::IS_COMPONENTS_V2)
                                        .components(vec![component.clone()]),
                                )
                                .await
                            {
                                error!(
                                    error = ?e,
                                    guild_id = row.guild_id,
                                    "marathon: failed to post news item"
                                );
                            }
                        }
                    }
                }
            })
        })
    }
}
