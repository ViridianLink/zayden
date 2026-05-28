/// Per-guild configuration loaded from the `guild_config` table.
#[derive(Debug, Clone)]
pub struct GuildConfig;

/// Partial update applied to a `GuildConfig` row.
#[derive(Debug, Default)]
pub struct GuildConfigPatch;
