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
use crate::{
    CARD_DECK,
    GamblingData,
    GamblingError,
    Result,
    card_deck,
    card_to_num,
};

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

        let deck_ref = if let Some(d) = CARD_DECK.get() {
            d
        } else {
            let new_deck = card_deck(&emojis)?;
            let _ = CARD_DECK.set(new_deck);
            CARD_DECK.get().ok_or_else(|| {
                GamblingError::Internal("CARD_DECK init failed".to_string())
            })?
        };
        let mut deck = deck_ref.clone();
        deck.shuffle(&mut rng());

        let emoji = deck.pop().ok_or_else(|| {
            GamblingError::Internal("higher_lower deck is empty".to_string())
        })?;
        let card_map = card_to_num(&emojis)?;
        let num = card_map.get(&emoji).ok_or_else(|| {
            GamblingError::Internal("emoji not in card_to_num map".to_string())
        })?;

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
