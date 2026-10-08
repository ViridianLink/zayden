use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::see_other;
use topcoat::router::request::uri;
use topcoat::router::response::{IntoResponse, Response};
use topcoat::router::{StatusCode, route};
use url::{Url, form_urlencoded};

use crate::auth::web_state;

const GUILD_PARAM: &str = "guild";

const AUTHORIZE_HOSTS: &[&str] = &[
    "discord.com",
    "www.discord.com",
    "ptb.discord.com",
    "canary.discord.com",
    "discordapp.com",
];

const PRESELECT_KEYS: [&str; 2] = ["guild_id", "disable_guild_select"];

fn requested_guild(cx: &Cx) -> Option<u64> {
    let (_, raw) = form_urlencoded::parse(uri(cx).query()?.as_bytes())
        .find(|(key, _)| key == GUILD_PARAM)?;

    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok().filter(|id| *id != 0)
}

fn is_authorize_path(path: &str) -> bool {
    let path = path.trim_end_matches('/');
    let Some(rest) = path.strip_suffix("/oauth2/authorize") else {
        return false;
    };
    rest.strip_prefix("/api").map_or_else(
        || rest.is_empty(),
        |version| {
            version.is_empty()
                || version.strip_prefix("/v").is_some_and(|digits| {
                    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                })
        },
    )
}

/// The invite address with the server preselected, when the invite is a
/// Discord `OAuth2` authorize address; `None` leaves the invite unchanged.
fn preselect(address: &str, guild: u64) -> Option<String> {
    let mut url = Url::parse(address).ok()?;
    let discord = url.scheme() == "https"
        && url.host_str().is_some_and(|host| AUTHORIZE_HOSTS.contains(&host))
        && is_authorize_path(url.path());
    if !discord {
        return None;
    }

    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| !PRESELECT_KEYS.contains(&key.as_ref()))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.query_pairs_mut()
        .clear()
        .extend_pairs(kept)
        .append_pair("guild_id", &guild.to_string())
        .append_pair("disable_guild_select", "true");

    Some(url.into())
}

#[route(GET "/invite")]
pub(crate) async fn invite(cx: &Cx) -> Result<Response> {
    let Some(address) = web_state(cx)?.urls.invite.as_deref() else {
        return StatusCode::NOT_FOUND.into_response(cx);
    };
    let target = requested_guild(cx)
        .and_then(|guild| preselect(address, guild))
        .unwrap_or_else(|| address.to_owned());

    see_other(target).into_response(cx)
}
