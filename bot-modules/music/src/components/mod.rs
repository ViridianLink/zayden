pub mod control_panel;
pub mod queue_pager;

use std::sync::Arc;

pub use control_panel::ControlPanel;
pub use queue_pager::QueuePager;
use serenity::all::{ComponentInteraction, GuildId, Http, UserId};
use songbird::Songbird;
use zayden_app::config::{MusicSettingsRow, SettingsStore};

use crate::error::{MusicError, Result};
use crate::manager::MusicManager;
use crate::permissions;
use crate::resolve::TrackResolver;
use crate::voice::Playback;

pub struct PanelServices {
    pub http: Arc<Http>,
    pub songbird: Arc<Songbird>,
    pub music: Arc<MusicManager>,
    pub resolver: Arc<dyn TrackResolver>,
    pub settings: Arc<SettingsStore<MusicSettingsRow>>,
}

pub struct PanelCtx<'a> {
    pub http: &'a Http,
    pub http_owned: Arc<Http>,
    pub interaction: &'a ComponentInteraction,
    pub guild_id: GuildId,
    pub bot_id: UserId,
    pub songbird: Arc<Songbird>,
    pub music: Arc<MusicManager>,
    pub resolver: Arc<dyn TrackResolver>,
    pub settings: Arc<SettingsStore<MusicSettingsRow>>,
}

impl<'a> PanelCtx<'a> {
    pub fn new(
        http: &'a Http,
        interaction: &'a ComponentInteraction,
        bot_id: UserId,
        services: PanelServices,
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
        })
    }

    #[must_use]
    pub fn playback(&self) -> Playback {
        Playback {
            http: Arc::clone(&self.http_owned),
            songbird: Arc::clone(&self.songbird),
            music: Arc::clone(&self.music),
            resolver: Arc::clone(&self.resolver),
        }
    }

    pub async fn settings(&self) -> Result<Arc<MusicSettingsRow>> {
        Ok(self.settings.get(zayden_core::as_i64(self.guild_id.get())).await?)
    }

    pub fn require_privileged(&self, settings: &MusicSettingsRow) -> Result<()> {
        let member = self.interaction.member.as_ref();
        let privileged = permissions::is_privileged(
            member.map_or(&[], |m| m.roles.as_slice()),
            member.and_then(|m| m.permissions),
            settings.dj_role_id,
        );
        if privileged { Ok(()) } else { Err(MusicError::NotPrivileged) }
    }
}

pub const CONTROL_PANEL_PREFIX: &str = "music_control:";
pub const QUEUE_PAGER_PREFIX: &str = "music_queue_page:";
