use std::fmt::Write as _;

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::StatusCode;
use topcoat::router::request::uri;
use url::form_urlencoded;

use crate::engagement::EngagementError;
use crate::util::server_error_text;

const SELECTOR: &str = "action";

pub(super) trait FormAction: Copy + PartialEq + 'static {
    const ALL: &'static [Self];

    fn name(self) -> &'static str;

    fn flag(self) -> Option<&'static str>;
}

pub(super) fn requested<A: FormAction>(cx: &Cx) -> Option<A> {
    let (_, name) = form_urlencoded::parse(uri(cx).query()?.as_bytes())
        .find(|(key, _)| key == SELECTOR)?;

    A::ALL.iter().copied().find(|action| action.name() == name)
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

pub(super) fn form_action(
    guild_id: &str,
    page: &str,
    action: impl FormAction,
) -> String {
    format!("/guild/{}/{page}?{SELECTOR}={}", path_segment(guild_id), action.name())
}

pub(super) fn flagged<A: FormAction>(cx: &Cx) -> Option<Submitted<A>> {
    let query = uri(cx).query()?;

    form_urlencoded::parse(query.as_bytes())
        .filter(|(_, value)| value == "1")
        .find_map(|(key, _)| {
            A::ALL.iter().copied().find(|action| action.flag() == Some(&*key))
        })
        .map(|action| Submitted { action, outcome: Ok(()) })
}

pub(super) fn loaded<T>(
    result: std::result::Result<T, EngagementError>,
) -> Result<std::result::Result<T, String>> {
    match result {
        Ok(value) => Ok(Ok(value)),
        Err(error) => Ok(Err(server_error_text(error.redirect_unauthenticated()?))),
    }
}

pub(super) struct Submitted<A> {
    action: A,
    outcome: std::result::Result<(), String>,
}

impl<A: FormAction> Submitted<A> {
    pub(super) fn new(
        action: A,
        result: std::result::Result<(), EngagementError>,
    ) -> Result<Self> {
        let outcome = match result {
            Ok(()) => Ok(()),
            Err(error) => Err(error.redirect_unauthenticated()?.to_string()),
        };

        Ok(Self { action, outcome })
    }

    pub(super) fn success_location(
        &self,
        guild_id: &str,
        page: &str,
    ) -> Option<String> {
        let flag = self.action.flag()?;

        self.outcome
            .is_ok()
            .then(|| format!("/guild/{}/{page}?{flag}=1", path_segment(guild_id)))
    }

    pub(super) const fn status(&self) -> StatusCode {
        match self.outcome {
            Ok(()) => StatusCode::OK,
            Err(_) => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

pub(super) fn feedback<A: FormAction>(
    submitted: Option<&Submitted<A>>,
    action: A,
) -> Option<std::result::Result<(), &str>> {
    submitted
        .filter(|submitted| submitted.action == action)
        .map(|submitted| submitted.outcome.as_ref().copied().map_err(String::as_str))
}
