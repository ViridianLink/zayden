use std::{marker::PhantomData, sync::Arc, time::Duration};

use futures::StreamExt;
use rand::{rng, seq::IndexedRandom};
use serenity::all::{
    ActionRowComponent, ButtonStyle, CollectComponentInteractions, Colour, CommandInteraction,
    CommandOptionType, Component, ComponentInteraction, Context, CreateActionRow, CreateButton,
    CreateCommand, CreateCommandOption, CreateComponent, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, Http, Mentionable, ReactionType,
    ResolvedOption, ResolvedValue, UserId,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::parse_options;

use crate::{
    BLANK, COIN, Coins, EffectsManager, GamblingData, GameCache, GameManager, GameRow,
    GoalsManager, Result,
    events::{Dispatch, Event, GameEvent},
    models::GamblingManager,
};

use super::Commands;

const EMOJI_P1: char = '❌';
const EMOJI_P2: char = '⭕';

impl Commands {
    pub async fn tictactoe<
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

        let row = GameHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let data = ctx.data::<RwLock<Data>>();

        GameCache::can_play(Arc::clone(&data), interaction.user.id).await?;

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
        GameCache::update(Arc::clone(&data), interaction.user.id).await;

        let embed = CreateEmbed::new().title("TicTacToe").description(format!(
            "{} wants to play tic-tac-toe ({size}x{size}) for **{bet}** <:coin:{COIN}>",
            interaction.user.mention(),
        ));

        let msg = interaction
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

        let mut stream = msg
            .id
            .collect_component_interactions(ctx)
            .timeout(Duration::from_secs(120))
            .stream();

        let mut state =
            GameState::<Db, GameHandler>::new(interaction.user.id, size.parse().unwrap(), bet);

        while let Some(component) = stream.next().await {
            if !run_component::<Db, GamblingHandler, EffectsHandler, GameHandler>(
                &ctx.http,
                interaction,
                component,
                pool,
                &mut state,
            )
            .await?
            {
                break;
            }
        }

        let mut p1_row = state.p1_row(pool).await;
        let mut p2_row = state.p2_row(pool).await;

        let [p1, p2] = state.players;

        let embed = if let Some(winner) = state.winner {
            let row = if p1 == winner {
                &mut p1_row
            } else {
                &mut p2_row
            };

            row.add_coins(bet * 2);

            CreateEmbed::new()
                .title("TicTacToe")
                .description(format!("Winner! {} 🎉", winner.mention()))
                .colour(Colour::DARK_GREEN)
        } else if p1 != p2 {
            p1_row.add_coins(bet);
            p2_row.add_coins(bet);

            CreateEmbed::new()
                .title("TicTacToe")
                .description("It's a draw!")
                .colour(Colour::ORANGE)
        } else {
            p1_row.add_coins(bet);

            CreateEmbed::new()
                .title("TicTacToe")
                .description("This game timed out after 2 minutes of inactivity")
                .colour(Colour::TEAL)
        };

        let dispatch = Dispatch::<Db, GoalHandler>::new(&ctx.http, pool);

        dispatch
            .fire(
                interaction.channel_id,
                &mut p1_row,
                Event::Game(GameEvent::new("rps", p1, state.bet, state.bet, false)), // TODO: Fix win logic
            )
            .await?;

        dispatch
            .fire(
                interaction.channel_id,
                &mut p2_row,
                Event::Game(GameEvent::new("rps", p2, state.bet, state.bet, false)), // TODO: Fix win logic
            )
            .await?;

        GameHandler::save(pool, p1_row).await?;
        GameHandler::save(pool, p2_row).await?;

        GameCache::update(Arc::clone(&data), p1).await;
        GameCache::update(data, p2).await;

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(Vec::new()),
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

async fn run_component<
    Db: Database,
    GamblingHandler: GamblingManager<Db>,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    http: &Http,
    interaction: &CommandInteraction,
    component: ComponentInteraction,
    pool: &Pool<Db>,
    state: &mut GameState<Db, GameHandler>,
) -> Result<bool> {
    let custom_id = &component.data.custom_id;

    if custom_id == "ttt_cancel" && component.user == interaction.user {
        let embed = CreateEmbed::new()
            .title("TicTacToe")
            .description("Game cancelled");

        let msg = CreateInteractionResponseMessage::new()
            .embed(embed)
            .components(Vec::new());

        component
            .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
            .await
            .unwrap();

        return Ok(false);
    }

    if custom_id == "ttt_accept" && component.user == interaction.user {
        return Ok(true);
    }

    if custom_id == "ttt_accept" {
        let msg = accept::<Db, GamblingHandler, EffectsHandler, GameHandler>(
            pool,
            state,
            component.user.id,
        )
        .await?;

        component
            .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
            .await
            .unwrap();

        return Ok(true);
    }

    if component.user.id != state.current_turn {
        return Ok(true);
    }

    let mut pos = custom_id.strip_prefix("ttt_").unwrap().chars();
    let i = pos.next().unwrap().to_digit(10).unwrap() as usize;
    let j = pos.next().unwrap().to_digit(10).unwrap() as usize;

    let mut components = component.message.components.clone();

    let Component::ActionRow(action_row) = components.get_mut(i).unwrap() else {
        unreachable!("Component must be an action row")
    };

    let ActionRowComponent::Button(button) = action_row.components.get_mut(j).unwrap() else {
        unreachable!("Component must be a button")
    };

    if button.emoji == Some(EMOJI_P1.into()) || button.emoji == Some(EMOJI_P2.into()) {
        return Ok(true);
    }

    let emoji = if state.current_turn == state.players[0] {
        ReactionType::from(EMOJI_P1)
    } else {
        ReactionType::from(EMOJI_P2)
    };

    button.emoji = Some(emoji.clone());

    if check_win(state, &components, emoji) {
        state.winner = Some(state.current_turn);
        return Ok(false);
    } else if check_draw(&components) {
        return Ok(false);
    }

    let components = components
        .into_iter()
        .map(|component| {
            let Component::ActionRow(row) = component else {
                unreachable!("Component must be an action row")
            };

            let buttons = row
                .components
                .into_iter()
                .map(|c| {
                    let ActionRowComponent::Button(b) = c else {
                        unreachable!("Component must be of type Button")
                    };

                    b.into()
                })
                .collect::<Vec<CreateButton>>();

            CreateComponent::ActionRow(CreateActionRow::buttons(buttons))
        })
        .collect::<Vec<_>>();

    // Next player
    state.current_turn = if state.current_turn == state.players[0] {
        state.players[1]
    } else {
        state.players[0]
    };

    let embed = CreateEmbed::new()
        .title("TicTacToe")
        .description(format!("{}'s Turn", state.current_turn.mention()));

    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components);

    component
        .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
        .await
        .unwrap();

    Ok(true)
}

