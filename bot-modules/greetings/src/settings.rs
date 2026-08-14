use serenity::all::{GuildId, Mentionable, UserId};
use zayden_app::config::{GreetingsSettingsRow, SettingsStore};
use zayden_core::as_i64;

use crate::error::{GreetingsError, Result};
use crate::kind::GreetingKind;

pub type GreetingsStore = SettingsStore<GreetingsSettingsRow>;

pub const MAX_MESSAGE_LEN: usize = 1500;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GreetingsConfig {
    pub morning_message: Option<String>,
    pub night_message: Option<String>,
}

impl From<&GreetingsSettingsRow> for GreetingsConfig {
    fn from(row: &GreetingsSettingsRow) -> Self {
        Self {
            morning_message: row.morning_message.clone(),
            night_message: row.night_message.clone(),
        }
    }
}

impl GreetingsConfig {
    #[must_use]
    pub fn message_for(&self, kind: GreetingKind) -> Option<&str> {
        match kind {
            GreetingKind::Morning => self.morning_message.as_deref(),
            GreetingKind::Night => self.night_message.as_deref(),
        }
    }

    pub fn from_form(morning: &str, night: &str) -> Result<Self> {
        Ok(Self {
            morning_message: parse_message(morning)?,
            night_message: parse_message(night)?,
        })
    }

    pub fn apply(self, row: &mut GreetingsSettingsRow) {
        row.morning_message = self.morning_message;
        row.night_message = self.night_message;
    }
}

fn parse_message(raw: &str) -> Result<Option<String>> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    if trimmed.chars().count() > MAX_MESSAGE_LEN {
        return Err(GreetingsError::MessageTooLong(MAX_MESSAGE_LEN));
    }

    Ok(Some(trimmed.to_string()))
}

#[must_use]
pub fn render(template: &str, target: UserId, invoker: UserId) -> String {
    template
        .replace("{user}", &target.mention().to_string())
        .replace("{author}", &invoker.mention().to_string())
}

pub struct GreetingsSettings;

impl GreetingsSettings {
    pub async fn get(
        store: &GreetingsStore,
        guild_id: GuildId,
    ) -> Result<GreetingsConfig> {
        let row = store.get(as_i64(guild_id.get())).await?;

        Ok(GreetingsConfig::from(row.as_ref()))
    }

    pub async fn save(
        store: &GreetingsStore,
        guild_id: GuildId,
        config: GreetingsConfig,
    ) -> Result<GreetingsConfig> {
        let row =
            store.update(as_i64(guild_id.get()), |row| config.apply(row)).await?;

        Ok(GreetingsConfig::from(row.as_ref()))
    }
}
