use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use dashboard::dto::YoutubeOutcome;
use dashboard::dto::youtube::OUTCOME_PARAM;
use dashboard::server::auth::{
    GuildAdminContext,
    guild_admin_for,
    session_identity,
};
use dashboard::ui::nav;
use rand::RngExt;
use serde::Deserialize;
use tower_cookies::cookie::time::Duration;
use tower_cookies::{Cookie, Cookies};
use tracing::warn;
use youtube::websub::{self, Mode};
use youtube::{YoutubeChannelRow, YoutubeConnection, YoutubeError};
use zayden_app::state::AppState as ZaydenAppState;

use crate::state::{DiscordState, IntegrationsState};
use crate::web::cookie::{self, SESSION_COOKIE};

const YOUTUBE_STATE_COOKIE: &str = "youtube_oauth_state";

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, location)
        .body(Body::empty())
        .unwrap_or_else(|_e| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn settings_url(guild_id: &str, outcome: YoutubeOutcome) -> String {
    let base = nav::settings_href(guild_id, "youtube")
        .unwrap_or_else(|| format!("/guild/{guild_id}/settings"));

    format!("{base}?{OUTCOME_PARAM}={}", outcome.as_key())
}

fn random_hex() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    dashboard::util::hex_encode(&bytes)
}

async fn admin(
    app: &ZaydenAppState,
    discord: &DiscordState,
    cookies: &Cookies,
    guild: &str,
) -> Option<(GuildAdminContext, i64)> {
    let token = cookies.get(SESSION_COOKIE).map(|c| c.value().to_owned())?;
    let identity = session_identity(&app.db, &token).await.ok()?;
    let user_id = identity.user_id;

    let context = guild_admin_for(
        &app.db,
        &identity,
        guild,
        Some(&discord.http),
        Some(&discord.user_guilds),
    )
    .await
    .ok()?;

    Some((context, user_id))
}

#[derive(Deserialize)]
pub(super) struct ConnectQuery {
    guild: String,
}

