use rand::rng;
use rand::seq::IndexedRandom;
use regex::Regex;
use serenity::all::{
    ActionRowComponent,
    ButtonStyle,
    Component,
    ComponentInteraction,
    Context,
    CreateActionRow,
    CreateButton,
    CreateComponent,
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EditInteractionResponse,
    Http,
    Mentionable,
    MessageInteractionMetadata,
    ReactionType,
    UserId,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData};

use crate::games::tiktactoe::{EMOJI_P1, EMOJI_P2};
use crate::{Coins, EffectsManager, GamblingManager, GameManager, GameRow, Result};

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
            return Ok(());
        };

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        match interaction.data.custom_id.as_str() {
            "ttt_cancel" if metadata.user == interaction.user => {
                cancel(&ctx.http, interaction).await?;
            },
            "ttt_accept" => {
                accept::<Db, GamblingHandler, EffectsHandler, GameHandler>(
                    &ctx.http,
                    interaction,
                    pool,
                    &emojis,
                )
                .await?;
            },
            _ => {},
        }

        Ok(())

        // if interaction.user.id != state.current_turn {
        //     return Ok(true);
        // }

        // let mut pos = custom_id.strip_prefix("ttt_").unwrap().chars();
        // let i = pos.next().unwrap().to_digit(10).unwrap() as usize;
        // let j = pos.next().unwrap().to_digit(10).unwrap() as usize;

        // let mut components = interaction.message.components.clone();

        // let Component::ActionRow(action_row) = components.get_mut(i).unwrap()
        // else {     unreachable!("Component must be an action row")
        // };

        // let ActionRowComponent::Button(button) =
        // action_row.components.get_mut(j).unwrap() else {
        //     unreachable!("Component must be a button")
        // };

        // if button.emoji == Some(EMOJI_P1.into()) || button.emoji ==
        // Some(EMOJI_P2.into()) {     return Ok(true);
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
        //     .create_response(&ctx.http,
        // CreateInteractionResponse::UpdateMessage(msg))     .await
        //     .unwrap();

        // Ok(true)
    }

    async fn p1_row<Db: Database, Manager: GameManager<Db>>(
        &self,
        pool: &Pool<Db>,
    ) -> GameRow {
        let id = self.players[0];

        Manager::row(pool, id)
            .await
            .expect("async call")
            .unwrap_or_else(|| GameRow::new(id))
    }

    async fn p2_row<Db: Database, Manager: GameManager<Db>>(
        &self,
        pool: &Pool<Db>,
    ) -> GameRow {
        let id = self.players[1];

        Manager::row(pool, id)
            .await
            .expect("async call")
            .unwrap_or_else(|| GameRow::new(id))
    }
}

impl From<&ComponentInteraction> for TicTacToe {
    fn from(value: &ComponentInteraction) -> Self {
        let Some(MessageInteractionMetadata::Command(metadata)) =
            value.message.interaction_metadata.as_deref()
        else {
            // Return a default/stub struct
            return Self {
                size: 0,
                players: [value.user.id, value.user.id],
                current_turn: value.user.id,
                bet: 0,
            };
        };

        let players = [metadata.user.id, value.user.id];
        let current_turn =
            *players.choose(&mut rng()).expect("players slice is non-empty");

        let embed =
            value.message.embeds.first().expect("ttt message always has an embed");
        let re = Regex::new(r"for \*\*(\d+)\*\*").expect("valid static regex");

        let bet = re
            .captures(
                embed
                    .description
                    .as_ref()
                    .expect("ttt challenge embed always has description"),
            )
            .and_then(|caps| caps.get(1))
            .and_then(|matched| matched.as_str().parse::<i64>().ok())
            .expect("bet always present in ttt embed description");

        Self {
            size: value.message.components.len() as usize,
            players,
            current_turn,
            bet,
        }
    }
}

async fn cancel(http: &Http, interaction: &ComponentInteraction) -> Result<()> {
    let embed = CreateEmbed::new().title("TicTacToe").description("Game cancelled");

    let msg =
        CreateInteractionResponseMessage::new().embed(embed).components(Vec::new());

    interaction
        .create_response(http, CreateInteractionResponse::UpdateMessage(msg))
        .await?;

    Ok(())
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
    interaction.defer(http).await?;

    let mut state = TicTacToe::from(interaction);

    state.players[1] = interaction.user.id;

    let mut p1_row = state.p1_row::<Db, GameHandler>(pool).await;
    let mut p2_row = state.p2_row::<Db, GameHandler>(pool).await;

    EffectsHandler::bet_limit::<GamblingHandler>(
        pool,
        p1_row.user_id.cast_unsigned(),
        state.bet,
        p1_row.coins(),
    )
    .await?;
    EffectsHandler::bet_limit::<GamblingHandler>(
        pool,
        p2_row.user_id.cast_unsigned(),
        state.bet,
        p2_row.coins(),
    )
    .await?;

    state.current_turn =
        *state.players.choose(&mut rng()).expect("players slice is non-empty");

    p1_row.add_coins(-state.bet);
    p2_row.add_coins(-state.bet);

    GameHandler::save(pool, p1_row).await?;
    GameHandler::save(pool, p2_row).await?;

    let blank = emojis.emoji("blank").expect("blank emoji always registered");

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
            EditInteractionResponse::new().embed(embed).components(components),
        )
        .await?;

    Ok(())
}

#[expect(dead_code, reason = "used for future TicTacToe win/draw detection")]
fn check_win(
    state: &TicTacToe,
    components: &[Component],
    target: ReactionType,
) -> bool {
    let get_emoji = |r: usize, c: usize| -> Option<&ReactionType> {
        let Component::ActionRow(action_row) = components.get(r)? else {
            return None;
        };

        match action_row.components.get(c) {
            Some(ActionRowComponent::Button(b)) => b.emoji.as_ref(),
            _ => None,
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
    if (0..state.size).map(|i| get_emoji(i, i)).all(|emoji| emoji == target.as_ref())
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

#[expect(dead_code, reason = "used for future TicTacToe win/draw detection")]
fn check_draw(components: &[Component]) -> bool {
    let x_emoji = Some(ReactionType::from(EMOJI_P1));
    let o_emoji = Some(ReactionType::from(EMOJI_P2));

    components
        .iter()
        .filter_map(|component| match component {
            Component::ActionRow(action_row) => Some(action_row.components.iter()),
            Component::Section(_)
            | Component::TextDisplay(_)
            | Component::MediaGallery(_)
            | Component::File(_)
            | Component::Separator(_)
            | Component::Container(_)
            | Component::Label(_)
            | Component::Unknown(_)
            | _ => None,
        })
        .flatten()
        .filter_map(|component| match component {
            ActionRowComponent::Button(button) => Some(button),
            ActionRowComponent::SelectMenu(_) | _ => None,
        })
        .all(|button| button.emoji == x_emoji || button.emoji == o_emoji)
}
