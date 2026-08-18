use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateButton,
    CreateCommand,
    CreateCommandOption,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, parse_options};

use super::Commands;
use crate::components::HigherLowerCustomId;
use crate::games::higherlower::create_embed;
use crate::{
    CARD_DECK,
    Coins,
    EffectsManager,
    GamblingData,
    GamblingError,
    GameDelta,
    GameRow,
    Result,
    card_deck,
    card_to_num,
};

impl Commands {
    pub async fn higher_lower<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        let mut options = parse_options(options);

        let Some(ResolvedValue::Integer(bet)) = options.remove("bet") else {
            return Err(GamblingError::InvalidAmount);
        };

        let data_lock = ctx.data::<RwLock<Data>>();
        let emojis = {
            let data = data_lock.read().await;
            data.emojis()
        };

        data_lock.read().await.game_cache().check_and_set(interaction.user.id)?;

        let mut row = GameRow::get(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let before = row.clone();

        EffectsManager::bet_limit(pool, interaction.user.id, bet, row.coins())
            .await?;

        row.bet(bet);

        let delta = GameDelta::between(&before, &row);
        GameRow::commit(pool, interaction.user.id, &delta)
            .await?
            .ok_or(GamblingError::TransactionConflict)?;

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

        let embed = create_embed(&format!("<:{num}:{emoji}>"), 0, bet, 0, true);

        let higher_btn = CreateButton::new(HigherLowerCustomId::Higher.as_str())
            .emoji('☝')
            .label("Higher");
        let lower_btn = CreateButton::new(HigherLowerCustomId::Lower.as_str())
            .emoji('👇')
            .label("Lower");

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
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "bet",
                    "The amount to stake on the run",
                )
                .required(true)
                .min_int_value(1),
            )
    }
}
