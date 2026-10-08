use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::StatusCode;
use topcoat::router::error::see_other;

use crate::flash::{self, Flash, FlashKind};
use crate::guild::GuildError;
use crate::guild::parse::parse_flag;
use crate::util::server_error_text;

#[derive(Debug, Default)]
pub(super) struct PageState {
    notice: Option<Flash>,
    failure: Option<Failure>,
}

impl PageState {
    /// The state of a plain page load. Reads (and clears) the flash cookie, so
    /// call it in the page handler before the body renders.
    pub(super) fn load(cx: &Cx) -> Self {
        Self { notice: flash::take(cx), failure: None }
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
        error: &GuildError,
    ) -> Self {
        Self {
            form,
            values,
            message: plain(&error.to_string()).to_owned(),
            field: field_of(error),
        }
    }

    pub(super) fn with_message(
        form: &'static str,
        values: Vec<(String, String)>,
        message: &str,
        field: Option<&str>,
    ) -> Self {
        Self {
            form,
            values,
            message: plain(message).to_owned(),
            field: field.map(str::to_owned),
        }
    }
}

fn field_of(error: &GuildError) -> Option<String> {
    let field = match error {
        GuildError::UnknownField(name) | GuildError::DuplicateField(name) => {
            return Some(name.clone());
        },
        GuildError::MissingField(name) | GuildError::InvalidField(name) => *name,
        GuildError::InvalidWikiUrl | GuildError::WikiUrlScheme => "wiki_url",
        GuildError::InvalidRole | GuildError::DuplicateSupportRole => "role_id",
        GuildError::InvalidUserId => "user_id",
        GuildError::InvalidLink(_)
        | GuildError::LinkScheme
        | GuildError::LinkCredentials
        | GuildError::LinkTooLong => "link",
        GuildError::NoCategory => "temp_voice_category",
        GuildError::Auth(_)
        | GuildError::Server(_)
        | GuildError::PartlySaved { .. }
        | GuildError::InvalidChannelId
        | GuildError::InvalidEmail
        | GuildError::KofiEmailTaken
        | GuildError::InvalidApplicationId
        | GuildError::UnknownModule
        | GuildError::NoSettingsSwitch(_)
        | GuildError::DerivedModule(_)
        | GuildError::CommandNotRegistered(_)
        | GuildError::OperatorCommandPermissions(_)
        | GuildError::PermissionUpdateRejected { .. } => return None,
    };
    Some(field.to_owned())
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

    pub(super) fn value<'b>(self, name: &str, stored: Option<&'b str>) -> &'b str
    where
        'a: 'b,
    {
        self.submitted(name).or(stored).unwrap_or_default()
    }

    pub(super) fn flag(self, name: &str, stored: bool) -> bool {
        self.submitted(name).map_or(stored, parse_flag)
    }

    pub(super) fn error(self, name: &str) -> Option<&'a str> {
        let failure = self.0?;
        (failure.field.as_deref() == Some(name)).then_some(failure.message.as_str())
    }

    pub(super) fn summary(self) -> Option<&'a str> {
        self.0.map(|failure| failure.message.as_str())
    }
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
    result: std::result::Result<(), GuildError>,
    done: &Done<'_>,
) -> Result<Failure> {
    match result {
        Ok(()) => Err(succeed(cx, done)),
        Err(error) => {
            let error = error.redirect_unauthenticated()?;
            Ok(Failure::new(form, values, &error))
        },
    }
}

pub(super) fn succeed(cx: &Cx, done: &Done<'_>) -> topcoat::Error {
    let queued = done.section.map_or_else(
        || flash::set(cx, FlashKind::Success, done.message),
        |section| flash::set_in(cx, FlashKind::Success, done.message, section),
    );
    if let Err(error) = queued {
        return error.into();
    }
    let location = done.section.map_or_else(
        || done.page.clone(),
        |section| format!("{}#{section}", done.page),
    );
    see_other(location).into()
}
