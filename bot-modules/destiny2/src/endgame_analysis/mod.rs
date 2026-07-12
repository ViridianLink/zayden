pub mod cron;
pub mod dimwishlist;
pub mod error;
pub mod sheet;
pub mod tierlist;
pub mod weapon;

pub use cron::EndgameAnalysisSheetCron;
pub use dimwishlist::DimWishlistCommand;
pub use error::EndgameAnalysisError;
use error::Result;
pub use tierlist::TierListCommand;
pub use weapon::WeaponCommand;

pub struct DestinyWeapon {
    pub id: i64,
    pub icon: String,
    pub name: String,
    pub column_1: Vec<i64>,
    pub column_2: Vec<i64>,
    pub perk_1: Vec<i64>,
    pub perk_2: Vec<i64>,
}
