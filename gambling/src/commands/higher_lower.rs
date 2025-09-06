use std::sync::Arc;

use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    CommandInteraction, Context, CreateButton, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tokio::sync::RwLock;
use zayden_core::EmojiCacheData;

use crate::games::higherlower::create_embed;
use crate::{CARD_DECK, CARD_TO_NUM, GamblingData, GameCache, Result, card_deck, card_to_num};

use super::Commands;

impl Commands {
    pub async fn higher_lower<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<()> {
        let data_lock = ctx.data::<RwLock<Data>>();
        let emojis = {
            let data = data_lock.read().await;
            data.emojis()
        };

        GameCache::can_play(Arc::clone(&data_lock), interaction.user.id).await?;

        let mut deck = CARD_DECK.get_or_init(|| card_deck(&emojis)).to_vec();
        deck.shuffle(&mut rng());

        let emoji = deck.pop().unwrap();
        let num = CARD_TO_NUM
            .get_or_init(|| card_to_num(&emojis))
            .get(&emoji)
            .unwrap();

        let embed = create_embed(&format!("<:{num}:{emoji}>"), 0, true);

        let higher_btn = CreateButton::new("hol_higher").emoji('☝').label("Higher");
        let lower_btn = CreateButton::new("hol_lower").emoji('👇').label("Lower");

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .button(higher_btn)
                        .button(lower_btn),
                ),
            )
            .await
            .unwrap();

        GameCache::update(data_lock, interaction.user.id).await;

        Ok(())
    }

    pub fn register_higher_lower<'a>() -> CreateCommand<'a> {
        CreateCommand::new("higherorlower").description("Play a game of higher or lower")
    }
}
