use serenity::all::{GenericChannelId, GuildId};
use zayden_app::config::SettingsStore;
use zayden_app::config::tables::JellyfinSettingsRow;
use zayden_core::as_i64;

use crate::error::Result;

pub type JellyfinStore = SettingsStore<JellyfinSettingsRow>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JellyfinConfig {
    pub party_channel_id: Option<GenericChannelId>,
    pub game_channel_id: Option<GenericChannelId>,
    pub guests_enabled: bool,
    pub max_concurrent_guests: i32,
}

impl From<&JellyfinSettingsRow> for JellyfinConfig {
    fn from(row: &JellyfinSettingsRow) -> Self {
        Self {
            party_channel_id: row.party_channel_id.map(channel_id),
            game_channel_id: row.game_channel_id.map(channel_id),
            guests_enabled: row.guests_enabled,
            max_concurrent_guests: row.max_concurrent_guests,
        }
    }
}

const fn channel_id(raw: i64) -> GenericChannelId {
    GenericChannelId::new(raw.cast_unsigned())
}

pub struct JellyfinSettings;

impl JellyfinSettings {
    pub async fn get(
        store: &JellyfinStore,
        guild_id: GuildId,
    ) -> Result<JellyfinConfig> {
        let row = store.get(as_i64(guild_id.get())).await?;
        Ok(JellyfinConfig::from(row.as_ref()))
    }
}
