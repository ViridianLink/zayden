use std::sync::OnceLock;

use serenity::all::GuildId;

use crate::RegistryBuilder;

mod dungeon_report;
mod goof;
mod hello;
mod playlist;
mod raidreport;
mod sensitivity;
mod socials;

use dungeon_report::DungeonReport;
use goof::Goof;
use hello::Hello;
use playlist::Playlist;
use raidreport::RaidReport;
use sensitivity::Sensitivity;
use socials::Socials;

static LLAMA_GUILDS: OnceLock<GuildId> = OnceLock::new();

pub(super) fn llama_guild() -> Option<GuildId> {
    LLAMA_GUILDS.get().copied()
}

pub fn register(builder: &mut RegistryBuilder, llamad2_guild: u64) {
    LLAMA_GUILDS.get_or_init(|| GuildId::new(llamad2_guild));

    builder
        .add_command(DungeonReport)
        .add_command(Goof)
        .add_command(Hello)
        .add_command(Playlist)
        .add_command(RaidReport)
        .add_command(Sensitivity)
        .add_command(Socials);
}
