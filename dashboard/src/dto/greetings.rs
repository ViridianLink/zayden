use serde::{Deserialize, Serialize};

use crate::dto::Tier;

#[derive(Clone, Serialize, Deserialize)]
pub struct GreetingImageInfo {
    pub(crate) id: String,
    pub(crate) url: String,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct CooldownView {
    pub(crate) user_secs: i32,
    pub(crate) guild_secs: i32,
    pub(crate) floor_user_secs: i32,
    pub(crate) floor_guild_secs: i32,
    pub(crate) tier: Tier,
    pub(crate) next_tier: Option<Tier>,
    pub(crate) next_floor_user_secs: i32,
    pub(crate) next_floor_guild_secs: i32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GreetingsView {
    pub(crate) morning_message: String,
    pub(crate) night_message: String,
    pub(crate) morning: Vec<GreetingImageInfo>,
    pub(crate) night: Vec<GreetingImageInfo>,
    pub(crate) allowed_channels: Vec<String>,
    pub(crate) cooldowns: CooldownView,
}
