use accounts::AccountsError;
use accounts::links::AccountLink;
use accounts::provider::Provider;
use accounts::steam::openid::{STEAM_OPENID_ENDPOINT, authorize_url, verify};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::see_other;
use topcoat::router::request::uri;
use topcoat::router::response::{IntoResponse, Response};
use topcoat::router::{route, segment};
use tracing::warn;
use url::form_urlencoded;

use crate::app::providers::oauth::{
    nonce_matches,
    random_hex,
    remember_nonce,
    take_nonce,
};
use crate::auth::{check_session, current_user_id, remember_login_next, web_state};
use crate::flash::{FlashKind, set as set_flash};

segment!(kind = Group);

const STEAM_NONCE_COOKIE: &str = "steam_openid_nonce";
const LINK_PATH: &str = "/account/link/steam";
const CALLBACK_PATH: &str = "/account/link/steam/callback";
const ACCOUNT_PATH: &str = "/account";
const SIGN_IN_PATH: &str = "/auth/discord";

fn redirect(cx: &Cx, location: &str) -> Result<Response> {
    see_other(location).into_response(cx)
}

fn sign_in(cx: &Cx, next: &str) -> Result<Response> {
    remember_login_next(cx, next)?;
    redirect(cx, SIGN_IN_PATH)
}

fn back_to_account(cx: &Cx, kind: FlashKind, message: &str) -> Result<Response> {
    set_flash(cx, kind, message)?;
    redirect(cx, ACCOUNT_PATH)
}

fn verification_failed(cx: &Cx, error: &AccountsError) -> Result<Response> {
    warn!(error = %error, "Steam account verification failed");
    back_to_account(
        cx,
        FlashKind::Error,
        "Couldn't verify your Steam account. Please try again.",
    )
}

fn query_value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_str())
}

#[route(GET "./link/steam")]
pub(super) async fn steam_link(cx: &Cx) -> Result<Response> {
    if !check_session(cx).await? {
        return sign_in(cx, LINK_PATH);
    }

    let nonce = random_hex();
    remember_nonce(cx, STEAM_NONCE_COOKIE, nonce.clone())?;

    let base = &web_state(cx)?.urls.dashboard;
    let return_to = format!("{base}{CALLBACK_PATH}?n={nonce}");

    match authorize_url(base, &return_to) {
        Ok(url) => redirect(cx, url.as_str()),
        Err(error) => {
            warn!(error = %error, "failed to build the Steam sign-in URL");
            back_to_account(cx, FlashKind::Error, "Couldn't start Steam sign-in.")
        },
    }
}

#[route(GET "./link/steam/callback")]
pub(super) async fn steam_callback(cx: &Cx) -> Result<Response> {
    if !check_session(cx).await? {
        return sign_in(cx, ACCOUNT_PATH);
    }

    let pairs: Vec<(String, String)> =
        form_urlencoded::parse(uri(cx).query().unwrap_or_default().as_bytes())
            .into_owned()
            .collect();

    let cookie = take_nonce(cx, STEAM_NONCE_COOKIE)?;
    let Some(nonce) = query_value(&pairs, "n")
        .filter(|returned| nonce_matches(cookie.as_deref(), returned))
    else {
        return back_to_account(
            cx,
            FlashKind::Error,
            "Steam sign-in expired. Please try again.",
        );
    };

    if query_value(&pairs, "openid.mode") == Some("cancel") {
        return back_to_account(
            cx,
            FlashKind::Error,
            "Steam sign-in was cancelled.",
        );
    }

    let web = web_state(cx)?;
    let expected = format!("{}{CALLBACK_PATH}?n={nonce}", web.urls.dashboard);

    let id = match verify(&web.app.http, STEAM_OPENID_ENDPOINT, &pairs, &expected)
        .await
    {
        Ok(id) => id,
        Err(error) => return verification_failed(cx, &error),
    };

    let user_id = current_user_id(cx).await?;
    if let Err(error) = AccountLink::upsert_transfer(
        &web.app.db,
        user_id,
        Provider::Steam,
        &id.to_string(),
    )
    .await
    {
        return verification_failed(cx, &error);
    }

    back_to_account(cx, FlashKind::Success, "Steam account linked.")
}
