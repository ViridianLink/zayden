use honeypot::{GuildId, HoneypotConfig, HoneypotSettings};
use topcoat::context::Cx;
use twilight_model::channel::ChannelType;
use twilight_model::id::Id;
use zayden_app::config::MusicSettingsRow;
use zayden_app::state::AppState as ZaydenAppState;

use super::access::admin_app;
use super::error::{GuildError, server_err};
use super::form::form_args;
use super::parse::{parse_flag, parse_id, parse_max_partners};
use crate::auth::{AuthError, GuildIds, discord_client};

form_args! {
    ChannelSettingsForm {
        guild,
        rules_channel_id,
        general_channel_id,
        spoiler_channel_id,
    }
}

form_args! {
    RoleSettingsForm { guild, artist_role_id, sleep_role_id, verified_role_id }
}

form_args! {
    ServerSettingsForm {
        guild,
        rules_channel_id,
        general_channel_id,
        spoiler_channel_id,
        artist_role_id,
        sleep_role_id,
        verified_role_id,
    }
}

form_args! {
    TempVoiceSettingsForm { guild, temp_voice_category, temp_voice_creator_channel }
}

form_args! {
    CreatorChannelForm { guild, temp_voice_category }
}

form_args! {
    FamilySettingsForm { guild, max_partners }
}

form_args! {
    MusicSettingsForm {
        guild,
        dj_role_id,
        auto_disconnect_secs,
        announce_now_playing,
        announce_channel_id,
    }
}

form_args! {
    HoneypotSettingsForm {
        guild,
        channel_id,
        exempt_admins,
        exempt_role_id,
        purge_seconds,
    }
}

form_args! {
    AiSettingsForm { guild, enabled, channel_id }
}

form_args! {
    LfgSettingsForm {
        guild,
        lfg_channel_id,
        lfg_role_id,
        lfg_scheduled_thread_id,
    }
}

/// The voice channel name Zayden creates for temp voice.
pub const CREATOR_CHANNEL_NAME: &str = "\u{2795} Creator Channel";

/// Rules, general and spoiler channel ids, in that order.
type ChannelIds = [Option<i64>; 3];

/// Artist, sleep and verified role ids, in that order.
type RoleIds = [Option<i64>; 3];

fn channel_ids(rules: &str, general: &str, spoiler: &str) -> ChannelIds {
    [parse_id(rules), parse_id(general), parse_id(spoiler)]
}

fn role_ids(artist: &str, sleep: &str, verified: &str) -> RoleIds {
    [parse_id(artist), parse_id(sleep), parse_id(verified)]
}

async fn write_channels(
    app: &ZaydenAppState,
    guild_id: i64,
    [rules, general, spoiler]: ChannelIds,
) -> Result<(), sqlx::Error> {
    app.settings
        .channels
        .update(guild_id, |p| {
            p.rules_channel_id = rules;
            p.general_channel_id = general;
            p.spoiler_channel_id = spoiler;
        })
        .await
        .map(|_| ())
}

async fn write_roles(
    app: &ZaydenAppState,
    guild_id: i64,
    [artist, sleep, verified]: RoleIds,
) -> Result<(), sqlx::Error> {
    app.settings
        .roles
        .update(guild_id, |p| {
            p.artist_role_id = artist;
            p.sleep_role_id = sleep;
            p.verified_role_id = verified;
        })
        .await
        .map(|_| ())
}

pub async fn save_channel_settings(
    cx: &Cx,
    form: &ChannelSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channels = channel_ids(
        &form.rules_channel_id,
        &form.general_channel_id,
        &form.spoiler_channel_id,
    );

    channels
        .iter()
        .fold(GuildIds::default(), |ids, id| ids.channel(*id))
        .ensure_in(cx, guild_id)
        .await?;

    write_channels(app, guild_id, channels).await.map_err(server_err)
}

pub async fn save_role_settings(
    cx: &Cx,
    form: &RoleSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let roles =
        role_ids(&form.artist_role_id, &form.sleep_role_id, &form.verified_role_id);

    roles
        .iter()
        .fold(GuildIds::default(), |ids, id| ids.role(*id))
        .ensure_in(cx, guild_id)
        .await?;

    write_roles(app, guild_id, roles).await.map_err(server_err)
}

/// Saves the channels and roles of Server settings together: every id is
/// checked against the server before either table is written.
pub async fn save_server_settings(
    cx: &Cx,
    form: &ServerSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channels = channel_ids(
        &form.rules_channel_id,
        &form.general_channel_id,
        &form.spoiler_channel_id,
    );
    let roles =
        role_ids(&form.artist_role_id, &form.sleep_role_id, &form.verified_role_id);

    let ids = channels.iter().fold(GuildIds::default(), |ids, id| ids.channel(*id));
    roles.iter().fold(ids, |ids, id| ids.role(*id)).ensure_in(cx, guild_id).await?;

    write_channels(app, guild_id, channels).await.map_err(server_err)?;
    write_roles(app, guild_id, roles).await.map_err(|e| GuildError::PartlySaved {
        saved: "The channels",
        unsaved: "the roles",
        reason: e.to_string(),
    })
}

