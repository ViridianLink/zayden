use std::{marker::PhantomData, sync::Arc};

use serenity::all::{
    ButtonStyle, CommandInteraction, CommandOptionType, Context, CreateButton, CreateCommand,
    CreateCommandOption, CreateEmbed, EditInteractionResponse, Mentionable, ResolvedOption,
    ResolvedValue, UserId,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, parse_options};

use crate::{
    Coins, EffectsManager, GamblingData, GameCache, GameManager, GameRow, GoalsManager, Result,
    models::GamblingManager,
};

use super::Commands;

impl Commands {
    pub async fn tictactoe<
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
        interaction.defer(&ctx.http).await.unwrap();

        let row = GameHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let data_lock = ctx.data::<RwLock<Data>>();

        GameCache::can_play(Arc::clone(&data_lock), interaction.user.id).await?;

        let mut options = parse_options(options);

        let ResolvedValue::String(size) = options.remove("size").unwrap() else {
            unreachable!("size is required")
        };

        let ResolvedValue::Integer(bet) = options.remove("bet").unwrap() else {
            unreachable!("bet is required option")
        };

        EffectsHandler::bet_limit::<GamblingHandler>(pool, interaction.user.id, bet, row.coins())
            .await?;

        GameHandler::save(pool, row).await.unwrap();
        GameCache::update(Arc::clone(&data_lock), interaction.user.id).await;

        let coin = {
            let data = data_lock.read().await;
            let emojis = data.emojis();
            emojis.emoji("heads").unwrap()
        };

        let embed = CreateEmbed::new().title("TicTacToe").description(format!(
            "{} wants to play tic-tac-toe ({size}x{size}) for **{bet}** <:coin:{coin}>",
            interaction.user.mention(),
        ));

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed.clone())
                    .button(
                        CreateButton::new("ttt_accept")
                            .label("Accept")
                            .emoji('✅')
                            .style(ButtonStyle::Secondary),
                    )
                    .button(
                        CreateButton::new("ttt_cancel")
                            .label("Cancel")
                            .emoji('❌')
                            .style(ButtonStyle::Secondary),
                    ),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_tictactoe<'a>() -> CreateCommand<'a> {
        CreateCommand::new("tictactoe")
            .description("Play a game of tic tac toe")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "size",
                    "Choose the board size to play.",
                )
                .add_string_choice("3x3", "3")
                .add_string_choice("4x4", "4")
                .add_string_choice("5x5", "5")
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(CommandOptionType::Integer, "bet", "The amount to bet.")
                    .required(true),
            )
    }
}

struct GameState<Db: Database, Manager: GameManager<Db>> {
    size: usize,
    players: [UserId; 2],
    current_turn: UserId,
    bet: i64,
    winner: Option<UserId>,

    _db: PhantomData<Db>,
    _manager: PhantomData<Manager>,
}

impl<Db, Manager> GameState<Db, Manager>
where
    Db: Database,
    Manager: GameManager<Db>,
{
    fn new(p1: impl Into<UserId>, size: usize, bet: i64) -> Self {
        let p1 = p1.into();

        Self {
            size,
            players: [p1, p1],
            current_turn: p1,
            bet,
            winner: None,

            _db: PhantomData,
            _manager: PhantomData,
        }
    }

    async fn p1_row(&self, pool: &Pool<Db>) -> GameRow {
        let id = self.players[0];

        Manager::row(pool, id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(id))
    }

    async fn p2_row(&self, pool: &Pool<Db>) -> GameRow {
        let id = self.players[1];

        Manager::row(pool, id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(id))
    }
}
