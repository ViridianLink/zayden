use std::{collections::HashSet, str::FromStr};

use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    Colour, ComponentInteraction, Context, CreateActionRow, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, EmojiId, parse_emoji,
};
use sqlx::{Database, Pool};
use zayden_core::FormatNum;

use crate::events::{Dispatch, Event, GameEvent};
use crate::games::blackjack::{
    CARD_TO_NUM, double_button, hit_button, in_play_embed, stand_button, sum_cards,
};
use crate::{
    CARD_DECK, COIN, Coins, EffectsManager, GamblingManager, GameCache, GameManager, GameRow,
    GoalsManager, Result,
};

pub struct Blackjack;

impl Blackjack {
    pub async fn hit<
        Db: Database,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let desc = interaction
            .message
            .as_ref()
            .embeds
            .first()
            .unwrap()
            .description
            .as_deref()
            .unwrap();

        let mut game = desc.parse::<GameDetails>().unwrap();

        game.add_card();

        if game.player_value() > 21 {
            game_end::<Db, GoalsHandler, EffectsHandler, GameHandler>(
                ctx,
                interaction,
                pool,
                desc.parse().unwrap(),
            )
            .await;
        }

        let embed = in_play_embed(game.bet(), game.player_hand(), game.dealer_hand()[0]);

        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(vec![CreateActionRow::Buttons(vec![
                            hit_button(),
                            stand_button(),
                            double_button(0, 0).disabled(true),
                        ])]),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }

    pub async fn stand<
        Db: Database,
        GoalsHandler: GoalsManager<Db>,
        EffectsHandler: EffectsManager<Db> + Send,
        GameHandler: GameManager<Db>,
    >(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let desc = interaction
            .message
            .as_ref()
            .embeds
            .first()
            .unwrap()
            .description
            .as_deref()
            .unwrap();

        game_end::<Db, GoalsHandler, EffectsHandler, GameHandler>(
            ctx,
            interaction,
            pool,
            desc.parse().unwrap(),
        )
        .await;

        Ok(())
    }

    pub async fn double<
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
        let desc = interaction
            .message
            .as_ref()
            .embeds
            .first()
            .unwrap()
            .description
            .as_deref()
            .unwrap();

        let mut game = desc.parse::<GameDetails>().unwrap();

        GamblingHandler::bet(pool, interaction.user.id, game.bet())
            .await
            .unwrap();

        game.double_bet();
        game.add_card();

        game_end::<Db, GoalsHandler, EffectsHandler, GameHandler>(ctx, interaction, pool, game)
            .await;

        Ok(())
    }
}

struct GameDetails {
    bet: i64,
    player_hand: Vec<EmojiId>,
    dealer_card: EmojiId,
    card_shoe: Vec<EmojiId>,
}

impl GameDetails {
    pub fn bet(&self) -> i64 {
        self.bet
    }

    pub fn double_bet(&mut self) {
        self.bet *= 2
    }

    pub fn player_hand(&self) -> &[EmojiId] {
        &self.player_hand
    }

    pub fn player_value(&self) -> u8 {
        sum_cards(&self.player_hand)
    }

    pub fn player_hand_str(&self) -> String {
        self.player_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect()
    }

    pub fn add_card(&mut self) {
        self.player_hand.push(self.card_shoe.pop().unwrap())
    }

    pub fn dealer_hand(&self) -> Vec<EmojiId> {
        vec![self.dealer_card]
    }

    pub fn next_card(&mut self) -> EmojiId {
        self.card_shoe.pop().unwrap()
    }
}

impl FromStr for GameDetails {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut lines = s.lines();

        let bet_line = lines.next().unwrap();
        let bet = bet_line
            .strip_prefix("Your bet: ")
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .replace(',', "")
            .parse::<i64>()
            .unwrap();

        lines.next();
        lines.next();

        let player_hand_line = lines.next().unwrap();
        let player_hand = player_hand_line
            .split(" - ")
            .next()
            .unwrap()
            .split_whitespace()
            .map(parse_emoji)
            .map(|emoji| emoji.map(|emoji| emoji.id))
            .collect::<Option<Vec<EmojiId>>>()
            .unwrap();

