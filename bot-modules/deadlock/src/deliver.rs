use std::collections::BTreeMap;
use std::time::Duration;

use deadlock_api::Priority;
use futures::StreamExt;
use serenity::all::{ChannelId, CreateMessage, Http, UserId};
use sqlx::PgPool;
use tracing::{error, warn};

use crate::client::DeadlockApi;
use crate::db::{guild_posts, summary_channel, user_settings};
use crate::embeds;
use crate::error::Result;
use crate::heroes::HeroIndex;
use crate::items::ItemIndex;
use crate::match_poll::SummaryJob;

pub const DM_INTERVAL: Duration = Duration::from_secs(1);

const MAX_POST_SECTIONS: usize = 4;
const CONCURRENCY: usize = 5;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DeliveryOutcome {
    pub dms: usize,
    pub posts: usize,
}

pub async fn deliver(
    http: &Http,
    pool: &PgPool,
    api: &DeadlockApi,
    application_id: i64,
    jobs: Vec<SummaryJob>,
) -> Result<DeliveryOutcome> {
    let heroes = match api.heroes(None, Priority::Background).await {
        Ok(fetched) => HeroIndex::new(&fetched.value),
        Err(e) => {
            warn!(error = ?e, "deadlock: hero names unavailable for match summaries");
            HeroIndex::new(&[])
        },
    };
    let items = match api.items(None, Priority::Background).await {
        Ok(fetched) => ItemIndex::new(&fetched.value),
        Err(e) => {
            warn!(error = ?e, "deadlock: item names unavailable for match summaries");
            ItemIndex::new(&[])
        },
    };

    let dms = send_dms(http, pool, &jobs, &heroes, &items).await?;
    let posts =
        post_to_guilds(http, pool, application_id, &jobs, &heroes, &items).await?;

    Ok(DeliveryOutcome { dms, posts })
}

async fn send_dms(
    http: &Http,
    pool: &PgPool,
    jobs: &[SummaryJob],
    heroes: &HeroIndex,
    items: &ItemIndex,
) -> Result<usize> {
    let mut sent = 0;
    let mut first = true;

    for job in jobs {
        if job.discord_id == 0
            || !user_settings::dm_summaries(pool, job.discord_id).await?
        {
            continue;
        }

        if !first {
            tokio::time::sleep(DM_INTERVAL).await;
        }
        first = false;

        let message =
            embeds::summary_dm(&job.summary, heroes.get(job.summary.hero_id), items);
        let user = UserId::new(job.discord_id.cast_unsigned());

        match user.direct_message(http, message).await {
            Ok(_) => sent += 1,
            Err(e) => warn!(
                error = ?e,
                discord_id = job.discord_id,
                "deadlock: failed to DM a match summary; it is not retried"
            ),
        }
    }

    Ok(sent)
}

async fn post_to_guilds(
    http: &Http,
    pool: &PgPool,
    application_id: i64,
    jobs: &[SummaryJob],
    heroes: &HeroIndex,
    items: &ItemIndex,
) -> Result<usize> {
    let mut by_match: BTreeMap<u64, Vec<&SummaryJob>> = BTreeMap::new();
    for job in jobs {
        by_match.entry(job.summary.match_id).or_default().push(job);
    }

    let mut posts = Vec::new();

    for (match_id, match_jobs) in by_match {
        let mut by_guild: BTreeMap<i64, (i64, Vec<&SummaryJob>)> = BTreeMap::new();

        for job in match_jobs {
            let targets =
                summary_channel::targets_for(pool, job.discord_id, application_id)
                    .await?;

            for target in targets {
                by_guild
                    .entry(target.guild_id)
                    .or_insert_with(|| (target.channel_id, Vec::new()))
                    .1
                    .push(job);
            }
        }

        for (guild_id, (channel_id, guild_jobs)) in by_guild {
            if channel_id == 0
                || !guild_posts::claim(pool, guild_id, match_id).await?
            {
                continue;
            }

            let sections: Vec<_> = guild_jobs
                .iter()
                .take(MAX_POST_SECTIONS)
                .map(|job| (&job.summary, job.discord_id))
                .collect();
            let message = embeds::summary_post(&sections, heroes, items);

            posts.push(post(http, guild_id, channel_id, message));
        }
    }

    let sent = futures::stream::iter(posts)
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

    Ok(sent.into_iter().filter(|ok| *ok).count())
}

async fn post(
    http: &Http,
    guild_id: i64,
    channel_id: i64,
    message: CreateMessage<'static>,
) -> bool {
    let channel = ChannelId::new(channel_id.cast_unsigned());

    match channel.widen().send_message(http, message).await {
        Ok(_) => true,
        Err(e) => {
            error!(
                error = ?e,
                guild_id,
                "deadlock: failed to post a match summary; it is not retried"
            );
            false
        },
    }
}
