mod answer;
pub mod article;
mod auto;
pub(crate) mod embed;
mod generate;
pub mod hit;
pub mod index;
mod keywords;
pub mod linked;
mod lookup;
pub mod render;
pub mod scrub;
pub mod transcript;
pub mod triage;
mod tuning;
pub(crate) mod view;
mod writer;

pub use article::{FaqArticle, NewArticle};
pub(crate) use auto::{TicketOpening, on_ticket_opened};
pub(crate) use generate::on_ticket_solved;
pub use index::{Target, WikiIndex};
use serenity::all::GuildId;
use zayden_app::config::{FaqSettingsRow, SettingsStore};
use zayden_core::as_i64;

pub(crate) use crate::faq::answer::answer;
pub use crate::faq::tuning::AnswerTuning;
pub(crate) use crate::faq::{embed as embeds, view as views};
use crate::wiki::{WikiConfig, WikiError};

pub struct FaqContext {
    pub wiki: WikiConfig,
    pub tuning: AnswerTuning,
    pub auto_triage: bool,
    pub auto_generate: bool,
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
            auto_generate: row.auto_generate,
        }))
    }

    pub async fn generation_enabled(
        store: &SettingsStore<FaqSettingsRow>,
        guild_id: GuildId,
    ) -> Result<bool, sqlx::Error> {
        let row = store.get(as_i64(guild_id.get())).await?;

        Ok(row.enabled && row.auto_generate)
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
