use std::sync::Arc;

use reqwest::Client;
use serenity::all::GuildId;
use songbird::Songbird;

use crate::{GuildMusic, guild_music::MusicQueue};

pub trait MusicData: Send + Sync + 'static {
    fn http(&self) -> Client;

    fn songbird(&self) -> Arc<Songbird>;

    fn guild_music(&self, guild: GuildId) -> Option<&GuildMusic>;

    fn guild_music_mut(&mut self, guild: GuildId) -> &mut GuildMusic;

    fn queue(&self, guild: GuildId) -> Option<&MusicQueue> {
        self.guild_music(guild).map(|music| music.queue())
    }

    fn queue_mut(&mut self, guild: GuildId) -> &mut MusicQueue {
        self.guild_music_mut(guild).queue_mut()
    }
}
