use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    EditInteractionResponse,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, parse_options};

use super::Commands;
use crate::events::{Dispatch, Event, GameEvent};
use crate::utils::{GameEmbed, GameResult};
use crate::{
    Coins,
    EffectsManager,
    GamblingData,
    GamblingError,
    GameDelta,
    GameRow,
    Result,
};

impl Commands {
    pub async fn roll<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(dice)) = options.remove("dice") else {
            return Err(GamblingError::InvalidAmount);
        };

        let n_sides =
            dice.parse::<i64>().map_err(|_e| GamblingError::InvalidAmount)?;

        let Some(ResolvedValue::Integer(prediction)) = options.remove("prediction")
        else {
            return Err(GamblingError::InvalidPrediction);
        };

        verify_prediction(prediction, 1, n_sides)?;

        let mut row = GameRow::get(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let before = row.clone();

        let data = ctx.data::<RwLock<Data>>();

        data.read().await.game_cache().check_and_set(interaction.user.id)?;

        let Some(ResolvedValue::Integer(bet)) = options.remove("bet") else {
            return Err(GamblingError::InvalidAmount);
        };

        EffectsManager::bet_limit(pool, interaction.user.id, bet, row.coins())
            .await?;
        row.bet(bet);

        let roll = rand::random_range(1..=n_sides);

        let (title, mut payout) = if roll == prediction {
            ("🎲 Dice Roll 🎲 - You Won!", bet * n_sides)
        } else {
            ("🎲 Dice Roll 🎲 - You Lost!", 0)
        };

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        Dispatch::new(&ctx.http, pool, &emojis)
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Game(GameEvent::new(
                    "roll",
                    interaction.user.id,
                    bet,
                    payout,
                    roll == prediction,
                )),
            )
            .await?;

        let payout_result = EffectsManager::payout(
            pool,
            interaction.user.id,
            "roll",
            bet,
            payout,
            Some(roll == prediction),
        )
        .await;
        payout = payout_result.payout;

        row.add_coins(payout);

        let delta = GameDelta::between(&before, &row);

        let coins = GameRow::commit(pool, interaction.user.id, &delta)
            .await?
            .ok_or(GamblingError::TransactionConflict)?
            .coins;

        let embed = GameEmbed {
            title,
            prediction: GameResult::new_with_str(prediction.to_string(), "🎲"),
            outcome_text: "Result",
            outcome: GameResult::new_with_str(roll.to_string(), "🎲"),
            bet,
            payout,
            coins,
            effects: &payout_result.effects,
        }
        .build(&emojis)?;

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register_roll<'a>() -> CreateCommand<'a> {
        CreateCommand::new("roll")
            .description("Roll the dice")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "dice",
                    "The type of dice to roll",
                )
                .add_string_choice("4-sides", "4")
                .add_string_choice("6-sides", "6")
                .add_string_choice("8-sides", "8")
                .add_string_choice("10-sides", "10")
                .add_string_choice("12-sides", "12")
                .add_string_choice("20-sides", "20")
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "prediction",
                    "What number will the dice land on?",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "bet",
                    "Roll the dice",
                )
                .required(true),
            )
    }
}

const fn verify_prediction(prediction: i64, min: i64, max: i64) -> Result<()> {
    if prediction > max || prediction < min {
        return Err(GamblingError::InvalidPrediction);
    }

    Ok(())
}
