pub const GUEST_PREFIX: &str = "zayden-party-";
pub const LIBRARY_PREFIX: &str = "Zayden Party ";

#[must_use]
pub fn guest_username(party_id: i64, discord_id: u64) -> String {
    format!("{GUEST_PREFIX}{party_id}-{discord_id}")
}

#[must_use]
pub fn library_name(party_id: i64) -> String {
    format!("{LIBRARY_PREFIX}{party_id}")
}

#[must_use]
pub fn is_managed_guest(name: &str) -> bool {
    let Some(rest) = name.strip_prefix(GUEST_PREFIX) else {
        return false;
    };

    let Some((party, discord)) = rest.split_once('-') else {
        return false;
    };

    !party.is_empty()
        && !discord.is_empty()
        && party.bytes().all(|b| b.is_ascii_digit())
        && discord.bytes().all(|b| b.is_ascii_digit())
}

#[must_use]
pub fn is_managed_library(name: &str) -> bool {
    name.strip_prefix(LIBRARY_PREFIX).is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
    })
}

#[must_use]
pub fn party_id_of_guest(name: &str) -> Option<i64> {
    name.strip_prefix(GUEST_PREFIX)?.split_once('-')?.0.parse().ok()
}

#[must_use]
pub fn party_id_of_library(name: &str) -> Option<i64> {
    name.strip_prefix(LIBRARY_PREFIX)?.parse().ok()
}
