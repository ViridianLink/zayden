use crate::auth::{ChannelInfo, RoleInfo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelperLinkInfo {
    pub user_id: String,
    pub name: String,
    pub link: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PatreonStatus {
    pub connected: bool,
    pub disabled: bool,
    pub creator_name: Option<String>,
    pub campaign_id: Option<String>,
    pub webhook_registered: bool,
    pub channel_id: Option<String>,
    pub public_only: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct YoutubeStatus {
    pub connected: bool,
    pub channel_title: Option<String>,
    pub push_active: bool,
    pub channel_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildDirectory {
    pub channels: Result<Vec<ChannelInfo>, String>,
    pub roles: Result<Vec<RoleInfo>, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralSection {
    pub rules_channel_id: Option<String>,
    pub general_channel_id: Option<String>,
    pub spoiler_channel_id: Option<String>,
    pub artist_role_id: Option<String>,
    pub sleep_role_id: Option<String>,
    pub verified_role_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSection {
    pub enabled: bool,
    pub channel_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilySection {
    pub max_partners: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoneypotSection {
    pub channel_id: Option<String>,
    pub exempt_admins: bool,
    pub exempt_role_id: Option<String>,
    pub purge_seconds: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfgSection {
    pub channel_id: Option<String>,
    pub role_id: Option<String>,
    pub scheduled_thread_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicSection {
    pub dj_role_id: Option<String>,
    pub auto_disconnect_secs: String,
    pub announce_now_playing: bool,
    pub announce_channel_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempVoiceSection {
    pub category: Option<String>,
    pub creator_channel: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaqSection {
    pub enabled: bool,
    pub auto_triage: bool,
    pub auto_generate: bool,
    pub wiki_url: String,
    pub wiki_api_key_set: bool,
    pub wiki_locale: String,
    pub max_results: String,
    pub answer_max_tokens: String,
    pub answer_temperature: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportSection {
    pub support_channel_id: Option<String>,
    pub solved_tag_id: Option<String>,
    pub closed_tag_id: Option<String>,
    pub solved_archive_secs: String,
    pub idle_enabled: bool,
    pub idle_after_secs: String,
    pub idle_close_enabled: bool,
    pub idle_close_after_secs: String,
    pub stale_enabled: bool,
    pub stale_tag_id: Option<String>,
    pub stale_after_secs: String,
    pub suggestions_channel_id: Option<String>,
    pub review_channel_id: Option<String>,
    pub promote_threshold: String,
    pub demote_threshold: String,
    pub faq: FaqSection,
    pub support_roles: Result<Vec<String>, String>,
    pub helper_links: Result<Vec<HelperLinkInfo>, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionSettings {
    General(GeneralSection),
    Ai(AiSection),
    Family(FamilySection),
    Honeypot(HoneypotSection),
    Lfg(LfgSection),
    Music(MusicSection),
    Patreon(Result<PatreonStatus, String>),
    Support(Box<SupportSection>),
    TempVoice(TempVoiceSection),
    Youtube(Result<YoutubeStatus, String>),
}
