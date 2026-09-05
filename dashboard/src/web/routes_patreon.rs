use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use dashboard::server::auth::{
    GuildAdminContext,
    guild_admin_for,
    session_identity,
};
use patreon::oauth::PatreonApp;
use patreon::{
    PATREON_EVENT_HEADER,
    PATREON_SIGNATURE_HEADER,
    POST_PUBLISH,
    PatreonConnection,
};
use rand::RngExt;
use serde::Deserialize;
use tower_cookies::cookie::SameSite;
use tower_cookies::cookie::time::Duration;
use tower_cookies::{Cookie, Cookies};
use tracing::warn;

use crate::WebState;
use crate::web::SESSION_COOKIE;

const PATREON_STATE_COOKIE: &str = "patreon_oauth_state";

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, location)
        .body(Body::empty())
        .unwrap_or_else(|_e| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn settings_url(guild_id: &str, outcome: &str) -> String {
    format!("/guilds/{guild_id}/patreon?patreon={outcome}")
}

fn app(state: &WebState) -> Option<PatreonApp> {
    state.patreon.clone()
}

async fn admin(
    state: &WebState,
    cookies: &Cookies,
    guild: &str,
) -> Option<(GuildAdminContext, i64)> {
    let token = cookies.get(SESSION_COOKIE).map(|c| c.value().to_owned())?;
    let identity = session_identity(&state.app.db, &token).await.ok()?;
    let user_id = identity.user_id;

    let context =
        guild_admin_for(&state.app.db, &identity, guild, Some(&state.discord_http))
            .await
            .ok()?;

    Some((context, user_id))
}

#[derive(Deserialize)]
pub(super) struct ConnectQuery {
    guild: String,
}

pub(super) async fn patreon_connect_handler(
    Query(query): Query<ConnectQuery>,
    cookies: Cookies,
    State(state): State<WebState>,
) -> Response {
    let Some(app) = app(&state) else {
        warn!("Patreon connect attempted while PATREON_CLIENT_ID is unset");
        return redirect(&settings_url(&query.guild, "unconfigured"));
    };

    // Proves the caller administers this guild before anything is stored.
    if admin(&state, &cookies, &query.guild).await.is_none() {
        warn!(guild = %query.guild, "Patreon connect rejected: not a guild admin");
        return redirect(&settings_url(&query.guild, "forbidden"));
    }

    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    let nonce = dashboard::util::hex_encode(&bytes);

    let oauth_state = format!("{nonce}.{}", query.guild);

    let Ok(url) = app.authorize_url(&oauth_state) else {
        return redirect(&settings_url(&query.guild, "error"));
    };

    let cookie = Cookie::build((PATREON_STATE_COOKIE, nonce))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax)
        .max_age(Duration::minutes(10));

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, url)
        .header(header::SET_COOKIE, cookie.to_string())
        .body(Body::empty())
        .unwrap_or_else(|_e| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[derive(Deserialize)]
pub(super) struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

pub(super) async fn patreon_callback_handler(
    Query(query): Query<CallbackQuery>,
    cookies: Cookies,
    State(state): State<WebState>,
) -> Response {
    let nonce = cookies.get(PATREON_STATE_COOKIE).map(|c| c.value().to_owned());
    let mut removal = Cookie::from(PATREON_STATE_COOKIE);
    removal.set_path("/");
    cookies.remove(removal);

    let Some((returned_nonce, guild)) =
        query.state.as_deref().and_then(|s| s.split_once('.'))
    else {
        warn!("Patreon callback rejected: malformed state");
        return redirect("/guilds");
    };

    if !matches!(&nonce, Some(n) if n == returned_nonce && !n.is_empty()) {
        warn!(
            guild,
            "Patreon callback rejected: state cookie missing or mismatched"
        );
        return redirect(&settings_url(guild, "state_mismatch"));
    }

    // The creator declined, or Patreon returned an error instead of a code.
    let Some(code) = query.code.as_deref() else {
        return redirect(&settings_url(guild, "declined"));
    };

    let Some(app) = app(&state) else {
        return redirect(&settings_url(guild, "unconfigured"));
    };

    // Re-checked after the round trip: the cookie proves the browser started
    // the flow, this proves it still has the right to bind this guild.
    let Some((context, user_id)) = admin(&state, &cookies, guild).await else {
        warn!(guild, "Patreon callback rejected: not a guild admin");
        return redirect(&settings_url(guild, "forbidden"));
    };

    let tokens = match app.exchange_code(&state.app.http, code).await {
        Ok(tokens) => tokens,
        Err(e) => {
            warn!(?e, guild, "Patreon token exchange failed");
            return redirect(&settings_url(guild, "error"));
        },
    };

    let (campaign_id, creator_name) =
        match patreon::api::fetch_campaign(&state.app.http, &tokens.access_token)
            .await
        {
            Ok(campaign) => campaign,
            Err(e) => {
                warn!(?e, guild, "Patreon account has no readable campaign");
                return redirect(&settings_url(guild, "no_campaign"));
            },
        };

    // Best-effort: a guild with no webhook still gets its posts from the poll,
    // just up to fifteen minutes later.
    let webhook = match patreon::webhook::register(
        &state.app.http,
        &tokens.access_token,
        &campaign_id,
        &state.patreon_webhook_uri,
    )
    .await
    {
        Ok(webhook) => Some(webhook),
        Err(e) => {
            warn!(?e, guild, "Patreon webhook registration failed; polling only");
            None
        },
    };

    let stored = PatreonConnection::connect(
        &state.app.db,
        context.guild_id,
        &campaign_id,
        creator_name.as_deref(),
        user_id,
        &tokens,
        webhook.as_ref().map(|(id, secret)| (id.as_str(), secret.as_str())),
    )
    .await;

    if let Err(e) = stored {
        warn!(?e, guild, "failed to store the Patreon connection");
        return redirect(&settings_url(guild, "error"));
    }

    redirect(&settings_url(guild, "connected"))
}

#[derive(Deserialize)]
pub(super) struct DisconnectQuery {
    guild: String,
}

pub(super) async fn patreon_disconnect_handler(
    Query(query): Query<DisconnectQuery>,
    cookies: Cookies,
    State(state): State<WebState>,
) -> Response {
    let Some((context, _user_id)) = admin(&state, &cookies, &query.guild).await
    else {
        warn!(guild = %query.guild, "Patreon disconnect rejected: not a guild admin");
        return redirect(&settings_url(&query.guild, "forbidden"));
    };

    let connection = PatreonConnection::select(&state.app.db, context.guild_id)
        .await
        .ok()
        .flatten();

    if let (Some(app), Some(connection)) = (app(&state), connection.as_ref())
        && let Some(webhook_id) = connection.webhook_id.as_deref()
        && let Ok(token) = patreon::oauth::access_token(
            &state.app.db,
            &state.app.http,
            &app,
            connection,
        )
        .await
    {
        patreon::webhook::unregister(&state.app.http, &token, webhook_id).await;
    }

    if let Err(e) = PatreonConnection::delete(&state.app.db, context.guild_id).await
    {
        warn!(?e, guild = %query.guild, "failed to delete the Patreon connection");
        return redirect(&settings_url(&query.guild, "error"));
    }

    redirect(&settings_url(&query.guild, "disconnected"))
}

pub(super) async fn patreon_webhook_handler(
    State(state): State<WebState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let event = headers
        .get(PATREON_EVENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if event != POST_PUBLISH {
        return StatusCode::OK;
    }

    let post = match patreon::webhook::parse_post(&body) {
        Ok(post) => post,
        Err(e) => {
            warn!(?e, "failed to parse Patreon webhook payload");
            return StatusCode::OK;
        },
    };

    // The campaign in an unverified payload only selects which secrets to try;
    // a forged one simply fails every signature check below.
    let secrets = match patreon::webhook_secrets(&state.app.db, &post.campaign_id)
        .await
    {
        Ok(secrets) => secrets,
        Err(e) => {
            warn!(?e, campaign_id = %post.campaign_id, "failed to load webhook secrets");
            return StatusCode::OK;
        },
    };

    let signature = headers
        .get(PATREON_SIGNATURE_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if !patreon::webhook::verify_any(&body, signature, &secrets) {
        warn!(campaign_id = %post.campaign_id, "Patreon webhook rejected: signature mismatch");
        return StatusCode::OK;
    }

    match patreon::is_subscribed(&state.app.db, &post.campaign_id).await {
        Ok(true) => {},
        Ok(false) => return StatusCode::OK,
        Err(e) => {
            warn!(?e, campaign_id = %post.campaign_id, "failed to check Patreon subscribers");
            return StatusCode::OK;
        },
    }

    match patreon::insert_post(&state.app.db, &post, false).await {
        // Already stored by a poll or an earlier delivery; the announce path
        // has it either way.
        Ok(false) => return StatusCode::OK,
        Ok(true) => {},
        Err(e) => {
            warn!(?e, post_id = %post.id, "failed to store Patreon post");
            return StatusCode::OK;
        },
    }

    // The bot is a separate process, so the wake-up travels over the same
    // Postgres LISTEN/NOTIFY bus the settings cache uses.
    if let Err(e) = sqlx::query!("SELECT pg_notify('patreon_post', $1)", post.id)
        .execute(&state.app.db)
        .await
    {
        warn!(?e, post_id = %post.id, "failed to notify the bot of a Patreon post");
    }

    StatusCode::OK
}
