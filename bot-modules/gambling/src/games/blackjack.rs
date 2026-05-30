use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::OnceLock;

use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    ButtonStyle,
    Colour,
    Context,
    CreateButton,
    CreateContainerComponent,
    CreateEmbed,
    CreateTextDisplay,
    EditInteractionResponse,
    EmojiId,
    GenericChannelId,
    ReactionType,
    UserId,
    parse_emoji,
};
use serenity::small_fixed_array::FixedString;
use sqlx::{Database, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCache, EmojiCacheData, FormatNum};

use crate::ctx_data::GamblingData;
use crate::events::{Dispatch, Event, GameEvent};
use crate::{
    CARD_DECK,
    Coins,
    EffectsManager,
    GameCache,
    GameManager,
    GameRow,
    GoalsManager,
    card_deck,
};

pub static CARD_VALUES: OnceLock<HashMap<EmojiId, u8>> = OnceLock::new();

pub fn card_values(emojis: &EmojiCache) -> HashMap<EmojiId, u8> {
    CARD_DECK
        .get_or_init(|| card_deck(emojis))
        .iter()
        .copied()
        .zip(
            (1u8..=13)
                .map(|rank| match rank {
                    11..=13 => 10,
                    _ => rank,
                })
                .cycle()
                .take(52),
        )
        .collect()
}

pub struct GameDetails {
    bet: i64,
    player_hand: Vec<EmojiId>,
    dealer_card: EmojiId,
    card_shoe: Vec<EmojiId>,
}

impl GameDetails {
    #[must_use]
    pub const fn new(
        bet: i64,
        player_hand: Vec<EmojiId>,
        dealer_card: EmojiId,
    ) -> Self {
        Self { bet, player_hand, dealer_card, card_shoe: Vec::new() }
    }

    #[must_use]
    pub const fn bet(&self) -> i64 {
        self.bet
    }

    pub const fn double_bet(&mut self) {
        self.bet *= 2;
    }

    #[must_use]
    pub fn player_hand(&self) -> &[EmojiId] {
        &self.player_hand
    }

    #[must_use]
    pub fn player_value(&self, emojis: &EmojiCache) -> u8 {
        sum_cards(emojis, &self.player_hand)
    }

    pub fn player_hand_str(&self, emojis: &EmojiCache) -> String {
        let mut s = String::new();

        for id in &self.player_hand {
            let num = *CARD_VALUES
                .get_or_init(|| card_values(emojis))
                .get(id)
                .expect("player hand card always in CARD_VALUES");

            let _ = write!(s, "<:{num}:{id}> ");
        }

        s
    }

    pub fn add_card(&mut self) {
        self.player_hand.push(self.card_shoe.pop().expect("card shoe not empty"));
    }

    #[must_use]
    pub fn dealer_hand(&self) -> Vec<EmojiId> {
        vec![self.dealer_card]
    }

    pub fn next_card(&mut self) -> EmojiId {
        self.card_shoe.pop().expect("card shoe not empty")
    }

    fn card_shoe(&self, emojis: &EmojiCache) -> Vec<EmojiId> {
        let mut cards = self
            .player_hand
            .iter()
            .copied()
            .chain([self.dealer_card])
            .collect::<HashSet<_>>();
        cards.insert(self.dealer_card);

        let mut shoe = CARD_DECK
            .get_or_init(|| card_deck(emojis))
            .iter()
            .copied()
            .filter(|card| !cards.remove(card))
            .collect::<Vec<_>>();

        shoe.shuffle(&mut rng());

        shoe
    }

    pub fn from_str(emojis: &EmojiCache, s: &str) -> Self {
        let mut lines = s.lines().skip(1);

        let bet_line = lines.next().expect("game message has expected format");
        let bet = bet_line
            .strip_prefix("Your bet: ")
            .expect("bet line has expected prefix")
            .split_whitespace()
            .next()
            .expect("bet line has non-empty content")
            .replace(',', "")
            .parse::<i64>()
            .expect("bet is always a valid integer");

        lines.next();
        lines.next();

        let player_hand_line =
            lines.next().expect("game message has expected format");
        let player_hand = player_hand_line
            .split(" - ")
            .next()
            .expect("player hand line has expected format")
            .split_whitespace()
            .map(parse_emoji)
            .map(|emoji| emoji.map(|emoji| emoji.id))
            .collect::<Option<Vec<EmojiId>>>()
            .expect("player hand emojis are all valid");

        lines.next();
        lines.next();

        let dealer_hand_line =
            lines.next().expect("game message has expected format");
        let dealer_card_str = dealer_hand_line
            .split_whitespace()
            .next()
            .expect("dealer hand line has content");
        let dealer_card =
            parse_emoji(dealer_card_str).expect("dealer card is valid emoji").id;

        let mut game = Self::new(bet, player_hand, dealer_card);
        game.card_shoe = game.card_shoe(emojis);

        game
    }
}

