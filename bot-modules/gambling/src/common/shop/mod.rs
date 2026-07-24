use std::fmt::Write as _;

use serenity::all::{
    ButtonStyle,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateEmbed,
    UserId,
};
use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, PgPool};
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

    pub async fn buy_save(
        pool: &PgPool,
        row: ShopRow,
    ) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems;",
            row.user_id,
            row.coins,
            row.gems,
        )
        .execute(&mut *tx)
        .await?;

        let result3 = sqlx::query!(
            "INSERT INTO gambling_mine (user_id, miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes, tech, utility, production)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (user_id) DO UPDATE
            SET
            miners = EXCLUDED.miners,
            mines = EXCLUDED.mines,
            land = EXCLUDED.land,
            countries = EXCLUDED.countries,
            continents = EXCLUDED.continents,
            planets = EXCLUDED.planets,
            solar_systems = EXCLUDED.solar_systems,
            galaxies = EXCLUDED.galaxies,
            universes = EXCLUDED.universes,
            tech = EXCLUDED.tech,
            utility = EXCLUDED.utility,
            production = EXCLUDED.production;",
            row.user_id,
            row.miners,
            row.mines,
            row.land,
            row.countries,
            row.continents,
            row.planets,
            row.solar_systems,
            row.galaxies,
            row.universes,
            row.tech,
            row.utility,
            row.production,
        ).execute(&mut *tx).await?;

        result.extend([result3]);

        tx.commit().await?;

        Ok(result)
    }

    pub async fn save_inventory(
        pool: &PgPool,
        user_id: UserId,
        rows: GamblingItems,
    ) -> sqlx::Result<PgQueryResult> {
        let mut item_ids = Vec::with_capacity(rows.0.len());
        let mut quantities = Vec::with_capacity(rows.0.len());

        for item in rows.0 {
            item_ids.push(item.item_id);
            quantities.push(item.quantity);
        }

        sqlx::query!(
            "INSERT INTO gambling_inventory (user_id, item_id, quantity)
            SELECT $1, * FROM UNNEST($2::text[], $3::bigint[])
            ON CONFLICT (user_id, item_id) DO UPDATE
            SET quantity = EXCLUDED.quantity",
            as_i64(user_id.get()),
            &item_ids,
            &quantities
        )
        .execute(pool)
        .await
    }

    pub async fn sell_row(
        pool: &PgPool,
        id: UserId,
        item_id: &str,
    ) -> sqlx::Result<Option<SellRow>> {
        sqlx::query_as!(
            SellRow,
            r#"
            SELECT
                g.user_id,
                g.coins,

                i.id AS "item_row_id?",
                i.quantity AS "item_quantity?"
            FROM
                gambling g
            LEFT JOIN
                gambling_inventory i ON g.user_id = i.user_id AND i.item_id = $2
            WHERE
                g.user_id = $1
            "#,
            as_i64(id.get()),
            item_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn sell_save(
        pool: &PgPool,
        row: SellRow,
    ) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (user_id, coins)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins;",
            row.user_id,
            row.coins,
        )
        .execute(&mut *tx)
        .await?;

        let result2 = if row.item_quantity == Some(0) {
            sqlx::query!(
                "DELETE FROM gambling_inventory WHERE id = $1",
                row.item_row_id
            )
            .execute(&mut *tx)
            .await?
        } else {
            sqlx::query!(
                "UPDATE gambling_inventory SET quantity = $1 WHERE id = $2",
                row.item_quantity,
                row.item_row_id
            )
            .execute(&mut *tx)
            .await?
        };

        result.extend([result2]);

        tx.commit().await?;

        Ok(result)
    }
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
