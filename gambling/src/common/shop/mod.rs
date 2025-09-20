use async_trait::async_trait;
use serenity::all::{
    ButtonStyle, CreateActionRow, CreateButton, CreateComponent, CreateEmbed, UserId,
};
use sqlx::{Database, FromRow, Pool, types::Json};
use zayden_core::{EmojiCache, FormatNum};

use crate::commands::shop::SellRow;
use crate::{Coins, GamblingItem, Gems, ItemInventory, MaxBet, Mining, Prestige};

pub mod currency;
pub mod items;
pub mod pages;

pub use currency::ShopCurrency;
pub use items::*;
pub use pages::ShopPage;

pub const SALES_TAX: f64 = 0.1;

#[async_trait]
pub trait ShopManager<Db: Database> {
    async fn buy_row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<ShopRow>>;

    async fn buy_save(pool: &Pool<Db>, row: ShopRow) -> sqlx::Result<Db::QueryResult>;

    async fn sell_row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
        item_id: &str,
    ) -> sqlx::Result<Option<SellRow>>;

    async fn sell_save(pool: &Pool<Db>, row: SellRow) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct ShopRow {
    pub id: i64,
    pub coins: i64,
    pub gems: i64,
    pub level: Option<i32>,
    pub inventory: Option<Json<Vec<GamblingItem>>>,
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
            id: id.get() as i64,
            coins: 0,
            gems: 0,
            level: Some(0),
            inventory: Some(Json(Vec::new())),
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

impl ItemInventory for ShopRow {
    fn inventory(&self) -> &[GamblingItem] {
        match self.inventory.as_ref() {
            Some(vec_ref) => &vec_ref.0,
            None => &[],
        }
    }

    fn inventory_mut(&mut self) -> &mut Vec<GamblingItem> {
        self.inventory.get_or_insert_with(|| Json(Vec::new()))
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
        unimplemented!()
    }

    fn iron(&self) -> i64 {
        unimplemented!()
    }

    fn gold(&self) -> i64 {
        unimplemented!()
    }

    fn redstone(&self) -> i64 {
        unimplemented!()
    }

    fn lapis(&self) -> i64 {
        unimplemented!()
    }

    fn diamonds(&self) -> i64 {
        unimplemented!()
    }

    fn emeralds(&self) -> i64 {
        unimplemented!()
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
    title: Option<&str>,
    page_change: i8,
) -> (CreateEmbed<'a>, CreateComponent<'a>) {
    let current_cat = title
        .map(|title| title.strip_suffix(" Shop").unwrap().parse().unwrap())
        .unwrap_or(ShopPage::Item);

    let category_idx = ShopPage::pages()
        .iter()
        .position(|cat| *cat == current_cat)
        .unwrap() as i8;

    let category = ShopPage::pages()
        .get(usize::try_from(category_idx + page_change).unwrap_or_default())
        .copied()
        .unwrap_or(ShopPage::Item);

    let embed = create_embed(emojis, category, row);

    let prev = CreateButton::new("shop_prev")
        .label("<")
        .style(ButtonStyle::Secondary);
    let next = CreateButton::new("shop_next")
        .label(">")
        .style(ButtonStyle::Secondary);

    (
        embed,
        CreateComponent::ActionRow(CreateActionRow::buttons(vec![prev, next])),
    )
}

fn create_embed<'a>(emojis: &EmojiCache, category: ShopPage, row: &ShopRow) -> CreateEmbed<'a> {
    let inv = row.inventory();

    let items = SHOP_ITEMS
        .iter()
        .filter(|item| item.category == category)
        .map(|item| {
            let costs = item
                .costs(1)
                .into_iter()
                .map(|(cost, currency)| format!("`{}` {}", cost.format(), currency.emoji(emojis)))
                .collect::<Vec<_>>();

            let mut s = format!("**{}**", item.as_str(emojis));

            if !item.description.is_empty() {
                s.push('\n');
                s.push_str(item.description);
            }

            s.push_str(&format!(
                "\nOwned: `{}`\nCost:",
                inv.iter()
                    .find(|inv_item| inv_item.item_id == item.id)
                    .map(|item| item.quantity)
                    .unwrap_or_default()
            ));

            if costs.len() == 1 {
                s.push(' ');
                s.push_str(&costs.join(""));
            } else {
                s.push('\n');
                s.push_str(&costs.join("\n"));
            }

            s
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let coin = emojis.emoji("heads").unwrap();

    let desc = format!(
        "Sales tax: {}%\nYour coins: {}  <:coin:{coin}>\n--------------------\n{items}\n--------------------\nBuy with `/shop buy <item> <amount>`\nSell with `/shop sell <item> <amount>`",
        SALES_TAX * 100.0,
        row.coins_str()
    );

    CreateEmbed::new()
        .title(format!("{category} Shop"))
        .description(desc)
}
