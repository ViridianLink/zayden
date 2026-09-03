use serde::{Deserialize, Serialize};

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
pub struct GuildSettings {
    pub(crate) support_channel_id: Option<String>,
    pub(crate) solved_tag_id: Option<String>,
    pub(crate) helper_role_id: Option<String>,
    pub(crate) solved_archive_secs: String,
    pub(crate) suggestions_channel_id: Option<String>,
    pub(crate) review_channel_id: Option<String>,
    pub(crate) suggestions_promote_threshold: String,
    pub(crate) suggestions_demote_threshold: String,
    pub(crate) rules_channel_id: Option<String>,
    pub(crate) general_channel_id: Option<String>,
    pub(crate) spoiler_channel_id: Option<String>,
    pub(crate) artist_role_id: Option<String>,
    pub(crate) sleep_role_id: Option<String>,
    pub(crate) verified_role_id: Option<String>,
    pub(crate) temp_voice_category: Option<String>,
    pub(crate) temp_voice_creator_channel: Option<String>,
    pub(crate) lfg_channel_id: Option<String>,
    pub(crate) lfg_role_id: Option<String>,
    pub(crate) lfg_scheduled_thread_id: Option<String>,
    pub(crate) family_max_partners: String,
    pub(crate) music_dj_role_id: Option<String>,
    pub(crate) music_auto_disconnect_secs: String,
    pub(crate) music_announce_now_playing: bool,
    pub(crate) music_announce_channel_id: Option<String>,
    pub(crate) honeypot_channel_id: Option<String>,
    pub(crate) honeypot_exempt_admins: bool,
    pub(crate) honeypot_exempt_role_id: Option<String>,
    pub(crate) honeypot_purge_seconds: String,
    pub(crate) ai_enabled: bool,
    pub(crate) ai_channel_id: Option<String>,
    pub(crate) faq_enabled: bool,
    pub(crate) faq_auto_triage: bool,
    pub(crate) faq_auto_generate: bool,
    pub(crate) faq_wiki_url: String,
    pub(crate) faq_wiki_api_key: String,
    pub(crate) faq_wiki_locale: String,
    pub(crate) faq_max_results: String,
    pub(crate) faq_answer_max_tokens: String,
    pub(crate) faq_answer_temperature: String,
}
