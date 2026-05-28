use serenity::all::CreateCommand;
use zayden_core::ApplicationCommand;

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
        DungeonReport {}.command(),
        Goof {}.command(),
        Hello {}.command(),
        Playlist {}.command(),
        RaidReport {}.command(),
        Sensitivity {}.command(),
        Socials {}.command(),
    ]
}
