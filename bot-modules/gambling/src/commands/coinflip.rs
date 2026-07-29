use std::fmt::Display;
use std::str::FromStr;

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
use crate::utils::{Emoji, GameEmbed, GameResult};
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
    pub async fn coinflip<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(prediction)) = options.remove("prediction")
        else {
            return Err(GamblingError::InvalidPrediction);
        };
        let prediction = prediction
            .parse::<CoinSide>()
            .map_err(|()| GamblingError::InvalidPrediction)?;

        let Some(ResolvedValue::Integer(bet)) = options.remove("bet") else {
            return Err(GamblingError::InvalidAmount);
        };

        let mut row = GameRow::get(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let before = row.clone();

        let data = ctx.data::<RwLock<Data>>();

        data.read().await.game_cache().check_and_set(interaction.user.id)?;
        EffectsManager::bet_limit(pool, interaction.user.id, bet, row.coins())
            .await?;
        row.bet(bet);

        let heads = rand::random_bool(0.5);
        let winner = matches!(prediction, CoinSide::Heads) == heads;
        let edge = rand::random_bool(1.0 / 5000.0);

        let mut payout = match (winner, edge) {
            (true, true) => bet * 1000,
            (true, false) => bet * 2,
            _ => 0,
        };

        let emojis = {
            let data = data.read().await;
            data.emojis()
        };

        Dispatch::new(&ctx.http, pool, &emojis)
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Game(GameEvent::new(
                    "coinflip",
                    interaction.user.id,
                    bet,
                    payout,
                    winner,
                )),
            )
            .await?;

        let payout_result = EffectsManager::payout(
            pool,
            interaction.user.id,
            "coinflip",
            bet,
            payout,
            Some(winner),
        )
        .await;
        payout = payout_result.payout;

        row.add_coins(payout);

        let delta = GameDelta::between(&before, &row);

        let coins = GameRow::commit(pool, interaction.user.id, &delta)
            .await?
            .ok_or(GamblingError::TransactionConflict)?
            .coins;

        let (coin, title) = if edge {
            (prediction, "Coin Flip - EDGE ROLL!")
        } else if winner {
            (prediction, "Coin Flip - You Won!")
        } else {
            (prediction.opposite(), "Coin Flip - You Lost!")
        };

        let embed = GameEmbed {
            title,
            prediction: prediction.into(),
            outcome_text: "Coin landed on",
            outcome: coin.into(),
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

    pub fn register_coinflip<'a>() -> CreateCommand<'a> {
        CreateCommand::new("coinflip")
            .description("Flip a coin!")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "prediction",
                    "Choose whether you think the coin will be heads or tails",
                )
                .add_string_choice("Heads", "heads")
                .add_string_choice("Tails", "tails")
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "bet",
                    "The amount to bet.",
                )
                .required(true),
            )
    }
}

#[derive(Debug, Clone, Copy)]
enum CoinSide {
    Heads,
    Tails,
}

impl CoinSide {
    const fn opposite(self) -> Self {
        match self {
            Self::Heads => Self::Tails,
            Self::Tails => Self::Heads,
        }
    }
}

impl Display for CoinSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Heads => write!(f, "Heads"),
            Self::Tails => write!(f, "Tails"),
        }
    }
}

impl FromStr for CoinSide {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "heads" => Ok(Self::Heads),
            "tails" => Ok(Self::Tails),
            _ => Err(()),
        }
    }
}

impl From<CoinSide> for GameResult {
    fn from(value: CoinSide) -> Self {
        let emoji = match value {
            CoinSide::Heads => Emoji::Id("heads"),
            CoinSide::Tails => Emoji::Id("tails"),
        };

        Self { name: value.to_string(), emoji }
    }
}
