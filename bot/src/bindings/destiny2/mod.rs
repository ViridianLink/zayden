pub mod endgame_analysis;
pub mod loadouts;
pub mod perk;
pub mod raid_guide;

use endgame_analysis::slash_commands::{DimWishlist, TierList, Weapon};
use loadouts::Loadout;
use perk::Perk;
use raid_guide::RaidGuide;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder
        .add_command(Perk)
        .add_autocomplete(Perk)
        .add_command(DimWishlist)
        .add_command(TierList)
        .add_autocomplete(TierList)
        .add_command(Weapon)
        .add_autocomplete(Weapon)
        .add_command(Loadout)
        .add_command(RaidGuide);
}
