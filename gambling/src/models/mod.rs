use std::collections::HashMap;

pub mod gambling;
mod gambling_effects;
mod gambling_goals;
pub mod gambling_inventory;
pub mod gambling_stats;
mod game_row;

use chrono::{NaiveDateTime, Timelike, Utc};
pub use gambling::GamblingManager;
pub use gambling_effects::{EffectsManager, EffectsRow};
pub use gambling_goals::GamblingGoalsRow;
pub use gambling_inventory::{GamblingItem, InventoryManager, InventoryRow};
pub use gambling_stats::StatsManager;
pub use game_row::{GameManager, GameRow};
use sqlx::Database;
use zayden_core::{EmojiCache, FormatNum};

use crate::shop::ShopCurrency;
use crate::{Error, Result, StaminaCron, StaminaManager};

pub trait Coins {
    fn coins(&self) -> i64;

    fn coins_str(&self) -> String {
        self.coins().format()
    }

    fn coins_mut(&mut self) -> &mut i64;

    fn add_coins(&mut self, payout: i64) {
        *self.coins_mut() += payout;
    }

    fn bet(&mut self, bet: i64) {
        *self.coins_mut() -= bet;
    }
}

pub trait Gems {
    fn gems(&self) -> i64;

    fn gems_str(&self) -> String {
        self.gems().format()
    }

    fn gems_mut(&mut self) -> &mut i64;

    fn add_gems(&mut self, amount: i64) {
        *self.gems_mut() += amount;
    }
}

pub trait Stamina {
    const MAX_STAMINA: i32 = 3;

    fn stamina(&self) -> i32;

    fn stamina_str(&self) -> String {
        format!(
            "{}{}",
            "🟩 ".repeat(self.stamina() as usize),
            "⬛ ".repeat((Self::MAX_STAMINA - self.stamina()).max(0) as usize)
        )
    }

    fn stamina_mut(&mut self) -> &mut i32;

    fn done_work(&mut self) {
        *self.stamina_mut() -= 1
    }

    fn verify_work<Db: Database, Manager: StaminaManager<Db>>(&self) -> Result<()> {
        if self.stamina() <= 0 {
            let next_timestamp = StaminaCron::cron_job::<Db, Manager>()
                .schedule
                .upcoming(chrono::Utc)
                .next()
                .unwrap_or_default()
                .timestamp();

            return Err(Error::OutOfStamina(next_timestamp));
        }

        Ok(())
    }
}

pub trait ItemInventory {
    fn inventory(&self) -> &[GamblingItem];

    fn inventory_mut(&mut self) -> &mut Vec<GamblingItem>;

    fn edit_item_quantity(&mut self, item_id: &str, amount: i64) -> Option<i64> {
        let inv = self.inventory_mut();

        let item = inv.iter_mut().find(|item| item.item_id == item_id)?;

        item.quantity += amount;

        let quantity = item.quantity;

        if quantity == 0 {
            inv.retain(|inv_item| inv_item.item_id != item_id);
        }

        Some(quantity)
    }
}

pub trait Mining {
    fn miners(&self) -> i64;

    fn mines(&self) -> i64;

    fn land(&self) -> i64;

    fn countries(&self) -> i64;

    fn continents(&self) -> i64;

    fn planets(&self) -> i64;

    fn solar_systems(&self) -> i64;

    fn galaxies(&self) -> i64;

    fn universes(&self) -> i64;

    fn tech(&self) -> i64;

    fn utility(&self) -> i64;

    fn production(&self) -> i64;

    fn coal(&self) -> i64;

    fn iron(&self) -> i64;

    fn gold(&self) -> i64;

    fn redstone(&self) -> i64;

    fn lapis(&self) -> i64;

    fn diamonds(&self) -> i64;

    fn emeralds(&self) -> i64;

    fn str_to_value(&self, s: &str) -> Option<i64> {
        match s {
            "miner" => Some(self.miners()),
            "mine" => Some(self.mines()),
            "land" => Some(self.land()),
            "country" => Some(self.countries()),
            "continent" => Some(self.continents()),
            "planet" => Some(self.planets()),
            "solar_system" => Some(self.solar_systems()),
            "galaxy" => Some(self.galaxies()),
            "universe" => Some(self.universes()),
            _ => None,
        }
    }

    fn resources(&self, emojis: &EmojiCache) -> String {
        format!(
            "{} `{}` coal
        {} `{}` iron
        {} `{}` gold
        {} `{}` redstone
        {} `{}` lapis
        {} `{}` diamonds
        {} `{}` emeralds",
            ShopCurrency::Coal.emoji(emojis),
            self.coal().format(),
            ShopCurrency::Iron.emoji(emojis),
            self.iron().format(),
            ShopCurrency::Gold.emoji(emojis),
            self.gold().format(),
            ShopCurrency::Redstone.emoji(emojis),
            self.redstone().format(),
            ShopCurrency::Lapis.emoji(emojis),
            self.lapis().format(),
            ShopCurrency::Diamonds.emoji(emojis),
            self.diamonds().format(),
            ShopCurrency::Emeralds.emoji(emojis),
            self.emeralds().format(),
        )
    }

