use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use jiff::{Span, Timestamp};
use serenity::all::{Context, GuildId};
use sqlx::PgPool;
use tracing::error;
use zayden_core::{CronJobData, as_i64};

use crate::party::cron::{self, PROVISION_LEAD};
use crate::party::lifecycle;
use crate::party::row::PartyRow;

pub async fn guild_create<Data: CronJobData>(
    ctx: &Context,
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    guild_id: GuildId,
) {
    let parties = match PartyRow::live(pool).await {
        Ok(parties) => parties,
        Err(e) => {
            error!(error = ?e, "jellyfin: could not load live parties");
            return;
        },
    };

    let guild = as_i64(guild_id.get());
    let now = Timestamp::now();

    for party in parties.iter().filter(|p| p.guild_id == guild) {
        // Overdue cleanup runs immediately rather than waiting for a schedule
        // that has already passed.
        if party.cleanup_after() <= now {
            if let Err(e) = lifecycle::cleanup(runtime, pool, party.id).await {
                error!(error = ?e, party_id = party.id, "catch-up cleanup failed");
            }
            continue;
        }

        // The provisioning window opened while we were down: do it now, so
        // guests are not left without the access they were promised.
        let provision_at = party.starts_at() - Span::new().minutes(PROVISION_LEAD);
        if provision_at <= now
            && party.state == "scheduled"
            && let Err(e) =
                lifecycle::provision(&ctx.http, runtime, pool, party.id).await
        {
            error!(error = ?e, party_id = party.id, "catch-up provisioning failed");
        }

        // Whatever is still in the future gets its jobs back. Missed reminders
        // are deliberately not replayed — a "starts in 24 hours" ping sent after
        // the party is worse than silence.
        cron::create_jobs::<Data>(ctx, runtime, party).await;
    }
}
