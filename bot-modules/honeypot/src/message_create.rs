use std::sync::Arc;
use std::time::Duration;

use serenity::all::{ChannelId, Context, GuildId, Message, UserId};
use tracing::{debug, error, warn};
use zayden_app::config::HoneypotSettingsRow;
use zayden_core::as_u64;

use crate::error::Result;
use crate::guard::GUARD;
use crate::policy::{self, ExemptionPolicy};

const PURGE_WINDOW: Duration = Duration::from_hours(24);
const REASON: &str = "Honeypot: posted in the honeypot channel";

fn purge_seconds() -> u32 {
    u32::try_from(PURGE_WINDOW.as_secs()).unwrap_or(u32::MAX)
}

#[derive(Debug, Clone)]
pub struct HoneypotHit {
    pub user_id: UserId,
    pub username: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
}

pub async fn message_create(
    ctx: &Context,
    msg: &Message,
    settings: &Arc<HoneypotSettingsRow>,
) -> Result<Option<HoneypotHit>> {
    let Some(guild_id) = msg.guild_id else {
        return Ok(None);
    };

    let Some(honeypot_channel) = settings.channel_id else {
        return Ok(None);
    };

    let channel_id = msg.channel_id.expect_channel();
    if channel_id.get() != as_u64(honeypot_channel) {
        return Ok(None);
    }

    let author_id = msg.author.id;

    // A flood arrives faster than the ban lands; act once per offender.
    if !GUARD.claim(guild_id, author_id).await {
        debug!(%guild_id, %author_id, "honeypot already actioned this user");
        return Ok(None);
    }

    let member_roles: &[_] = msg.member.as_ref().map_or(&[], |member| &member.roles);

    let facts = match GUARD.facts(ctx, guild_id).await {
        Ok(facts) => facts,
        Err(e) => {
            GUARD.release(guild_id, author_id).await;
            return Err(e);
        },
    };

    let policy = ExemptionPolicy::from(settings.as_ref());
    if policy::is_exempt(author_id, member_roles, &facts, &policy) {
        debug!(%guild_id, %author_id, "honeypot post from an exempt member");
        GUARD.release(guild_id, author_id).await;
        return Ok(None);
    }

    if let Err(e) =
        guild_id.ban(&ctx.http, author_id, purge_seconds(), Some(REASON)).await
    {
        GUARD.release(guild_id, author_id).await;
        return Err(e.into());
    }

    if let Err(e) = guild_id.unban(&ctx.http, author_id, Some(REASON)).await {
        error!(
            %guild_id,
            %author_id,
            error = %e,
            "honeypot soft-ban left a standing ban: unban failed",
        );
    }

    warn!(
        %guild_id,
        %author_id,
        username = %msg.author.name,
        %channel_id,
        "honeypot soft-banned a member",
    );

    Ok(Some(HoneypotHit {
        user_id: author_id,
        username: msg.author.name.to_string(),
        guild_id,
        channel_id,
    }))
}
