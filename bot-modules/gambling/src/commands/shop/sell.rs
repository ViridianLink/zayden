use serenity::all::{
    CommandInteraction,
    Context,
    EditInteractionResponse,
    ResolvedOption,
    ResolvedValue,
    UserId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, as_i64, parse_options_ref};

use crate::shop::SALES_RETURN;
use crate::{Coins, GamblingError, Result, SHOP_ITEMS, ShopManager};

#[derive(FromRow)]
pub struct SellRow {
    pub user_id: i64,
    pub coins: i64,
    pub item_row_id: Option<i32>,
    pub item_quantity: Option<i64>,
}

impl SellRow {
    fn new(id: impl Into<UserId> + Send) -> Self {
        let id = id.into();

        Self {
            user_id: as_i64(id.get()),
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

pub async fn sell<Data: EmojiCacheData, Db: Database, Manager: ShopManager<Db>>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    options: &[ResolvedOption<'_>],
) -> Result<()> {
    let mut options = parse_options_ref(options);

    let Some(ResolvedValue::String(item)) = options.remove("item") else {
        return Err(GamblingError::InvalidAmount);
    };

    let Some(ResolvedValue::Integer(amount)) = options.remove("amount") else {
        return Err(GamblingError::InvalidAmount);
    };
    let amount = *amount;

    if amount.is_negative() {
        return Err(GamblingError::NegativeAmount);
    }

    let Some(item) = SHOP_ITEMS.get(item) else {
        return Err(GamblingError::InvalidAmount);
    };

    let total_coin_cost = item.coin_cost().unwrap_or(0) * amount;
    let payment = total_coin_cost * SALES_RETURN / 100;

    let mut row = Manager::sell_row(pool, interaction.user.id, item.id)
        .await?
        .unwrap_or_else(|| SellRow::new(interaction.user.id));

    let quantity = match &mut row.item_quantity {
        Some(quantity) if *quantity < amount => {
            return Err(GamblingError::InsufficientItemQuantity(*quantity));
        },
        Some(quantity) => {
            *quantity -= amount;
            *quantity
        },
        None => return Err(GamblingError::ItemNotInInventory),
    };

    row.add_coins(payment);

    Manager::sell_save(pool, row).await?;

    let emojis = {
        let data_lock = ctx.data::<RwLock<Data>>();
        let data = data_lock.read().await;
        data.emojis()
    };

    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new().content(format!(
                "You sold {} {} for {} <:coin:{coin}>\nYou now have {}.",
                amount.format(),
                item.as_str(&emojis)?,
                payment.format(),
                quantity.format()
            )),
        )
        .await?;

    Ok(())
}