pub(super) async fn youtube_connect_handler(
    Query(query): Query<ConnectQuery>,
    cookies: Cookies,
    State(app): State<Arc<ZaydenAppState>>,
    State(discord): State<DiscordState>,
    State(integrations): State<Arc<IntegrationsState>>,
) -> Response {
    let Some(youtube_app) = integrations.youtube.clone() else {
        warn!("YouTube connect attempted while YOUTUBE_CLIENT_ID is unset");
        return redirect(&settings_url(&query.guild, YoutubeOutcome::Unconfigured));
    };

    if admin(&app, &discord, &cookies, &query.guild).await.is_none() {
        warn!(guild = %query.guild, "YouTube connect rejected: not a guild admin");
        return redirect(&settings_url(&query.guild, YoutubeOutcome::Forbidden));
    }

    let nonce = random_hex();
    let oauth_state = format!("{nonce}.{}", query.guild);

    let Ok(url) = youtube_app.authorize_url(&oauth_state) else {
        return redirect(&settings_url(&query.guild, YoutubeOutcome::Error));
    };

    let state_cookie =
        cookie::build(YOUTUBE_STATE_COOKIE, nonce, Duration::minutes(10));

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, url)
        .header(header::SET_COOKIE, state_cookie.to_string())
        .body(Body::empty())
        .unwrap_or_else(|_e| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[derive(Deserialize)]
pub(super) struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

pub(super) async fn youtube_callback_handler(
    Query(query): Query<CallbackQuery>,
    cookies: Cookies,
    State(app): State<Arc<ZaydenAppState>>,
    State(discord): State<DiscordState>,
    State(integrations): State<Arc<IntegrationsState>>,
) -> Response {
    let nonce = cookies.get(YOUTUBE_STATE_COOKIE).map(|c| c.value().to_owned());
    let mut removal = Cookie::from(YOUTUBE_STATE_COOKIE);
    removal.set_path("/");
    cookies.remove(removal);

    let Some((returned_nonce, guild)) =
        query.state.as_deref().and_then(|s| s.split_once('.'))
    else {
        warn!("YouTube callback rejected: malformed state");
        return redirect("/guilds");
    };

    if !matches!(&nonce, Some(n) if n == returned_nonce && !n.is_empty()) {
        warn!(
            guild,
            "YouTube callback rejected: state cookie missing or mismatched"
        );
        return redirect(&settings_url(guild, YoutubeOutcome::StateMismatch));
    }

    let Some(code) = query.code.as_deref() else {
        return redirect(&settings_url(guild, YoutubeOutcome::Declined));
    };

    let (Some(youtube_app), Some(runtime)) =
        (integrations.youtube.clone(), integrations.youtube_runtime.clone())
    else {
        return redirect(&settings_url(guild, YoutubeOutcome::Unconfigured));
    };

    // Re-checked after the round trip: the cookie proves the browser started
    // the flow, this proves it still has the right to bind this guild.
    let Some((context, user_id)) = admin(&app, &discord, &cookies, guild).await
    else {
        warn!(guild, "YouTube callback rejected: not a guild admin");
        return redirect(&settings_url(guild, YoutubeOutcome::Forbidden));
    };

    let access_token = match youtube_app.exchange_code(&app.http, code).await {
        Ok(token) => token,
        Err(e) => {
            warn!(?e, guild, "YouTube token exchange failed");
            return redirect(&settings_url(guild, YoutubeOutcome::Error));
        },
    };

    let channel = youtube::api::fetch_own_channel(&app.http, &access_token).await;

    // The grant only proves ownership; uploads are read with the API key, so
    // nothing is kept that could read the creator's private data later.
    youtube::oauth::revoke(&app.http, &access_token).await;

    let channel = match channel {
        Ok(channel) => channel,
        Err(YoutubeError::NoChannel) => {
            return redirect(&settings_url(guild, YoutubeOutcome::NoChannel));
        },
        Err(e) => {
            warn!(?e, guild, "failed to read the authorised YouTube channel");
            return redirect(&settings_url(guild, YoutubeOutcome::Error));
        },
    };

    let previous = YoutubeConnection::select(&app.db, context.guild_id)
        .await
        .ok()
        .flatten()
        .map(|c| c.channel_id)
        .filter(|previous| *previous != channel.id);

    let secret = match YoutubeConnection::connect(
        &app.db,
        context.guild_id,
        &channel,
        user_id,
        &random_hex(),
    )
    .await
    {
        Ok(secret) => secret,
        Err(e) => {
            warn!(?e, guild, "failed to store the YouTube connection");
            return redirect(&settings_url(guild, YoutubeOutcome::Error));
        },
    };

    if let Some(previous) = previous {
        youtube::release_channel(
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
    if let Err(e) =
        youtube::poll::poll_by_id(&app.http, &app.db, &runtime.api_key, &channel.id)
            .await
    {
        warn!(?e, guild, "initial YouTube poll failed; the cron will seed it");
    }

    redirect(&settings_url(guild, YoutubeOutcome::Connected))
}

#[derive(Deserialize)]
pub(super) struct VerifyQuery {
    channel: Option<String>,
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.topic")]
    topic: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
    #[serde(rename = "hub.lease_seconds")]
    lease_seconds: Option<i64>,
}

pub(super) async fn youtube_verify_handler(
    Query(query): Query<VerifyQuery>,
    State(app): State<Arc<ZaydenAppState>>,
) -> Response {
    let (Some(channel), Some(mode), Some(topic), Some(challenge)) =
        (query.channel, query.mode, query.topic, query.challenge)
    else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if websub::channel_from_topic(&topic).as_deref() != Some(channel.as_str()) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Ok(wanted) =
        YoutubeConnection::channel_has_connections(&app.db, &channel).await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let recorded = match Mode::from_param(&mode) {
        Some(Mode::Subscribe) if wanted => {
            let lease = query.lease_seconds.unwrap_or(websub::LEASE_SECONDS);
            YoutubeChannelRow::set_lease(&app.db, &channel, lease).await
        },
        Some(Mode::Unsubscribe) if !wanted => {
            YoutubeChannelRow::clear_lease(&app.db, &channel).await
        },
        _ => return StatusCode::NOT_FOUND.into_response(),
    };

    if let Err(e) = recorded {
        warn!(?e, channel, "failed to record a YouTube WebSub verification");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    (StatusCode::OK, challenge).into_response()
}

#[derive(Deserialize)]
pub(super) struct NotifyQuery {
    channel: Option<String>,
}

pub(super) async fn youtube_notify_handler(
    Query(query): Query<NotifyQuery>,
    State(app): State<Arc<ZaydenAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let Some(channel_id) = query.channel else { return StatusCode::OK };

    let secret = match YoutubeChannelRow::select(&app.db, &channel_id).await {
        Ok(Some(channel)) => channel.websub_secret,
        Ok(None) => return StatusCode::OK,
        Err(e) => {
            warn!(?e, channel_id, "failed to load the YouTube WebSub secret");
            return StatusCode::OK;
        },
    };

    let signature = headers
        .get(websub::SIGNATURE_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if !websub::verify(&body, signature, &secret) {
        warn!(channel_id, "YouTube notification rejected: signature mismatch");
        return StatusCode::OK;
    }

    if !matches!(youtube::is_subscribed(&app.db, &channel_id).await, Ok(true)) {
        return StatusCode::OK;
    }

    // The notification is only a wake-up: the bot re-reads the uploads
    // playlist, so edits and deletions the hub also reports are harmless.
    if let Err(e) =
        sqlx::query!("SELECT pg_notify('youtube_upload', $1)", channel_id)
            .execute(&app.db)
            .await
    {
        warn!(?e, channel_id, "failed to notify the bot of a YouTube upload");
    }

    StatusCode::OK
}
