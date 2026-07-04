use std::sync::Arc;

use dashmap::DashMap;
use serenity::all::{GenericChannelId, GuildId};
use tokio::sync::Mutex;

use crate::occupancy::VoiceOccupancy;
use crate::player::GuildPlayer;

#[derive(Default)]
pub struct MusicManager {
    players: DashMap<GuildId, Arc<Mutex<GuildPlayer>>>,
    occupancy: VoiceOccupancy,
}

impl MusicManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn occupancy(&self) -> &VoiceOccupancy {
        &self.occupancy
    }

    #[must_use]
    pub fn get_or_create_player(
        &self,
        guild_id: GuildId,
        text_channel: GenericChannelId,
        default_volume: u8,
    ) -> Arc<Mutex<GuildPlayer>> {
        Arc::clone(
            self.players
                .entry(guild_id)
                .or_insert_with(|| {
                    Arc::new(Mutex::new(GuildPlayer::new(
                        text_channel,
                        default_volume,
                    )))
                })
                .value(),
        )
    }

    #[must_use]
    pub fn get(&self, guild_id: GuildId) -> Option<Arc<Mutex<GuildPlayer>>> {
        self.players.get(&guild_id).map(|entry| Arc::clone(&entry))
    }

    #[must_use]
    pub fn remove(&self, guild_id: GuildId) -> Option<Arc<Mutex<GuildPlayer>>> {
        self.players.remove(&guild_id).map(|(_, player)| player)
    }
}
