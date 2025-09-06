use std::sync::Arc;

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, parse_options};

use crate::events::{Dispatch, Event, GameEvent};
use crate::models::GamblingManager;
use crate::utils::{GameResult, game_embed};
use crate::{
    Coins, EffectsManager, Error, GamblingData, GameCache, GameManager, GameRow, GoalsManager,
    Result,
};

use super::Commands;

impl Commands {
    pub async fn roll<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(dice)) = options.remove("dice") else {
            unreachable!("dice option is required")
        };

        let n_sides = dice.parse::<i64>().unwrap();

        let Some(ResolvedValue::Integer(prediction)) = options.remove("prediction") else {
            unreachable!("prediction option is required")
        };

        verify_prediction(prediction, 1, n_sides)?;

        let mut row = GameHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let data = ctx.data::<RwLock<Data>>();

        GameCache::can_play(Arc::clone(&data), interaction.user.id).await?;

        let Some(ResolvedValue::Integer(bet)) = options.remove("bet") else {
            unreachable!("bet option is required")
        };

        EffectsHandler::bet_limit::<GamblingHandler>(pool, interaction.user.id, bet, row.coins())
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

        Dispatch::<Db, GoalHandler>::new(&ctx.http, pool, &emojis)
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

        payout = EffectsHandler::payout(
            pool,
            interaction.user.id,
            bet,
            payout,
            Some(roll == prediction),
        )
        .await;

        row.add_coins(payout);

        let coins = row.coins();

        GameHandler::save(pool, row).await.unwrap();
        GameCache::update(data, interaction.user.id).await;

        let embed = game_embed(
            &emojis,
            title,
            GameResult::new_with_str(prediction.to_string(), "🎲"),
            "Result",
            GameResult::new_with_str(roll.to_string(), "🎲"),
            bet,
            payout,
            coins,
        );

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

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
                CreateCommandOption::new(CommandOptionType::Integer, "bet", "Roll the dice")
                    .required(true),
            )
    }
}

fn verify_prediction(prediction: i64, min: i64, max: i64) -> Result<()> {
    if prediction > max || prediction < min {
        return Err(Error::InvalidPrediction);
    }

    Ok(())
}
