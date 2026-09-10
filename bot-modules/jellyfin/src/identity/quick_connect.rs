use std::sync::Arc;
use std::time::Duration;

use jiff::Timestamp;
use serenity::all::{
    CreateActionRow,
    CreateButton,
    EditInteractionResponse,
    Http,
    UserId,
};
use sqlx::PgPool;
use tracing::{debug, error, warn};
use zayden_core::error::Respond as _;

use crate::error::{JellyfinError, Result};
use crate::identity::link::JellyfinLinkRow;
use crate::identity::seer_user;
use crate::runtime::JellyfinRuntime;

pub const LINK_DEADLINE: Duration = Duration::from_mins(5);
const POLL_INTERVAL: Duration = Duration::from_secs(3);

pub const CANCEL_PREFIX: &str = "jellyfin_link_cancel:";

#[derive(Debug, Clone)]
pub struct PendingLink {
    pub secret: String,
    pub code: String,
    pub device_id: String,
    pub expires_at: Timestamp,
}

#[must_use]
pub fn device_id(user_id: UserId) -> String {
    format!("zayden-{user_id}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkOutcome {
    Linked { username: String },
    Cancelled,
    Expired,
}

pub async fn begin(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    user_id: UserId,
) -> Result<PendingLink> {
    if let Some(existing) = JellyfinLinkRow::get(pool, user_id).await? {
        return Err(JellyfinError::AlreadyLinked(existing.jellyfin_username));
    }

    if let Some(pending) = runtime.caches.pending_links.get(&user_id.get()).await {
        debug!(%user_id, "quick connect already in flight; re-showing the code");
        return Ok(pending);
    }

    if !runtime.jellyfin.quick_connect_enabled().await? {
        return Err(JellyfinError::QuickConnectDisabled);
    }

    let device_id = device_id(user_id);
    let state = runtime.jellyfin.quick_connect_initiate(&device_id).await?;

    let pending = PendingLink {
        secret: state.secret,
        code: state.code,
        device_id,
        expires_at: Timestamp::now() + LINK_DEADLINE,
    };

    runtime.caches.pending_links.insert(user_id.get(), pending.clone()).await;

    Ok(pending)
}

pub fn cancel_button(user_id: UserId) -> CreateActionRow<'static> {
    CreateActionRow::buttons(vec![
        CreateButton::new(format!("{CANCEL_PREFIX}{user_id}"))
            .label("Cancel")
            .style(serenity::all::ButtonStyle::Secondary),
    ])
}

pub async fn cancel(runtime: &Arc<JellyfinRuntime>, user_id: UserId) {
    runtime.caches.pending_links.invalidate(&user_id.get()).await;
}

#[must_use]
pub fn prompt(pending: &PendingLink, quick_connect_url: &str) -> String {
    format!(
        "**Link your Jellyfin account**\n\n\
         1. Open {quick_connect_url}\n\
         2. Enter this code: **{}**\n\
         3. Come back here — I will finish up on my own.\n\n\
         The code stops working <t:{}:R>. I never see your password.",
        pending.code,
        pending.expires_at.as_second(),
    )
}

pub fn spawn_poller(
    http: Arc<Http>,
    token: String,
    runtime: Arc<JellyfinRuntime>,
    pool: PgPool,
    user_id: UserId,
    discord_username: String,
) {
    tokio::spawn(async move {
        let outcome = poll(&runtime, &pool, user_id, &discord_username).await;

        let content = match outcome {
            Ok(LinkOutcome::Linked { username }) => format!(
                "Linked to Jellyfin as **{username}**. `/watch request` now files \
                 requests as you, and `/jellyfin streak` can see your history."
            ),
            Ok(LinkOutcome::Cancelled) => "Link cancelled.".to_owned(),
            Ok(LinkOutcome::Expired) => {
                "That code expired. Run `/jellyfin link` again.".to_owned()
            },
            Err(e) => {
                error!(error = ?e, %user_id, "quick connect link failed");
                e.user_message().map_or_else(
                    || {
                        "Something went wrong talking to Jellyfin. Try again in \
                         a minute."
                            .to_owned()
                    },
                    std::borrow::Cow::into_owned,
                )
            },
        };

        runtime.caches.pending_links.invalidate(&user_id.get()).await;

        if let Err(e) = http
            .edit_original_interaction_response(
                &token,
                &EditInteractionResponse::new().content(content).components(vec![]),
                Vec::new(),
            )
            .await
        {
            warn!(error = ?e, %user_id, "could not edit the link response");
        }
    });
}

async fn poll(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    user_id: UserId,
    discord_username: &str,
) -> Result<LinkOutcome> {
    loop {
        tokio::time::sleep(POLL_INTERVAL).await;

        // Gone means cancelled by the button, or aged out of the cache.
        let Some(pending) = runtime.caches.pending_links.get(&user_id.get()).await
        else {
            return Ok(LinkOutcome::Cancelled);
        };

        if Timestamp::now() >= pending.expires_at {
            return Ok(LinkOutcome::Expired);
        }

        let state = runtime.jellyfin.quick_connect_poll(&pending.secret).await?;
        if !state.authenticated {
            continue;
        }

        return finish(runtime, pool, user_id, discord_username, &pending).await;
    }
}

async fn finish(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    user_id: UserId,
    discord_username: &str,
    pending: &PendingLink,
) -> Result<LinkOutcome> {
    let auth = runtime
        .jellyfin
        .authenticate_with_quick_connect(&pending.secret, &pending.device_id)
        .await?;

    let insert = JellyfinLinkRow::insert(
        pool,
        user_id,
        discord_username,
        &auth.user.id,
        &auth.user.name,
    )
    .await;

    // Revoke whatever the handshake created, even when the insert failed. The
    // admin API key covers every later operation, so this token has no further
    // use and must not linger as a standing credential.
    if let Err(e) = runtime.jellyfin.revoke_device(&pending.device_id).await {
        warn!(error = ?e, %user_id, "could not revoke the quick connect device");
    }

    insert?;

    if let Err(e) = seer_user::refresh(runtime, pool, &auth.user.id).await {
        warn!(error = ?e, %user_id, "could not resolve the Jellyseerr account");
    }

    Ok(LinkOutcome::Linked { username: auth.user.name })
}
