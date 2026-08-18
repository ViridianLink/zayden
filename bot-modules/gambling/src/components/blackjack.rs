use std::cmp::Ordering;
use std::fmt::Write as _;

use serenity::all::{
    Colour,
    Component,
    ComponentInteraction,
    ContainerComponent,
    Context,
    CreateActionRow,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    EmojiId,
    MessageFlags,
};
use sqlx::PgPool;
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData, FormatNum};

use crate::events::{Dispatch, Event, GameEvent};
use crate::games::blackjack::{
    GameDetails,
    HandOutcome,
    SettledHand,
    card_values,
    double_button,
    final_board,
    hit_button,
    in_play_board,
    split_button,
    stand_button,
    sum_cards,
    surrender_button,
};
use crate::utils::effects_summary;
use crate::{
    Coins,
    EffectsManager,
    GamblingData,
    GamblingError,
    GamblingManager,
    GameDelta,
    GameRow,
    Result,
    ShopCurrency,
};

pub struct Blackjack;

impl Blackjack {
    pub async fn hit<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut game = GameDetails::from_components(&emojis, board(interaction))?;

        game.add_card()?;

        if game.player_value(&emojis)? > 21 && !game.advance_hand() {
            game_end(ctx, interaction, pool, &emojis, game).await?;

            return Ok(());
        }

        continue_round(ctx, interaction, &emojis, &game).await
    }

    pub async fn stand<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut game = GameDetails::from_components(&emojis, board(interaction))?;

        if game.advance_hand() {
            return continue_round(ctx, interaction, &emojis, &game).await;
        }

        game_end(ctx, interaction, pool, &emojis, game).await?;

        Ok(())
    }

    pub async fn double<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut game = GameDetails::from_components(&emojis, board(interaction))?;

        if !GamblingManager::bet(pool, interaction.user.id, game.bet()).await? {
            return Err(GamblingError::InsufficientFunds {
                required: game.bet(),
                currency: ShopCurrency::Coins,
            });
        }

        game.double_bet();
        game.add_card()?;

        game_end(ctx, interaction, pool, &emojis, game).await?;

        Ok(())
    }

    pub async fn split<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut game = GameDetails::from_components(&emojis, board(interaction))?;

        if !game.can_split(&emojis)? {
            return Err(GamblingError::Internal(
                "hand cannot be split - component state is stale".to_string(),
            ));
        }

        if !GamblingManager::bet(pool, interaction.user.id, game.bet()).await? {
            return Err(GamblingError::InsufficientFunds {
                required: game.bet(),
                currency: ShopCurrency::Coins,
            });
        }

        game.split()?;

        continue_round(ctx, interaction, &emojis, &game).await
    }

    pub async fn surrender<Data: GamblingData + EmojiCacheData>(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let mut game = GameDetails::from_components(&emojis, board(interaction))?;

        let player_value = game.player_value(&emojis)?;

        let mut row = GameRow::get(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let before = row.clone();

        let dispatch = Dispatch::new(&ctx.http, pool, &emojis);

        let mut payout = game.bet() / 2;

        dispatch
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Game(GameEvent::new(
                    "blackjack",
                    interaction.user.id,
                    game.bet(),
                    payout,
                    false,
                )),
            )
            .await?;

        let payout_result = EffectsManager::payout(
            pool,
            interaction.user.id,
            "blackjack",
            game.bet(),
            payout,
            Some(false),
        )
        .await;
        payout = payout_result.payout;

        row.add_coins(payout);

        let delta = GameDelta::between(&before, &row);

        let coins = GameRow::commit(pool, interaction.user.id, &delta)
            .await?
            .ok_or(GamblingError::TransactionConflict)?
            .coins;

        let coin = emojis.emoji("heads").map_err(|n| {
            GamblingError::Internal(format!("emoji '{n}' not in cache"))
        })?;

        let mut dealer_hand = vec![game.dealer_card()];
        dealer_hand.push(game.next_card()?);
        let dealer_value = sum_cards(&emojis, &dealer_hand)?;

        let board = final_board(
            "Surrender!",
            &format!("Your bet: {} <:coin:{coin}>", game.bet().format()),
            &[SettledHand {
                cards: game.player_hand_str(&emojis)?,
                value: player_value,
                outcome: HandOutcome::Lost,
            }],
            (&hand_str(&emojis, &dealer_hand)?, dealer_value),
            &format!(
                "You surrender!\n\nLost: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>{}",
                (payout - game.bet()).format(),
                coins.format(),
                effects_summary(&emojis, &payout_result.effects),
            ),
        );

        update(ctx, interaction, board, Colour::RED).await
    }
}

fn hand_str(emojis: &EmojiCache, hand: &[EmojiId]) -> Result<String> {
    let card_to_num = card_values(emojis)?;

    let mut s = String::new();
    for id in hand {
        let num = card_to_num.get(id).ok_or_else(|| {
            GamblingError::Internal("card ID not in CARD_VALUES".to_string())
        })?;
        let _ = write!(s, "<:{num}:{id}> ");
    }

    Ok(s)
}

