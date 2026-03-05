use std::sync::Arc;

use rand::{rng, seq::IndexedRandom};
use regex::Regex;
use serenity::all::{
    ActionRowComponent, ButtonStyle, Colour, Component, ComponentInteraction, Context,
    CreateActionRow, CreateButton, CreateComponent, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, Http, Mentionable,
    MessageInteractionMetadata, ReactionType, UserId,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData};

use crate::{
    Coins, EffectsManager, GamblingManager, GameManager, GameRow, Result,
    events::Dispatch,
    games::tiktactoe::{EMOJI_P1, EMOJI_P2},
};

pub struct TicTacToe {
    size: usize,
    players: [UserId; 2],
    current_turn: UserId,
    bet: i64,
}

impl TicTacToe {
    pub async fn run_component<
        Data: EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let Some(MessageInteractionMetadata::Command(metadata)) =
            interaction.message.interaction_metadata.as_deref()
        else {
            unreachable!("Message must be created from an command")
        };

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        match interaction.data.custom_id.as_str() {
            "ttt_cancel" if metadata.user == interaction.user => {
                cancel(&ctx.http, interaction).await;
            }
            "ttt_accept" => {
                accept::<Db, GamblingHandler, EffectsHandler, GameHandler>(
                    &ctx.http,
                    interaction,
                    pool,
                    &emojis,
                )
                .await?;
            }
            _ => {}
        }

        Ok(())

        // if interaction.user.id != state.current_turn {
        //     return Ok(true);
        // }

        // let mut pos = custom_id.strip_prefix("ttt_").unwrap().chars();
        // let i = pos.next().unwrap().to_digit(10).unwrap() as usize;
        // let j = pos.next().unwrap().to_digit(10).unwrap() as usize;

        // let mut components = interaction.message.components.clone();

        // let Component::ActionRow(action_row) = components.get_mut(i).unwrap() else {
        //     unreachable!("Component must be an action row")
        // };

        // let ActionRowComponent::Button(button) = action_row.components.get_mut(j).unwrap() else {
        //     unreachable!("Component must be a button")
        // };

        // if button.emoji == Some(EMOJI_P1.into()) || button.emoji == Some(EMOJI_P2.into()) {
        //     return Ok(true);
        // }

        // let emoji = if state.current_turn == state.players[0] {
        //     ReactionType::from(EMOJI_P1)
        // } else {
        //     ReactionType::from(EMOJI_P2)
        // };

        // button.emoji = Some(emoji.clone());

        // if check_win(&state, &components, emoji) {
        //     let winner = Some(state.current_turn);
        //     return Ok(false);
        // } else if check_draw(&components) {
        //     return Ok(false);
        // }

        // let components = components
        //     .into_iter()
        //     .map(|component| {
        //         let Component::ActionRow(row) = component else {
        //             unreachable!("Component must be an action row")
        //         };

        //         let buttons = row
        //             .components
        //             .into_iter()
        //             .map(|c| {
        //                 let ActionRowComponent::Button(b) = c else {
        //                     unreachable!("Component must be of type Button")
        //                 };

        //                 b.into()
        //             })
        //             .collect::<Vec<CreateButton>>();

        //         CreateComponent::ActionRow(CreateActionRow::buttons(buttons))
        //     })
        //     .collect::<Vec<_>>();

        // // Next player
        // state.current_turn = if state.current_turn == state.players[0] {
        //     state.players[1]
        // } else {
        //     state.players[0]
        // };

        // let embed = CreateEmbed::new()
        //     .title("TicTacToe")
        //     .description(format!("{}'s Turn", state.current_turn.mention()));

        // let msg = CreateInteractionResponseMessage::new()
        //     .embed(embed)
        //     .components(components);

        // interaction
        //     .create_response(&ctx.http, CreateInteractionResponse::UpdateMessage(msg))
        //     .await
        //     .unwrap();

        // Ok(true)
    }

    async fn p1_row<Db: Database, Manager: GameManager<Db>>(&self, pool: &Pool<Db>) -> GameRow {
        let id = self.players[0];

        Manager::row(pool, id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(id))
    }

    async fn p2_row<Db: Database, Manager: GameManager<Db>>(&self, pool: &Pool<Db>) -> GameRow {
        let id = self.players[0];

        Manager::row(pool, id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(id))
    }
}

