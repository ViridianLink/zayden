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
use zayden_core::{EmojiCache, FormatNum};

use crate::events::{Dispatch, Event, GameEvent};
use crate::{
    CARD_DECK,
    Coins,
    EffectsManager,
    GamblingError,
    GameManager,
    GameRow,
    GoalsManager,
    Result,
    card_deck,
};

pub static CARD_VALUES: OnceLock<HashMap<EmojiId, u8>> = OnceLock::new();

fn get_card_values(emojis: &EmojiCache) -> Result<&'static HashMap<EmojiId, u8>> {
    if let Some(map) = CARD_VALUES.get() {
        return Ok(map);
    }

    let deck = if let Some(d) = CARD_DECK.get() {
        d
    } else {
        let new_deck = card_deck(emojis)?;
        let _ = CARD_DECK.set(new_deck);
        CARD_DECK.get().ok_or_else(|| {
            GamblingError::Internal("CARD_DECK init failed".to_string())
        })?
    };

    let map: HashMap<EmojiId, u8> = deck
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
        .collect();

    let _ = CARD_VALUES.set(map);
    CARD_VALUES.get().ok_or_else(|| {
        GamblingError::Internal("CARD_VALUES init failed".to_string())
    })
}

pub fn card_values(emojis: &EmojiCache) -> Result<HashMap<EmojiId, u8>> {
    get_card_values(emojis).cloned()
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
    pub const fn dealer_card(&self) -> EmojiId {
        self.dealer_card
    }

    pub fn player_value(&self, emojis: &EmojiCache) -> Result<u8> {
        sum_cards(emojis, &self.player_hand)
    }

    pub fn player_hand_str(&self, emojis: &EmojiCache) -> Result<String> {
        let card_to_num = get_card_values(emojis)?;
        let mut s = String::new();
        for id in &self.player_hand {
            let num = card_to_num.get(id).ok_or_else(|| {
                GamblingError::Internal("card ID not in CARD_VALUES".to_string())
            })?;
            let _ = write!(s, "<:{num}:{id}> ");
        }

        Ok(s)
    }

    pub fn add_card(&mut self) -> Result<()> {
        let card = self.card_shoe.pop().ok_or_else(|| {
            GamblingError::Internal("blackjack card shoe is empty".to_string())
        })?;

        self.player_hand.push(card);

        Ok(())
    }

    pub fn next_card(&mut self) -> Result<EmojiId> {
        self.card_shoe.pop().ok_or_else(|| {
            GamblingError::Internal("blackjack card shoe is empty".to_string())
        })
    }

    fn card_shoe_init(&self, emojis: &EmojiCache) -> Result<Vec<EmojiId>> {
        let mut cards = self
            .player_hand
            .iter()
            .copied()
            .chain([self.dealer_card])
            .collect::<HashSet<_>>();
        cards.insert(self.dealer_card);

        let deck = if let Some(d) = CARD_DECK.get() {
            d
        } else {
            let new_deck = card_deck(emojis)?;
            let _ = CARD_DECK.set(new_deck);
            CARD_DECK.get().ok_or_else(|| {
                GamblingError::Internal("CARD_DECK init failed".to_string())
            })?
        };

        let mut shoe = deck
            .iter()
            .copied()
            .filter(|card| !cards.remove(card))
            .collect::<Vec<_>>();

        shoe.shuffle(&mut rng());

        Ok(shoe)
    }

    pub fn from_str(emojis: &EmojiCache, s: &str) -> Result<Self> {
        let mut lines = s.lines().skip(1);

        let bet_line = lines.next().ok_or_else(|| {
            GamblingError::Internal("game message missing bet line".to_string())
        })?;
        let bet = bet_line
            .strip_prefix("Your bet: ")
            .ok_or_else(|| {
                GamblingError::Internal("bet line missing prefix".to_string())
            })?
            .split_whitespace()
            .next()
            .ok_or_else(|| GamblingError::Internal("bet line is empty".to_string()))?
            .replace(',', "")
            .parse::<i64>()
            .map_err(|_e| {
                GamblingError::Internal("bet is not a valid integer".to_string())
            })?;

        lines.next();
        lines.next();

        let player_hand_line = lines.next().ok_or_else(|| {
            GamblingError::Internal(
                "game message missing player hand line".to_string(),
            )
        })?;
        let player_hand = player_hand_line
            .split(" - ")
            .next()
            .ok_or_else(|| {
                GamblingError::Internal(
                    "player hand line missing separator".to_string(),
                )
            })?
            .split_whitespace()
            .map(parse_emoji)
            .map(|emoji| emoji.map(|e| e.id))
            .collect::<Option<Vec<EmojiId>>>()
            .ok_or_else(|| {
                GamblingError::Internal(
                    "player hand contains invalid emoji".to_string(),
                )
            })?;

        lines.next();
        lines.next();

        let dealer_hand_line = lines.next().ok_or_else(|| {
            GamblingError::Internal(
                "game message missing dealer hand line".to_string(),
            )
        })?;
        let dealer_card_str =
            dealer_hand_line.split_whitespace().next().ok_or_else(|| {
                GamblingError::Internal("dealer hand line is empty".to_string())
            })?;
        let dealer_card = parse_emoji(dealer_card_str)
            .ok_or_else(|| {
                GamblingError::Internal(
                    "dealer card is not a valid emoji".to_string(),
                )
            })?
            .id;

        let mut game = Self::new(bet, player_hand, dealer_card);
        game.card_shoe = game.card_shoe_init(emojis)?;

        Ok(game)
    }
}

