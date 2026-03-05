use serenity::all::{
    CommandInteraction, Context, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, parse_options_ref};

use crate::commands::shop::{ShopManager, ShopRow};
use crate::events::{Dispatch, Event, ShopPurchaseEvent};
use crate::models::GamblingItem;
use crate::{
    Coins, Error, Gems, GoalsManager, ItemInventory, MaxValues, Result, SHOP_ITEMS, ShopCurrency,
    ShopItem, ShopPage,
};

pub async fn buy<
    Data: EmojiCacheData,
    Db: Database,
    GoalsHandler: GoalsManager<Db>,
    BuyHandler: ShopManager<Db>,
>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    options: &[ResolvedOption<'_>],
) -> Result<()> {
    let mut options = parse_options_ref(options);

    let Some(ResolvedValue::String(item)) = options.remove("item") else {
        unreachable!("item is required");
    };

    let item = SHOP_ITEMS
        .get(item)
        .expect("Preset choices so item should always exist");

    let Some(ResolvedValue::String(amount)) = options.remove("amount") else {
        unreachable!("amount is required")
    };

    let mut row = match BuyHandler::buy_row(pool, interaction.user.id).await? {
        Some(row) => row,
        None => ShopRow::new(interaction.user.id),
    };

    let amount: i64 = match amount.parse() {
        Ok(x) => x,
        Err(_) if *amount == "a" => {
            return Err(Error::PremiumRequired);

            // match costs.first().copied() {
            //     Some((coins, ShopCurrency::Coins)) => row.coins() / coins,
            //     Some((gems, ShopCurrency::Gems)) => row.gems() / gems,
            //     Some(_) => unimplemented!("Currency not implimented"),
            //     None => unreachable!("No cost found"),
            // }
        }
        _ => return Err(Error::InvalidAmount),
    };

    if amount.is_negative() {
        return Err(Error::NegativeAmount);
    }

    if amount == 0 {
        return Err(Error::ZeroAmount);
    }

    let costs = item.costs(amount);

    for (cost, currency) in costs.iter().copied() {
        let funds = match currency {
            ShopCurrency::Coins => row.coins_mut(),
            ShopCurrency::Gems => row.gems_mut(),
            ShopCurrency::Tech => &mut row.tech,
            ShopCurrency::Utility => &mut row.utility,
            ShopCurrency::Production => &mut row.production,
            _ => unimplemented!("Currnecy not implemented"),
        };

        *funds -= cost;

        if *funds < 0 {
            return Err(Error::InsufficientFunds {
                required: funds.abs(),
                currency,
            });
        }
    }

    let quantity = if matches!(item.category, ShopPage::Mine1 | ShopPage::Mine2) {
        edit_mine(&mut row, item, amount)?
    } else {
        edit_inv(&mut row, item, amount)
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
            Event::ShopPurchase(ShopPurchaseEvent::new(interaction.user.id, item.id)),
        )
        .await?;

    BuyHandler::buy_save(pool, row).await.unwrap();

    let cost = costs
        .into_iter()
        .map(|(cost, currency)| format!("`{}` {}", cost.format(), currency.emoji(&emojis)))
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

fn edit_inv(row: &mut ShopRow, item: &ShopItem<'_>, amount: i64) -> i64 {
    let inventory = row.inventory_mut();

    match inventory
        .iter_mut()
        .find(|inv_item| inv_item.item_id == item.id)
    {
        Some(item) => {
            item.quantity += amount;
            item.quantity
        }
        None => {
            let mut item = GamblingItem::from(item);
            item.quantity = amount;
            inventory.push(item);
            amount
        }
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
        _ => unreachable!("Invalid item id {}", item.id),
    };
    let current = *value;

    *value += amount;

    let quantity = *value;
    let max_value = *row.max_values().get(item.id).unwrap();

    if quantity > max_value {
        return Err(Error::InsufficientCapacity(max_value - current));
    }

    Ok(quantity)
}
