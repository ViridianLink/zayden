use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    CommandInteraction,
    Context,
    CreateButton,
    CreateCommand,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tokio::sync::RwLock;
use zayden_core::EmojiCacheData;

use super::Commands;
use crate::games::higherlower::create_embed;
use crate::{CARD_DECK, CARD_TO_NUM, GamblingData, Result, card_deck, card_to_num};

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

        data_lock.read().await.game_cache().check_and_set(interaction.user.id)?;

        let mut deck = CARD_DECK.get_or_init(|| card_deck(&emojis)).clone();
        deck.shuffle(&mut rng());

        let emoji = deck.pop().expect("deck not empty");
        let num = CARD_TO_NUM
            .get_or_init(|| card_to_num(&emojis))
            .get(&emoji)
            .expect("deck emoji always in card_to_num");

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
            .await?;

        Ok(())
    }

    pub fn register_higher_lower<'a>() -> CreateCommand<'a> {
        CreateCommand::new("higherorlower")
            .description("Play a game of higher or lower")
    }
}
