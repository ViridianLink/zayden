use serenity::all::{GuildId, UserId};

mod behind_the_scenes;
mod counting_fail;
mod dungeon_report;
mod goodmorning;
mod goof;
mod hello;
mod playlist;
mod raidreport;
mod sensitivity;
mod socials;
mod status_update;

pub use behind_the_scenes::BehindTheScenes;
pub use counting_fail::CountingFail;
pub use dungeon_report::DungeonReport;
pub use goodmorning::{GoodMorning, GoodMorningCache};
pub use goof::Goof;
pub use hello::Hello;
pub use playlist::Playlist;
pub use raidreport::RaidReport;
pub use sensitivity::Sensitivity;
pub use socials::Socials;
pub use status_update::StatusUpdate;

const LLAMA_GUILD: GuildId = GuildId::new(1133034263579734037);
const LLAMA_USER: UserId = UserId::new(367719520082591746);
