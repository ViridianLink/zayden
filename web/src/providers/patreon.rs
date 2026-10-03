use ::patreon::oauth::PatreonApp;
use ::patreon::{
    PATREON_EVENT_HEADER,
    PATREON_SIGNATURE_HEADER,
    POST_PUBLISH,
    PatreonConnection,
};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::{Bytes, headers};
use topcoat::router::response::Response;
use topcoat::router::route;
use tracing::{info, warn};
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
use crate::guild::dto::patreon::{OUTCOME_PARAM, PatreonOutcome};

const PATREON_STATE_COOKIE: &str = "patreon_oauth_state";
const SETTINGS_SLUG: &str = "patreon";
const GUILDS_PATH: &str = "/guilds";

fn outcome_url(guild: &str, outcome: PatreonOutcome) -> String {
    settings_url(guild, SETTINGS_SLUG, OUTCOME_PARAM, outcome.as_key())
}

/// Starts the Patreon authorization: proves the caller administers the
/// guild, remembers a nonce in a cookie and sends the browser to Patreon.
#[route(GET "/patreon/connect")]
pub(super) async fn patreon_connect_handler(cx: &Cx) -> Result<Response> {
    let [guild] = match fields::query(cx, ["guild"]) {
        Ok(fields) => fields,
        Err(rejection) => return reject(cx, &rejection),
    };
    let guild = match fields::required_query(guild, "guild") {
        Ok(guild) => guild,
        Err(rejection) => return reject(cx, &rejection),
    };

    let Some(patreon_app) = web_state(cx)?.integrations.patreon.as_ref() else {
        warn!("Patreon connect attempted while PATREON_CLIENT_ID is unset");
        return redirect(cx, &outcome_url(&guild, PatreonOutcome::Unconfigured));
    };

    if guild_admin(cx, &guild).await.is_none() {
        warn!(guild = %guild, "Patreon connect rejected: not a guild admin");
        return redirect(cx, &outcome_url(&guild, PatreonOutcome::Forbidden));
    }

    let nonce = random_hex();

    let Ok(url) = patreon_app.authorize_url(&oauth_state(&nonce, &guild)) else {
        return redirect(cx, &outcome_url(&guild, PatreonOutcome::Error));
    };

    remember_nonce(cx, PATREON_STATE_COOKIE, nonce)?;
    redirect(cx, &url)
}

