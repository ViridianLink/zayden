use ::youtube::websub::{self, Mode};
use ::youtube::{
    YoutubeApp,
    YoutubeChannelRow,
    YoutubeConnection,
    YoutubeError,
    YoutubeRuntime,
};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::{Bytes, headers};
use topcoat::router::response::{IntoResponse, Response};
use topcoat::router::{StatusCode, route};
use tracing::warn;
use zayden_app::events::Channel;
use zayden_app::state::AppState;

use super::access::guild_admin;
use super::fields;
use super::oauth::{
    nonce_matches,
    oauth_state,
    random_hex,
    remember_nonce,
    settings_url,
    split_state,
    take_nonce,
};
use super::reply::{acknowledge, redirect, reject};
use crate::auth::{app_state, web_state};
use crate::guild::dto::youtube::{OUTCOME_PARAM, YoutubeOutcome};

const YOUTUBE_STATE_COOKIE: &str = "youtube_oauth_state";
const SETTINGS_SLUG: &str = "youtube";
const GUILDS_PATH: &str = "/guilds";

fn outcome_url(guild: &str, outcome: YoutubeOutcome) -> String {
    settings_url(guild, SETTINGS_SLUG, OUTCOME_PARAM, outcome.as_key())
}

/// Starts the Google authorization: proves the caller administers the guild,
/// remembers a nonce in a cookie and sends the browser to Google.
#[route(GET "/youtube/connect")]
pub(super) async fn youtube_connect_handler(cx: &Cx) -> Result<Response> {
    let [guild] = match fields::query(cx, ["guild"]) {
        Ok(fields) => fields,
        Err(rejection) => return reject(cx, &rejection),
    };
    let guild = match fields::required_query(guild, "guild") {
        Ok(guild) => guild,
        Err(rejection) => return reject(cx, &rejection),
    };

    let Some(youtube_app) = web_state(cx)?.integrations.youtube.as_ref() else {
        warn!("YouTube connect attempted while YOUTUBE_CLIENT_ID is unset");
        return redirect(cx, &outcome_url(&guild, YoutubeOutcome::Unconfigured));
    };

    if guild_admin(cx, &guild).await.is_none() {
        warn!(guild = %guild, "YouTube connect rejected: not a guild admin");
        return redirect(cx, &outcome_url(&guild, YoutubeOutcome::Forbidden));
    }

    let nonce = random_hex();

    let Ok(url) = youtube_app.authorize_url(&oauth_state(&nonce, &guild)) else {
        return redirect(cx, &outcome_url(&guild, YoutubeOutcome::Error));
    };

    remember_nonce(cx, YOUTUBE_STATE_COOKIE, nonce)?;
    redirect(cx, &url)
}

/// Finishes the Google authorization: checks the nonce and the caller's
/// rights again, then binds the authorised channel to the guild.
#[route(GET "/youtube/callback")]
pub(super) async fn youtube_callback_handler(cx: &Cx) -> Result<Response> {
    let [code, state] = match fields::query(cx, ["code", "state"]) {
        Ok(fields) => fields,
        Err(rejection) => return reject(cx, &rejection),
    };

    let nonce = take_nonce(cx, YOUTUBE_STATE_COOKIE)?;

    let Some((returned_nonce, guild)) = split_state(state.as_deref()) else {
        warn!("YouTube callback rejected: malformed state");
        return redirect(cx, GUILDS_PATH);
    };

    if !nonce_matches(nonce.as_deref(), returned_nonce) {
        warn!(
            guild,
            "YouTube callback rejected: state cookie missing or mismatched"
        );
        return redirect(cx, &outcome_url(guild, YoutubeOutcome::StateMismatch));
    }

    let Some(code) = code.as_deref() else {
        return redirect(cx, &outcome_url(guild, YoutubeOutcome::Declined));
    };

    let web = web_state(cx)?;
    let (Some(youtube_app), Some(runtime)) = (
        web.integrations.youtube.as_ref(),
        web.integrations.youtube_runtime.as_ref(),
    ) else {
        return redirect(cx, &outcome_url(guild, YoutubeOutcome::Unconfigured));
    };

    // Re-checked after the round trip: the cookie proves the browser started
    // the flow, this proves it still has the right to bind this guild.
    let Some(admin) = guild_admin(cx, guild).await else {
        warn!(guild, "YouTube callback rejected: not a guild admin");
        return redirect(cx, &outcome_url(guild, YoutubeOutcome::Forbidden));
    };

    let outcome = connect(
        &web.app,
        youtube_app,
        runtime,
        guild,
        admin.context.guild_id,
        admin.user_id,
        code,
    )
    .await;

    redirect(cx, &outcome_url(guild, outcome))
}

