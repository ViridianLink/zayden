use serenity::all::{GuildId, Mentionable, UserId};
use zayden_app::config::{Cooldowns, GreetingsSettingsRow, SettingsStore};
use zayden_core::as_i64;

use crate::error::{GreetingsError, Result};
use crate::kind::GreetingKind;

pub type GreetingsStore = SettingsStore<GreetingsSettingsRow>;

pub const MAX_MESSAGE_LEN: usize = 1500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreetingsConfig {
    pub morning_message: Option<String>,
    pub night_message: Option<String>,
    pub cooldowns: Cooldowns,
}

impl From<&GreetingsSettingsRow> for GreetingsConfig {
    fn from(row: &GreetingsSettingsRow) -> Self {
        Self {
            morning_message: row.morning_message.clone(),
            night_message: row.night_message.clone(),
            cooldowns: Cooldowns::from(row),
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

pub fn parse_cooldown(raw: &str, floor: i32) -> Result<i32> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok(floor);
    }

    let secs = trimmed
        .parse::<i32>()
        .map_err(|_e| GreetingsError::InvalidCooldown(trimmed.to_string()))?;

    if !(0..=GreetingsSettingsRow::MAX_COOLDOWN_SECS).contains(&secs) {
        return Err(GreetingsError::InvalidCooldown(trimmed.to_string()));
    }

    Ok(secs)
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

    pub async fn save_messages(
        store: &GreetingsStore,
        guild_id: GuildId,
        morning: &str,
        night: &str,
    ) -> Result<()> {
        let morning = parse_message(morning)?;
        let night = parse_message(night)?;

        store
            .update(as_i64(guild_id.get()), |row| {
                row.morning_message = morning;
                row.night_message = night;
            })
            .await?;

        Ok(())
    }

    pub async fn save_cooldowns(
        store: &GreetingsStore,
        guild_id: GuildId,
        requested: Cooldowns,
        floor: Cooldowns,
    ) -> Result<Cooldowns> {
        let clamped = requested.clamp_to(floor);

        store
            .update(as_i64(guild_id.get()), |row| {
                row.user_cooldown_secs = clamped.user_secs;
                row.guild_cooldown_secs = clamped.guild_secs;
            })
            .await?;

        Ok(clamped)
    }
}
