use std::collections::HashMap;

use serenity::all::{Guild, GuildId, UserId};

pub trait GuildMembersCache: Send + Sync + 'static {
    fn get(&self) -> &HashMap<GuildId, Vec<UserId>>;

    fn get_mut(&mut self) -> &mut HashMap<GuildId, Vec<UserId>>;

    fn guild_create(&mut self, guild: &Guild) {
        self.get_mut()
            .insert(guild.id, guild.members.iter().map(|x| x.user.id).collect());
    }
}
