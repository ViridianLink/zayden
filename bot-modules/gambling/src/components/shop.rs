use serenity::all::{
    ComponentInteraction,
    Context,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::EmojiCacheData;

use crate::Result;
use crate::commands::inventory::InventoryManager;
use crate::common::shop::{ShopManager, ShopRow, shop_response};

pub struct Shop;

impl Shop {
    pub async fn run_components<
        Data: EmojiCacheData,
        Db: Database,
        Manager: ShopManager<Db> + InventoryManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let title = interaction
            .message
            .embeds
            .first()
            .and_then(|embed| embed.title.as_deref());

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let row = Manager::buy_row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| ShopRow::new(interaction.user.id));

        let inventory = Manager::inventory_items(pool, interaction.user.id).await?;

        let (embed, components) = if interaction.data.custom_id == "shop_next" {
            shop_response(&emojis, &row, &inventory, title, 1)?
        } else {
            shop_response(&emojis, &row, &inventory, title, -1)?
        };

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(vec![components]),
                ),
            )
            .await?;

        Ok(())
    }
}
