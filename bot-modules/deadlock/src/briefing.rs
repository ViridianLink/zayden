use std::collections::BTreeMap;

use jiff::{SignedDuration, Timestamp};
use serenity::all::{Http, UserId};
use sqlx::PgPool;
use tracing::warn;

use crate::db::briefings::{self, RecentHero};
use crate::db::patches;
use crate::diff::PatchDiff;
use crate::error::Result;
use crate::{deliver, embeds};

pub const BRIEFING_WINDOW: SignedDuration = SignedDuration::from_hours(48);
pub const RECENT_WINDOW: SignedDuration = SignedDuration::from_hours(24 * 14);
pub const MAX_HEROES: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient {
    pub discord_id: i64,
    pub hero_ids: Vec<u32>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct BriefingOutcome {
    pub patches: usize,
    pub sent: usize,
}

#[must_use]
pub fn recipients(recent: &[RecentHero], diff: &PatchDiff) -> Vec<Recipient> {
    let mut by_player: BTreeMap<i64, Vec<&RecentHero>> = BTreeMap::new();
    for row in recent {
        by_player.entry(row.discord_id).or_default().push(row);
    }

    by_player
        .into_iter()
        .filter_map(|(discord_id, mut rows)| {
            rows.sort_by(|a, b| {
                b.games
                    .cmp(&a.games)
                    .then_with(|| b.last_played.cmp(&a.last_played))
                    .then_with(|| a.hero_id.cmp(&b.hero_id))
            });

            let hero_ids: Vec<u32> = rows
                .iter()
                .take(MAX_HEROES)
                .map(|row| row.hero_id)
                .filter(|hero_id| {
                    diff.heroes.iter().any(|hero| hero.hero_id == *hero_id)
                })
                .collect();

            (!hero_ids.is_empty()).then_some(Recipient { discord_id, hero_ids })
        })
        .collect()
}

pub async fn run(
    http: &Http,
    pool: &PgPool,
    now: Timestamp,
) -> Result<BriefingOutcome> {
    let since = now.checked_sub(BRIEFING_WINDOW).unwrap_or(Timestamp::MIN);
    let mut outcome = BriefingOutcome::default();
    let mut first = true;

    for patch in patches::awaiting_briefing(pool, since).await? {
        let Some(diff) = patch.diff.as_ref().filter(|diff| !diff.heroes.is_empty())
        else {
            patches::mark_briefed(pool, &patch.guid).await?;
            outcome.patches += 1;
            continue;
        };

        let window_start =
            patch.published_at.checked_sub(RECENT_WINDOW).unwrap_or(Timestamp::MIN);
        let recent = briefings::recent_heroes(
            pool,
            &patch.guid,
            patch.published_at,
            window_start,
        )
        .await?;

        for recipient in recipients(&recent, diff) {
            if recipient.discord_id == 0
                || !briefings::claim(pool, &patch.guid, recipient.discord_id).await?
            {
                continue;
            }

            if !first {
                tokio::time::sleep(deliver::DM_INTERVAL).await;
            }
            first = false;

            let message = embeds::briefing_dm(&patch, &recipient.hero_ids);
            let user = UserId::new(recipient.discord_id.cast_unsigned());

            match user.direct_message(http, message).await {
                Ok(_) => outcome.sent += 1,
                Err(e) => warn!(
                    error = ?e,
                    discord_id = recipient.discord_id,
                    "deadlock: failed to DM a patch briefing; it is not retried"
                ),
            }
        }

        patches::mark_briefed(pool, &patch.guid).await?;
        outcome.patches += 1;
    }

    Ok(outcome)
}