pub async fn save_temp_voice_settings(
    cx: &Cx,
    form: &TempVoiceSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let category = parse_id(&form.temp_voice_category);
    let creator_channel = parse_id(&form.temp_voice_creator_channel);

    GuildIds::default()
        .channel(category)
        .channel(creator_channel)
        .ensure_in(cx, guild_id)
        .await?;

    app.settings
        .temp_voice
        .update(guild_id, |p| {
            p.temp_voice_category = category;
            p.temp_voice_creator_channel = creator_channel;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn create_temp_voice_creator_channel(
    cx: &Cx,
    form: &CreatorChannelForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let category = parse_id(&form.temp_voice_category)
        .and_then(|id| Id::new_checked(id.cast_unsigned()));
    let Some(category) = category else {
        return Err(GuildError::NoCategory);
    };

    GuildIds::default()
        .channel(Some(category.get().cast_signed()))
        .ensure_in(cx, guild_id)
        .await?;

    let guild_ref = Id::new_checked(guild_id.cast_unsigned())
        .ok_or(AuthError::InvalidGuildId)?;

    let channel = discord_client(cx)?
        .create_guild_channel(guild_ref, CREATOR_CHANNEL_NAME)
        .kind(ChannelType::GuildVoice)
        .parent_id(category)
        .await?
        .model()
        .await?;

    app.settings
        .temp_voice
        .update(guild_id, |p| {
            p.temp_voice_category = Some(category.get().cast_signed());
            p.temp_voice_creator_channel = Some(channel.id.get().cast_signed());
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_family_settings(
    cx: &Cx,
    form: &FamilySettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let max_partners = parse_max_partners(&form.max_partners);

    app.settings
        .family
        .update(guild_id, |p| {
            p.max_partners = max_partners;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_music_settings(
    cx: &Cx,
    form: &MusicSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let auto_disconnect_secs =
        MusicSettingsRow::parse_auto_disconnect_secs(&form.auto_disconnect_secs);
    let announce_now_playing = parse_flag(&form.announce_now_playing);
    let dj_role_id = parse_id(&form.dj_role_id);
    let announce_channel_id = parse_id(&form.announce_channel_id);

    GuildIds::default()
        .role(dj_role_id)
        .channel(announce_channel_id)
        .ensure_in(cx, guild_id)
        .await?;

    app.settings
        .music
        .update(guild_id, |p| {
            p.dj_role_id = dj_role_id;
            p.auto_disconnect_secs = auto_disconnect_secs;
            p.announce_now_playing = announce_now_playing;
            p.announce_channel_id = announce_channel_id;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_honeypot_settings(
    cx: &Cx,
    form: &HoneypotSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let config = HoneypotConfig::from_form(
        &form.channel_id,
        parse_flag(&form.exempt_admins),
        &form.exempt_role_id,
        &form.purge_seconds,
    )
    .map_err(server_err)?;

    GuildIds::default()
        .channel(config.channel_id.map(|id| id.get().cast_signed()))
        .role(config.exempt_role_id.map(|id| id.get().cast_signed()))
        .ensure_in(cx, guild_id)
        .await?;

    HoneypotSettings::save(
        &app.settings.honeypot,
        GuildId::new(guild_id.cast_unsigned()),
        config,
    )
    .await
    .map(|_| ())
    .map_err(server_err)
}

pub async fn save_ai_settings(
    cx: &Cx,
    form: &AiSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let enabled = parse_flag(&form.enabled);
    let channel_id = parse_id(&form.channel_id);

    GuildIds::default().channel(channel_id).ensure_in(cx, guild_id).await?;

    app.settings
        .ai
        .update(guild_id, |p| {
            p.enabled = enabled;
            p.channel_id = channel_id;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}

pub async fn save_lfg_settings(
    cx: &Cx,
    form: &LfgSettingsForm,
) -> Result<(), GuildError> {
    let (guild_id, app) = admin_app(cx, &form.guild).await?;

    let channel_id = parse_id(&form.lfg_channel_id);
    let role_id = parse_id(&form.lfg_role_id);
    let scheduled_thread_id = parse_id(&form.lfg_scheduled_thread_id);

    GuildIds::default()
        .channel(channel_id)
        .role(role_id)
        .thread(scheduled_thread_id)
        .ensure_in(cx, guild_id)
        .await?;

    app.settings
        .lfg
        .update(guild_id, |p| {
            p.lfg_channel_id = channel_id;
            p.lfg_role_id = role_id;
            p.lfg_scheduled_thread_id = scheduled_thread_id;
        })
        .await
        .map(|_| ())
        .map_err(server_err)
}