async fn accept<
    'a,
    Db: Database,
    GamblingHandler: GamblingManager<Db>,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    pool: &Pool<Db>,
    state: &mut GameState<Db, GameHandler>,
    p2: UserId,
) -> Result<CreateInteractionResponseMessage<'a>> {
    state.players[1] = p2;

    let mut p1_row = state.p1_row(pool).await;
    let mut p2_row = state.p2_row(pool).await;

    EffectsHandler::bet_limit::<GamblingHandler>(pool, p1_row.id as u64, state.bet, p1_row.coins())
        .await?;
    EffectsHandler::bet_limit::<GamblingHandler>(pool, p2_row.id as u64, state.bet, p2_row.coins())
        .await?;

    state.current_turn = *state.players.choose(&mut rng()).unwrap();

    p1_row.add_coins(-state.bet);
    p2_row.add_coins(-state.bet);

    GameHandler::save(pool, p1_row).await.unwrap();
    GameHandler::save(pool, p2_row).await.unwrap();

    let embed = CreateEmbed::new()
        .title("TicTacToe")
        .description(format!("{}'s Turn", state.current_turn.mention()));

    let components = (0..state.size)
        .map(|i| {
            let row = (0..state.size)
                .map(|j| {
                    CreateButton::new(format!("ttt_{i}{j}"))
                        .emoji(BLANK)
                        .style(ButtonStyle::Secondary)
                })
                .collect::<Vec<_>>();

            CreateComponent::ActionRow(CreateActionRow::buttons(row))
        })
        .collect::<Vec<_>>();

    Ok(CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components))
}

fn check_win<Db: Database, Manager: GameManager<Db>>(
    state: &GameState<Db, Manager>,
    components: &[Component],
    target: ReactionType,
) -> bool {
    let get_emoji = |r: usize, c: usize| -> Option<&ReactionType> {
        let Some(Component::ActionRow(action_row)) = components.get(r) else {
            unreachable!("Component must be an action row")
        };

        match action_row.components.get(c) {
            Some(ActionRowComponent::Button(b)) => b.emoji.as_ref(),
            _ => unreachable!("Component must be a button"),
        }
    };

    let target = Some(target);

    // Check rows
    for r in 0..3 {
        if (0..state.size)
            .map(|c| get_emoji(r, c))
            .all(|emoji| emoji == target.as_ref())
        {
            return true;
        }
    }

    // Check columns
    for c in 0..3 {
        if (0..state.size)
            .map(|r| get_emoji(r, c))
            .all(|emoji| emoji == target.as_ref())
        {
            return true;
        }
    }

    // Check diagonals
    if (0..state.size)
        .map(|i| get_emoji(i, i))
        .all(|emoji| emoji == target.as_ref())
    {
        return true;
    }

    if (0..state.size)
        .map(|row| get_emoji(row, state.size - 1 - row)) // Get element at (row, n-1-row)
        .all(|emoji| emoji == target.as_ref())
    {
        return true;
    }

    // No win condition met
    false
}

fn check_draw(components: &[Component]) -> bool {
    let x_emoji = Some(ReactionType::from(EMOJI_P1));
    let o_emoji = Some(ReactionType::from(EMOJI_P2));

    components
        .iter()
        .flat_map(|component| match component {
            Component::ActionRow(action_row) => action_row.components.iter(),
            _ => unreachable!("Component must be an action row"),
        })
        .map(|component| match component {
            ActionRowComponent::Button(button) => button,
            _ => unreachable!("Component must be a button"),
        })
        .all(|button| button.emoji == x_emoji || button.emoji == o_emoji)
}
