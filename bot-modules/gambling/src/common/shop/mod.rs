use std::fmt::Write as _;

use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateEmbed,
    UserId,
};
use sqlx::{FromRow, PgPool};
use zayden_core::{EmojiCache, FormatNum, as_i64};

use crate::{
    Coins,
    GamblingError,
    GamblingItems,
    Gems,
    MaxBet,
    MaxValues,
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

pub struct ShopManager;

impl ShopManager {
    pub async fn buy_row(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<Option<ShopRow>> {
        sqlx::query_as!(ShopRow,
            r#"SELECT
            g.user_id,
            g.coins,
            g.gems,
            
            COALESCE(l.level, 0) AS level,

            COALESCE(m.miners, 0) AS "miners!",
            COALESCE(m.mines, 0) AS "mines!",
            COALESCE(m.land, 0) AS "land!",
            COALESCE(m.countries, 0) AS "countries!",
            COALESCE(m.continents, 0) AS "continents!",
            COALESCE(m.planets, 0) AS "planets!",
            COALESCE(m.solar_systems, 0) AS "solar_systems!",
            COALESCE(m.galaxies, 0) AS "galaxies!",
            COALESCE(m.universes, 0) AS "universes!",
            COALESCE(m.prestige, 0) AS "prestige!",
            COALESCE(m.tech, 0) AS "tech!",
            COALESCE(m.utility, 0) AS "utility!",
            COALESCE(m.production, 0) AS "production!"

            FROM gambling g LEFT JOIN levels l ON g.user_id = l.user_id LEFT JOIN gambling_mine m ON g.user_id = m.user_id WHERE g.user_id = $1;"#,
            as_i64(id.get())
        ).fetch_optional(pool).await
    }

    pub async fn commit_purchase(
        pool: &PgPool,
        id: UserId,
        delta: &ShopDelta,
        item: Option<(&str, i64)>,
    ) -> sqlx::Result<Option<PurchaseCommit>> {
        let user_id = as_i64(id.get());

        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO gambling (user_id) VALUES ($1)
            ON CONFLICT (user_id) DO NOTHING;",
            user_id
        )
        .execute(&mut *tx)
        .await?;

        let Some(balance) = sqlx::query!(
            "UPDATE gambling
            SET coins = coins + $2, gems = gems + $3
            WHERE user_id = $1
                AND ($2::bigint = 0 OR coins + $2 >= 0)
                AND ($3::bigint = 0 OR gems + $3 >= 0)
            RETURNING coins, gems;",
            user_id,
            delta.coins,
            delta.gems,
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(None);
        };

        let mine = if delta.is_mine_noop() {
            None
        } else {
            sqlx::query!(
                "INSERT INTO gambling_mine (user_id) VALUES ($1)
                ON CONFLICT (user_id) DO NOTHING;",
                user_id
            )
            .execute(&mut *tx)
            .await?;

            let Some(mine) = sqlx::query_as!(
                MineCommit,
                "UPDATE gambling_mine
                SET miners = miners + $2,
                    mines = mines + $3,
                    land = land + $4,
                    countries = countries + $5,
                    continents = continents + $6,
                    planets = planets + $7,
                    solar_systems = solar_systems + $8,
                    galaxies = galaxies + $9,
                    universes = universes + $10,
                    tech = tech + $11,
                    utility = utility + $12,
                    production = production + $13
                WHERE user_id = $1
                    AND ($2::bigint = 0 OR miners + $2 BETWEEN 0 AND $14 * (mines + $3 + 1))
                    AND ($3::bigint = 0 OR mines + $3 BETWEEN 0 AND $15 * (land + $4 + 1))
                    AND ($4::bigint = 0 OR land + $4 BETWEEN 0 AND $16 * (countries + $5 + 1))
                    AND ($5::bigint = 0 OR countries + $5 BETWEEN 0 AND $17 * (continents + $6 + 1))
                    AND ($6::bigint = 0 OR continents + $6 BETWEEN 0 AND $18 * (planets + $7 + 1))
                    AND ($7::bigint = 0 OR planets + $7 BETWEEN 0 AND $19 * (solar_systems + $8 + 1))
                    AND ($8::bigint = 0 OR solar_systems + $8 BETWEEN 0 AND $20 * (galaxies + $9 + 1))
                    AND ($9::bigint = 0 OR galaxies + $9 BETWEEN 0 AND $21 * (universes + $10 + 1))
                    AND ($10::bigint = 0 OR universes + $10 BETWEEN 0 AND prestige + 1)
                    AND ($11::bigint = 0 OR tech + $11 >= 0)
                    AND ($12::bigint = 0 OR utility + $12 >= 0)
                    AND ($13::bigint = 0 OR production + $13 >= 0)
                RETURNING miners, mines, land, countries, continents, planets,
                    solar_systems, galaxies, universes, tech, utility, production;",
                user_id,
                delta.miners,
                delta.mines,
                delta.land,
                delta.countries,
                delta.continents,
                delta.planets,
                delta.solar_systems,
                delta.galaxies,
                delta.universes,
                delta.tech,
                delta.utility,
                delta.production,
                <ShopRow as MaxValues>::miners_per_mine(),
                <ShopRow as MaxValues>::mines_per_land(),
                <ShopRow as MaxValues>::land_per_country(),
                <ShopRow as MaxValues>::countries_per_continent(),
                <ShopRow as MaxValues>::continents_per_plant(),
                <ShopRow as MaxValues>::plants_per_solar_system(),
                <ShopRow as MaxValues>::solar_system_per_galaxies(),
                <ShopRow as MaxValues>::galaxies_per_universe(),
            )
            .fetch_optional(&mut *tx)
            .await?
            else {
                return Ok(None);
            };

            Some(mine)
        };

        let item_quantity = match item {
            Some((item_id, amount)) => Some(
                sqlx::query_scalar!(
                    "INSERT INTO gambling_inventory (user_id, item_id, quantity)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (user_id, item_id) DO UPDATE
                    SET quantity = gambling_inventory.quantity + $3
                    RETURNING quantity;",
                    user_id,
                    item_id,
                    amount,
                )
                .fetch_one(&mut *tx)
                .await?,
            ),
            None => None,
        };

        tx.commit().await?;

        Ok(Some(PurchaseCommit {
            coins: balance.coins,
            gems: balance.gems,
            mine,
            item_quantity,
        }))
    }

    pub async fn sell_quantity(
        pool: &PgPool,
        id: UserId,
        item_id: &str,
    ) -> sqlx::Result<Option<i64>> {
        sqlx::query_scalar!(
            "SELECT quantity FROM gambling_inventory
            WHERE user_id = $1 AND item_id = $2;",
            as_i64(id.get()),
            item_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn commit_sale(
        pool: &PgPool,
        id: UserId,
        item_id: &str,
        delta: &SaleDelta,
    ) -> sqlx::Result<Option<SaleCommit>> {
        let user_id = as_i64(id.get());

        let mut tx = pool.begin().await?;

        let Some(coins) = sqlx::query_scalar!(
            "UPDATE gambling SET coins = coins + $2
            WHERE user_id = $1
            RETURNING coins;",
            user_id,
            delta.coins,
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(None);
        };

        let Some(quantity) = sqlx::query_scalar!(
            "UPDATE gambling_inventory SET quantity = quantity - $3
            WHERE user_id = $1 AND item_id = $2 AND quantity >= $3
            RETURNING quantity;",
            user_id,
            item_id,
            delta.quantity,
        )
        .fetch_optional(&mut *tx)
        .await?
        else {
            return Ok(None);
        };

        if quantity == 0 {
            sqlx::query!(
                "DELETE FROM gambling_inventory
                WHERE user_id = $1 AND item_id = $2;",
                user_id,
                item_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(Some(SaleCommit { coins, quantity }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SaleDelta {
    pub coins: i64,
    pub quantity: i64,
}

impl SaleDelta {
    #[must_use]
    pub const fn new(unit_coin_cost: i64, amount: i64) -> Self {
        Self {
            coins: unit_coin_cost * amount * SALES_RETURN / 100,
            quantity: amount,
        }
    }
}

pub struct SaleCommit {
    pub coins: i64,
    pub quantity: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShopDelta {
    pub coins: i64,
    pub gems: i64,
    pub miners: i64,
    pub mines: i64,
    pub land: i64,
    pub countries: i64,
    pub continents: i64,
    pub planets: i64,
    pub solar_systems: i64,
    pub galaxies: i64,
    pub universes: i64,
    pub tech: i64,
    pub utility: i64,
    pub production: i64,
}

impl ShopDelta {
    #[must_use]
    pub const fn between(before: &ShopRow, after: &ShopRow) -> Self {
        Self {
            coins: after.coins - before.coins,
            gems: after.gems - before.gems,
            miners: after.miners - before.miners,
            mines: after.mines - before.mines,
            land: after.land - before.land,
            countries: after.countries - before.countries,
            continents: after.continents - before.continents,
            planets: after.planets - before.planets,
            solar_systems: after.solar_systems - before.solar_systems,
            galaxies: after.galaxies - before.galaxies,
            universes: after.universes - before.universes,
            tech: after.tech - before.tech,
            utility: after.utility - before.utility,
            production: after.production - before.production,
        }
    }

    #[must_use]
    pub fn is_noop(&self) -> bool {
        *self == Self::default()
    }

    #[must_use]
    pub const fn is_mine_noop(&self) -> bool {
        self.miners == 0
            && self.mines == 0
            && self.land == 0
            && self.countries == 0
            && self.continents == 0
            && self.planets == 0
            && self.solar_systems == 0
            && self.galaxies == 0
            && self.universes == 0
            && self.tech == 0
            && self.utility == 0
            && self.production == 0
    }
}

pub struct PurchaseCommit {
    pub coins: i64,
    pub gems: i64,
    pub mine: Option<MineCommit>,
    pub item_quantity: Option<i64>,
}

pub struct MineCommit {
    pub miners: i64,
    pub mines: i64,
    pub land: i64,
    pub countries: i64,
    pub continents: i64,
    pub planets: i64,
    pub solar_systems: i64,
    pub galaxies: i64,
    pub universes: i64,
    pub tech: i64,
    pub utility: i64,
    pub production: i64,
}

impl MineCommit {
    #[must_use]
    pub fn quantity(&self, item_id: &str) -> Option<i64> {
        Some(match item_id {
            "miner" => self.miners,
            "mine" => self.mines,
            "land" => self.land,
            "country" => self.countries,
            "continent" => self.continents,
            "planet" => self.planets,
            "solar_system" => self.solar_systems,
            "galaxy" => self.galaxies,
            "universe" => self.universes,
            _ => return None,
        })
    }
}

#[derive(Clone, FromRow)]
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
    #[must_use]
    pub const fn new(id: UserId) -> Self {
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
