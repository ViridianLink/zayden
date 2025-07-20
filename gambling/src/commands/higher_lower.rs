use std::sync::Arc;

use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    CommandInteraction, Context, CreateButton, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use tokio::sync::RwLock;

use crate::games::higherlower::{CARD_TO_NUM, create_embed};
use crate::{CARD_DECK, GamblingData, GameCache, Result};

use super::Commands;

impl Commands {
    pub async fn higher_lower<Data: GamblingData>(
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> Result<()> {
        let data = ctx.data::<RwLock<Data>>();

        GameCache::can_play(Arc::clone(&data), interaction.user.id).await?;

        let mut deck = CARD_DECK.to_vec();
        deck.shuffle(&mut rng());

        let emoji = deck.pop().unwrap();
        let num = CARD_TO_NUM.get(&emoji).unwrap();

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

        GameCache::update(data, interaction.user.id).await;

        Ok(())
    }

    pub fn register_higher_lower<'a>() -> CreateCommand<'a> {
        CreateCommand::new("higherorlower").description("Play a game of higher or lower")
    }
}
