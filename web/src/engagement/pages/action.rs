use std::fmt::Write as _;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::see_other;

use crate::flash::{FlashKind, set as set_flash};
use crate::settings::NOT_SAVED;

/// A form an engagement page posts. Its `name` is the last segment of the
/// form's address, `/guild/{id}/<page>/<name>`; for the forms that existed
/// before, it is the legacy `?action=` value verbatim.
pub(super) trait FormAction: Copy + PartialEq + 'static {
    const ALL: &'static [Self];

    fn name(self) -> &'static str;

    fn find(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|action| action.name() == name)
    }
}

pub(super) fn path_segment(raw: &str) -> String {
    raw.bytes().fold(String::with_capacity(raw.len()), |mut out, byte| {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
        {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
        out
    })
}

pub(super) fn page_href(guild_id: &str, page: &str) -> String {
    format!("/guild/{}/{page}", path_segment(guild_id))
}

pub(super) fn form_action(
    guild_id: &str,
    page: &str,
    action: impl FormAction,
) -> String {
    format!("{}/{}", page_href(guild_id, page), action.name())
}

/// A post to a page's own address is a form from before the action paths:
/// nothing is applied and the page says so.
pub(super) fn not_saved(cx: &Cx, guild_id: &str, page: &str) -> Result<()> {
    set_flash(cx, FlashKind::Error, NOT_SAVED)?;
    Err(see_other(page_href(guild_id, page)).into())
}
