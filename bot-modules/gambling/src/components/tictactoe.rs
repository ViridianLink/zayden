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
    CreateEmbedFooter,
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

type Board = Vec<Vec<Option<ReactionType>>>;

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
            custom_id => {
                let Some(pos_str) = custom_id.strip_prefix("ttt_") else {
                    return Ok(());
                };

                let mut chars = pos_str.chars();
                let Some(i) = chars.next().and_then(|c| c.to_digit(10)) else {
                    return Ok(());
                };
                let Some(j) = chars.next().and_then(|c| c.to_digit(10)) else {
                    return Ok(());
                };

                make_move::<Db, GameHandler>(
                    &ctx.http,
                    interaction,
                    pool,
                    i as usize,
                    j as usize,
                )
                .await?;
            },
        }

        Ok(())
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
        .description(format!("{}'s Turn", state.current_turn.mention()))
        .footer(CreateEmbedFooter::new(format!(
            "{}:{}:{}",
            state.players[0], state.players[1], state.bet
        )));

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

async fn make_move<Db: Database, GameHandler: GameManager<Db>>(
    http: &Http,
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
    i: usize,
    j: usize,
) -> Result<()> {
    let Some((p1, p2, bet)) = parse_footer(interaction) else {
        return Ok(());
    };
    let Some(current_turn) = parse_current_turn(interaction) else {
        return Ok(());
    };

    let players = [p1, p2];
    let size = interaction.message.components.len() as usize;

    if interaction.user.id != current_turn {
        interaction.defer(http).await?;
        return Ok(());
    }

    let x_emoji = ReactionType::from(EMOJI_P1);
    let o_emoji = ReactionType::from(EMOJI_P2);

    let cell_occupied = interaction
        .message
        .components
        .get(i)
        .and_then(|c| {
            if let Component::ActionRow(row) = c {
                row.components.get(j)
            } else {
                None
            }
        })
        .and_then(|c| {
            if let ActionRowComponent::Button(btn) = c {
                btn.emoji.as_ref()
            } else {
                None
            }
        })
        .is_some_and(|e| e == &x_emoji || e == &o_emoji);

    if cell_occupied {
        interaction.defer(http).await?;
        return Ok(());
    }

    let player_emoji = if current_turn == players[0] { x_emoji } else { o_emoji };

    let board =
        build_board(&interaction.message.components, size, i, j, &player_emoji);

    let won = check_win(&board, &player_emoji);
    let draw = !won && check_draw(&board);

    if won {
        let winner = current_turn;
        let mut row = GameHandler::row(pool, winner)
            .await
            .expect("async call")
            .unwrap_or_else(|| GameRow::new(winner));
        row.add_coins(2 * bet);
        GameHandler::save(pool, row).await?;

        let embed = CreateEmbed::new()
            .title("TicTacToe")
            .description(format!("{} wins!", winner.mention()));

        interaction
            .create_response(
                http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(board_to_components(&board, true)),
                ),
            )
            .await?;
    } else if draw {
        for &player in &players {
            let mut row = GameHandler::row(pool, player)
                .await
                .expect("async call")
                .unwrap_or_else(|| GameRow::new(player));
            row.add_coins(bet);
            GameHandler::save(pool, row).await?;
        }

        let embed =
            CreateEmbed::new().title("TicTacToe").description("It's a draw!");

        interaction
            .create_response(
                http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(board_to_components(&board, true)),
                ),
            )
            .await?;
    } else {
        let next_turn =
            if current_turn == players[0] { players[1] } else { players[0] };

        let embed = CreateEmbed::new()
            .title("TicTacToe")
            .description(format!("{}'s Turn", next_turn.mention()))
            .footer(CreateEmbedFooter::new(format!(
                "{}:{}:{}",
                players[0], players[1], bet
            )));

        interaction
            .create_response(
                http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(board_to_components(&board, false)),
                ),
            )
            .await?;
    }

    Ok(())
}

fn parse_footer(
    interaction: &ComponentInteraction,
) -> Option<(UserId, UserId, i64)> {
    let embed = interaction.message.embeds.first()?;
    let footer = embed.footer.as_ref()?;
    let text: &str = &footer.text;
    let mut parts = text.split(':');
    let p1: u64 = parts.next()?.parse().ok()?;
    let p2: u64 = parts.next()?.parse().ok()?;
    let bet: i64 = parts.next()?.parse().ok()?;

    Some((UserId::new(p1), UserId::new(p2), bet))
}

fn parse_current_turn(interaction: &ComponentInteraction) -> Option<UserId> {
    let embed = interaction.message.embeds.first()?;
    let desc: &str = embed.description.as_deref()?;

    let mention = desc.strip_suffix("'s Turn")?.trim();
    let id_str = mention.strip_prefix("<@")?.strip_suffix('>')?;
    let id: u64 = id_str.parse().ok()?;
    Some(UserId::new(id))
}

fn build_board(
    components: &[Component],
    size: usize,
    move_i: usize,
    move_j: usize,
    move_emoji: &ReactionType,
) -> Board {
    (0..size)
        .map(|r| {
            (0..size)
                .map(|c| {
                    if r == move_i && c == move_j {
                        return Some(move_emoji.clone());
                    }
                    components
                        .get(r)
                        .and_then(|comp| {
                            if let Component::ActionRow(row) = comp {
                                row.components.get(c)
                            } else {
                                None
                            }
                        })
                        .and_then(|cell| {
                            if let ActionRowComponent::Button(btn) = cell {
                                btn.emoji.clone()
                            } else {
                                None
                            }
                        })
                })
                .collect()
        })
        .collect()
}

fn board_to_components(board: &Board, disabled: bool) -> Vec<CreateComponent<'_>> {
    board
        .iter()
        .enumerate()
        .map(|(r, row)| {
            let buttons = row
                .iter()
                .enumerate()
                .map(|(c, cell)| {
                    let mut btn = CreateButton::new(format!("ttt_{r}{c}"))
                        .style(ButtonStyle::Secondary)
                        .disabled(disabled);
                    if let Some(emoji) = cell {
                        btn = btn.emoji(emoji.clone());
                    }
                    btn
                })
                .collect::<Vec<_>>();
            CreateComponent::ActionRow(CreateActionRow::buttons(buttons))
        })
        .collect()
}

fn check_win(board: &Board, target: &ReactionType) -> bool {
    let n = board.len();
    let target = Some(target);

    // Rows
    if board.iter().any(|row| row.iter().all(|e| e.as_ref() == target)) {
        return true;
    }

    // Columns
    if (0..n).any(|c| {
        board.iter().all(|row| row.get(c).and_then(Option::as_ref) == target)
    }) {
        return true;
    }

    // Main diagonal
    if board
        .iter()
        .enumerate()
        .all(|(i, row)| row.get(i).and_then(Option::as_ref) == target)
    {
        return true;
    }

    // Anti-diagonal
    if board
        .iter()
        .zip((0..n).rev())
        .all(|(row, c)| row.get(c).and_then(Option::as_ref) == target)
    {
        return true;
    }

    false
}

fn check_draw(board: &Board) -> bool {
    let x_emoji = ReactionType::from(EMOJI_P1);
    let o_emoji = ReactionType::from(EMOJI_P2);

    board
        .iter()
        .flatten()
        .all(|cell| cell.as_ref().is_some_and(|e| e == &x_emoji || e == &o_emoji))
}
