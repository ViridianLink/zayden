use std::ops::Deref;
use std::time::Duration;

use zayden_core::EmojiCache;

use super::{ShopCurrency, ShopPage};
use crate::error::Result;
use crate::utils::Emoji;
use crate::{GamblingError, GamblingItem};

#[derive(Clone, Copy)]
pub struct ShopItem<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub emoji: Emoji,
    pub description: &'a str,
    pub costs: [Option<(i64, ShopCurrency)>; 4],
    pub category: ShopPage,
    pub sellable: bool,
    pub useable: bool,
    pub effect_fn: fn(i64, i64) -> i64,
    pub effect_duration: Option<Duration>,
}

impl<'a> ShopItem<'a> {
    const fn new(
        id: &'a str,
        name: &'a str,
        emoji: Emoji,
        desc: &'a str,
        cost: i64,
        currency: ShopCurrency,
        category: ShopPage,
    ) -> Self {
        ShopItem {
            id,
            name,
            emoji,
            description: desc,
            costs: [Some((cost, currency)), None, None, None],
            category,
            sellable: false,
            useable: false,
            effect_fn: |_, payout| payout,
            effect_duration: None,
        }
    }

    #[expect(
        clippy::indexing_slicing,
        reason = "i is bounded by self.costs.len(); get_mut is not usable in const fn"
    )]
    const fn add_cost(mut self, cost: i64, currency: ShopCurrency) -> Self {
        let mut i = 0;
        while i < self.costs.len() {
            if self.costs[i].is_none() {
                self.costs[i] = Some((cost, currency));
                break;
            }

            i += 1;
        }

        self
    }

    const fn sellable(mut self, value: bool) -> Self {
        self.sellable = value;
        self
    }

    const fn useable(mut self, value: bool) -> Self {
        self.useable = value;
        self
    }

    const fn effect_fn(mut self, f: fn(i64, i64) -> i64) -> Self {
        self.effect_fn = f;
        self
    }

    const fn duration(mut self, d: Duration) -> Self {
        self.effect_duration = Some(d);
        self
    }

    pub fn emoji(&self, emojis: &EmojiCache) -> Result<String> {
        match self.emoji {
            Emoji::Id(name) => emojis.emoji_str(name).map_err(|n| {
                GamblingError::Internal(format!("emoji '{n}' not in cache"))
            }),
            Emoji::Str(emoji) => Ok(String::from(emoji)),
            Emoji::None => Ok(String::new()),
        }
    }

    pub fn cost_desc(&self, emojis: &EmojiCache) -> Result<String> {
        let mut parts = Vec::new();
        for cost in self.costs.iter().filter_map(|c| c.as_ref()) {
            let (amount, currency) = cost;
            parts.push(format!("`{amount}` {}", currency.emoji(emojis)?));
        }
        Ok(parts.join("\n"))
    }

    #[must_use]
    pub fn coin_cost(&self) -> Option<i64> {
        self.costs
            .iter()
            .filter_map(|x| x.as_ref())
            .find(|(_, currency)| matches!(currency, ShopCurrency::Coins))
            .map(|(cost, _)| cost)
            .copied()
    }

    #[must_use]
    pub fn costs(&self, amount: i64) -> Vec<(i64, ShopCurrency)> {
        let iter = self.costs.iter().copied().flatten();

        iter.map(|(cost, currency)| (cost * amount, currency)).collect()
    }

    pub fn as_str(&self, emojis: &EmojiCache) -> Result<String> {
        Ok(format!("{} {}", self.emoji(emojis)?, self.name))
    }
}

impl TryFrom<&GamblingItem> for ShopItem<'_> {
    type Error = GamblingError;

    fn try_from(value: &GamblingItem) -> Result<Self> {
        SHOP_ITEMS.iter().find(|item| item.id == value.item_id).copied().ok_or_else(
            || {
                GamblingError::Internal(format!(
                    "GamblingItem item_id '{}' not in SHOP_ITEMS",
                    value.item_id
                ))
            },
        )
    }
}

pub const LOTTO_TICKET: ShopItem<'static> = ShopItem::new(
    "lottoticket",
    "Lottery Ticket",
    Emoji::Str("🎟️"),
    "Enter the daily lottery.\nThe more tickets bought have the higher the jackpot.",
    5_000,
    ShopCurrency::Coins,
    ShopPage::Item,
);

