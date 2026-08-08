use serenity::all::{ChannelId, GuildId, RoleId};
use zayden_app::config::{HoneypotSettingsRow, SettingsStore};
use zayden_core::{as_i64, as_u64};

use crate::error::{HoneypotError, Result};

pub type HoneypotStore = SettingsStore<HoneypotSettingsRow>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoneypotConfig {
    pub channel_id: Option<ChannelId>,
    pub exempt_admins: bool,
    pub exempt_role_id: Option<RoleId>,
    pub purge_seconds: i32,
}

impl Default for HoneypotConfig {
    fn default() -> Self {
        Self {
            channel_id: None,
            exempt_admins: false,
            exempt_role_id: None,
            purge_seconds: HoneypotSettingsRow::DEFAULT_PURGE_SECONDS,
        }
    }
}

impl From<&HoneypotSettingsRow> for HoneypotConfig {
    fn from(row: &HoneypotSettingsRow) -> Self {
        Self {
            channel_id: row.channel_id.map(|id| ChannelId::new(as_u64(id))),
            exempt_admins: row.exempt_admins,
            exempt_role_id: row.exempt_role_id.map(|id| RoleId::new(as_u64(id))),
            purge_seconds: row.purge_seconds,
        }
    }
}

impl HoneypotConfig {
    #[must_use]
    pub const fn is_armed(&self) -> bool {
        self.channel_id.is_some()
    }

    pub fn from_form(
        channel_id: &str,
        exempt_admins: bool,
        exempt_role_id: &str,
        purge_seconds: &str,
    ) -> Result<Self> {
        Ok(Self {
            channel_id: parse_optional_id("channel", channel_id)?
                .map(ChannelId::new),
            exempt_admins,
            exempt_role_id: parse_optional_id("exempt role", exempt_role_id)?
                .map(RoleId::new),
            purge_seconds: HoneypotSettingsRow::parse_purge_seconds(purge_seconds),
        })
    }

    #[must_use]
    pub fn purge_seconds_u32(&self) -> u32 {
        u32::try_from(
            self.purge_seconds.clamp(0, HoneypotSettingsRow::MAX_PURGE_SECONDS),
        )
        .unwrap_or(0)
    }

    pub const fn apply(self, row: &mut HoneypotSettingsRow) {
        row.channel_id = match self.channel_id {
            Some(id) => Some(as_i64(id.get())),
            None => None,
        };
        row.exempt_admins = self.exempt_admins;
        row.exempt_role_id = match self.exempt_role_id {
            Some(id) => Some(as_i64(id.get())),
            None => None,
        };
        row.purge_seconds = self.purge_seconds;
    }

    pub const fn arm_row(row: &mut HoneypotSettingsRow, channel_id: ChannelId) {
        row.channel_id = Some(as_i64(channel_id.get()));
    }

    pub const fn disarm_row(row: &mut HoneypotSettingsRow) {
        row.channel_id = None;
    }
}

fn parse_optional_id(field: &'static str, raw: &str) -> Result<Option<u64>> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed.parse::<u64>().map(Some).map_err(|_e| HoneypotError::InvalidSnowflake {
        field,
        value: trimmed.to_string(),
    })
}

pub struct HoneypotSettings;

impl HoneypotSettings {
    pub async fn get(
        store: &HoneypotStore,
        guild_id: GuildId,
    ) -> Result<HoneypotConfig> {
        let row = store.get(as_i64(guild_id.get())).await?;

        Ok(HoneypotConfig::from(row.as_ref()))
    }

    pub async fn arm(
        store: &HoneypotStore,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> Result<HoneypotConfig> {
        let row = store
            .update(as_i64(guild_id.get()), |row| {
                HoneypotConfig::arm_row(row, channel_id);
            })
            .await?;

        Ok(HoneypotConfig::from(row.as_ref()))
    }

    pub async fn disarm(
        store: &HoneypotStore,
        guild_id: GuildId,
    ) -> Result<HoneypotConfig> {
        let row =
            store.update(as_i64(guild_id.get()), HoneypotConfig::disarm_row).await?;

        Ok(HoneypotConfig::from(row.as_ref()))
    }

    pub async fn save(
        store: &HoneypotStore,
        guild_id: GuildId,
        config: HoneypotConfig,
    ) -> Result<HoneypotConfig> {
        let row =
            store.update(as_i64(guild_id.get()), |row| config.apply(row)).await?;

        Ok(HoneypotConfig::from(row.as_ref()))
    }
}
