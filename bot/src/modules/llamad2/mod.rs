use serenity::all::CreateCommand;

mod dungeon_report;
mod goof;
mod hello;
mod playlist;
mod raidreport;
mod sensitivity;
mod socials;

pub use dungeon_report::DungeonReport;
pub use goof::Goof;
pub use hello::Hello;
pub use playlist::Playlist;
pub use raidreport::RaidReport;
pub use sensitivity::Sensitivity;
pub use socials::Socials;

pub fn register() -> [CreateCommand<'static>; 7] {
    [
        llamad2::DungeonReport::register(),
        llamad2::Goof::register(),
        llamad2::Hello::register(),
        llamad2::Playlist::register(),
        llamad2::RaidReport::register(),
        llamad2::Sensitivity::register(),
        llamad2::Socials::register(),
    ]
}
