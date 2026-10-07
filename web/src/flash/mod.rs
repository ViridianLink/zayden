mod error;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use topcoat::context::Cx;
use topcoat::cookie::Cookies;
use topcoat::cookie::time::Duration;

pub use self::error::FlashError;
use crate::auth::{build_cookie, cookie_jar};

pub const FLASH_COOKIE: &str = "flash";
pub const MAX_MESSAGE_CHARS: usize = 240;
const LIFETIME_SECONDS: i64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashKind {
    Success,
    Error,
}

impl FlashKind {
    const fn tag(self) -> char {
        match self {
            Self::Success => 's',
            Self::Error => 'e',
        }
    }

    const fn from_tag(tag: char) -> Option<Self> {
        match tag {
            's' => Some(Self::Success),
            'e' => Some(Self::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flash {
    pub kind: FlashKind,
    pub message: String,
}

impl Flash {
    #[must_use]
    pub fn new(kind: FlashKind, message: &str) -> Self {
        Self { kind, message: message.chars().take(MAX_MESSAGE_CHARS).collect() }
    }

    #[must_use]
    pub fn encode(&self) -> String {
        URL_SAFE_NO_PAD.encode(format!("{}{}", self.kind.tag(), self.message))
    }

    #[must_use]
    pub fn decode(value: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(value).ok()?;
        let text = String::from_utf8(bytes).ok()?;
        let mut chars = text.chars();
        let kind = FlashKind::from_tag(chars.next()?)?;
        let message: String = chars.collect();
        (!message.is_empty() && message.chars().count() <= MAX_MESSAGE_CHARS)
            .then_some(Self { kind, message })
    }
}

pub fn set(cx: &Cx, kind: FlashKind, message: &str) -> Result<(), FlashError> {
    cookie_jar(cx)?.add(build_cookie(
        FLASH_COOKIE,
        Flash::new(kind, message).encode(),
        Duration::seconds(LIFETIME_SECONDS),
    ));
    Ok(())
}

#[must_use]
pub fn take(cx: &Cx) -> Option<Flash> {
    let jar = cookie_jar(cx).ok()?;
    let value = jar.get(FLASH_COOKIE)?.value().to_owned();
    jar.add(build_cookie(FLASH_COOKIE, "", Duration::ZERO));
    Flash::decode(&value)
}
