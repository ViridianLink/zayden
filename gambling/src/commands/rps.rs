use std::sync::Arc;
use std::{fmt::Display, str::FromStr};

use rand::seq::IndexedRandom;
use serenity::all::{
    Colour, CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, EditInteractionResponse, ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{FormatNum, parse_options};

use crate::events::{Dispatch, Event, GameEvent};
use crate::models::GamblingManager;
use crate::{
    COIN, Coins, EffectsManager, GamblingData, GameCache, GameManager, GameRow, GoalsManager,
    Result,
};

use super::Commands;

impl Commands {
    pub async fn rps<
        Data: GamblingData,
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
        interaction.defer(&ctx.http).await.unwrap();

        let mut options = parse_options(options);

        let Some(ResolvedValue::String(selection)) = options.remove("selection") else {
            unreachable!("selection is required")
        };
        let user_choice = selection.parse::<RPSChoice>().unwrap();

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

        let computer_choice = *CHOICES.choose(&mut rand::rng()).unwrap();
        let winner = user_choice.winner(&computer_choice);

        let mut payout = if winner == Some(true) {
            bet * 2
        } else if winner.is_none() {
            bet
        } else {
            0
        };

        Dispatch::<Db, GoalHandler>::new(&ctx.http, pool)
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Game(GameEvent::new(
                    "rps",
                    interaction.user.id,
                    bet,
                    winner == Some(true),
                )),
            )
            .await?;

        payout = EffectsHandler::payout(pool, interaction.user.id, bet, payout, winner).await;

        row.add_coins(payout);

        let coins = row.coins();

        GameHandler::save(pool, row).await?;
        GameCache::update(data, interaction.user.id).await;

        let title = if winner == Some(true) {
            "Rock 🪨 Paper 🗞️ Scissors ✂ - You Won!"
        } else if winner == Some(false) {
            "Rock 🪨 Paper 🗞️ Scissors ✂ - You Lost!"
        } else {
            "Rock 🪨 Paper 🗞️ Scissors ✂ - You Tied!"
        };

        let desc = format!(
            "Your bet: {} <:coin:{COIN}>
        
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

        let embed = CreateEmbed::new()
            .title(title)
            .description(desc)
            .colour(colour);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

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
                CreateCommandOption::new(CommandOptionType::Integer, "bet", "The amount to bet.")
                    .required(true),
            )
    }
}

const CHOICES: [RPSChoice; 3] = [RPSChoice::Rock, RPSChoice::Paper, RPSChoice::Scissors];

#[derive(Clone, Copy, PartialEq, Eq)]
enum RPSChoice {
    Rock,
    Paper,
    Scissors,
}

impl RPSChoice {
    fn winner(&self, opponent: &Self) -> Option<bool> {
        match (self, opponent) {
            (a, b) if a == b => None,
            (Self::Rock, Self::Scissors)
            | (Self::Paper, Self::Rock)
            | (Self::Scissors, Self::Paper) => Some(true),
            _ => Some(false),
        }
    }

    fn emoji(&self) -> &str {
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
            RPSChoice::Rock => write!(f, "Rock"),
            RPSChoice::Paper => write!(f, "Paper"),
            RPSChoice::Scissors => write!(f, "Scissors"),
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