pub const EGGPLANT: ShopItem<'static> = ShopItem::new(
    "eggplant",
    "Eggplant",
    Emoji::Str("🍆"),
    "Who has the biggest eggplant?",
    10_000,
    ShopCurrency::Coins,
    ShopPage::Item,
)
.sellable(true);

pub const WEAPON_CRATE: ShopItem<'static> = ShopItem::new(
    "weaponcrate",
    "Weapon Crate",
    Emoji::Str("📦"),
    "Unlock for a weapon to display on your profile",
    100_000,
    ShopCurrency::Coins,
    ShopPage::Item,
)
.sellable(true)
.useable(true);

pub const LUCKY_CHIP: ShopItem<'static> = ShopItem::new(
    "luckychip",
    "Lucky Chip",
    Emoji::Str("⭐"),
    "Refund your bet if you lose",
    3,
    ShopCurrency::Gems,
    ShopPage::Boost1,
)
.useable(true)
.effect_fn(|bet, _| bet);

pub const ALL_INS: ShopItem<'static> = ShopItem::new(
    "allins",
    "Infinite All Ins",
    Emoji::Str("♾️"),
    "Removes your max bet limit | Duration: `+2 minutes`",
    20,
    ShopCurrency::Gems,
    ShopPage::Boost1,
)
.useable(true)
.duration(Duration::from_mins(2));

#[expect(dead_code, reason = "reserved for future implementation")]
const RIGGED_LUCK: ShopItem<'static> = ShopItem::new(
    "riggedluck",
    "Rigged Luck",
    Emoji::Str("⚪"),
    "Double your chances! Your win probability is increased by 100% for the next game. (Max 75% total win chance)",
    30,
    ShopCurrency::Gems,
    ShopPage::Boost1,
).useable(true);

const PAYOUT_X2: ShopItem<'static> = ShopItem::new(
    "payout2x",
    "Payout x2",
    Emoji::Id("chip_2"),
    "Double payout from winning | Duration: `+15 minute`",
    2,
    ShopCurrency::Gems,
    ShopPage::Boost2,
)
.useable(true)
.effect_fn(|_, payout| {
    if payout < 0 {
        return payout;
    }

    payout * 2
})
.duration(Duration::from_mins(15));

const PAYOUT_X5: ShopItem<'static> = ShopItem::new(
    "payout5x",
    "Payout x5",
    Emoji::Id("chip_5"),
    "Five times payout from winning | Duration: `+10 minute`",
    5,
    ShopCurrency::Gems,
    ShopPage::Boost2,
)
.useable(true)
.effect_fn(|_, payout| {
    if payout < 0 {
        return payout;
    }

    payout * 5
})
.duration(Duration::from_mins(10));

const PAYOUT_X10: ShopItem<'static> = ShopItem::new(
    "payout10x",
    "Payout x10",
    Emoji::Id("chip_10"),
    "Ten times payout from winning | Duration: `+5 minute`",
    10,
    ShopCurrency::Gems,
    ShopPage::Boost2,
)
.useable(true)
.effect_fn(|_, payout| {
    if payout < 0 {
        return payout;
    }

    payout * 10
})
.duration(Duration::from_mins(5));

const PAYOUT_X50: ShopItem<'static> = ShopItem::new(
    "payout50x",
    "Payout x50",
    Emoji::Id("chip_50"),
    "Fifty times payout from winning | Duration: `+2 minute`",
    20,
    ShopCurrency::Gems,
    ShopPage::Boost2,
)
.useable(true)
.effect_fn(|_, payout| {
    if payout < 0 {
        return payout;
    }

    payout * 50
})
.duration(Duration::from_mins(2));

const PAYOUT_X100: ShopItem<'static> = ShopItem::new(
    "payout100x",
    "Payout x100",
    Emoji::Id("chip_100"),
    "One hundered times payout from winning | Duration: `+1 minute`",
    25,
    ShopCurrency::Gems,
    ShopPage::Boost2,
)
.useable(true)
.effect_fn(|_, payout| {
    if payout < 0 {
        return payout;
    }

    payout * 100
})
.duration(Duration::from_mins(1));

