use endgame_analysis::slash_commands::{DimWishlist, TierList, Weapon};
use loadouts::Loadout;
pub use perk::Perk;
use raid_guide::RaidGuide;
use serenity::all::CreateCommand;
use zayden_core::ApplicationCommand;

pub mod endgame_analysis;
pub mod loadouts;
pub mod perk;
pub mod raid_guide;

pub fn register<'a>() -> [CreateCommand<'a>; 6] {
    [
        DimWishlist {}.command(),
        Weapon {}.command(),
        TierList {}.command(),
        Perk {}.command(),
        Loadout {}.command(),
        RaidGuide {}.command(),
    ]
}
