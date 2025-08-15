use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::LazyLock,
};

use rand::{rng, seq::SliceRandom};
use serenity::all::{
    ButtonStyle, Colour, Context, CreateButton, CreateEmbed, EditInteractionResponse, EmojiId,
    GenericChannelId, UserId, parse_emoji,
};
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::FormatNum;

use crate::{
    CARD_BACK, CARD_DECK, COIN, Coins, EffectsManager, GameCache, GameManager, GameRow,
    GoalsManager,
    ctx_data::GamblingData,
    events::{Dispatch, Event, GameEvent},
};

pub static CARD_TO_NUM: LazyLock<HashMap<EmojiId, u8>> = LazyLock::new(|| {
    CARD_DECK
        .iter()
        .copied()
        .zip(
            (1u8..=13)
                .cycle()
                .map(|rank| match rank {
                    11..=13 => 10,
                    _ => rank,
                })
                .take(52),
        )
        .collect()
});

pub struct GameDetails {
    bet: i64,
    player_hand: Vec<EmojiId>,
    dealer_card: EmojiId,
    card_shoe: Vec<EmojiId>,
}

impl GameDetails {
    pub fn new(bet: i64, player_hand: Vec<EmojiId>, dealer_card: EmojiId) -> Self {
        Self {
            bet,
            player_hand,
            dealer_card,
            card_shoe: Vec::new(),
        }
    }

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

    fn card_shoe(&self) -> Vec<EmojiId> {
        let mut cards = self
            .player_hand
            .iter()
            .copied()
            .chain([self.dealer_card])
            .collect::<HashSet<_>>();
        cards.insert(self.dealer_card);

        let mut shoe = CARD_DECK
            .iter()
            .copied()
            .filter(|card| !cards.remove(card))
            .collect::<Vec<_>>();

        shoe.shuffle(&mut rng());

        shoe
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

        let mut game = GameDetails::new(bet, player_hand, dealer_card);
        game.card_shoe = game.card_shoe();

        Ok(game)
    }
}

pub fn sum_cards(hand: &[EmojiId]) -> u8 {
    let (aces, rest) = hand
        .iter()
        .map(|id| *CARD_TO_NUM.get(id).unwrap())
        .partition::<Vec<_>, _>(|num| *num == 1);

    let mut sum = rest.iter().sum();
    let mut num_aces = aces.len();

    sum += num_aces as u8 * 11;

    while sum > 21 && num_aces > 0 {
        sum -= 10;
        num_aces -= 1;
    }

    sum
}

pub fn in_play_embed(bet: i64, player_hand: &[EmojiId], dealer_card: EmojiId) -> CreateEmbed<'_> {
    let player_value = sum_cards(player_hand);
    let dealer_value = sum_cards(&[dealer_card]);

    let desc = format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n<:{}:{dealer_card}> <:blank:{CARD_BACK}> - {dealer_value}",
        bet.format(),
        player_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
        CARD_TO_NUM.get(&dealer_card).unwrap(),
    );

    CreateEmbed::new()
        .title("Blackjack")
        .description(desc)
        .colour(Colour::TEAL)
}

pub fn hit_button<'a>() -> CreateButton<'a> {
    CreateButton::new("blackjack_hit")
        .emoji('🎯')
        .label("Hit")
        .style(ButtonStyle::Secondary)
}

pub fn stand_button<'a>() -> CreateButton<'a> {
    CreateButton::new("blackjack_stand")
        .emoji('🛑')
        .label("Stand")
        .style(ButtonStyle::Secondary)
}

pub fn double_button<'a>(coins: i64, bet: i64) -> CreateButton<'a> {
    CreateButton::new("blackjack_double")
        .emoji('⏫')
        .label("Double Down")
        .style(ButtonStyle::Secondary)
        .disabled(coins < bet * 2)
}

