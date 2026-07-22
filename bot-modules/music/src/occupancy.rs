use std::collections::HashSet;

use dashmap::DashMap;
use serenity::all::{ChannelId, Guild, GuildId, UserId, VoiceState};

#[derive(Default)]
pub struct VoiceOccupancy {
    members: DashMap<(UserId, GuildId), ChannelId>,
}

impl VoiceOccupancy {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn guild_create(&self, guild: &Guild) {
        for state in &guild.voice_states {
            if let Some(channel_id) = state.channel_id {
                self.members.insert((state.user_id, guild.id), channel_id);
            }
        }
    }

    pub fn update(&self, state: &VoiceState) {
        let Some(guild_id) = state.guild_id else {
            return;
        };

        match state.channel_id {
            Some(channel_id) => {
                self.members.insert((state.user_id, guild_id), channel_id);
            },
            None => {
                self.members.remove(&(state.user_id, guild_id));
            },
        }
    }

    #[must_use]
    pub fn non_bot_count(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
        bot_id: UserId,
    ) -> usize {
        self.members
            .iter()
            .filter(|entry| {
                let (user_id, g) = *entry.key();
                g == guild_id && *entry.value() == channel_id && user_id != bot_id
            })
            .count()
    }

    #[must_use]
    pub fn channel_of(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Option<ChannelId> {
        self.members.get(&(user_id, guild_id)).map(|entry| *entry.value())
    }

    #[must_use]
    pub fn members_in_channel(
        &self,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) -> HashSet<UserId> {
        self.members
            .iter()
            .filter_map(|entry| {
                let (user_id, g) = *entry.key();
                (g == guild_id && *entry.value() == channel_id).then_some(user_id)
            })
            .collect()
    }
}
