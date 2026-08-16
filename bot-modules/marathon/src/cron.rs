use std::sync::Arc;

use futures::future;
use reqwest::Client;
use serenity::all::{ChannelId, CreateMessage, MessageFlags};
use sqlx::PgPool;
use tracing::{debug, error};
use zayden_core::{CronJob, as_u64};

use crate::announce::{MarathonAnnounceRow, NewsSeenRow};
use crate::client::MarathonClient;
use crate::embeds;
use crate::model::NewsItem;
use crate::news::{
    self,
    BLUESKY_ACTORS,
    BLUESKY_FEED_URL,
    BUNGIE_NEWS_SOURCE,
    BlueskyFeed,
    BungieNewsFeed,
};

pub struct MarathonAnnounceCron;

impl MarathonAnnounceCron {
    pub fn cron_job(
        client: Arc<MarathonClient>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
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

async fn collect_feed(
    pool: &PgPool,
    feed_key: &str,
    fetched: crate::error::Result<Vec<NewsItem>>,
    out: &mut Vec<NewsItem>,
) {
    let items = match fetched {
        Ok(items) => items,
        Err(e) => {
            error!(error = ?e, feed_key, "marathon: failed to fetch news feed");
            return;
        },
    };

    match diff_and_seed(pool, feed_key, &items).await {
        Ok(mut new) => out.append(&mut new),
        Err(e) => {
            error!(error = ?e, feed_key, "marathon: failed to diff news feed");
        },
    }
}

async fn poll_feeds(
    pool: &PgPool,
    client: &Client,
    bungie_api_key: Option<&str>,
) -> Vec<NewsItem> {
    let bungie = async {
        match bungie_api_key {
            Some(api_key) => Some(BungieNewsFeed::fetch(client, api_key).await),
            None => {
                debug!("marathon: BUNGIE_API_KEY unset, skipping Tier 1 news feed");
                None
            },
        }
    };
    let bluesky = future::join_all(
        BLUESKY_ACTORS
            .map(|actor| BlueskyFeed::fetch_actor(client, BLUESKY_FEED_URL, actor)),
    );

    let (bungie, bluesky) = future::join(bungie, bluesky).await;

    let mut new_items = Vec::new();

    if let Some(fetched) = bungie {
        collect_feed(pool, BUNGIE_NEWS_SOURCE, fetched, &mut new_items).await;
    }

    for (actor, fetched) in BLUESKY_ACTORS.iter().zip(bluesky) {
        collect_feed(pool, &format!("bluesky:{actor}"), fetched, &mut new_items)
            .await;
    }

    new_items
}

pub struct MarathonNewsCron;

impl MarathonNewsCron {
    pub fn cron_job(
        client: Client,
        bungie_api_key: Option<String>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("marathon_news_announce", "0 0,30 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let client = client.clone();
                let bungie_api_key = bungie_api_key.clone();
                async move {
                    let new_items =
                        poll_feeds(&pool, &client, bungie_api_key.as_deref()).await;

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
