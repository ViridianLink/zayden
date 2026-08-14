use std::sync::LazyLock;
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};
use moka::future::Cache;
use serenity::all::{GuildId, UserId};
use zayden_app::config::Cooldowns;

pub const STATE_TTL: Duration = Duration::from_hours(24);
const USER_CAPACITY: u64 = 100_000;
const GUILD_CAPACITY: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allowed,
    UserWait(i64),
    GuildWait(i64),
}

#[must_use]
fn remaining(now: Timestamp, last: Timestamp, secs: i32) -> Option<i64> {
    if secs <= 0 {
        return None;
    }

    let deadline =
        last.checked_add(SignedDuration::from_secs(i64::from(secs))).ok()?;
    if deadline <= now {
        return None;
    }

    let left = deadline.duration_since(now);
    let rounded = left.as_secs() + i64::from(left.subsec_nanos() > 0);

    Some(rounded.max(1))
}

#[must_use]
pub fn verdict(
    now: Timestamp,
    last_user: Option<Timestamp>,
    last_guild: Option<Timestamp>,
    limits: Cooldowns,
) -> Verdict {
    if let Some(wait) =
        last_user.and_then(|last| remaining(now, last, limits.user_secs))
    {
        return Verdict::UserWait(wait);
    }

    if let Some(wait) =
        last_guild.and_then(|last| remaining(now, last, limits.guild_secs))
    {
        return Verdict::GuildWait(wait);
    }

    Verdict::Allowed
}

pub struct CooldownState {
    users: Cache<(GuildId, UserId), Timestamp>,
    guilds: Cache<GuildId, Timestamp>,
}

impl CooldownState {
    fn new() -> Self {
        Self {
            users: Cache::builder()
                .time_to_live(STATE_TTL)
                .max_capacity(USER_CAPACITY)
                .build(),
            guilds: Cache::builder()
                .time_to_live(STATE_TTL)
                .max_capacity(GUILD_CAPACITY)
                .build(),
        }
    }

    pub async fn check_and_record(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        limits: Cooldowns,
    ) -> Verdict {
        let now = Timestamp::now();

        let verdict = verdict(
            now,
            self.users.get(&(guild_id, user_id)).await,
            self.guilds.get(&guild_id).await,
            limits,
        );

        if verdict == Verdict::Allowed {
            self.users.insert((guild_id, user_id), now).await;
            self.guilds.insert(guild_id, now).await;
        }

        verdict
    }
}

pub static COOLDOWNS: LazyLock<CooldownState> = LazyLock::new(CooldownState::new);
