use serenity::all::GuildId;

use crate::{LLAMAD2_GUILD, RegistryBuilder};

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

static LLAMA_GUILDS: [GuildId; 1] = [LLAMAD2_GUILD];

pub fn register(builder: &mut RegistryBuilder) {
    builder
        .add_command(DungeonReport)
        .add_command(Goof)
        .add_command(Hello)
        .add_command(Playlist)
        .add_command(RaidReport)
        .add_command(Sensitivity)
        .add_command(Socials);
}
