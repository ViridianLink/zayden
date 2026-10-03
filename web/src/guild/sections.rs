use sqlx::PgPool;
use topcoat::context::Cx;
use twilight_http::Client;
use zayden_app::state::AppState as ZaydenAppState;

use super::access::admin_app;
use super::dto::{
    AiSection,
    FamilySection,
    FaqSection,
    GeneralSection,
    HoneypotSection,
    LfgSection,
    MusicSection,
    SectionSettings,
    SupportSection,
    TempVoiceSection,
};
use super::error::{GuildError, server_err};
use super::parse::opt_str;
use super::patreon::fetch_patreon_status;
use super::support::{fetch_helper_links, fetch_support_roles};
use super::youtube::fetch_youtube_status;
use crate::auth::{db_pool, discord_client};
use crate::util::server_error_text;

async fn general_section(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<GeneralSection, GuildError> {
    let (channels, roles) = tokio::join!(
        app.settings.channels.get(guild_id),
        app.settings.roles.get(guild_id),
    );
    let channels = channels.map_err(server_err)?;
    let roles = roles.map_err(server_err)?;

    Ok(GeneralSection {
        rules_channel_id: opt_str(channels.rules_channel_id),
        general_channel_id: opt_str(channels.general_channel_id),
        spoiler_channel_id: opt_str(channels.spoiler_channel_id),
        artist_role_id: opt_str(roles.artist_role_id),
        sleep_role_id: opt_str(roles.sleep_role_id),
        verified_role_id: opt_str(roles.verified_role_id),
    })
}

async fn ai_section(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<AiSection, GuildError> {
    let ai = app.settings.ai.get(guild_id).await.map_err(server_err)?;

    Ok(AiSection { enabled: ai.enabled, channel_id: opt_str(ai.channel_id) })
}

async fn family_section(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<FamilySection, GuildError> {
    let family = app.settings.family.get(guild_id).await.map_err(server_err)?;

    Ok(FamilySection { max_partners: family.max_partners.to_string() })
}

async fn honeypot_section(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<HoneypotSection, GuildError> {
    let honeypot = app.settings.honeypot.get(guild_id).await.map_err(server_err)?;

    Ok(HoneypotSection {
        channel_id: opt_str(honeypot.channel_id),
        exempt_admins: honeypot.exempt_admins,
        exempt_role_id: opt_str(honeypot.exempt_role_id),
        purge_seconds: honeypot.purge_seconds.to_string(),
    })
}

async fn lfg_section(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<LfgSection, GuildError> {
    let lfg = app.settings.lfg.get(guild_id).await.map_err(server_err)?;

    Ok(LfgSection {
        channel_id: opt_str(lfg.lfg_channel_id),
        role_id: opt_str(lfg.lfg_role_id),
        scheduled_thread_id: opt_str(lfg.lfg_scheduled_thread_id),
    })
}

async fn music_section(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<MusicSection, GuildError> {
    let music = app.settings.music.get(guild_id).await.map_err(server_err)?;

    Ok(MusicSection {
        dj_role_id: opt_str(music.dj_role_id),
        auto_disconnect_secs: music.auto_disconnect_secs.to_string(),
        announce_now_playing: music.announce_now_playing,
        announce_channel_id: opt_str(music.announce_channel_id),
    })
}

async fn temp_voice_section(
    app: &ZaydenAppState,
    guild_id: i64,
) -> Result<TempVoiceSection, GuildError> {
    let temp_voice =
        app.settings.temp_voice.get(guild_id).await.map_err(server_err)?;

    Ok(TempVoiceSection {
        category: opt_str(temp_voice.temp_voice_category),
        creator_channel: opt_str(temp_voice.temp_voice_creator_channel),
    })
}

async fn support_section(
    app: &ZaydenAppState,
    pool: &PgPool,
    http: &Client,
    guild_id: i64,
) -> Result<SupportSection, GuildError> {
    let (support, suggestions, faq, support_roles, helper_links) = tokio::join!(
        app.settings.support.get(guild_id),
        app.settings.suggestions.get(guild_id),
        app.settings.faq.get(guild_id),
        fetch_support_roles(pool, guild_id),
        fetch_helper_links(pool, http, guild_id),
    );

    let support = support.map_err(server_err)?;
    let suggestions = suggestions.map_err(server_err)?;
    let faq = faq.map_err(server_err)?;

    Ok(SupportSection {
        support_channel_id: opt_str(support.support_channel_id),
        solved_tag_id: opt_str(support.solved_tag_id),
        closed_tag_id: opt_str(support.closed_tag_id),
        solved_archive_secs: support.solved_archive_secs.to_string(),
        idle_enabled: support.idle_enabled,
        idle_after_secs: support.idle_after_secs.to_string(),
        idle_close_enabled: support.idle_close_enabled,
        idle_close_after_secs: support.idle_close_after_secs.to_string(),
        stale_enabled: support.stale_enabled,
        stale_tag_id: opt_str(support.stale_tag_id),
        stale_after_secs: support.stale_after_secs.to_string(),
        suggestions_channel_id: opt_str(suggestions.suggestions_channel_id),
        review_channel_id: opt_str(suggestions.review_channel_id),
        promote_threshold: suggestions.promote_threshold.to_string(),
        demote_threshold: suggestions.demote_threshold.to_string(),
        faq: FaqSection {
            enabled: faq.enabled,
            auto_triage: faq.auto_triage,
            auto_generate: faq.auto_generate,
            wiki_url: faq.wiki_url.clone().unwrap_or_default(),
            wiki_api_key_set: faq
                .wiki_api_key
                .as_deref()
                .is_some_and(|key| !key.trim().is_empty()),
            wiki_locale: faq.wiki_locale.clone(),
            max_results: faq.max_results.to_string(),
            answer_max_tokens: faq.answer_max_tokens.to_string(),
            answer_temperature: faq.answer_temperature.to_string(),
        },
        support_roles: support_roles.map_err(server_error_text),
        helper_links: helper_links.map_err(server_error_text),
    })
}

/// The stored settings of one section, by its slug. An unknown slug reads
/// the general section.
pub async fn get_section_settings(
    cx: &Cx,
    guild: &str,
    section: &str,
) -> Result<SectionSettings, GuildError> {
    let (guild_id, app) = admin_app(cx, guild).await?;

    Ok(match section {
        "ai" => SectionSettings::Ai(ai_section(app, guild_id).await?),
        "family" => SectionSettings::Family(family_section(app, guild_id).await?),
        "honeypot" => {
            SectionSettings::Honeypot(honeypot_section(app, guild_id).await?)
        },
        "lfg" => SectionSettings::Lfg(lfg_section(app, guild_id).await?),
        "music" => SectionSettings::Music(music_section(app, guild_id).await?),
        "temp-voice" => {
            SectionSettings::TempVoice(temp_voice_section(app, guild_id).await?)
        },
        "patreon" => SectionSettings::Patreon(
            fetch_patreon_status(app, guild_id).await.map_err(server_error_text),
        ),
        "youtube" => SectionSettings::Youtube(
            fetch_youtube_status(app, guild_id).await.map_err(server_error_text),
        ),
        "support" => {
            let pool = db_pool(cx)?;
            let http = discord_client(cx)?;
            SectionSettings::Support(Box::new(
                support_section(app, pool, http, guild_id).await?,
            ))
        },
        _ => SectionSettings::General(general_section(app, guild_id).await?),
    })
}
