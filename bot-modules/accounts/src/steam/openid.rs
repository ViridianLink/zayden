use std::collections::HashSet;

use url::Url;

use super::id::SteamId64;
use crate::error::{AccountsError, Result};

pub const STEAM_OPENID_ENDPOINT: &str = "https://steamcommunity.com/openid/login";
pub const OPENID_NS: &str = "http://specs.openid.net/auth/2.0";
const IDENTIFIER_SELECT: &str = "http://specs.openid.net/auth/2.0/identifier_select";
const REQUIRED_SIGNED: [&str; 6] = [
    "op_endpoint",
    "claimed_id",
    "identity",
    "return_to",
    "response_nonce",
    "assoc_handle",
];

pub fn authorize_url(realm: &str, return_to: &str) -> Result<Url> {
    Ok(Url::parse_with_params(STEAM_OPENID_ENDPOINT, [
        ("openid.ns", OPENID_NS),
        ("openid.mode", "checkid_setup"),
        ("openid.return_to", return_to),
        ("openid.realm", realm),
        ("openid.identity", IDENTIFIER_SELECT),
        ("openid.claimed_id", IDENTIFIER_SELECT),
    ])?)
}

fn param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_str())
}

fn expect(
    params: &[(String, String)],
    key: &str,
    expected: &str,
    field: &'static str,
) -> Result<()> {
    if param(params, key) == Some(expected) {
        Ok(())
    } else {
        Err(AccountsError::InvalidAssertion(field))
    }
}

fn has_duplicate_openid_key(params: &[(String, String)]) -> bool {
    let mut seen = HashSet::new();
    params
        .iter()
        .filter(|(key, _)| key.starts_with("openid."))
        .any(|(key, _)| !seen.insert(key.as_str()))
}

pub fn validate_assertion(
    params: &[(String, String)],
    expected_return_to: &str,
) -> Result<SteamId64> {
    if has_duplicate_openid_key(params) {
        return Err(AccountsError::InvalidAssertion("duplicate"));
    }
    expect(params, "openid.ns", OPENID_NS, "ns")?;
    expect(params, "openid.mode", "id_res", "mode")?;
    expect(params, "openid.op_endpoint", STEAM_OPENID_ENDPOINT, "op_endpoint")?;
    expect(params, "openid.return_to", expected_return_to, "return_to")?;

    let claimed_id = param(params, "openid.claimed_id")
        .ok_or(AccountsError::InvalidAssertion("claimed_id"))?;
    expect(params, "openid.identity", claimed_id, "identity")?;

    let signed = param(params, "openid.signed")
        .ok_or(AccountsError::InvalidAssertion("signed"))?;
    let signed_fields: Vec<&str> = signed.split(',').collect();
    if !REQUIRED_SIGNED.iter().all(|field| signed_fields.contains(field)) {
        return Err(AccountsError::InvalidAssertion("signed"));
    }

    SteamId64::from_claimed_id(claimed_id)
        .ok_or(AccountsError::InvalidAssertion("claimed_id"))
}

pub async fn verify(
    http: &reqwest::Client,
    endpoint: &str,
    params: &[(String, String)],
    expected_return_to: &str,
) -> Result<SteamId64> {
    let id = validate_assertion(params, expected_return_to)?;

    let form: Vec<(&str, &str)> = params
        .iter()
        .filter(|(key, _)| key.starts_with("openid."))
        .map(|(key, value)| {
            if key == "openid.mode" {
                (key.as_str(), "check_authentication")
            } else {
                (key.as_str(), value.as_str())
            }
        })
        .collect();

    let body = http
        .post(endpoint)
        .form(&form)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    if body.lines().any(|line| line.trim() == "is_valid:true") {
        Ok(id)
    } else {
        Err(AccountsError::VerificationRejected)
    }
}
