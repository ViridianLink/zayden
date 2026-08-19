use std::fmt::{Display, Write as _};
use std::num::ParseIntError;

use jiff::Timestamp;
use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    Mentionable,
    ResolvedOption,
    ResolvedValue,
    UserId,
};
use serenity::small_fixed_array::FixedArray;
use sqlx::prelude::FromRow;
use sqlx::{PgConnection, PgPool};
use tokio::sync::RwLock;
use zayden_core::{
    EmojiCache,
    EmojiCacheData,
    as_i64,
    parse_options,
    parse_subcommand,
};

use super::Commands;
use crate::models::gambling_inventory::{GamblingItem, GamblingItems};
use crate::shop::{SHOP_ITEMS, ShopCurrency, ShopItem, ShopPage};
use crate::{
    Coins,
    EffectsManager,
    GEM,
    GamblingError,
    Gems,
    ItemInventory,
    Mining,
    Result,
};

struct InventoryItem<'a> {
    id: &'a str,
    name: &'a str,
    emoji: String,
    cost: [Option<(i64, ShopCurrency)>; 4],
    quantity: i64,
}

impl<'a> InventoryItem<'a> {
    pub(crate) fn from_shop_item(
        item: &ShopItem<'a>,
        emojis: &EmojiCache,
    ) -> Result<Self> {
        Ok(Self {
            id: item.id,
            name: item.name,
            emoji: item.emoji(emojis)?,
            cost: item.costs,
            quantity: 0,
        })
    }
}

impl Display for InventoryItem<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.emoji, self.name)
    }
}

pub struct InventoryManager;

impl InventoryManager {
    pub async fn gambling_row(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<Option<InventoryRow>> {
        sqlx::query_as!(
            InventoryRow,
            r#"SELECT
            g.coins,
            g.gems,

            COALESCE(m.tech, 0) AS "tech!",
            COALESCE(m.utility, 0) AS "utility!",
            COALESCE(m.production, 0) AS "production!",
            COALESCE(m.coal, 0) AS "coal!",
            COALESCE(m.iron, 0) AS "iron!",
            COALESCE(m.gold, 0) AS "gold!",
            COALESCE(m.redstone, 0) AS "redstone!",
            COALESCE(m.lapis, 0) AS "lapis!",
            COALESCE(m.diamonds, 0) AS "diamonds!",
            COALESCE(m.emeralds, 0) AS "emeralds!"

            FROM gambling g LEFT JOIN gambling_mine m ON g.user_id = m.user_id WHERE g.user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn inventory_items(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<GamblingItems> {
        let items = sqlx::query_as!(
            GamblingItem,
            r#"SELECT item_id, quantity
            FROM gambling_inventory
            WHERE user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_all(pool)
        .await?;

        Ok(GamblingItems(items))
    }

    pub async fn edit_item_quantity(
        conn: &mut PgConnection,
        id: UserId,
        item_id: &str,
        amount: i64,
    ) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            r#"
        WITH updated_row AS (
            UPDATE gambling_inventory
            SET quantity = quantity - $3
            WHERE user_id = $1
              AND item_id = $2
              AND $3 <= gambling_inventory.quantity
            RETURNING quantity
        ),
        deleted_row AS (
            DELETE FROM gambling_inventory
            WHERE user_id = $1 AND item_id = $2
            AND EXISTS (SELECT 1 FROM updated_row ur WHERE ur.quantity <= 0)
            RETURNING item_id
        )
        SELECT
            ur.quantity
        FROM
            updated_row ur
        "#,
            as_i64(id.get()),
            item_id,
            amount
        )
        .fetch_one(conn)
        .await
    }
}

#[derive(Default, FromRow)]
pub struct InventoryRow {
    pub coins: i64,
    pub gems: i64,
    pub tech: i64,
    pub utility: i64,
    pub production: i64,
    pub coal: i64,
    pub iron: i64,
    pub gold: i64,
    pub redstone: i64,
    pub lapis: i64,
    pub diamonds: i64,
    pub emeralds: i64,
}

impl Coins for InventoryRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for InventoryRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Mining for InventoryRow {
    fn miners(&self) -> i64 {
        0
    }

    fn mines(&self) -> i64 {
        0
    }

    fn land(&self) -> i64 {
        0
    }

    fn countries(&self) -> i64 {
        0
    }

    fn continents(&self) -> i64 {
        0
    }

    fn planets(&self) -> i64 {
        0
    }

    fn solar_systems(&self) -> i64 {
        0
    }

    fn galaxies(&self) -> i64 {
        0
    }

    fn universes(&self) -> i64 {
        0
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
        self.coal
    }

    fn iron(&self) -> i64 {
        self.iron
    }

    fn gold(&self) -> i64 {
        self.gold
    }

    fn redstone(&self) -> i64 {
        self.redstone
    }

    fn lapis(&self) -> i64 {
        self.lapis
    }

    fn diamonds(&self) -> i64 {
        self.diamonds
    }

    fn emeralds(&self) -> i64 {
        self.emeralds
    }
}

impl Commands {
    pub async fn inventory<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let (name, options) = parse_subcommand(options)?;

        match name {
            "show" => show::<Data>(ctx, interaction, pool).await,
            "use" => use_item::<Data>(ctx, interaction, options, pool).await,
            _ => Err(GamblingError::InvalidAmount),
        }
    }

    pub fn register_inventory<'a>() -> CreateCommand<'a> {
        let mut item_opt = CreateCommandOption::new(
            CommandOptionType::String,
            "item",
            "Select the item you want to activate",
        )
        .required(true);

        for item in SHOP_ITEMS.iter().filter(|item| item.useable) {
            item_opt = item_opt.add_string_choice(item.name, item.id);
        }

        let use_item = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "use",
            "Activate an item in your inventory",
        )
        .add_sub_option(item_opt)
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "amount",
            "Enter the number of items to activate",
        ));

