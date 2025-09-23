use endgame_analysis::slash_commands::{DimWishlist, TierList, Weapon};
use loadouts::Loadout;
pub use perk::Perk;
use raid_guide::RaidGuide;
use serenity::all::{Context, CreateCommand};
use zayden_core::ApplicationCommand;

pub mod endgame_analysis;
pub mod loadouts;
pub mod perk;
pub mod raid_guide;

pub fn register(ctx: &Context) -> [CreateCommand<'_>; 6] {
    [
        DimWishlist::register(ctx).unwrap(),
        Weapon::register(ctx).unwrap(),
        TierList::register(ctx).unwrap(),
        Perk::register(ctx).unwrap(),
        Loadout::register(ctx).unwrap(),
        RaidGuide::register(ctx).unwrap(),
    ]
}
