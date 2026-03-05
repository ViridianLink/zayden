use serenity::all::{
    CommandInteraction, Context, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::EmojiCacheData;

use crate::shop::shop_response;
use crate::{Result, ShopManager, ShopPage, ShopRow};

pub async fn list<Data: EmojiCacheData, Db: Database, Manager: ShopManager<Db>>(
    ctx: &Context,
    interaction: &CommandInteraction,
    pool: &Pool<Db>,
    options: &[ResolvedOption<'_>],
) -> Result<()> {
    let page = match options.first().map(|opt| &opt.value) {
        Some(ResolvedValue::String(page)) => page.parse::<ShopPage>().unwrap(),
        _ => ShopPage::Item,
    };

    let row = match Manager::buy_row(pool, interaction.user.id).await.unwrap() {
        Some(row) => row,
        None => ShopRow::new(interaction.user.id),
    };

    let emojis = {
        let data_lock = ctx.data::<RwLock<Data>>();
        let data = data_lock.read().await;
        data.emojis()
    };

    let title = format!("{page} Shop");

    let (embed, components) = shop_response(&emojis, &row, Some(&title), 0);

    interaction
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .embed(embed)
                .components(vec![components]),
        )
        .await?;

    Ok(())
}
