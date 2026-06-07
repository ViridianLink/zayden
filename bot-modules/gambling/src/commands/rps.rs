use std::fmt::Display;
use std::str::FromStr;

use rand::seq::IndexedRandom;
use serenity::all::{
    Colour,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    ResolvedOption,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, parse_options};

use super::Commands;
use crate::events::{Dispatch, Event, GameEvent};
use crate::models::GamblingManager;
use crate::{
    Coins,
    EffectsManager,
    GamblingData,
    GamblingError,
    GameManager,
    GameRow,
    GoalsManager,
    Result,
};

impl Commands {
    pub async fn rps<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalHandler: GoalsManager<Db> + Send + Sync,
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

        let Some(ResolvedValue::String(selection)) = options.remove("selection")
        else {
            return Err(GamblingError::InvalidAmount);
        };

        let user_choice = selection
            .parse::<RPSChoice>()
            .map_err(|()| GamblingError::InvalidPrediction)?;

        let Some(ResolvedValue::Integer(bet)) = options.remove("bet") else {
            return Err(GamblingError::InvalidAmount);
        };

        let mut row = GameHandler::row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let data = ctx.data::<RwLock<Data>>();

        data.read().await.game_cache().check_and_set(interaction.user.id)?;
        EffectsHandler::bet_limit::<GamblingHandler>(
            pool,
            interaction.user.id,
            bet,
            row.coins(),
        )
        .await?;
        row.bet(bet);

        let computer_choice =
            *CHOICES.choose(&mut rand::rng()).unwrap_or(&RPSChoice::Rock);
        let winner = user_choice.winner(computer_choice);

        let mut payout = if winner == Some(true) {
            bet * 2
        } else if winner.is_none() {
            bet
        } else {
            0
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
                    "rps",
                    interaction.user.id,
                    bet,
                    payout,
                    winner == Some(true),
                )),
            )
            .await?;

        payout =
            EffectsHandler::payout(pool, interaction.user.id, bet, payout, winner)
                .await;

        row.add_coins(payout);

        let coins = row.coins();

        GameHandler::save(pool, row).await?;

        let title = if winner == Some(true) {
            "Rock 🪨 Paper 🗞️ Scissors ✂ - You Won!"
        } else if winner == Some(false) {
            "Rock 🪨 Paper 🗞️ Scissors ✂ - You Lost!"
        } else {
            "Rock 🪨 Paper 🗞️ Scissors ✂ - You Tied!"
        };

        let coin = emojis.emoji("heads").map_err(|n| {
            GamblingError::Internal(format!("emoji '{n}' not in cache"))
        })?;

        let desc = format!(
            "Your bet: {} <:coin:{coin}>
        
            **You picked:** {} ({user_choice})
            **Zayden picked:** {} ({computer_choice})
            
            Payout: {} ({})
            Your coins: {}",
            bet.format(),
            user_choice.emoji(),
            computer_choice.emoji(),
            payout.format(),
            (payout - bet).format(),
            coins.format()
        );

        let colour = if winner == Some(true) {
            Colour::DARK_GREEN
        } else if winner == Some(false) {
            Colour::RED
        } else {
            Colour::DARKER_GREY
        };

        let embed = CreateEmbed::new().title(title).description(desc).colour(colour);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register_rps<'a>() -> CreateCommand<'a> {
        CreateCommand::new("rps")
            .description("Play a game of rock paper scissors against the bot")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "selection",
                    "Your choice of Rock, Paper or Scissors",
                )
                .required(true)
                .add_string_choice("Rock", "rock")
                .add_string_choice("Paper", "paper")
                .add_string_choice("Scissors", "scissors"),
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

const CHOICES: [RPSChoice; 3] =
    [RPSChoice::Rock, RPSChoice::Paper, RPSChoice::Scissors];

#[derive(Clone, Copy, PartialEq, Eq)]
enum RPSChoice {
    Rock,
    Paper,
    Scissors,
}

impl RPSChoice {
    const fn winner(self, opponent: Self) -> Option<bool> {
        match (self, opponent) {
            (Self::Rock, Self::Rock)
            | (Self::Paper, Self::Paper)
            | (Self::Scissors, Self::Scissors) => None,
            (Self::Rock, Self::Scissors)
            | (Self::Paper, Self::Rock)
            | (Self::Scissors, Self::Paper) => Some(true),
            (Self::Rock, Self::Paper)
            | (Self::Paper, Self::Scissors)
            | (Self::Scissors, Self::Rock) => Some(false),
        }
    }

    const fn emoji(self) -> &'static str {
        match self {
            Self::Rock => "🪨",
            Self::Paper => "🗞️",
            Self::Scissors => "✂",
        }
    }
}

impl Display for RPSChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rock => write!(f, "Rock"),
            Self::Paper => write!(f, "Paper"),
            Self::Scissors => write!(f, "Scissors"),
        }
    }
}

impl FromStr for RPSChoice {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "rock" => Ok(Self::Rock),
            "paper" => Ok(Self::Paper),
            "scissors" => Ok(Self::Scissors),
            _ => Err(()),
        }
    }
}
