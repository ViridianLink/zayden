use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::parse_options;

use crate::events::{Dispatch, Event, GameEvent};
use crate::models::gambling::GamblingManager;
use crate::utils::{Emoji, GameResult, game_embed};
use crate::{
    COIN, Coins, EffectsManager, GamblingData, GameCache, GameManager, GameRow, GoalsManager,
    Result, TAILS,
};

use super::Commands;

impl Commands {
    pub async fn coinflip<
        Data: GamblingData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(prediction)) = options.remove("prediction") else {
            unreachable!("prediction is required")
        };
        let prediction = prediction.parse::<CoinSide>().unwrap();

        let Some(ResolvedValue::Integer(bet)) = options.remove("bet") else {
            unreachable!("bet is required")
        };

        let mut row = GameHandler::row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let data = ctx.data::<RwLock<Data>>();

        GameCache::can_play(Arc::clone(&data), interaction.user.id).await?;
        EffectsHandler::bet_limit::<GamblingHandler>(pool, interaction.user.id, bet, row.coins())
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

        Dispatch::<Db, GoalsHandler>::new(&ctx.http, pool)
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Game(GameEvent::new("coinflip", interaction.user.id, bet, winner)),
            )
            .await?;

        payout = EffectsHandler::payout(pool, interaction.user.id, bet, payout, Some(winner)).await;

        row.add_coins(payout);

        let coins = row.coins();

        GameHandler::save(pool, row).await.unwrap();
        GameCache::update(data, interaction.user.id).await;

        let (coin, title) = if edge {
            (prediction, "Coin Flip - EDGE ROLL!")
        } else if winner {
            (prediction, "Coin Flip - You Won!")
        } else {
            (prediction.opposite(), "Coin Flip - You Lost!")
        };

        let embed = game_embed(
            title,
            prediction,
            "Coin landed on",
            coin,
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
                CreateCommandOption::new(CommandOptionType::Integer, "bet", "The amount to bet.")
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
    fn opposite(&self) -> CoinSide {
        match self {
            CoinSide::Heads => CoinSide::Tails,
            CoinSide::Tails => CoinSide::Heads,
        }
    }
}

impl Display for CoinSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoinSide::Heads => write!(f, "Heads"),
            CoinSide::Tails => write!(f, "Tails"),
        }
    }
}

impl FromStr for CoinSide {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "heads" => Ok(CoinSide::Heads),
            "tails" => Ok(CoinSide::Tails),
            _ => Err(()),
        }
    }
}

impl From<CoinSide> for GameResult<'_> {
    fn from(value: CoinSide) -> Self {
        let emoji = match value {
            CoinSide::Heads => Emoji::Id(COIN),
            CoinSide::Tails => Emoji::Id(TAILS),
        };

        Self {
            name: value.to_string(),
            emoji,
        }
    }
}