    fn crafted(&self, emojis: &EmojiCache) -> String {
        format!(
            "{} `{}` tech packs
            {} `{}` utility packs
            {} `{}` production packs",
            ShopCurrency::Tech.emoji(emojis),
            self.tech().format(),
            ShopCurrency::Utility.emoji(emojis),
            self.utility().format(),
            ShopCurrency::Production.emoji(emojis),
            self.production().format()
        )
    }
}

pub trait Prestige {
    fn prestige(&self) -> i64;

    fn prestige_mult_100(&self) -> i64 {
        100 + self.prestige()
    }

    fn prestige_mult_10(&self) -> i64 {
        10 + self.prestige()
    }
}

pub trait MaxValues: Mining + Prestige {
    #[inline(always)]
    fn miners_per_mine() -> i64 {
        10
    }

    #[inline(always)]
    fn mines_per_land() -> i64 {
        10
    }

    #[inline(always)]
    fn land_per_country() -> i64 {
        10
    }

    #[inline(always)]
    fn countries_per_continent() -> i64 {
        10
    }

    #[inline(always)]
    fn continents_per_plant() -> i64 {
        10
    }

    #[inline(always)]
    fn plants_per_solar_system() -> i64 {
        10
    }

    #[inline(always)]
    fn solar_system_per_galaxies() -> i64 {
        10
    }

    #[inline(always)]
    fn galaxies_per_universe() -> i64 {
        10
    }

    fn max_values(&self) -> HashMap<&str, i64> {
        HashMap::from([
            ("miner", Self::miners_per_mine() * (self.mines() + 1)),
            ("mine", Self::mines_per_land() * (self.land() + 1)),
            ("land", Self::land_per_country() * (self.countries() + 1)),
            (
                "country",
                Self::countries_per_continent() * (self.continents() + 1),
            ),
            (
                "continent",
                Self::continents_per_plant() * (self.planets() + 1),
            ),
            (
                "planet",
                Self::plants_per_solar_system() * (self.solar_systems() + 1),
            ),
            (
                "solar_system",
                Self::solar_system_per_galaxies() * (self.galaxies() + 1),
            ),
            (
                "galaxy",
                Self::galaxies_per_universe() * (self.universes() + 1),
            ),
            ("universe", self.prestige() + 1),
        ])
    }

    fn units(&self) -> String {
        let max_values = self.max_values();

        [
            ("miner", self.miners()),
            ("mine", self.mines()),
            ("land", self.land()),
            ("country", self.countries()),
            ("continent", self.continents()),
            ("planet", self.planets()),
            ("solar_system", self.solar_systems()),
            ("galaxy", self.galaxies()),
            ("universe", self.universes()),
        ]
        .map(|(unit, amount)| {
            let max = *max_values.get(unit).unwrap();
            let display = match unit {
                "land" => String::from("plots of land"),
                "country" => String::from("countries"),
                "solar_system" => String::from("solar systems"),
                "galaxy" => String::from("galaxies"),
                _ if max > 1 => format!("{unit}s"),
                _ => unit.to_string(),
            };

            if amount >= max {
                format!("✅ {display} full")
            } else {
                format!("`{} / {}` {display}", amount.format(), max.format())
            }
        })
        .join("\n")
    }
}

impl<T: Mining + Prestige> MaxValues for T {}

pub trait MineHourly: Prestige {
    fn miners(&self) -> i64;

    fn hourly(&self) -> i64 {
        let miners = self.miners();

        if miners <= 0 {
            return 0;
        }

        (miners * self.prestige_mult_100()) / 100
    }
}

pub trait MaxBet: Prestige {
    fn level(&self) -> i32;

    fn max_bet(&self) -> i64 {
        let base_amount = (self.level() * 10_000).max(10_000);

        (base_amount as i64 * self.prestige_mult_10()) / 10
    }

    fn max_bet_str(&self) -> String {
        self.max_bet().format()
    }
}

pub trait MineAmount: MineHourly {
    fn mine_activity(&self) -> NaiveDateTime;

    fn mine_amount(&self) -> i64 {
        let mine_activity = self.mine_activity();

        let mine_hour = mine_activity
            .date()
            .and_hms_opt(mine_activity.hour(), 0, 0)
            .unwrap()
            .and_utc();

        let duration = Utc::now() - mine_hour;

        duration.num_hours().min(24) * self.hourly()
    }
}
