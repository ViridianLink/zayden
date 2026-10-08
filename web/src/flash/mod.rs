mod error;

use std::sync::LazyLock;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use topcoat::context::{Cx, try_app_context};
use topcoat::cookie::time::Duration;
use topcoat::cookie::{Cookies, Key};

pub use self::error::FlashError;
use crate::auth::{build_cookie, cookie_jar};

pub const FLASH_COOKIE: &str = "flash";
pub const MAX_MESSAGE_CHARS: usize = 240;
const LIFETIME_SECONDS: i64 = 120;
const SECTION_END: char = '\n';

static PROCESS_KEY: LazyLock<Option<Key>> = LazyLock::new(Key::try_generate);

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
    pub section: Option<String>,
}

impl Flash {
    #[must_use]
    pub fn new(kind: FlashKind, message: &str) -> Self {
        Self {
            kind,
            message: message.chars().take(MAX_MESSAGE_CHARS).collect(),
            section: None,
        }
    }

    #[must_use]
    pub fn in_section(kind: FlashKind, message: &str, section: &str) -> Self {
        Self {
            section: is_section_id(section).then(|| section.to_owned()),
            ..Self::new(kind, message)
        }
    }

    #[must_use]
    pub fn encode(&self) -> String {
        let section = self.section.as_deref().unwrap_or_default();
        URL_SAFE_NO_PAD.encode(format!(
            "{}{section}{SECTION_END}{}",
            self.kind.tag(),
            self.message
        ))
    }

    #[must_use]
    pub fn decode(value: &str) -> Option<Self> {
        let bytes = URL_SAFE_NO_PAD.decode(value).ok()?;
        let text = String::from_utf8(bytes).ok()?;
        let mut chars = text.chars();
        let kind = FlashKind::from_tag(chars.next()?)?;
        let (section, message) = chars.as_str().split_once(SECTION_END)?;
        let section = match section {
            "" => None,
            id if is_section_id(id) => Some(id.to_owned()),
            _ => return None,
        };
        (!message.trim().is_empty() && message.chars().count() <= MAX_MESSAGE_CHARS)
            .then(|| Self { kind, message: message.to_owned(), section })
    }

    #[must_use]
    pub fn is_for(&self, section: &str) -> bool {
        self.section.as_deref() == Some(section)
    }
}

fn is_section_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn key(cx: &Cx) -> Option<&Key> {
    try_app_context::<Key>(cx).or(PROCESS_KEY.as_ref())
}

pub fn set(cx: &Cx, kind: FlashKind, message: &str) -> Result<(), FlashError> {
    write(cx, &Flash::new(kind, message))
}

pub fn set_in(
    cx: &Cx,
    kind: FlashKind,
    message: &str,
    section: &str,
) -> Result<(), FlashError> {
    write(cx, &Flash::in_section(kind, message, section))
}

fn write(cx: &Cx, flash: &Flash) -> Result<(), FlashError> {
    if flash.message.trim().is_empty() {
        return Err(FlashError::EmptyMessage);
    }
    let key = key(cx).ok_or(FlashError::NoKey)?;
    cookie_jar(cx)?.signed(key).add(build_cookie(
        FLASH_COOKIE,
        flash.encode(),
        Duration::seconds(LIFETIME_SECONDS),
    ));
    Ok(())
}

#[must_use]
pub fn take(cx: &Cx) -> Option<Flash> {
    let jar = cookie_jar(cx).ok()?;
    jar.get(FLASH_COOKIE)?;
    let verified = key(cx).and_then(|key| jar.signed(key).get(FLASH_COOKIE));
    jar.add(build_cookie(FLASH_COOKIE, "", Duration::ZERO));
    Flash::decode(verified?.value())
}
