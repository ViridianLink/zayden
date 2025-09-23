use serenity::all::{
    Colour, Component, ComponentInteraction, Context, CreateActionRow, CreateComponent,
    CreateContainer, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    CreateTextDisplay, MessageFlags, TextDisplay,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData, FormatNum};

use crate::events::{Dispatch, Event, GameEvent};
use crate::games::blackjack::{
    CARD_VALUES, GameDetails, card_values, double_button, hit_button, in_play_text, split_button,
    stand_button, sum_cards, surrender_button,
};
use crate::{
    Coins, EffectsManager, GamblingData, GamblingManager, GameCache, GameManager, GameRow,
    GoalsManager, Result,
};

pub struct Blackjack;

impl Blackjack {
    pub async fn hit<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let text = text(interaction);

        let mut game = GameDetails::from_str(&emojis, &text.content);

        game.add_card();

        if game.player_value(&emojis) > 21 {
            game_end::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
                ctx,
                interaction,
                pool,
                &emojis,
                game,
            )
            .await;

            return Ok(());
        }

        let text = in_play_text(
            &emojis,
            game.bet(),
            game.player_hand(),
            game.dealer_hand()[0],
        );

        let action_row = CreateComponent::ActionRow(CreateActionRow::buttons(vec![
            hit_button(),
            stand_button(),
            split_button().disabled(true),
            double_button().disabled(true),
            surrender_button().disabled(true),
        ]));

        let container = CreateComponent::Container(
            CreateContainer::new(vec![text, action_row]).accent_colour(Colour::TEAL),
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
            .await
            .unwrap();

        Ok(())
    }

    pub async fn stand<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let text = text(interaction);

        game_end::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
            ctx,
            interaction,
            pool,
            &emojis,
            GameDetails::from_str(&emojis, &text.content),
        )
        .await;

        Ok(())
    }

    pub async fn double<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let text = text(interaction);

        let mut game = GameDetails::from_str(&emojis, &text.content);

        GamblingHandler::bet(pool, interaction.user.id, game.bet())
            .await
            .unwrap();

        game.double_bet();
        game.add_card();

        game_end::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
            ctx,
            interaction,
            pool,
            &emojis,
            game,
        )
        .await;

        Ok(())
    }

    pub async fn split<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        todo!()
    }

    pub async fn surrender<
        Data: GamblingData + EmojiCacheData,
        Db: Database,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let text = text(interaction);

        let mut game = GameDetails::from_str(&emojis, &text.content);

        let player_value = game.player_value(&emojis);

        let mut row = GameHandler::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| GameRow::new(interaction.user.id));

        let dispatch = Dispatch::<Db, GoalsHandler>::new(&ctx.http, pool, &emojis);

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
            .await
            .unwrap();

        payout = EffectsHandler::payout(pool, interaction.user.id, game.bet(), payout, Some(false))
            .await;

        row.add_coins(payout);

        let coins = row.coins();

        GameHandler::save(pool, row).await.unwrap();
        GameCache::update(ctx.data::<RwLock<Data>>(), interaction.user.id).await;

        let card_to_num = CARD_VALUES.get_or_init(|| card_values(&emojis));
        let coin = emojis.emoji("heads").unwrap();

        let mut dealer_hand = game.dealer_hand();
        dealer_hand.push(game.next_card());
        let dealer_value = sum_cards(&emojis, &dealer_hand);

        let desc = format!(
            "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{} - {dealer_value}",
            game.bet().format(),
            game.player_hand_str(&emojis),
            dealer_hand
                .iter()
                .map(|id| {
                    let num = card_to_num.get(id).unwrap();
                    format!("<:{num}:{id}> ")
                })
                .collect::<String>()
        );

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .flags(MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![CreateComponent::Container(CreateContainer::new(
                            vec![CreateComponent::TextDisplay(CreateTextDisplay::new(
                                format!(
                                    "### Blackjack - Surrender!\n{desc}\n\nYou surrender!\n\nLost: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>",
                                    (payout - game.bet()).format(),
                                    coins.format()
                                ),
                            ))],
                        ).accent_colour(Colour::RED))])
                ),
            )
            .await
            .unwrap();

        Ok(())
    }
}

fn text(interaction: &ComponentInteraction) -> &TextDisplay {
    let Some(Component::Container(container)) = interaction.message.as_ref().components.first()
    else {
        unreachable!("Message must have a container component")
    };

    let Some(Component::TextDisplay(text)) = container.components.first() else {
        unreachable!("First component must be text")
    };

    text
}

async fn game_end<
    Data: GamblingData,
    Db: Database,
    GoalsHandler: GoalsManager<Db>,
    EffectsHandler: EffectsManager<Db>,
    GameHandler: GameManager<Db>,
