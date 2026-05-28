use std::borrow::Cow;
use std::time::Duration;

use serenity::all::{GuildId, Permissions};
use zayden_app::entitlement::Tier;

#[derive(Debug, Clone, Default)]
pub enum CommandScope {
    #[default]
    Global,
    Guilds(Cow<'static, [GuildId]>),
    ExcludeGuilds(Cow<'static, [GuildId]>),
}

#[derive(Debug, Clone)]
pub struct CommandMetadata {
    pub required_tier: Tier,
    pub required_perms: Permissions,
    pub cooldown: Option<Duration>,
}

impl Default for CommandMetadata {
    fn default() -> Self {
        Self {
            required_tier: Tier::Free,
            required_perms: Permissions::empty(),
            cooldown: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum IdMatch {
    Exact(Cow<'static, str>),
    Prefix(Cow<'static, str>),
}