        CreateCommand::new("inventory")
            .description("Inventory commands")
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "show",
                "Show your inventory and any active items",
            ))
            .add_option(use_item)
    }
}

async fn show<Data: EmojiCacheData>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
) -> Result<()> {
    let gambling_row = InventoryManager::gambling_row(pool, interaction.user.id)
        .await?
        .unwrap_or_default();

    let inventory_items =
        InventoryManager::inventory_items(pool, interaction.user.id).await?;

    let active_effects = {
        let mut conn = pool.acquire().await?;
        EffectsManager::active_effects(&mut conn, interaction.user.id).await?
    };

    let emojis = {
        let data_lock = ctx.data::<RwLock<Data>>();
        let data = data_lock.read().await;
        data.emojis()
    };

    let mut inv_items = SHOP_ITEMS
        .iter()
        .filter(|item| {
            matches!(
                item.category,
                ShopPage::Item | ShopPage::Boost1 | ShopPage::Boost2
            )
        })
        .map(|item| InventoryItem::from_shop_item(item, &emojis))
        .collect::<Result<Vec<_>>>()?;

    for item in &mut inv_items {
        if let Some(inv_item) = inventory_items
            .inventory()
            .iter()
            .find(|inv_item| inv_item.item_id == item.id)
        {
            item.quantity = inv_item.quantity;
        }
    }

    let (items, boosts) = inv_items.into_iter().partition::<Vec<_>, _>(|item| {
        matches!(item.cost[0], Some((_, ShopCurrency::Coins)))
    });

    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let active_boosts = if active_effects.is_empty() {
        "No active boosts".to_string()
    } else {
        active_effects
            .iter()
            .map(|row| {
                let label = SHOP_ITEMS
                    .get(row.item_id.as_str())
                    .and_then(|item| item.as_str(&emojis).ok())
                    .unwrap_or_else(|| row.item_id.clone());

                row.expiry.map_or_else(
                    || format!("{label} (ready)"),
                    |expiry| {
                        format!("{label} (<t:{}:R>)", expiry.to_jiff().as_second())
                    },
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut embed = CreateEmbed::new()
        .field(
            "Currencies",
            format!(
                "<:coin:{coin}> {} coins\n{GEM} {} gems",
                gambling_row.coins_str(),
                gambling_row.gems_str()
            ),
            false,
        )
        .field(
            "Items",
            items
                .into_iter()
                .map(|item| {
                    format!("{} `{}` {}", item.emoji, item.quantity, item.name)
                })
                .collect::<Vec<_>>()
                .join("\n"),
            true,
        )
        .field(
            "Boosts",
            boosts
                .into_iter()
                .map(|item| {
                    format!("{} `{}` {}", item.emoji, item.quantity, item.name)
                })
                .collect::<Vec<_>>()
                .join("\n"),
            true,
        )
        .field("Active Boosts", active_boosts, false)
        .field("Resources", gambling_row.resources(&emojis)?, true)
        .field("Crafted", gambling_row.crafted(&emojis)?, false)
        .field(
            "Weapons",
            format!(
                "{} is fighting with just their fists 👊",
                interaction.user.mention()
            ),
            false,
        );

    if let Some(avatar) = interaction.user.avatar_url() {
        embed = embed.thumbnail(avatar, None);
    }

    interaction
        .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}

async fn use_item<Data: EmojiCacheData>(
    ctx: &Context,
    interaction: &CommandInteraction,
    options: FixedArray<ResolvedOption<'_>>,
    pool: &PgPool,
) -> Result<()> {
    let mut options = parse_options(options);

    let Some(ResolvedValue::String(item_id)) = options.remove("item") else {
        return Err(GamblingError::InvalidAmount);
    };

    let Some(item) = SHOP_ITEMS.get(item_id) else {
        return Err(GamblingError::InvalidAmount);
    };

    let amount = match options.remove("amount") {
        Some(ResolvedValue::String(amount)) => {
            amount.parse().map_err(|e: ParseIntError| {
                tracing::debug!(error = %e, "failed to parse item amount");
                GamblingError::InvalidAmount
            })?
        },
        _ => 1,
    };

    if amount < 0 {
        return Err(GamblingError::NegativeAmount);
    }

    if amount == 0 {
        return Err(GamblingError::ZeroAmount);
    }

    let mut tx = pool.begin().await?;

    let quantity = match InventoryManager::edit_item_quantity(
        &mut tx,
        interaction.user.id,
        item_id,
        amount,
    )
    .await
    {
        Ok(q) => q,
        Err(sqlx::Error::RowNotFound) => return Err(GamblingError::InvalidAmount),
        r => r?,
    };

    for _ in 0..amount {
        EffectsManager::add_effect(&mut tx, interaction.user.id, item).await?;
    }

    tx.commit().await?;

    let emojis = {
        let data_lock = ctx.data::<RwLock<Data>>();
        let data = data_lock.read().await;
        data.emojis()
    };

    let mut description =
        format!("Successfully activated item:\n**{}**", item.as_str(&emojis)?);
    if let Some(duration) = item.effect_duration {
        let expiry = Timestamp::now().as_second().saturating_add(duration.as_secs());
        let _ = write!(description, "(<t:{expiry}:R>)");
    }

    let embed = CreateEmbed::new()
        .description(format!("{description}\nUses left:{quantity}"));

    interaction
        .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}
