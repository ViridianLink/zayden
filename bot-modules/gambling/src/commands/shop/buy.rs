use serenity::all::{
    CommandInteraction,
    Context,
    EditInteractionResponse,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, parse_options_ref};

use crate::commands::inventory::InventoryManager;
use crate::commands::shop::{ShopManager, ShopRow};
use crate::events::{Dispatch, Event, ShopPurchaseEvent};
use crate::models::GamblingItem;
use crate::{
    Coins,
    GamblingError,
    GamblingItems,
    Gems,
    GoalsManager,
    MaxValues,
    Result,
    SHOP_ITEMS,
    ShopCurrency,
    ShopItem,
    ShopPage,
};

pub async fn buy<
    Data: EmojiCacheData,
    Db: Database,
    GoalsHandler: GoalsManager<Db> + Send + Sync,
    BuyHandler: ShopManager<Db> + InventoryManager<Db>,
>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    options: &[ResolvedOption<'_>],
) -> Result<()> {
    let mut options = parse_options_ref(options);

    let Some(ResolvedValue::String(item)) = options.remove("item") else {
        return Err(GamblingError::InvalidAmount);
    };

    let item =
        SHOP_ITEMS.get(item).expect("Preset choices so item should always exist");

    let Some(ResolvedValue::String(amount)) = options.remove("amount") else {
        return Err(GamblingError::InvalidAmount);
    };

    let mut row = BuyHandler::buy_row(pool, interaction.user.id)
        .await?
        .unwrap_or_else(|| ShopRow::new(interaction.user.id));

    let amount: i64 = match amount.parse() {
        Ok(x) => x,
        Err(_) if *amount == "a" => {
            let unit_costs = item.costs(1);
            if unit_costs.is_empty() {
                return Err(GamblingError::InvalidAmount);
            }
            let mut affordable = i64::MAX;
            for (unit_cost, currency) in unit_costs {
                let balance = match currency {
                    ShopCurrency::Coins => row.coins(),
                    ShopCurrency::Gems => row.gems(),
                    ShopCurrency::Tech => row.tech,
                    ShopCurrency::Utility => row.utility,
                    ShopCurrency::Production => row.production,
                    ShopCurrency::Coal
                    | ShopCurrency::Iron
                    | ShopCurrency::Gold
                    | ShopCurrency::Redstone
                    | ShopCurrency::Lapis
                    | ShopCurrency::Diamonds
                    | ShopCurrency::Emeralds => {
                        return Err(GamblingError::InvalidAmount);
                    },
                };
                affordable = affordable.min(balance / unit_cost);
            }
            affordable
        },
        _ => return Err(GamblingError::InvalidAmount),
    };

    if amount.is_negative() {
        return Err(GamblingError::NegativeAmount);
    }

    if amount == 0 {
        return Err(GamblingError::ZeroAmount);
    }

    let costs = item.costs(amount);

    for (cost, currency) in costs.iter().copied() {
        let funds = match currency {
            ShopCurrency::Coins => row.coins_mut(),
            ShopCurrency::Gems => row.gems_mut(),
            ShopCurrency::Tech => &mut row.tech,
            ShopCurrency::Utility => &mut row.utility,
            ShopCurrency::Production => &mut row.production,
            ShopCurrency::Coal
            | ShopCurrency::Iron
            | ShopCurrency::Gold
            | ShopCurrency::Redstone
            | ShopCurrency::Lapis
            | ShopCurrency::Diamonds
            | ShopCurrency::Emeralds => return Err(GamblingError::InvalidAmount),
        };

        *funds -= cost;

        if *funds < 0 {
            return Err(GamblingError::InsufficientFunds {
                required: funds.abs(),
                currency,
            });
        }
    }

    let quantity = if matches!(item.category, ShopPage::Mine1 | ShopPage::Mine2) {
        edit_mine(&mut row, item, amount)?
    } else {
        let mut inventory =
            BuyHandler::inventory_items(pool, interaction.user.id).await?;

        let quantity = edit_inv(&mut inventory, item, amount);

        BuyHandler::save_inventory(pool, interaction.user.id, inventory).await?;

        quantity
    };

    let emojis = {
        let data_lock = ctx.data::<RwLock<Data>>();
        let data = data_lock.read().await;
        data.emojis()
    };

    Dispatch::<Db, GoalsHandler>::new(&ctx.http, pool, &emojis)
        .fire(
            interaction.channel_id,
            &mut row,
            Event::ShopPurchase(ShopPurchaseEvent::new(
                interaction.user.id,
                item.id,
            )),
        )
        .await?;

    BuyHandler::buy_save(pool, row).await?;

    let cost = costs
        .into_iter()
        .map(|(cost, currency)| {
            format!("`{}` {}", cost.format(), currency.emoji(&emojis))
        })
        .collect::<Vec<_>>();

    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "You bought {} {} for {}\nYou now have {}.",
                amount.format(),
                item.as_str(&emojis),
                cost.join("\n"),
                quantity.format()
            )),
        )
        .await?;

    Ok(())
}

fn edit_inv(inventory: &mut GamblingItems, item: &ShopItem<'_>, amount: i64) -> i64 {
    if let Some(item) =
        inventory.0.iter_mut().find(|inv_item| inv_item.item_id == item.id)
    {
        item.quantity += amount;
        item.quantity
    } else {
        let mut item = GamblingItem::from(item);
        item.quantity = amount;
        inventory.0.push(item);
        amount
    }
}

fn edit_mine(row: &mut ShopRow, item: &ShopItem<'_>, amount: i64) -> Result<i64> {
    let value = match item.id {
        "miner" => &mut row.miners,
        "mine" => &mut row.mines,
        "land" => &mut row.land,
        "country" => &mut row.countries,
        "continent" => &mut row.continents,
        "planet" => &mut row.planets,
        "solar_system" => &mut row.solar_systems,
        "galaxy" => &mut row.galaxies,
        "universe" => &mut row.universes,
        _ => return Err(GamblingError::InvalidAmount),
    };
    let current = *value;

    *value += amount;

    let quantity = *value;
    let max_value = *row.max_values().get(item.id).expect("item ID in max_values");

    if quantity > max_value {
        return Err(GamblingError::InsufficientCapacity(max_value - current));
    }

    Ok(quantity)
}