// region: Mine
pub const MINER: ShopItem<'static> = ShopItem::new(
    "miner",
    "Miner",
    Emoji::None,
    "Increases passive mine income and boosts resource gains from dig",
    100,
    ShopCurrency::Coins,
    ShopPage::Mine1,
);

const MINE: ShopItem<'static> = ShopItem::new(
    "mine",
    "Mine",
    Emoji::None,
    "Allows you to hire 10 extra miners per mine",
    10_000,
    ShopCurrency::Coins,
    ShopPage::Mine1,
)
.add_cost(1, ShopCurrency::Tech);

const LAND: ShopItem<'static> = ShopItem::new(
    "land",
    "Land",
    Emoji::None,
    "Allows you to buy 10 extra mines per land",
    50_000,
    ShopCurrency::Coins,
    ShopPage::Mine1,
)
.add_cost(10, ShopCurrency::Tech);

const COUNTRY: ShopItem<'static> = ShopItem::new(
    "country",
    "Country",
    Emoji::None,
    "Allows you to buy 10 extra plots of land per country",
    200_000,
    ShopCurrency::Coins,
    ShopPage::Mine1,
)
.add_cost(100, ShopCurrency::Tech)
.add_cost(1, ShopCurrency::Utility);

const CONTINENT: ShopItem<'static> = ShopItem::new(
    "continent",
    "Continent",
    Emoji::None,
    "Allows you to buy 10 extra countries per continent",
    500_000,
    ShopCurrency::Coins,
    ShopPage::Mine1,
)
.add_cost(1000, ShopCurrency::Tech)
.add_cost(10, ShopCurrency::Utility);

const PLANET: ShopItem<'static> = ShopItem::new(
    "planet",
    "Planet",
    Emoji::None,
    "Allows you to buy 10 extra continents per planet",
    2_500_000,
    ShopCurrency::Coins,
    ShopPage::Mine2,
)
.add_cost(10_000, ShopCurrency::Tech)
.add_cost(100, ShopCurrency::Utility)
.add_cost(1, ShopCurrency::Production);

const SOLAR_SYSTEM: ShopItem<'static> = ShopItem::new(
    "solar_system",
    "Solar System",
    Emoji::None,
    "Allows you to buy 10 extra planets per solar system",
    5_000_000,
    ShopCurrency::Coins,
    ShopPage::Mine2,
)
.add_cost(100_000, ShopCurrency::Tech)
.add_cost(1000, ShopCurrency::Utility)
.add_cost(10, ShopCurrency::Production);

const GALAXY: ShopItem<'static> = ShopItem::new(
    "galaxy",
    "Galaxy",
    Emoji::None,
    "Allows you to buy 10 extra planets per solar system",
    25_000_000,
    ShopCurrency::Coins,
    ShopPage::Mine2,
)
.add_cost(1_000_000, ShopCurrency::Tech)
.add_cost(10_000, ShopCurrency::Utility)
.add_cost(100, ShopCurrency::Production);

const UNIVERSE: ShopItem<'static> = ShopItem::new(
    "universe",
    "Universe",
    Emoji::None,
    "Allows you to buy 10 extra galaxies per universe",
    50_000_000,
    ShopCurrency::Coins,
    ShopPage::Mine2,
)
.add_cost(10_000_000, ShopCurrency::Tech)
.add_cost(100_000, ShopCurrency::Utility)
.add_cost(1000, ShopCurrency::Production);
// endregion

pub struct ShopItems<'a>([ShopItem<'a>; 18]);

impl ShopItems<'_> {
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ShopItem<'_>> {
        self.0.iter().find(|item| item.id == id)
    }
}

impl<'a> Deref for ShopItems<'a> {
    type Target = [ShopItem<'a>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub const SHOP_ITEMS: ShopItems<'static> = ShopItems([
    LOTTO_TICKET,
    EGGPLANT,
    // WEAPON_CRATE,
    LUCKY_CHIP,
    ALL_INS,
    // RIGGED_LUCK,
    PAYOUT_X2,
    PAYOUT_X5,
    PAYOUT_X10,
    PAYOUT_X50,
    PAYOUT_X100,
    MINER,
    MINE,
    LAND,
    COUNTRY,
    CONTINENT,
    PLANET,
    SOLAR_SYSTEM,
    GALAXY,
    UNIVERSE,
]);