pub fn sum_cards(emojis: &EmojiCache, hand: &[EmojiId]) -> Result<u8> {
    let card_to_num = get_card_values(emojis)?;

    let mut aces = 0u8;
    let mut sum: u8 = 0;
    for id in hand {
        let val = *card_to_num.get(id).ok_or_else(|| {
            GamblingError::Internal("card ID not in CARD_VALUES".to_string())
        })?;
        if val == 1 {
            aces += 1;
        } else {
            sum = sum.saturating_add(val);
        }
    }

    sum = sum.saturating_add(aces.saturating_mul(11));

    let mut num_aces = aces as usize;
    while sum > 21 && num_aces > 0 {
        sum -= 10;
        num_aces -= 1;
    }

    Ok(sum)
}

pub fn in_play_text<'a>(
    emojis: &EmojiCache,
    bet: i64,
    player_hand: &[EmojiId],
    dealer_card: EmojiId,
) -> Result<CreateContainerComponent<'a>> {
    let player_value = sum_cards(emojis, player_hand)?;
    let dealer_value = sum_cards(emojis, &[dealer_card])?;

    let card_to_num = get_card_values(emojis)?;
    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let card_back = emojis
        .emoji("card_back")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let mut player_hand_str = String::new();
    for id in player_hand {
        let num = card_to_num.get(id).ok_or_else(|| {
            GamblingError::Internal("card ID not in CARD_VALUES".to_string())
        })?;
        let _ = write!(player_hand_str, "<:{num}:{id}> ");
    }

    let dealer_num = card_to_num.get(&dealer_card).ok_or_else(|| {
        GamblingError::Internal("dealer card not in CARD_VALUES".to_string())
    })?;

    let desc = format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{player_hand_str}- {player_value}\n\n**Dealer Hand**\n<:{dealer_num}:{dealer_card}> <:blank:{card_back}> - {dealer_value}",
        bet.format(),
    );

    Ok(CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
        "### Blackjack\n{desc}"
    ))))
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
) -> Result<(i64, i64)> {
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

    Ok((payout, coins))
}

fn build_hand_str(
    card_to_num: &HashMap<EmojiId, u8>,
    hand: &[EmojiId],
) -> Result<String> {
    let mut s = String::new();
    for id in hand {
        let num = card_to_num.get(id).ok_or_else(|| {
            GamblingError::Internal("card ID not in CARD_VALUES".to_string())
        })?;
        let _ = write!(s, "<:{num}:{id}> ");
    }

    Ok(s)
}

pub async fn game_end_draw<
    'a,
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
) -> Result<EditInteractionResponse<'a>> {
    let bet = game.bet();
    let dealer_value = sum_cards(emojis, dealer_hand)?;

    let (_, coins) =
        game_end_common::<Db, GoalsHandler, EffectsHandler, GameHandler>(
            ctx, pool, emojis, user_id, channel_id, bet, bet,
        )
        .await?;

    let card_to_num = get_card_values(emojis)?;
    let coin = emojis.get("heads").ok_or_else(|| {
        GamblingError::Internal("emoji 'heads' not in cache".to_string())
    })?;

    let dealer_hand_str = build_hand_str(card_to_num, dealer_hand)?;

    let desc = format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {}\n\n**Dealer Hand**\n{dealer_hand_str} - {dealer_value}",
        bet.format(),
        game.player_hand_str(emojis)?,
        game.player_value(emojis)?,
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
) -> Result<EditInteractionResponse<'a>> {
    let bet = game.bet();
    let payout = bet + (3 * bet) / 2;
    let dealer_value = sum_cards(emojis, dealer_hand)?;

    let (payout, coins) =
        game_end_common::<Db, GoalsHandler, EffectsHandler, GameHandler>(
            ctx, pool, emojis, user_id, channel_id, bet, payout,
        )
        .await?;

    let card_to_num = get_card_values(emojis)?;
    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let dealer_hand_str = build_hand_str(card_to_num, dealer_hand)?;

    let desc = format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{}- {}\n\n**Dealer Hand**\n{dealer_hand_str} - {dealer_value}",
        bet.format(),
        game.player_hand_str(emojis)?,
        game.player_value(emojis)?,
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
) -> Result<String> {
    let player_value = sum_cards(emojis, player_hand)?;
    let dealer_value = sum_cards(emojis, dealer_hand)?;

    let card_to_num = get_card_values(emojis)?;
    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let player_hand_str = build_hand_str(card_to_num, player_hand)?;
    let dealer_hand_str = build_hand_str(card_to_num, dealer_hand)?;

    Ok(format!(
        "Your bet: {} <:coin:{coin}>\n\n**Your Hand**\n{player_hand_str}- {player_value}\n\n**Dealer Hand**\n{dealer_hand_str} - {dealer_value}\n\nBust!\n\nLost: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>",
        bet.format(),
        (payout - bet).format(),
        coins.format()
    ))
}
