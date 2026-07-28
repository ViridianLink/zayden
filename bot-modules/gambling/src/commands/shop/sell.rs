use serenity::all::{
    CommandInteraction,
    Context,
    EditInteractionResponse,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, parse_options_ref};

use crate::common::shop::SaleDelta;
use crate::{GamblingError, Result, SHOP_ITEMS, ShopManager};

pub async fn sell<Data: EmojiCacheData>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &PgPool,
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

    match ShopManager::sell_quantity(pool, interaction.user.id, item.id).await? {
        Some(held) if held < amount => {
            return Err(GamblingError::InsufficientItemQuantity(held));
        },
        Some(_) => {},
        None => return Err(GamblingError::ItemNotInInventory),
    }

    let delta = SaleDelta::new(item.coin_cost().unwrap_or(0), amount);

    let committed =
        ShopManager::commit_sale(pool, interaction.user.id, item.id, &delta)
            .await?
            .ok_or(GamblingError::TransactionConflict)?;

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
                delta.coins.format(),
                committed.quantity.format()
            )),
        )
        .await?;

    Ok(())
}