async fn game_end_common<
    Data: GamblingData,
    Db: Database,
    GoalsHandler: GoalsManager<Db>,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    ctx: &Context,
    pool: &Pool<Db>,
    user_id: UserId,
    channel_id: GenericChannelId,
    bet: i64,
    mut payout: i64,
) -> (i64, i64) {
    let mut row = GameHandler::row(pool, user_id)
        .await
        .unwrap()
        .unwrap_or_else(|| GameRow::new(user_id));

    let dispatch = Dispatch::<Db, GoalsHandler>::new(&ctx.http, pool);

    dispatch
        .fire(
            channel_id,
            &mut row,
            Event::Game(GameEvent::new("blackjack", user_id, bet, payout, true)),
        )
        .await
        .unwrap();

    payout = EffectsHandler::payout(pool, user_id, bet, payout, None).await;

    row.add_coins(payout);

    let coins = row.coins();

    GameHandler::save(pool, row).await.unwrap();
    GameCache::update(ctx.data::<RwLock<Data>>(), user_id).await;

    (payout, coins)
}

pub async fn game_end_draw<
    'a,
    Data: GamblingData,
    Db: Database,
    GoalsHandler: GoalsManager<Db>,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    ctx: &Context,
    pool: &Pool<Db>,
    user_id: UserId,
    channel_id: GenericChannelId,
    game: GameDetails,
    dealer_hand: &[EmojiId],
) -> EditInteractionResponse<'a> {
    let bet = game.bet();
    let dealer_value = sum_cards(dealer_hand);

    let (_, coins) = game_end_common::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
        ctx, pool, user_id, channel_id, bet, bet,
    )
    .await;

    let desc = format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {}\n\n**Dealer Hand**\n{} - {dealer_value}",
        bet.format(),
        game.player_hand_str(),
        game.player_value(),
        dealer_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
    );

    let embed = CreateEmbed::new()
        .title("Blackjack - Draw!")
        .description(format!(
            "{desc}\n\nDraw! Have your money back.\n\nYour coins: {} <:coin:{COIN}>",
            coins.format()
        ))
        .colour(Colour::DARKER_GREY);

    EditInteractionResponse::new()
        .embed(embed)
        .components(Vec::new())
}

pub async fn game_end_blackjack<
    'a,
    Data: GamblingData,
    Db: Database,
    GoalsHandler: GoalsManager<Db>,
    EffectsHandler: EffectsManager<Db>,
    GameHandler: GameManager<Db>,
>(
    ctx: &Context,
    pool: &Pool<Db>,
    user_id: UserId,
    channel_id: GenericChannelId,
    game: GameDetails,
    dealer_hand: &[EmojiId],
) -> EditInteractionResponse<'a> {
    let bet = game.bet();
    let dealer_value = sum_cards(dealer_hand);

    let (payout, coins) = game_end_common::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
        ctx,
        pool,
        user_id,
        channel_id,
        bet,
        (3 * bet) / 2,
    )
    .await;

    let desc = format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {}\n\n**Dealer Hand**\n{} - {dealer_value}",
        bet.format(),
        game.player_hand_str(),
        game.player_value(),
        dealer_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
    );

    let embed = CreateEmbed::new()
        .title("Blackjack - You Won!")
        .description(format!(
            "{desc}\n\nBLACKJACK!\n\nProfit: {} <:coin:{COIN}>\nYour coins: {} <:coin:{COIN}>",
            (payout - game.bet()).format(),
            coins.format()
        ))
        .colour(Colour::DARK_GREEN);

    EditInteractionResponse::new()
        .embed(embed)
        .components(Vec::new())
}

pub fn game_end_desc(
    bet: i64,
    player_hand: &[EmojiId],
    dealer_hand: &[EmojiId],
    payout: i64,
    coins: i64,
) -> String {
    let player_value = sum_cards(player_hand);
    let dealer_value = sum_cards(dealer_hand);

    format!(
        "Your bet: {} <:coin:{COIN}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{} - {dealer_value}\n\nBust!\n\nLost: {} <:coin:{COIN}>\nYour coins: {} <:coin:{COIN}>",
        bet.format(),
        player_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
        dealer_hand
            .iter()
            .map(|id| (*CARD_TO_NUM.get(id).unwrap(), id))
            .map(|(num, id)| format!("<:{num}:{id}> "))
            .collect::<String>(),
        (payout - bet).format(),
        coins.format()
    )
}