async fn update(
    ctx: &Context,
    interaction: &ComponentInteraction,
    components: Vec<CreateContainerComponent<'_>>,
    colour: Colour,
) -> Result<()> {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .flags(MessageFlags::IS_COMPONENTS_V2)
                    .components(vec![CreateComponent::Container(
                        CreateContainer::new(components).accent_colour(colour),
                    )]),
            ),
        )
        .await?;

    Ok(())
}

fn board(interaction: &ComponentInteraction) -> &[ContainerComponent] {
    let Some(Component::Container(container)) =
        interaction.message.as_ref().components.first()
    else {
        return &[];
    };

    &container.components
}

async fn continue_round(
    ctx: &Context,
    interaction: &ComponentInteraction,
    emojis: &EmojiCache,
    game: &GameDetails,
) -> Result<()> {
    let first_action = game.player_hand().len() == 2;

    let action_row =
        CreateContainerComponent::ActionRow(CreateActionRow::buttons(vec![
            hit_button(),
            stand_button(),
            split_button().disabled(!game.can_split(emojis)?),
            double_button().disabled(!first_action),
            surrender_button().disabled(!first_action || game.is_split()),
        ]));

    let mut components = in_play_board(emojis, game)?;
    components.push(action_row);

    let container = CreateComponent::Container(
        CreateContainer::new(components).accent_colour(Colour::TEAL),
    );

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .flags(MessageFlags::IS_COMPONENTS_V2)
                    .components(vec![container]),
            ),
        )
        .await?;

    Ok(())
}

async fn game_end(
    ctx: &Context,
    interaction: &ComponentInteraction,
    pool: &PgPool,
    emojis: &EmojiCache,
    mut game: GameDetails,
) -> Result<()> {
    let hand_values = (0..game.hands().len())
        .map(|index| game.hand_value(emojis, index))
        .collect::<Result<Vec<_>>>()?;

    let all_bust = hand_values.iter().all(|&value| value > 21);

    let mut row = GameRow::get(pool, interaction.user.id)
        .await?
        .unwrap_or_else(|| GameRow::new(interaction.user.id));

    let before = row.clone();

    let dispatch = Dispatch::new(&ctx.http, pool, emojis);

    let mut dealer_hand = vec![game.dealer_card()];
    if all_bust {
        dealer_hand.push(game.next_card()?);
    } else {
        while sum_cards(emojis, &dealer_hand)? < 17 {
            dealer_hand.push(game.next_card()?);
        }
    }

    let dealer_value = sum_cards(emojis, &dealer_hand)?;

    let bet = game.bet();
    let total_bet = game.total_bet();

    let outcomes = hand_values
        .iter()
        .map(|&value| HandOutcome::settle(value, dealer_value))
        .collect::<Vec<_>>();

    let mut payout = outcomes.iter().map(|outcome| outcome.payout(bet)).sum::<i64>();

    let win = match payout.cmp(&total_bet) {
        Ordering::Greater => Some(true),
        Ordering::Equal => None,
        Ordering::Less => Some(false),
    };

    dispatch
        .fire(
            interaction.channel_id,
            &mut row,
            Event::Game(GameEvent::new(
                "blackjack",
                interaction.user.id,
                total_bet,
                payout,
                win == Some(true),
            )),
        )
        .await?;

    let payout_result = EffectsManager::payout(
        pool,
        interaction.user.id,
        "blackjack",
        total_bet,
        payout,
        win,
    )
    .await;
    payout = payout_result.payout;

    row.add_coins(payout);

    let delta = GameDelta::between(&before, &row);

    let coins = GameRow::commit(pool, interaction.user.id, &delta)
        .await?
        .ok_or(GamblingError::TransactionConflict)?
        .coins;

    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let hands = outcomes
        .iter()
        .enumerate()
        .map(|(index, &outcome)| {
            Ok(SettledHand {
                cards: game.hand_str(emojis, index)?,
                value: hand_values.get(index).copied().unwrap_or_default(),
                outcome,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let (title, summary, colour) = match win {
        Some(true) => (
            "You Won!",
            format!(
                "Profit: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>{}",
                (payout - total_bet).format(),
                coins.format(),
                effects_summary(emojis, &payout_result.effects)
            ),
            Colour::DARK_GREEN,
        ),
        Some(false) => (
            "You Lost!",
            format!(
                "Dealer wins!\n\nLost: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>{}",
                (payout - total_bet).format(),
                coins.format(),
                effects_summary(emojis, &payout_result.effects)
            ),
            Colour::RED,
        ),
        None => (
            "Draw!",
            format!(
                "Draw! Have your money back.\n\nYour coins: {} <:coin:{coin}>",
                coins.format()
            ),
            Colour::DARKER_GREY,
        ),
    };

    let board = final_board(
        title,
        &format!("Your bet: {} <:coin:{coin}>", total_bet.format()),
        &hands,
        (&hand_str(emojis, &dealer_hand)?, dealer_value),
        &summary,
    );

    update(ctx, interaction, board, colour).await
}
