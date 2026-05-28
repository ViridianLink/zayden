use std::collections::HashMap;

use jiff::Timestamp;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct GuildConfig {
    pub id: i64,
    pub support_channel_id: Option<i64>,
    pub support_thread_id: i32,
    pub support_role_id: Option<i64>,
    pub faq_channel_id: Option<i64>,
    pub suggestions_channel_id: Option<i64>,
    pub review_channel_id: Option<i64>,
    pub rules_channel_id: Option<i64>,
    pub general_channel_id: Option<i64>,
    pub spoiler_channel_id: Option<i64>,
    pub artist_role_id: Option<i64>,
    pub sleep_role_id: Option<i64>,
    pub temp_voice_category: Option<i64>,
    pub temp_voice_creator_channel: Option<i64>,
    pub thread_id: i32,
    pub lfg_channel_id: Option<i64>,
    pub lfg_role_id: Option<i64>,
    pub lfg_scheduled_thread_id: Option<i64>,
    pub updated_at: Timestamp,
}

#[derive(Debug, Default, Clone)]
pub struct GuildConfigPatch {
    pub support_channel_id: Option<i64>,
    pub support_thread_id: Option<i32>,
    pub support_role_id: Option<i64>,
    pub faq_channel_id: Option<i64>,
    pub suggestions_channel_id: Option<i64>,
    pub review_channel_id: Option<i64>,
    pub rules_channel_id: Option<i64>,
    pub general_channel_id: Option<i64>,
    pub spoiler_channel_id: Option<i64>,
    pub artist_role_id: Option<i64>,
    pub sleep_role_id: Option<i64>,
    pub temp_voice_category: Option<i64>,
    pub temp_voice_creator_channel: Option<i64>,
    pub thread_id: Option<i32>,
    pub lfg_channel_id: Option<i64>,
    pub lfg_role_id: Option<i64>,
    pub lfg_scheduled_thread_id: Option<i64>,
}

pub trait ModuleConfig: Sized {
    fn module_name() -> &'static str;
    fn from_kv(kv: &HashMap<String, Value>) -> Self;
    fn to_kv(&self) -> HashMap<String, Value>;
}