>(
    ctx: &Context,
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
    emojis: &EmojiCache,
    mut game: GameDetails,
) {
    let player_value = game.player_value(emojis);

    let mut row = GameHandler::row(pool, interaction.user.id)
        .await
        .unwrap()
        .unwrap_or_else(|| GameRow::new(interaction.user.id));

    let dispatch = Dispatch::<Db, GoalsHandler>::new(&ctx.http, pool, emojis);

    if player_value > 21 {
        let desc = bust::<Db, GoalsHandler, EffectsHandler, GameHandler>(
            interaction,
            pool,
            emojis,
            game,
            player_value,
            row,
            dispatch,
        )
        .await;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().components(vec![
                        CreateComponent::Container(
                            CreateContainer::new(vec![CreateComponent::TextDisplay(
                                CreateTextDisplay::new(format!(
                                    "### Blackjack - You Lost!\n{desc}"
                                )),
                            )])
                            .accent_colour(Colour::RED),
                        ),
                    ]),
                ),
            )
            .await
            .unwrap();

        return;
    }

    let mut dealer_hand = game.dealer_hand();
    let mut dealer_value = sum_cards(emojis, &dealer_hand);

    while dealer_value < 17 {
        dealer_hand.push(game.next_card());
        dealer_value = sum_cards(emojis, &dealer_hand);
    }

    let (win, mut payout) = if dealer_value > 21 || player_value > dealer_value {
        (Some(true), game.bet() * 2)
    } else if player_value == dealer_value {
        (None, game.bet())
    } else {
        (Some(false), 0)
    };

    dispatch
        .fire(
            interaction.channel_id,
            &mut row,
            Event::Game(GameEvent::new(
                "blackjack",
                interaction.user.id,
                game.bet(),
                payout,
                win == Some(true),
            )),
        )
        .await
        .unwrap();

    payout = EffectsHandler::payout(pool, interaction.user.id, game.bet(), payout, win).await;

    row.add_coins(payout);

    let coins = row.coins();

    GameHandler::save(pool, row).await.unwrap();
    GameCache::update(ctx.data::<RwLock<Data>>(), interaction.user.id).await;

    let card_to_num = CARD_VALUES.get_or_init(|| card_values(emojis));
    let coin = emojis.emoji("heads").unwrap();

    let desc = format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{} - {dealer_value}",
        game.bet().format(),
        game.player_hand_str(emojis),
        dealer_hand
            .iter()
            .map(|id| {
                let num = card_to_num.get(id).unwrap();
                format!("<:{num}:{id}> ")
            })
            .collect::<String>()
    );

    let container = if win == Some(true) {
        CreateContainer::new(vec![CreateComponent::TextDisplay(CreateTextDisplay::new(
            format!("### Blackjack - You Won!\n{desc}\n\nProfit: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>", (payout - game.bet()).format(), coins.format()),
        ))])
        .accent_colour(Colour::DARK_GREEN)
    } else if win == Some(false) {
        CreateContainer::new(vec![CreateComponent::TextDisplay(CreateTextDisplay::new(
            format!("### Blackjack - You Lost!\n{desc}\n\nDealer wins!\n\nLost: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>", (payout - game.bet()).format(), coins.format()),
        ))])
        .accent_colour(Colour::RED)
    } else {
        CreateContainer::new(vec![CreateComponent::TextDisplay(CreateTextDisplay::new(
            format!("### Blackjack - Draw!\n{desc}\n\nDraw! Have your money back.\n\nYour coins: {} <:coin:{coin}>", coins.format()),
        ))])
        .accent_colour(Colour::DARKER_GREY)
    };

    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .flags(MessageFlags::IS_COMPONENTS_V2)
                    .components(vec![CreateComponent::Container(container)]),
            ),
        )
        .await
        .unwrap();
}

async fn bust<
    Db: Database,
    GoalsHandler: GoalsManager<Db>,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
    emojis: &EmojiCache,
    mut game: GameDetails,
    player_value: u8,
    mut row: GameRow,
    dispatch: Dispatch<'_, Db, GoalsHandler>,
) -> String {
    dispatch
        .fire(
            interaction.channel_id,
            &mut row,
            Event::Game(GameEvent::new(
                "blackjack",
                interaction.user.id,
                game.bet(),
                0,
                false,
            )),
        )
        .await
        .unwrap();

    let payout =
        EffectsHandler::payout(pool, interaction.user.id, game.bet(), 0, Some(false)).await;

    row.add_coins(payout);

    let coins = row.coins();

    GameHandler::save(pool, row).await.unwrap();

    let mut dealer_hand = game.dealer_hand();
    dealer_hand.push(game.next_card());

    let dealer_value = sum_cards(emojis, &dealer_hand);

    let dealer_hand_str = dealer_hand
        .iter()
        .map(|id| {
            let num = *CARD_VALUES
                .get_or_init(|| card_values(emojis))
                .get(id)
                .unwrap();

            format!("<:{num}:{id}> ")
        })
        .collect::<String>();

    let coin = emojis.emoji("heads").unwrap();

    format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{dealer_hand_str} - {dealer_value}\n\nBust!\n\nLost: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>",
        game.bet().format(),
        game.player_hand_str(emojis),
        (payout - game.bet()).format(),
        coins.format()
    )
}