/// Finishes the Patreon authorization: checks the nonce and the caller's
/// rights again, then binds the creator's campaign to the guild.
#[route(GET "/patreon/callback")]
pub(super) async fn patreon_callback_handler(cx: &Cx) -> Result<Response> {
    let [code, state] = match fields::query(cx, ["code", "state"]) {
        Ok(fields) => fields,
        Err(rejection) => return reject(cx, &rejection),
    };

    let nonce = take_nonce(cx, PATREON_STATE_COOKIE)?;

    let Some((returned_nonce, guild)) = split_state(state.as_deref()) else {
        warn!("Patreon callback rejected: malformed state");
        return redirect(cx, GUILDS_PATH);
    };

    if !nonce_matches(nonce.as_deref(), returned_nonce) {
        warn!(
            guild,
            "Patreon callback rejected: state cookie missing or mismatched"
        );
        return redirect(cx, &outcome_url(guild, PatreonOutcome::StateMismatch));
    }

    // The creator declined, or Patreon returned an error instead of a code.
    let Some(code) = code.as_deref() else {
        return redirect(cx, &outcome_url(guild, PatreonOutcome::Declined));
    };

    let web = web_state(cx)?;
    let Some(patreon_app) = web.integrations.patreon.as_ref() else {
        return redirect(cx, &outcome_url(guild, PatreonOutcome::Unconfigured));
    };

    // Re-checked after the round trip: the cookie proves the browser started
    // the flow, this proves it still has the right to bind this guild.
    let Some(admin) = guild_admin(cx, guild).await else {
        warn!(guild, "Patreon callback rejected: not a guild admin");
        return redirect(cx, &outcome_url(guild, PatreonOutcome::Forbidden));
    };

    let outcome = connect(
        &web.app,
        patreon_app,
        &web.integrations.patreon_webhook_uri,
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
    patreon_app: &PatreonApp,
    webhook_uri: &str,
    guild: &str,
    guild_id: i64,
    user_id: i64,
    code: &str,
) -> PatreonOutcome {
    let tokens = match patreon_app.exchange_code(&app.http, code).await {
        Ok(tokens) => tokens,
        Err(e) => {
            warn!(?e, guild, "Patreon token exchange failed");
            return PatreonOutcome::Error;
        },
    };

    let (campaign_id, creator_name) = match ::patreon::api::fetch_campaign(
        &app.http,
        &tokens.access_token,
    )
    .await
    {
        Ok(campaign) => campaign,
        Err(e) => {
            warn!(?e, guild, "Patreon account has no readable campaign");
            return PatreonOutcome::NoCampaign;
        },
    };

    // Best-effort: a guild with no webhook still gets its posts from the poll,
    // just up to fifteen minutes later.
    let webhook = match ::patreon::webhook::register(
        &app.http,
        &tokens.access_token,
        &campaign_id,
        webhook_uri,
    )
    .await
    {
        Ok(webhook) => Some(webhook),
        Err(e) => {
            warn!(?e, guild, "Patreon webhook registration failed; polling only");
            None
        },
    };

    let previous_campaign = PatreonConnection::select(&app.db, guild_id)
        .await
        .inspect_err(|e| {
            warn!(?e, guild, "failed to load the previous Patreon connection");
        })
        .ok()
        .flatten()
        .map(|c| c.campaign_id)
        .filter(|previous| *previous != campaign_id);

    let previous = previous_webhook(
        app,
        patreon_app,
        guild_id,
        &campaign_id,
        &tokens.access_token,
    )
    .await;

    let stored = PatreonConnection::connect(
        &app.db,
        guild_id,
        &campaign_id,
        creator_name.as_deref(),
        user_id,
        &tokens,
        webhook.as_ref().map(|(id, secret)| (id.as_str(), secret.as_str())),
    )
    .await;

    if let Err(e) = stored {
        warn!(?e, guild, "failed to store the Patreon connection");
        return PatreonOutcome::Error;
    }

    // The overwritten secret can no longer verify the old webhook's
    // deliveries, so leaving it registered only produces rejected requests.
    if let Some((webhook_id, token)) = previous {
        ::patreon::webhook::unregister(&app.http, &token, &webhook_id).await;
    }

    if let Some(previous) = previous_campaign {
        ::patreon::forget_campaign(&app.db, &previous).await;
    }

    info!(
        guild,
        campaign_id,
        webhook = webhook.is_some(),
        "Patreon campaign connected"
    );

    PatreonOutcome::Connected
}

async fn previous_webhook(
    app: &AppState,
    patreon_app: &PatreonApp,
    guild_id: i64,
    campaign_id: &str,
    new_token: &str,
) -> Option<(String, String)> {
    let previous = PatreonConnection::select(&app.db, guild_id).await.ok()??;
    let webhook_id = previous.webhook_id.clone()?;

    let token = if previous.campaign_id == campaign_id {
        new_token.to_owned()
    } else {
        ::patreon::oauth::access_token(&app.db, &app.http, patreon_app, &previous)
            .await
            .inspect_err(|e| {
                warn!(
                    ?e,
                    guild_id,
                    webhook_id,
                    "no usable token for the previous campaign; its webhook stays \
                     registered on Patreon"
                );
            })
            .ok()?
    };

    Some((webhook_id, token))
}

/// Receives a Patreon post event. Every outcome is a `200` so Patreon does
/// not retry or disable the webhook.
#[route(POST "/webhooks/patreon")]
pub(super) async fn patreon_webhook_handler(
    cx: &Cx,
    body: Bytes,
) -> Result<Response> {
    let header_text = |name: &str| {
        headers(cx)
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
    };

    process(
        app_state(cx)?,
        header_text(PATREON_EVENT_HEADER),
        header_text(PATREON_SIGNATURE_HEADER),
        &body,
    )
    .await;

    acknowledge(cx)
}

async fn process(app: &AppState, event: &str, signature: &str, body: &[u8]) {
    if event != POST_PUBLISH {
        warn!(event, "Patreon webhook ignored: unexpected event");
        return;
    }

    let post = match ::patreon::webhook::parse_post(body) {
        Ok(post) => post,
        Err(e) => {
            let raw = String::from_utf8_lossy(body);
            warn!(
                ?e,
                body = ::patreon::api::truncate_log(&raw),
                "failed to parse Patreon webhook payload"
            );
            return;
        },
    };

    // The campaign in an unverified payload only selects which secrets to try;
    // a forged one simply fails every signature check below.
    let secrets = match ::patreon::webhook_secrets(&app.db, &post.campaign_id).await
    {
        Ok(secrets) => secrets,
        Err(e) => {
            warn!(?e, campaign_id = %post.campaign_id, "failed to load webhook secrets");
            return;
        },
    };

    if secrets.is_empty() {
        warn!(
            campaign_id = %post.campaign_id,
            post_id = %post.id,
            "Patreon webhook rejected: no active connection holds a secret for \
             this campaign"
        );
        return;
    }

    if signature.is_empty() {
        warn!(campaign_id = %post.campaign_id, "Patreon webhook rejected: no signature header");
        return;
    }

    if !::patreon::webhook::verify_any(body, signature, &secrets) {
        warn!(campaign_id = %post.campaign_id, "Patreon webhook rejected: signature mismatch");
        return;
    }

    match ::patreon::is_subscribed(&app.db, &post.campaign_id).await {
        Ok(true) => {},
        Ok(false) => {
            warn!(
                campaign_id = %post.campaign_id,
                post_id = %post.id,
                "Patreon webhook ignored: no guild has an announce channel for \
                 this campaign"
            );
            return;
        },
        Err(e) => {
            warn!(?e, campaign_id = %post.campaign_id, "failed to check Patreon subscribers");
            return;
        },
    }

    match ::patreon::insert_post(&app.db, &post, false).await {
        Ok(false) => {
            info!(post_id = %post.id, "Patreon webhook: post already stored");
            return;
        },
        Ok(true) => {
            info!(
                post_id = %post.id,
                campaign_id = %post.campaign_id,
                "Patreon webhook: post stored"
            );
        },
        Err(e) => {
            warn!(?e, post_id = %post.id, "failed to store Patreon post");
            return;
        },
    }

    // The bot is a separate process, so the wake-up travels over the same
    // Postgres LISTEN/NOTIFY bus the settings cache uses.
    if let Err(e) = Channel::PatreonPost.notify(&app.db, &post.id).await {
        warn!(?e, post_id = %post.id, "failed to notify the bot of a Patreon post");
    }
}