impl From<&ComponentInteraction> for TicTacToe {
    fn from(value: &ComponentInteraction) -> Self {
        let Some(MessageInteractionMetadata::Command(metadata)) =
            value.message.interaction_metadata.as_deref()
        else {
            unreachable!("Message must be created from an command")
        };

        let players = [metadata.user.id, value.user.id];
        let current_turn = *players.choose(&mut rng()).unwrap();

        let embed = &value.message.embeds[0];
        let re = Regex::new(r#"for \*\*(\d+)\*\*"#).unwrap();

        let bet = re
            .captures(embed.description.as_ref().unwrap())
            .and_then(|caps| caps.get(1))
            .and_then(|matched| matched.as_str().parse::<i64>().ok())
            .unwrap();

        Self {
            size: value.message.components.len() as usize,
            players,
            current_turn,
            bet,
        }
    }
}

async fn cancel(http: &Http, interaction: &ComponentInteraction) {
    let embed = CreateEmbed::new()
        .title("TicTacToe")
        .description("Game cancelled");

    let msg = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(Vec::new());

    interaction
        .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
        .await
        .unwrap();
}

async fn accept<
    Db: Database,
    GamblingHandler: GamblingManager<Db>,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    http: &Http,
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
    emojis: &EmojiCache,
) -> Result<()> {
    interaction.defer(http).await.unwrap();

    let mut state = TicTacToe::from(interaction);

    state.players[1] = interaction.user.id;

    let mut p1_row = state.p1_row::<Db, GameHandler>(pool).await;
    let mut p2_row = state.p2_row::<Db, GameHandler>(pool).await;

    EffectsHandler::bet_limit::<GamblingHandler>(pool, p1_row.id as u64, state.bet, p1_row.coins())
        .await?;
    EffectsHandler::bet_limit::<GamblingHandler>(pool, p2_row.id as u64, state.bet, p2_row.coins())
        .await?;

    state.current_turn = *state.players.choose(&mut rng()).unwrap();

    p1_row.add_coins(-state.bet);
    p2_row.add_coins(-state.bet);

    GameHandler::save(pool, p1_row).await.unwrap();
    GameHandler::save(pool, p2_row).await.unwrap();

    let blank = emojis.emoji("blank").unwrap();

    let embed = CreateEmbed::new()
        .title("TicTacToe")
        .description(format!("{}'s Turn", state.current_turn.mention()));

    let components = (0..state.size)
        .map(|i| {
            let row = (0..state.size)
                .map(|j| {
                    CreateButton::new(format!("ttt_{i}{j}"))
                        .emoji(blank)
                        .style(ButtonStyle::Secondary)
                })
                .collect::<Vec<_>>();

            CreateComponent::ActionRow(CreateActionRow::buttons(row))
        })
        .collect::<Vec<_>>();

    interaction
        .edit_response(
            http,
            EditInteractionResponse::new()
                .embed(embed)
                .components(components),
        )
        .await
        .unwrap();

    Ok(())
}

fn check_win(state: &TicTacToe, components: &[Component], target: ReactionType) -> bool {
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

async fn after_play<Data: EmojiCacheData, Db: Database, Manager: GameManager<Db>>(
    http: &Http,
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
    data_lock: Arc<RwLock<Data>>,
    state: &TicTacToe,
    winner: Option<UserId>,
) {
    let mut p1_row = state.p1_row::<Db, Manager>(pool).await;
    let mut p2_row = state.p2_row::<Db, Manager>(pool).await;

    let [p1, p2] = state.players;

    let embed = if let Some(winner) = winner {
        let row = if p1 == winner {
            &mut p1_row
        } else {
            &mut p2_row
        };

        row.add_coins(state.bet * 2);

        CreateEmbed::new()
            .title("TicTacToe")
            .description(format!("Winner! {} 🎉", winner.mention()))
            .colour(Colour::DARK_GREEN)
    } else if p1 != p2 {
        p1_row.add_coins(state.bet);
        p2_row.add_coins(state.bet);

        CreateEmbed::new()
            .title("TicTacToe")
            .description("It's a draw!")
            .colour(Colour::ORANGE)
    } else {
        p1_row.add_coins(state.bet);

        CreateEmbed::new()
            .title("TicTacToe")
            .description("This game timed out after 2 minutes of inactivity")
            .colour(Colour::TEAL)
    };

    todo!()

    // {
    //     let data = data_lock.read().await;
    //     let emojis = data.emojis();
    //     let dispatch = Dispatch::<Db, GoalHandler>::new(http, pool, emojis);

    //     dispatch
    //         .fire(
    //             interaction.channel_id,
    //             &mut p1_row,
    //             Event::Game(GameEvent::new("rps", p1, state.bet, state.bet, false)), // TODO: Fix win logic
    //         )
    //         .await?;

    //     dispatch
    //         .fire(
    //             interaction.channel_id,
    //             &mut p2_row,
    //             Event::Game(GameEvent::new("rps", p2, state.bet, state.bet, false)), // TODO: Fix win logic
    //         )
    //         .await?;
    // };

    // GameHandler::save(pool, p1_row).await?;
    // GameHandler::save(pool, p2_row).await?;

    // GameCache::update(Arc::clone(&data_lock), p1).await;
    // GameCache::update(data_lock, p2).await;

    // interaction
    //     .edit_response(
    //         http,
    //         EditInteractionResponse::new()
    //             .embed(embed)
    //             .components(Vec::new()),
    //     )
    //     .await
    //     .unwrap();
}
