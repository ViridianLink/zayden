use std::sync::Arc;
use std::time::Duration;

use serenity::all::{ChannelId, Context, GuildId, Message, RoleId, UserId};
use tracing::{debug, error, warn};
use zayden_app::config::HoneypotSettingsRow;
use zayden_core::as_u64;
use zayden_core::retry::{RetryBudget, retry_transient};

use crate::error::Result;
use crate::guard::GUARD;
use crate::policy::{self, ExemptionPolicy, GuildFacts};

pub const BAN_REASON: &str = "Honeypot: posted in the honeypot channel";
const UNBAN_RETRY: RetryBudget = RetryBudget::new(3, Duration::from_millis(250));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoneypotOutcome {
    SoftBanned,
    BanStanding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Spare,
    Ban { purge_seconds: u32 },
}

#[must_use]
pub fn is_decoy_hit(channel: ChannelId, honeypot_channel: Option<i64>) -> bool {
    honeypot_channel.is_some_and(|armed| channel.get() == as_u64(armed))
}

#[must_use]
pub fn decide(
    author_id: UserId,
    member_roles: &[RoleId],
    facts: &GuildFacts,
    policy: &ExemptionPolicy,
    purge_seconds: u32,
) -> Action {
    if policy::is_exempt(author_id, member_roles, facts, policy) {
        Action::Spare
    } else {
        Action::Ban { purge_seconds }
    }
}

#[must_use]
pub const fn outcome_of<E>(unban: &std::result::Result<(), E>) -> HoneypotOutcome {
    match unban {
        Ok(()) => HoneypotOutcome::SoftBanned,
        Err(_) => HoneypotOutcome::BanStanding,
    }
}

#[derive(Debug, Clone)]
pub struct HoneypotHit {
    pub user_id: UserId,
    pub username: String,
    pub guild_id: GuildId,
    pub outcome: HoneypotOutcome,
}

pub async fn message_create(
    ctx: &Context,
    msg: &Message,
    settings: &Arc<HoneypotSettingsRow>,
) -> Result<Option<HoneypotHit>> {
    let Some(guild_id) = msg.guild_id else {
        return Ok(None);
    };

    let channel_id = msg.channel_id.expect_channel();
    if !is_decoy_hit(channel_id, settings.channel_id) {
        return Ok(None);
    }

    let author_id = msg.author.id;

    // A flood arrives faster than the ban lands; act once per offender.
    if !GUARD.claim(guild_id, author_id).await {
        debug!(%guild_id, %author_id, "honeypot already actioned this user");
        return Ok(None);
    }

    let facts = match GUARD.facts(ctx, guild_id).await {
        Ok(facts) => facts,
        Err(e) => {
            GUARD.release(guild_id, author_id).await;
            return Err(e);
        },
    };

    let member_roles: &[_] = msg.member.as_ref().map_or(&[], |member| &member.roles);
    let policy = ExemptionPolicy::from(settings.as_ref());
    let purge_seconds = settings.purge_seconds_u32();

    let purge_seconds =
        match decide(author_id, member_roles, &facts, &policy, purge_seconds) {
            Action::Spare => {
                debug!(%guild_id, %author_id, "honeypot post from an exempt member");
                GUARD.release(guild_id, author_id).await;
                return Ok(None);
            },
            Action::Ban { purge_seconds } => purge_seconds,
        };

    if let Err(e) =
        guild_id.ban(&ctx.http, author_id, purge_seconds, Some(BAN_REASON)).await
    {
        GUARD.release(guild_id, author_id).await;
        return Err(e.into());
    }

    let unban = retry_transient(UNBAN_RETRY, || {
        guild_id.unban(&ctx.http, author_id, Some(BAN_REASON))
    })
    .await;

    let outcome = outcome_of(&unban);
    match &unban {
        Ok(()) => warn!(
            %guild_id,
            %author_id,
            username = %msg.author.name,
            %channel_id,
            "honeypot soft-banned a member",
        ),
        Err(e) => error!(
            %guild_id,
            %author_id,
            username = %msg.author.name,
            %channel_id,
            error = %e,
            attempts = UNBAN_RETRY.attempts,
            "honeypot unban failed after retries: the ban is still standing",
        ),
    }

    Ok(Some(HoneypotHit {
        user_id: author_id,
        username: msg.author.name.to_string(),
        guild_id,
        outcome,
    }))
}
