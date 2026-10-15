use std::fmt;
use std::str::FromStr;

use crate::error::AccountsError;

pub const STEAM_ID64_BASE: u64 = 76_561_197_960_265_728;

const CLAIMED_ID_PREFIX: &str = "https://steamcommunity.com/openid/id/";
const STEAM_ID64_DIGITS: usize = 17;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SteamId64(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccountId(u32);

impl SteamId64 {
    #[must_use]
    pub fn new(raw: u64) -> Option<Self> {
        let offset = raw.checked_sub(STEAM_ID64_BASE)?;
        u32::try_from(offset).ok().map(|_| Self(raw))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn account_id(self) -> AccountId {
        AccountId(u32::try_from(self.0 - STEAM_ID64_BASE).unwrap_or_default())
    }

    #[must_use]
    pub fn from_claimed_id(claimed: &str) -> Option<Self> {
        claimed.strip_prefix(CLAIMED_ID_PREFIX).and_then(parse_digits)
    }
}

impl AccountId {
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn steam_id64(self) -> SteamId64 {
        SteamId64(STEAM_ID64_BASE + u64::from(self.0))
    }
}

fn parse_digits(digits: &str) -> Option<SteamId64> {
    let well_formed = digits.len() == STEAM_ID64_DIGITS
        && digits.bytes().all(|b| b.is_ascii_digit());
    if !well_formed {
        return None;
    }
    digits.parse().ok().and_then(SteamId64::new)
}

impl FromStr for SteamId64 {
    type Err = AccountsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_digits(s).ok_or(AccountsError::InvalidAssertion("steam id"))
    }
}

impl fmt::Display for SteamId64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[U:1:{}]", self.0)
    }
}