        lines.next();
        lines.next();

        let dealer_hand_line = lines.next().unwrap();
        let dealer_card_str = dealer_hand_line.split_whitespace().next().unwrap();
        let dealer_card = parse_emoji(dealer_card_str).unwrap().id;

        let card_shoe = card_shoe(player_hand.iter().copied().chain([dealer_card]).collect());

        Ok(GameDetails {
            bet,
            player_hand,
            dealer_card,
            card_shoe,
        })
    }
}

fn card_shoe(mut cards: HashSet<EmojiId>) -> Vec<EmojiId> {
    let mut shoe = CARD_DECK
        .iter()
        .copied()
        .filter(|card| !cards.remove(card))
        .collect::<Vec<_>>();

    shoe.shuffle(&mut rng());

    shoe
}

async fn game_end<
    Db: Database,
    GoalsHandler: GoalsManager<Db>,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    ctx: &Context,
    interaction: &ComponentInteraction,
    pool: &Pool<Db>,
    mut game: GameDetails,
) {
    let player_value = game.player_value();

    let mut row = GameHandler::row(pool, interaction.user.id)
        .await
        .unwrap()
        .unwrap_or_else(|| GameRow::new(interaction.user.id));

    let dispatch = Dispatch::<Db, GoalsHandler>::new(ctx, pool);

    if player_value > 21 {
        let desc = bust::<Db, GoalsHandler, EffectsHandler, GameHandler>(
            interaction,
            pool,
            game,
            player_value,
            row,
            dispatch,
        )
        .await;

        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .title("Blackjack - You Lost!")
                                .description(desc)
                                .colour(Colour::RED),
                        )
                        .components(Vec::new()),
                ),
            )
            .await
            .unwrap();

        return;
    }

    let mut dealer_hand = game.dealer_hand();
    let mut dealer_value = sum_cards(&dealer_hand);

    while dealer_value < 17 {
        dealer_hand.push(game.next_card());
        dealer_value = sum_cards(&dealer_hand);
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
                win == Some(true),
            )),
        )
        .await
        .unwrap();

    payout = EffectsHandler::payout(pool, interaction.user.id, game.bet(), payout, win).await;

    row.add_coins(payout);

    let coins = row.coins();

    GameHandler::save(pool, row).await.unwrap();
    GameCache::update(ctx, interaction.user.id).await;

    let desc = format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{} - {dealer_value}",
        game.bet().format(),
        game.player_hand_str(),
        dealer_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
    );

    let embed = if win == Some(true) {
        CreateEmbed::new()
            .title("Blackjack - You Won!")
            .description(format!(
                "{desc}\n\nProfit: {} <:coin:{COIN}>\nYour coins: {} <:coin:{COIN}>",
                (payout - game.bet()).format(),
                coins.format()
            ))
            .colour(Colour::DARK_GREEN)
    } else if win == Some(false) {
        CreateEmbed::new()
            .title("Blackjack - You Lost!")
            .description(format!(
                "{desc}\n\nDealer wins!\n\nLost: {} <:coin:{COIN}>\nYour coins: {} <:coin:{COIN}>",
                (payout - game.bet()).format(),
                coins.format()
            ))
            .colour(Colour::RED)
    } else {
        CreateEmbed::new()
            .title("Blackjack - Draw!")
            .description(format!(
                "{desc}\n\nDraw! Have your money back.\n\nYour coins: {} <:coin:{COIN}>",
                coins.format()
            ))
            .colour(Colour::DARKER_GREY)
    };

    interaction
        .create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .embed(embed)
                    .components(Vec::new()),
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

    let dealer_value = sum_cards(&dealer_hand);

    format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{} - {dealer_value}\n\nBust!\n\nLost: {} <:coin:{COIN}>\nYour coins: {} <:coin:{COIN}>",
        game.bet().format(),
        game.player_hand_str(),
        dealer_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
        (payout - game.bet()).format(),
        coins.format()
    )
}
