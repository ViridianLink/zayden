mod answer;
mod auto;
pub mod embed;
mod keywords;
mod lookup;
pub mod markdown;
mod triage;
mod tuning;

pub(crate) use auto::on_ticket_opened;
use serenity::all::GuildId;
use zayden_app::config::{FaqSettingsRow, SettingsStore};
use zayden_core::as_i64;

pub(crate) use crate::faq::answer::answer;
pub(crate) use crate::faq::embed as embeds;
pub use crate::faq::tuning::AnswerTuning;
use crate::wiki::{WikiConfig, WikiError};

pub struct FaqContext {
    pub wiki: WikiConfig,
    pub tuning: AnswerTuning,
    pub auto_triage: bool,
}

impl FaqContext {
    pub async fn load(
        store: &SettingsStore<FaqSettingsRow>,
        guild_id: GuildId,
    ) -> Result<Option<Self>, FaqLoadError> {
        let row = store.get(as_i64(guild_id.get())).await?;

        let Some(wiki) = WikiConfig::from_settings(&row)? else {
            return Ok(None);
        };

        Ok(Some(Self {
            wiki,
            tuning: AnswerTuning::from_settings(&row),
            auto_triage: row.auto_triage,
        }))
    }
}

#[derive(Debug)]
pub enum FaqLoadError {
    Sqlx(sqlx::Error),
    Wiki(WikiError),
}

impl std::fmt::Display for FaqLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlx(e) => e.fmt(f),
            Self::Wiki(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for FaqLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlx(e) => Some(e),
            Self::Wiki(e) => Some(e),
        }
    }
}

impl From<sqlx::Error> for FaqLoadError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<WikiError> for FaqLoadError {
    fn from(value: WikiError) -> Self {
        Self::Wiki(value)
    }
}
