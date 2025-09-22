use async_trait::async_trait;
use sqlx::{Database, FromRow, Pool};

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

#[derive(FromRow)]
pub struct DestinyWeapon {
    pub id: i64,
    pub icon: String,
    pub name: String,
    pub column_1: Vec<i64>,
    pub column_2: Vec<i64>,
    pub perk_1: Vec<i64>,
    pub perk_2: Vec<i64>,
}

#[async_trait]
pub trait DestinyPerkManager<Db: Database> {
    async fn get(pool: &Pool<Db>, name: &str) -> sqlx::Result<DestinyPerk>;

    async fn get_all(pool: &Pool<Db>, names: &[String]) -> sqlx::Result<Vec<DestinyPerk>>;
}

#[derive(FromRow)]
pub struct DestinyPerk {
    pub id: i64,
    pub name: String,
    pub description: String,
}
