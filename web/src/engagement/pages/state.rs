use topcoat::Result;
use topcoat::context::{Cx, try_request_context};
use topcoat::router::error::{rewrite, see_other};
use topcoat::router::request::headers;
use topcoat::router::{Body, Method, StatusCode, header};

use crate::engagement::EngagementError;
use crate::flash::{Flash, FlashKind, set_in, take};
use crate::util::server_error_text;

#[derive(Debug, Default)]
pub(super) struct PageState {
    notice: Option<Flash>,
    failure: Option<Failure>,
}

impl PageState {
    pub(super) fn load(cx: &Cx) -> Self {
        try_request_context::<Failure>(cx).map_or_else(
            || Self { notice: take(cx), failure: None },
            |failure| Self::failed(failure.clone()),
        )
    }

    pub(super) const fn failed(failure: Failure) -> Self {
        Self { notice: None, failure: Some(failure) }
    }

    pub(super) const fn status(&self) -> StatusCode {
        if self.failure.is_some() {
            StatusCode::UNPROCESSABLE_ENTITY
        } else {
            StatusCode::OK
        }
    }

    pub(super) fn top_notice(&self) -> Option<&Flash> {
        self.notice.as_ref().filter(|notice| notice.section.is_none())
    }

    pub(super) fn notice_for(&self, section: &str) -> Option<&Flash> {
        self.notice.as_ref().filter(|notice| notice.is_for(section))
    }

    pub(super) fn sent(&self, form: &str) -> Sent<'_> {
        Sent(self.failure.as_ref().filter(|failure| failure.form == form))
    }

    pub(super) fn any_failure(&self) -> Option<&str> {
        self.failure.as_ref().map(|failure| failure.message.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Failure {
    form: &'static str,
    values: Vec<(String, String)>,
    message: String,
    field: Option<String>,
}

impl Failure {
    pub(super) fn new(
        form: &'static str,
        values: Vec<(String, String)>,
        error: &EngagementError,
    ) -> Self {
        let message = if error.is_denied() {
            DENIED.to_owned()
        } else {
            plain(&error.to_string()).to_owned()
        };
        Self { form, values, message, field: field_of(error).map(str::to_owned) }
    }
}

const DENIED: &str = "you need Manage Server in this server to change this";

fn field_of(error: &EngagementError) -> Option<&str> {
    match error {
        EngagementError::UnknownField(name)
        | EngagementError::DuplicateField(name) => Some(name),
        EngagementError::MissingField(name) => Some(name),
        EngagementError::Invalid("channel id")
        | EngagementError::ChannelAlreadyListed
        | EngagementError::ChannelListFull(_)
        | EngagementError::ChannelNotListed => Some("channel_id"),
        EngagementError::EmojiAlreadyMapped => Some("emoji"),
        EngagementError::CooldownBelowFloor { label, .. } => {
            Some(if *label == "per-member" {
                "user_cooldown"
            } else {
                "guild_cooldown"
            })
        },
        EngagementError::Invalid(_)
        | EngagementError::Auth(_)
        | EngagementError::Guild(_)
        | EngagementError::Server(_)
        | EngagementError::ImageNotFound
        | EngagementError::GuildMismatch => None,
    }
}

pub(super) fn plain(message: &str) -> &str {
    let prefix = server_error_text("");
    message.strip_prefix(prefix.as_str()).unwrap_or(message)
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Sent<'a>(Option<&'a Failure>);

impl<'a> Sent<'a> {
    fn submitted(self, name: &str) -> Option<&'a str> {
        self.0?
            .values
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub(super) fn value<'b>(self, name: &str, stored: &'b str) -> &'b str
    where
        'a: 'b,
    {
        self.submitted(name).unwrap_or(stored)
    }

    pub(super) fn error(self, name: &str) -> Option<&'a str> {
        let failure = self.0?;
        (failure.field.as_deref() == Some(name)).then_some(failure.message.as_str())
    }

    pub(super) fn summary(self) -> Option<&'a str> {
        self.0.map(|failure| failure.message.as_str())
    }
}

pub(super) fn rerender(cx: &Cx, page: &str, failure: Failure) -> topcoat::Error {
    let mut sent = headers(cx).clone();
    sent.remove(header::CONTENT_TYPE);
    sent.remove(header::CONTENT_LENGTH);

    rewrite(page, Body::empty())
        .method(Method::GET)
        .headers(sent)
        .with(failure)
        .into()
}

pub(super) struct Done<'a> {
    pub(super) page: String,
    pub(super) section: Option<&'a str>,
    pub(super) message: &'a str,
}

pub(super) fn settle(
    cx: &Cx,
    form: &'static str,
    values: Vec<(String, String)>,
    result: std::result::Result<(), EngagementError>,
    done: &Done<'_>,
) -> Result<Failure> {
    let error = match result {
        Ok(()) => {
            if let Some(section) = done.section {
                set_in(cx, FlashKind::Success, done.message, section)?;
            } else {
                crate::flash::set(cx, FlashKind::Success, done.message)?;
            }
            let location = done.section.map_or_else(
                || done.page.clone(),
                |section| format!("{}#{section}", done.page),
            );
            return Err(see_other(location).into());
        },
        Err(error) => error.redirect_unauthenticated()?,
    };

    Ok(Failure::new(form, values, &error))
}
