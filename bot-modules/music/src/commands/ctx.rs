use std::sync::Arc;

use serenity::all::{CommandInteraction, GuildId, Http, UserId};
use songbird::Songbird;
use zayden_app::config::{MusicSettingsRow, SettingsStore};
use zayden_app::entitlement::EntitlementService;

use crate::error::{MusicError, Result};
use crate::manager::MusicManager;
use crate::permissions;
use crate::resolve::TrackResolver;
use crate::voice::SessionRequest;

pub struct MusicServices {
    pub http: Arc<Http>,
    pub songbird: Arc<Songbird>,
    pub music: Arc<MusicManager>,
    pub resolver: Arc<dyn TrackResolver>,
    pub settings: Arc<SettingsStore<MusicSettingsRow>>,
    pub entitlements: Arc<EntitlementService>,
}

pub struct MusicCtx<'a> {
    pub http: &'a Http,
    /// Owned handle for the lazy playlist-tail background task.
    pub http_owned: Arc<Http>,
    pub interaction: &'a CommandInteraction,
    pub guild_id: GuildId,
    pub bot_id: UserId,
    pub songbird: Arc<Songbird>,
    pub music: Arc<MusicManager>,
    pub resolver: Arc<dyn TrackResolver>,
    pub settings: Arc<SettingsStore<MusicSettingsRow>>,
    pub entitlements: Arc<EntitlementService>,
}

impl<'a> MusicCtx<'a> {
    pub fn new(
        http: &'a Http,
        interaction: &'a CommandInteraction,
        bot_id: UserId,
        services: MusicServices,
    ) -> Result<Self> {
        let guild_id = interaction.guild_id.ok_or(MusicError::MissingGuildId)?;
        Ok(Self {
            http,
            http_owned: services.http,
            interaction,
            guild_id,
            bot_id,
            songbird: services.songbird,
            music: services.music,
            resolver: services.resolver,
            settings: services.settings,
            entitlements: services.entitlements,
        })
    }

    pub async fn settings(&self) -> Result<Arc<MusicSettingsRow>> {
        Ok(self.settings.get(zayden_core::as_i64(self.guild_id.get())).await?)
    }

    #[must_use]
    pub fn session_request(&self, settings: &MusicSettingsRow) -> SessionRequest {
        SessionRequest {
            guild_id: self.guild_id,
            user_id: self.interaction.user.id,
            bot_id: self.bot_id,
            text_channel: self.interaction.channel_id,
            default_volume: u8::try_from(settings.default_volume).unwrap_or(100),
            auto_disconnect_secs: zayden_core::as_u64(i64::from(
                settings.auto_disconnect_secs,
            )),
            stay_connected: settings.stay_connected,
        }
    }

    #[must_use]
    pub fn is_privileged(&self, settings: &MusicSettingsRow) -> bool {
        let member = self.interaction.member.as_ref();
        permissions::is_privileged(
            member.map_or(&[], |m| m.roles.as_slice()),
            member.and_then(|m| m.permissions),
            settings.dj_role_id,
        )
    }

    pub fn require_privileged(&self, settings: &MusicSettingsRow) -> Result<()> {
        if self.is_privileged(settings) {
            Ok(())
        } else {
            Err(MusicError::NotPrivileged)
        }
    }
}
