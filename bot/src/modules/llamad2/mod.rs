use serenity::all::{Context, CreateCommand};

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
use zayden_core::ApplicationCommand;

pub fn register(ctx: &Context) -> [CreateCommand<'_>; 7] {
    [
        DungeonReport::register(ctx).unwrap(),
        Goof::register(ctx).unwrap(),
        Hello::register(ctx).unwrap(),
        Playlist::register(ctx).unwrap(),
        RaidReport::register(ctx).unwrap(),
        Sensitivity::register(ctx).unwrap(),
        Socials::register(ctx).unwrap(),
    ]
}
