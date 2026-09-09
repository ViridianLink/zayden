use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use dashboard::dto::PatreonOutcome;
use dashboard::dto::patreon::OUTCOME_PARAM;
use dashboard::server::auth::{
    GuildAdminContext,
    guild_admin_for,
    session_identity,
};
use dashboard::ui::nav;
use patreon::{
    PATREON_EVENT_HEADER,
    PATREON_SIGNATURE_HEADER,
    POST_PUBLISH,
    PatreonConnection,
};
use rand::RngExt;
use serde::Deserialize;
use tower_cookies::cookie::time::Duration;
use tower_cookies::{Cookie, Cookies};
use tracing::warn;
use zayden_app::state::AppState as ZaydenAppState;

use crate::state::{DiscordState, IntegrationsState};
use crate::web::cookie::{self, SESSION_COOKIE};

const PATREON_STATE_COOKIE: &str = "patreon_oauth_state";

fn redirect(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, location)
        .body(Body::empty())
        .unwrap_or_else(|_e| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn settings_url(guild_id: &str, outcome: PatreonOutcome) -> String {
    let base = nav::settings_href(guild_id, "patreon")
        .unwrap_or_else(|| format!("/guild/{guild_id}/settings"));

    format!("{base}?{OUTCOME_PARAM}={}", outcome.as_key())
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

pub(super) async fn patreon_connect_handler(
    Query(query): Query<ConnectQuery>,
    cookies: Cookies,
    State(app): State<Arc<ZaydenAppState>>,
    State(discord): State<DiscordState>,
    State(integrations): State<Arc<IntegrationsState>>,
) -> Response {
    let Some(patreon_app) = integrations.patreon.clone() else {
        warn!("Patreon connect attempted while PATREON_CLIENT_ID is unset");
        return redirect(&settings_url(&query.guild, PatreonOutcome::Unconfigured));
    };

    // Proves the caller administers this guild before anything is stored.
    if admin(&app, &discord, &cookies, &query.guild).await.is_none() {
        warn!(guild = %query.guild, "Patreon connect rejected: not a guild admin");
        return redirect(&settings_url(&query.guild, PatreonOutcome::Forbidden));
    }

    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes[..]);
    let nonce = dashboard::util::hex_encode(&bytes);

    let oauth_state = format!("{nonce}.{}", query.guild);

    let Ok(url) = patreon_app.authorize_url(&oauth_state) else {
        return redirect(&settings_url(&query.guild, PatreonOutcome::Error));
    };

    let state_cookie =
        cookie::build(PATREON_STATE_COOKIE, nonce, Duration::minutes(10));

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

pub(super) async fn patreon_callback_handler(
    Query(query): Query<CallbackQuery>,
    cookies: Cookies,
    State(app): State<Arc<ZaydenAppState>>,
    State(discord): State<DiscordState>,
    State(integrations): State<Arc<IntegrationsState>>,
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
        return redirect(&settings_url(guild, PatreonOutcome::StateMismatch));
    }

    // The creator declined, or Patreon returned an error instead of a code.
    let Some(code) = query.code.as_deref() else {
        return redirect(&settings_url(guild, PatreonOutcome::Declined));
    };

    let Some(patreon_app) = integrations.patreon.clone() else {
        return redirect(&settings_url(guild, PatreonOutcome::Unconfigured));
    };

    // Re-checked after the round trip: the cookie proves the browser started
    // the flow, this proves it still has the right to bind this guild.
    let Some((context, user_id)) = admin(&app, &discord, &cookies, guild).await
    else {
        warn!(guild, "Patreon callback rejected: not a guild admin");
        return redirect(&settings_url(guild, PatreonOutcome::Forbidden));
    };

    let tokens = match patreon_app.exchange_code(&app.http, code).await {
        Ok(tokens) => tokens,
        Err(e) => {
            warn!(?e, guild, "Patreon token exchange failed");
            return redirect(&settings_url(guild, PatreonOutcome::Error));
        },
    };

    let (campaign_id, creator_name) =
        match patreon::api::fetch_campaign(&app.http, &tokens.access_token).await {
            Ok(campaign) => campaign,
            Err(e) => {
                warn!(?e, guild, "Patreon account has no readable campaign");
                return redirect(&settings_url(guild, PatreonOutcome::NoCampaign));
            },
        };

    // Best-effort: a guild with no webhook still gets its posts from the poll,
    // just up to fifteen minutes later.
    let webhook = match patreon::webhook::register(
        &app.http,
        &tokens.access_token,
        &campaign_id,
        &integrations.patreon_webhook_uri,
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
        &app.db,
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
        return redirect(&settings_url(guild, PatreonOutcome::Error));
    }

    redirect(&settings_url(guild, PatreonOutcome::Connected))
}

pub(super) async fn patreon_webhook_handler(
    State(app): State<Arc<ZaydenAppState>>,
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
    let secrets = match patreon::webhook_secrets(&app.db, &post.campaign_id).await {
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

    match patreon::is_subscribed(&app.db, &post.campaign_id).await {
        Ok(true) => {},
        Ok(false) => return StatusCode::OK,
        Err(e) => {
            warn!(?e, campaign_id = %post.campaign_id, "failed to check Patreon subscribers");
            return StatusCode::OK;
        },
    }

    match patreon::insert_post(&app.db, &post, false).await {
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
        .execute(&app.db)
        .await
    {
        warn!(?e, post_id = %post.id, "failed to notify the bot of a Patreon post");
    }

    StatusCode::OK
}
