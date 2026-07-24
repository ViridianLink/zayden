use serenity::all::{GuildId, UserId};

pub mod error;
pub use error::{LlamaD2Error, Result};

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

const LLAMA_GUILD: GuildId = GuildId::new(1_133_034_263_579_734_037);
const LLAMA_USER: UserId = UserId::new(367_719_520_082_591_746);
