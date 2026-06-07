use std::fmt::Write as _;

use async_trait::async_trait;
use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateEmbed,
    UserId,
};
use sqlx::{Database, FromRow, Pool};
use zayden_core::{EmojiCache, FormatNum, as_i64};

use crate::commands::shop::SellRow;
use crate::{
    Coins,
    GamblingError,
    GamblingItems,
    Gems,
    MaxBet,
    Mining,
    Prestige,
    Result,
};

pub mod currency;
pub mod items;
pub mod pages;

pub use currency::ShopCurrency;
pub use items::*;
pub use pages::ShopPage;

pub const SALES_RETURN: i64 = 90;

#[async_trait]
pub trait ShopManager<Db: Database> {
    async fn buy_row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<ShopRow>>;

    async fn buy_save(
        pool: &Pool<Db>,
        row: ShopRow,
    ) -> sqlx::Result<Db::QueryResult>;

    async fn save_inventory(
        pool: &Pool<Db>,
        user_id: UserId,
        row: GamblingItems,
    ) -> sqlx::Result<Db::QueryResult>;

    async fn sell_row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
        item_id: &str,
    ) -> sqlx::Result<Option<SellRow>>;

    async fn sell_save(
        pool: &Pool<Db>,
        row: SellRow,
    ) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct ShopRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub level: Option<i32>,
    pub miners: i64,
    pub mines: i64,
    pub land: i64,
    pub countries: i64,
    pub continents: i64,
    pub planets: i64,
    pub solar_systems: i64,
    pub galaxies: i64,
    pub universes: i64,
    pub prestige: i64,
    pub tech: i64,
    pub utility: i64,
    pub production: i64,
}

impl ShopRow {
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            user_id: as_i64(id.get()),
            coins: 0,
            gems: 0,
            level: Some(0),
            miners: 0,
            mines: 0,
            land: 0,
            countries: 0,
            continents: 0,
            planets: 0,
            solar_systems: 0,
            galaxies: 0,
            universes: 0,
            prestige: 0,
            tech: 0,
            utility: 0,
            production: 0,
        }
    }
}

impl Coins for ShopRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for ShopRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Mining for ShopRow {
    fn miners(&self) -> i64 {
        self.miners
    }

    fn mines(&self) -> i64 {
        self.mines
    }

    fn land(&self) -> i64 {
        self.land
    }

    fn countries(&self) -> i64 {
        self.countries
    }

    fn continents(&self) -> i64 {
        self.continents
    }

    fn planets(&self) -> i64 {
        self.planets
    }

    fn solar_systems(&self) -> i64 {
        self.solar_systems
    }

    fn galaxies(&self) -> i64 {
        self.galaxies
    }

    fn universes(&self) -> i64 {
        self.universes
    }

    fn tech(&self) -> i64 {
        self.tech
    }

    fn utility(&self) -> i64 {
        self.utility
    }

    fn production(&self) -> i64 {
        self.production
    }

    fn coal(&self) -> i64 {
        0
    }

    fn iron(&self) -> i64 {
        0
    }

    fn gold(&self) -> i64 {
        0
    }

    fn redstone(&self) -> i64 {
        0
    }

    fn lapis(&self) -> i64 {
        0
    }

    fn diamonds(&self) -> i64 {
        0
    }

    fn emeralds(&self) -> i64 {
        0
    }
}

impl Prestige for ShopRow {
    fn prestige(&self) -> i64 {
        self.prestige
    }
}

impl MaxBet for ShopRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

pub fn shop_response<'a>(
    emojis: &EmojiCache,
    row: &'a ShopRow,
    inventory: &GamblingItems,
    title: Option<&str>,
    page_change: i8,
) -> Result<(CreateEmbed<'a>, CreateComponent<'a>)> {
    let page_change = usize::try_from(page_change).unwrap_or_default();

    let current_cat = match title {
        None => ShopPage::Item,
        Some(title) => title
            .strip_suffix(" Shop")
            .ok_or_else(|| {
                GamblingError::Internal(
                    "shop embed title missing \" Shop\" suffix".to_string(),
                )
            })?
            .parse()
            .unwrap_or(ShopPage::Item),
    };

    let category_idx =
        ShopPage::pages().iter().position(|cat| *cat == current_cat).unwrap_or(0);

    let category = ShopPage::pages()
        .get(category_idx + page_change)
        .copied()
        .unwrap_or(ShopPage::Item);

    let embed = create_embed(emojis, category, row, inventory)?;

    let prev =
        CreateButton::new("shop_prev").label("<").style(ButtonStyle::Secondary);
    let next =
        CreateButton::new("shop_next").label(">").style(ButtonStyle::Secondary);

    Ok((
        embed,
        CreateComponent::ActionRow(CreateActionRow::buttons(vec![prev, next])),
    ))
}

fn create_embed<'a>(
    emojis: &EmojiCache,
    category: ShopPage,
    row: &ShopRow,
    inventory: &GamblingItems,
) -> Result<CreateEmbed<'a>> {
    let mut item_entries = Vec::new();
    for item in SHOP_ITEMS.iter().filter(|item| item.category == category) {
        let mut costs = Vec::new();
        for (cost, currency) in item.costs(1) {
            costs.push(format!("`{}` {}", cost.format(), currency.emoji(emojis)?));
        }

        let mut s = format!("**{}**", item.as_str(emojis)?);

        if !item.description.is_empty() {
            s.push('\n');
            s.push_str(item.description);
        }

        let _ = write!(
            s,
            "\nOwned: `{}`\nCost:",
            inventory
                .0
                .iter()
                .find(|inv_item| inv_item.item_id == item.id)
                .map(|item| item.quantity)
                .unwrap_or_default()
        );

        if costs.len() == 1 {
            s.push(' ');
            s.push_str(&costs.join(""));
        } else {
            s.push('\n');
            s.push_str(&costs.join("\n"));
        }

        item_entries.push(s);
    }

    let items = item_entries.join("\n\n");

    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let desc = format!(
        "Sales tax: {}%\nYour coins: {}  <:coin:{coin}>\n--------------------\n{items}\n--------------------\nBuy with `/shop buy <item> <amount>`\nSell with `/shop sell <item> <amount>`",
        100 - SALES_RETURN,
        row.coins_str()
    );

    Ok(CreateEmbed::new().title(format!("{category} Shop")).description(desc))
}
