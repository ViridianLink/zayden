use serde::{Deserialize, Serialize};

use crate::dto::{ChannelInfo, RoleInfo};

#[derive(Clone, Serialize, Deserialize)]
pub struct GuildInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) icon: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HelperLinkInfo {
    pub(crate) user_id: String,
    pub(crate) name: String,
    pub(crate) link: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct PatreonStatus {
    pub(crate) connected: bool,
    pub(crate) disabled: bool,
    pub(crate) creator_name: Option<String>,
    pub(crate) campaign_id: Option<String>,
    pub(crate) webhook_registered: bool,
    pub(crate) channel_id: Option<String>,
    pub(crate) public_only: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GuildDirectory {
    pub(crate) channels: Result<Vec<ChannelInfo>, String>,
    pub(crate) roles: Result<Vec<RoleInfo>, String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[expect(
    clippy::struct_field_names,
    reason = "every field here is a Discord snowflake; the `_id` suffix is what tells a channel id from a channel name at the use site, and it matches the column names the settings stores expose"
)]
pub struct GeneralSection {
    pub(crate) rules_channel_id: Option<String>,
    pub(crate) general_channel_id: Option<String>,
    pub(crate) spoiler_channel_id: Option<String>,
    pub(crate) artist_role_id: Option<String>,
    pub(crate) sleep_role_id: Option<String>,
    pub(crate) verified_role_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AiSection {
    pub(crate) enabled: bool,
    pub(crate) channel_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FamilySection {
    pub(crate) max_partners: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HoneypotSection {
    pub(crate) channel_id: Option<String>,
    pub(crate) exempt_admins: bool,
    pub(crate) exempt_role_id: Option<String>,
    pub(crate) purge_seconds: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[expect(
    clippy::struct_field_names,
    reason = "every field here is a Discord snowflake; the `_id` suffix is what tells a channel id from a channel name at the use site, and it matches the column names the settings stores expose"
)]
pub struct LfgSection {
    pub(crate) channel_id: Option<String>,
    pub(crate) role_id: Option<String>,
    pub(crate) scheduled_thread_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MusicSection {
    pub(crate) dj_role_id: Option<String>,
    pub(crate) auto_disconnect_secs: String,
    pub(crate) announce_now_playing: bool,
    pub(crate) announce_channel_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TempVoiceSection {
    pub(crate) category: Option<String>,
    pub(crate) creator_channel: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FaqSection {
    pub(crate) enabled: bool,
    pub(crate) auto_triage: bool,
    pub(crate) auto_generate: bool,
    pub(crate) wiki_url: String,
    pub(crate) wiki_api_key_set: bool,
    pub(crate) wiki_locale: String,
    pub(crate) max_results: String,
    pub(crate) answer_max_tokens: String,
    pub(crate) answer_temperature: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SupportSection {
    pub(crate) support_channel_id: Option<String>,
    pub(crate) solved_tag_id: Option<String>,
    pub(crate) closed_tag_id: Option<String>,
    pub(crate) solved_archive_secs: String,
    pub(crate) idle_enabled: bool,
    pub(crate) idle_after_secs: String,
    pub(crate) idle_close_enabled: bool,
    pub(crate) idle_close_after_secs: String,
    pub(crate) stale_enabled: bool,
    pub(crate) stale_tag_id: Option<String>,
    pub(crate) stale_after_secs: String,
    pub(crate) suggestions_channel_id: Option<String>,
    pub(crate) review_channel_id: Option<String>,
    pub(crate) promote_threshold: String,
    pub(crate) demote_threshold: String,
    pub(crate) faq: FaqSection,
    pub(crate) support_roles: Result<Vec<String>, String>,
    pub(crate) helper_links: Result<Vec<HelperLinkInfo>, String>,
}

#[derive(Clone, Serialize, Deserialize)]
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
}
