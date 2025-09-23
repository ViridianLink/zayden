pub mod dimwishlist;
pub mod endgame_analysis;
pub mod error;
pub mod tierlist;
pub mod weapon;

pub use dimwishlist::DimWishlistCommand;
pub use tierlist::TierListCommand;
pub use weapon::WeaponCommand;

pub use error::Error;
use error::Result;

pub struct DestinyWeapon {
    pub id: i64,
    pub icon: String,
    pub name: String,
    pub column_1: Vec<i64>,
    pub column_2: Vec<i64>,
    pub perk_1: Vec<i64>,
    pub perk_2: Vec<i64>,
}