async fn connect(
    app: &AppState,
    youtube_app: &YoutubeApp,
    runtime: &YoutubeRuntime,
    guild: &str,
    guild_id: i64,
    user_id: i64,
    code: &str,
) -> YoutubeOutcome {
    let access_token = match youtube_app.exchange_code(&app.http, code).await {
        Ok(token) => token,
        Err(e) => {
            warn!(?e, guild, "YouTube token exchange failed");
            return YoutubeOutcome::Error;
        },
    };

    let channel = ::youtube::api::fetch_own_channel(&app.http, &access_token).await;

    // The grant only proves ownership; uploads are read with the API key, so
    // nothing is kept that could read the creator's private data later.
    ::youtube::oauth::revoke(&app.http, &access_token).await;

    let channel = match channel {
        Ok(channel) => channel,
        Err(YoutubeError::NoChannel) => return YoutubeOutcome::NoChannel,
        Err(e) => {
            warn!(?e, guild, "failed to read the authorised YouTube channel");
            return YoutubeOutcome::Error;
        },
    };

    let previous = YoutubeConnection::select(&app.db, guild_id)
        .await
        .ok()
        .flatten()
        .map(|c| c.channel_id)
        .filter(|previous| *previous != channel.id);

    let secret = match YoutubeConnection::connect(
        &app.db,
        guild_id,
        &channel,
        user_id,
        &random_hex(),
    )
    .await
    {
        Ok(secret) => secret,
        Err(e) => {
            warn!(?e, guild, "failed to store the YouTube connection");
            return YoutubeOutcome::Error;
        },
    };

    if let Some(previous) = previous {
        ::youtube::release_channel(
            &app.http,
            &app.db,
            Some(&runtime.webhook_uri),
            &previous,
        )
        .await;
    }

    // Best-effort: without push, uploads still arrive on the 15-minute poll.
    if let Err(e) = websub::request(
        &app.http,
        Mode::Subscribe,
        &runtime.webhook_uri,
        &channel.id,
        &secret,
    )
    .await
    {
        warn!(?e, guild, "YouTube WebSub subscription failed; polling only");
    }

    // Absorbs the back catalogue now, so an upload between connecting and the
    // first scheduled poll is announced rather than mistaken for history.
    if let Err(e) = ::youtube::poll::poll_by_id(
        &app.http,
        &app.db,
        &runtime.api_key,
        &channel.id,
    )
    .await
    {
        warn!(?e, guild, "initial YouTube poll failed; the cron will seed it");
    }

    YoutubeOutcome::Connected
}

/// Answers the `WebSub` hub's subscription check: echoes the challenge when the
/// request matches a channel the dashboard wants (or no longer wants).
#[route(GET "/webhooks/youtube")]
pub(super) async fn youtube_verify_handler(cx: &Cx) -> Result<Response> {
    let [channel, mode, topic, challenge, lease_seconds] = match fields::query(cx, [
        "channel",
        "hub.mode",
        "hub.topic",
        "hub.challenge",
        "hub.lease_seconds",
    ]) {
        Ok(fields) => fields,
        Err(rejection) => return reject(cx, &rejection),
    };
    let lease_seconds =
        match fields::integer(lease_seconds.as_deref(), "hub.lease_seconds") {
            Ok(lease) => lease,
            Err(rejection) => return reject(cx, &rejection),
        };

    let (Some(channel), Some(mode), Some(topic), Some(challenge)) =
        (channel, mode, topic, challenge)
    else {
        return StatusCode::NOT_FOUND.into_response(cx);
    };

    if websub::channel_from_topic(&topic).as_deref() != Some(channel.as_str()) {
        return StatusCode::NOT_FOUND.into_response(cx);
    }

    let app = app_state(cx)?;

    let Ok(wanted) =
        YoutubeConnection::channel_has_connections(&app.db, &channel).await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response(cx);
    };

    let recorded = match Mode::from_param(&mode) {
        Some(Mode::Subscribe) if wanted => {
            let lease = lease_seconds.unwrap_or(websub::LEASE_SECONDS);
            YoutubeChannelRow::set_lease(&app.db, &channel, lease).await
        },
        Some(Mode::Unsubscribe) if !wanted => {
            YoutubeChannelRow::clear_lease(&app.db, &channel).await
        },
        _ => return StatusCode::NOT_FOUND.into_response(cx),
    };

    if let Err(e) = recorded {
        warn!(?e, channel, "failed to record a YouTube WebSub verification");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response(cx);
    }

    (StatusCode::OK, challenge).into_response(cx)
}

/// Receives a `WebSub` upload notification. Every outcome is a `200` so the
/// hub does not retry.
#[route(POST "/webhooks/youtube")]
pub(super) async fn youtube_notify_handler(
    cx: &Cx,
    body: Bytes,
) -> Result<Response> {
    let [channel] = match fields::query(cx, ["channel"]) {
        Ok(fields) => fields,
        Err(rejection) => return reject(cx, &rejection),
    };

    if let Some(channel_id) = channel {
        let signature = headers(cx)
            .get(websub::SIGNATURE_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();

        notify(app_state(cx)?, &channel_id, signature, &body).await;
    }

    acknowledge(cx)
}

async fn notify(app: &AppState, channel_id: &str, signature: &str, body: &[u8]) {
    let secret = match YoutubeChannelRow::select(&app.db, channel_id).await {
        Ok(Some(channel)) => channel.websub_secret,
        Ok(None) => return,
        Err(e) => {
            warn!(?e, channel_id, "failed to load the YouTube WebSub secret");
            return;
        },
    };

    if !websub::verify(body, signature, &secret) {
        warn!(channel_id, "YouTube notification rejected: signature mismatch");
        return;
    }

    if !matches!(::youtube::is_subscribed(&app.db, channel_id).await, Ok(true)) {
        return;
    }

    // The notification is only a wake-up: the bot re-reads the uploads
    // playlist, so edits and deletions the hub also reports are harmless.
    if let Err(e) = Channel::YoutubeUpload.notify(&app.db, channel_id).await {
        warn!(?e, channel_id, "failed to notify the bot of a YouTube upload");
    }
}
