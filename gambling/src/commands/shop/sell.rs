use serenity::all::{
    CommandInteraction, EditInteractionResponse, Http, ResolvedOption, ResolvedValue, UserId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use zayden_core::{FormatNum, parse_options_ref};

use crate::commands::shop::ShopManager;
use crate::shop::SALES_TAX;
use crate::{COIN, Coins, Error, Result, SHOP_ITEMS};

#[derive(FromRow)]
pub struct SellRow {
    pub id: i64,
    pub coins: i64,
    pub item_row_id: Option<i32>,
    pub item_quantity: Option<i64>,
}

impl SellRow {
    fn new(id: impl Into<UserId> + Send) -> Self {
        let id = id.into();

        Self {
            id: id.get() as i64,
            coins: 0,
            item_row_id: None,
            item_quantity: None,
        }
    }
}

impl Coins for SellRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

pub async fn sell<Db: Database, Manager: ShopManager<Db>>(
    http: &Http,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    options: &[ResolvedOption<'_>],
) -> Result<()> {
    let mut options = parse_options_ref(options);

    let Some(ResolvedValue::String(item)) = options.remove("item") else {
        unreachable!("item is required");
    };

    let Some(ResolvedValue::Integer(amount)) = options.remove("amount") else {
        unreachable!("amount is required")
    };
    let amount = *amount;

    if amount.is_negative() {
        return Err(Error::NegativeAmount);
    }

    let item = SHOP_ITEMS
        .get(item)
        .expect("Preset choices so item should always exist");
    let payment = ((item.coin_cost().unwrap() as f64) * (amount as f64) * (1.0 - SALES_TAX)) as i64;

    let mut row = match Manager::sell_row(pool, interaction.user.id, item.id)
        .await
        .unwrap()
    {
        Some(row) => row,
        None => SellRow::new(interaction.user.id),
    };

    let quantity = match &mut row.item_quantity {
        Some(quantity) if *quantity < amount => {
            return Err(Error::InsufficientItemQuantity(*quantity));
        }
        Some(quantity) => {
            *quantity -= amount;
            *quantity
        }
        None => return Err(Error::ItemNotInInventory),
    };

    row.add_coins(payment);

    Manager::sell_save(pool, row).await.unwrap();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new().content(format!(
                "You sold {} {item} for {} <:coin:{COIN}>\nYou now have {}.",
                amount.format(),
                payment.format(),
                quantity.format()
            )),
        )
        .await?;

    Ok(())
}