pub fn sum_cards(emojis: &EmojiCache, hand: &[EmojiId]) -> u8 {
    let card_to_num = CARD_VALUES.get_or_init(|| card_values(emojis));

    let (aces, rest) = hand
        .iter()
        .map(|id| *card_to_num.get(id).expect("card ID always in card_to_num"))
        .partition::<Vec<_>, _>(|num| *num == 1);

    let mut sum = rest.iter().sum();
    let mut num_aces = aces.len();

    sum += u8::try_from(num_aces).unwrap_or(u8::MAX).saturating_mul(11);

    while sum > 21 && num_aces > 0 {
        sum -= 10;
        num_aces -= 1;
    }

    sum
}

pub fn in_play_text<'a>(
    emojis: &EmojiCache,
    bet: i64,
    player_hand: &[EmojiId],
    dealer_card: EmojiId,
) -> CreateContainerComponent<'a> {
    let player_value = sum_cards(emojis, player_hand);
    let dealer_value = sum_cards(emojis, &[dealer_card]);

    let card_to_num = CARD_VALUES.get_or_init(|| card_values(emojis));
    let coin = emojis.emoji("heads").expect("emoji 'heads' in cache");
    let card_back = emojis.emoji("card_back").expect("emoji 'card_back' in cache");

    let desc = format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n<:{}:{dealer_card}> <:blank:{card_back}> - {dealer_value}",
        bet.format(),
        player_hand.iter().fold(String::new(), |mut acc, id| {
            let num = *card_to_num.get(id).expect("card ID always in card_to_num");
            let _ = write!(acc, "<:{num}:{id}> ");
            acc
        }),
        card_to_num.get(&dealer_card).expect("dealer card ID always in card_to_num"),
    );

    CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
        "### Blackjack\n{desc}"
    )))
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

pub fn double_button<'a>() -> CreateButton<'a> {
    CreateButton::new("blackjack_double")
        .emoji('⏫')
        .label("Double Down")
        .style(ButtonStyle::Secondary)
}

pub fn split_button<'a>() -> CreateButton<'a> {
    CreateButton::new("blackjack_split")
        .emoji(ReactionType::Unicode(FixedString::from_static_trunc("✂️")))
        .label("Split")
        .style(ButtonStyle::Secondary)
}

pub fn surrender_button<'a>() -> CreateButton<'a> {
    CreateButton::new("blackjack_surrender")
        .emoji(ReactionType::Unicode(FixedString::from_static_trunc("🏳️")))
        .label("Surrender")
        .style(ButtonStyle::Danger)
}

async fn game_end_common<
    Data: GamblingData,
    Db: Database,
    GoalsHandler: GoalsManager<Db> + Send + Sync,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    ctx: &Context,
    pool: &Pool<Db>,
    emojis: &EmojiCache,
    user_id: UserId,
    channel_id: GenericChannelId,
    bet: i64,
    mut payout: i64,
) -> crate::Result<(i64, i64)> {
    let mut row = GameHandler::row(pool, user_id)
        .await?
        .unwrap_or_else(|| GameRow::new(user_id));

    let dispatch = Dispatch::<Db, GoalsHandler>::new(&ctx.http, pool, emojis);

    dispatch
        .fire(
            channel_id,
            &mut row,
            Event::Game(GameEvent::new("blackjack", user_id, bet, payout, true)),
        )
        .await?;

    payout = EffectsHandler::payout(pool, user_id, bet, payout, None).await;

    row.add_coins(payout);

    let coins = row.coins();

    GameHandler::save(pool, row).await?;
    GameCache::update(ctx.data::<RwLock<Data>>(), user_id).await;

    Ok((payout, coins))
}

pub async fn game_end_draw<
    'a,
    Data: GamblingData + EmojiCacheData,
    Db: Database,
    GoalsHandler: GoalsManager<Db> + Send + Sync,
    EffectsHandler: EffectsManager<Db> + Send,
    GameHandler: GameManager<Db>,
>(
    ctx: &Context,
    pool: &Pool<Db>,
    emojis: &EmojiCache,
    user_id: UserId,
    channel_id: GenericChannelId,
    game: GameDetails,
    dealer_hand: &[EmojiId],
) -> crate::Result<EditInteractionResponse<'a>> {
    let bet = game.bet();
    let dealer_value = sum_cards(emojis, dealer_hand);

    let (_, coins) =
        game_end_common::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
            ctx, pool, emojis, user_id, channel_id, bet, bet,
        )
        .await?;

    let card_to_num = CARD_VALUES.get_or_init(|| card_values(emojis));

    let coin = emojis.get("heads").expect("emoji 'heads' in cache");

    let desc = format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {}\n\n**Dealer Hand**\n{} - {dealer_value}",
        bet.format(),
        game.player_hand_str(emojis),
        game.player_value(emojis),
        dealer_hand.iter().fold(String::new(), |mut acc, id| {
            let num = *card_to_num.get(id).expect("card ID always in card_to_num");
            let _ = write!(acc, "<:{num}:{id}> ");
            acc
        }),
    );

    let embed = CreateEmbed::new()
        .title("Blackjack - Draw!")
        .description(format!(
            "{desc}\n\nDraw! Have your money back.\n\nYour coins: {} <:coin:{coin}>",
            coins.format()
        ))
        .colour(Colour::DARKER_GREY);

    Ok(EditInteractionResponse::new().embed(embed).components(Vec::new()))
}

pub async fn game_end_blackjack<
    'a,
    Data: GamblingData,
    Db: Database,
    GoalsHandler: GoalsManager<Db> + Send + Sync,
    EffectsHandler: EffectsManager<Db>,
    GameHandler: GameManager<Db>,
>(
    ctx: &Context,
    pool: &Pool<Db>,
    emojis: &EmojiCache,
    user_id: UserId,
    channel_id: GenericChannelId,
    game: GameDetails,
    dealer_hand: &[EmojiId],
) -> crate::Result<EditInteractionResponse<'a>> {
    let bet = game.bet();
    let payout = bet + (3 * bet) / 2;
    let dealer_value = sum_cards(emojis, dealer_hand);

    let (payout, coins) =
        game_end_common::<Data, Db, GoalsHandler, EffectsHandler, GameHandler>(
            ctx, pool, emojis, user_id, channel_id, bet, payout,
        )
        .await?;

    let card_to_num = CARD_VALUES.get_or_init(|| card_values(emojis));
    let coin = emojis.emoji("heads").expect("emoji 'heads' in cache");

    let desc = format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {}\n\n**Dealer Hand**\n{} - {dealer_value}",
        bet.format(),
        game.player_hand_str(emojis),
        game.player_value(emojis),
        dealer_hand.iter().fold(String::new(), |mut acc, id| {
            let num = *card_to_num.get(id).expect("card ID always in card_to_num");
            let _ = write!(acc, "<:{num}:{id}> ");
            acc
        }),
    );

    let embed = CreateEmbed::new()
        .title("Blackjack - You Won!")
        .description(format!(
            "{desc}\n\nBLACKJACK!\n\nProfit: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>",
            (payout - game.bet()).format(),
            coins.format()
        ))
        .colour(Colour::DARK_GREEN);

    Ok(EditInteractionResponse::new().embed(embed).components(Vec::new()))
}

pub fn game_end_desc(
    emojis: &EmojiCache,
    bet: i64,
    player_hand: &[EmojiId],
    dealer_hand: &[EmojiId],
    payout: i64,
    coins: i64,
) -> String {
    let player_value = sum_cards(emojis, player_hand);
    let dealer_value = sum_cards(emojis, dealer_hand);

    let card_to_num = CARD_VALUES.get_or_init(|| card_values(emojis));
    let coin = emojis.emoji("heads").expect("emoji 'heads' in cache");

    format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {player_value}\n\n**Dealer Hand**\n{} - {dealer_value}\n\nBust!\n\nLost: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>",
        bet.format(),
        player_hand.iter().fold(String::new(), |mut acc, id| {
            let num = *card_to_num.get(id).expect("card ID always in card_to_num");
            let _ = write!(acc, "<:{num}:{id}> ");
            acc
        }),
        dealer_hand.iter().fold(String::new(), |mut acc, id| {
            let num = *card_to_num.get(id).expect("card ID always in card_to_num");
            let _ = write!(acc, "<:{num}:{id}> ");
            acc
        }),
        (payout - bet).format(),
        coins.format()
    )
}
