use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::OnceLock;

use rand::rng;
use rand::seq::SliceRandom;
use serenity::all::{
    ButtonKind,
    ButtonStyle,
    Colour,
    ContainerComponent,
    Context,
    CreateButton,
    CreateComponent,
    CreateContainer,
    CreateContainerComponent,
    CreateSection,
    CreateSectionAccessory,
    CreateSectionComponent,
    CreateSeparator,
    CreateTextDisplay,
    EditInteractionResponse,
    EmojiId,
    GenericChannelId,
    MessageFlags,
    ReactionType,
    Section,
    SectionAccessory,
    SectionComponent,
    SeparatorSpacingSize,
    UserId,
    parse_emoji,
};
use serenity::small_fixed_array::FixedString;
use sqlx::PgPool;
use zayden_core::{EmojiCache, FormatNum};

use crate::components::{BlackjackCustomId, HandState};
use crate::events::{Dispatch, Event, GameEvent};
use crate::utils::effects_summary;
use crate::{
    AppliedEffect,
    CARD_DECK,
    Coins,
    EffectsManager,
    GamblingError,
    GameDelta,
    GameRow,
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
    hands: Vec<Vec<EmojiId>>,
    active: usize,
    dealer_card: EmojiId,
    card_shoe: Vec<EmojiId>,
}

impl GameDetails {
    #[must_use]
    pub fn new(bet: i64, player_hand: Vec<EmojiId>, dealer_card: EmojiId) -> Self {
        Self {
            bet,
            hands: vec![player_hand],
            active: 0,
            dealer_card,
            card_shoe: Vec::new(),
        }
    }

    #[must_use]
    pub const fn bet(&self) -> i64 {
        self.bet
    }

    #[must_use]
    pub fn total_bet(&self) -> i64 {
        self.bet.saturating_mul(i64::try_from(self.hands.len()).unwrap_or(1))
    }

    pub const fn double_bet(&mut self) {
        self.bet *= 2;
    }

    #[must_use]
    pub fn hands(&self) -> &[Vec<EmojiId>] {
        &self.hands
    }

    #[must_use]
    pub const fn active(&self) -> usize {
        self.active
    }

    #[must_use]
    pub const fn is_split(&self) -> bool {
        self.hands.len() > 1
    }

    #[must_use]
    pub fn player_hand(&self) -> &[EmojiId] {
        self.hands.get(self.active).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub const fn dealer_card(&self) -> EmojiId {
        self.dealer_card
    }

    pub fn player_value(&self, emojis: &EmojiCache) -> Result<u8> {
        sum_cards(emojis, self.player_hand())
    }

    pub fn hand_value(&self, emojis: &EmojiCache, index: usize) -> Result<u8> {
        sum_cards(emojis, self.hands.get(index).map_or(&[], Vec::as_slice))
    }

    pub fn can_split(&self, emojis: &EmojiCache) -> Result<bool> {
        if self.is_split() {
            return Ok(false);
        }

        let hand = self.player_hand();
        let (Some(first), Some(second), 2) = (hand.first(), hand.get(1), hand.len())
        else {
            return Ok(false);
        };

        let card_to_num = get_card_values(emojis)?;
        let lookup = |id: &EmojiId| {
            card_to_num.get(id).copied().ok_or_else(|| {
                GamblingError::Internal("card ID not in CARD_VALUES".to_string())
            })
        };

        Ok(lookup(first)? == lookup(second)?)
    }

    pub fn split(&mut self) -> Result<()> {
        let hand = self.hands.first().ok_or_else(|| {
            GamblingError::Internal("no hand to split".to_string())
        })?;

        let (Some(first), Some(second)) =
            (hand.first().copied(), hand.get(1).copied())
        else {
            return Err(GamblingError::Internal(
                "split needs a two card hand".to_string(),
            ));
        };

        let (draw_one, draw_two) = (self.next_card()?, self.next_card()?);

        self.hands = vec![vec![first, draw_one], vec![second, draw_two]];
        self.active = 0;

        Ok(())
    }

    pub const fn advance_hand(&mut self) -> bool {
        if self.active + 1 < self.hands.len() {
            self.active += 1;
            return true;
        }

        false
    }

    pub fn player_hand_str(&self, emojis: &EmojiCache) -> Result<String> {
        self.hand_str(emojis, self.active)
    }

    pub fn hand_str(&self, emojis: &EmojiCache, index: usize) -> Result<String> {
        let card_to_num = get_card_values(emojis)?;

        build_hand_str(card_to_num, self.hands.get(index).map_or(&[], Vec::as_slice))
    }

    pub fn add_card(&mut self) -> Result<()> {
        let card = self.card_shoe.pop().ok_or_else(|| {
            GamblingError::Internal("blackjack card shoe is empty".to_string())
        })?;

        self.hands
            .get_mut(self.active)
            .ok_or_else(|| {
                GamblingError::Internal("no active hand to draw to".to_string())
            })?
            .push(card);

        Ok(())
    }

    pub fn next_card(&mut self) -> Result<EmojiId> {
        self.card_shoe.pop().ok_or_else(|| {
            GamblingError::Internal("blackjack card shoe is empty".to_string())
        })
    }

    fn card_shoe_init(&self, emojis: &EmojiCache) -> Result<Vec<EmojiId>> {
        let mut cards = self
            .hands
            .iter()
            .flatten()
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

    pub fn from_components(
        emojis: &EmojiCache,
        components: &[ContainerComponent],
    ) -> Result<Self> {
        let mut bet = None;
        let mut hands = Vec::new();
        let mut active = 0;
        let mut dealer_card = None;

        for component in components {
            if let ContainerComponent::TextDisplay(text) = component {
                bet = bet.or_else(|| parse_bet(&text.content));
                continue;
            }

            let ContainerComponent::Section(section) = component else {
                continue;
            };

            let SectionAccessory::Button(badge) = section.accessory.as_ref() else {
                continue;
            };

            let ButtonKind::NonLink { custom_id, .. } = &badge.data else {
                continue;
            };

            let Ok(id) = custom_id.parse::<BlackjackCustomId>() else {
                continue;
            };

            let hand = parse_hand(&section_text(section))?;

            match id {
                BlackjackCustomId::Hand { index, state } => {
                    if state == HandState::Active {
                        active = usize::from(index);
                    }

                    hands.push((index, hand));
                },
                BlackjackCustomId::Dealer { .. } => {
                    dealer_card = hand.first().copied();
                },
                BlackjackCustomId::Hit
                | BlackjackCustomId::Stand
                | BlackjackCustomId::Double
                | BlackjackCustomId::Split
                | BlackjackCustomId::Surrender => {},
            }
        }

        let bet = bet.ok_or_else(|| {
            GamblingError::Internal("game board has no bet".to_string())
        })?;

        let dealer_card = dealer_card.ok_or_else(|| {
            GamblingError::Internal("game board has no dealer card".to_string())
        })?;

        hands.sort_unstable_by_key(|(index, _)| *index);

        let mut game = Self::new(bet, Vec::new(), dealer_card);
        game.hands = hands.into_iter().map(|(_, hand)| hand).collect();

        if game.hands.is_empty() {
            return Err(GamblingError::Internal(
                "game board has no player hand".to_string(),
            ));
        }

        game.active = active.min(game.hands.len() - 1);
        game.card_shoe = game.card_shoe_init(emojis)?;

        Ok(game)
    }
}

fn parse_bet(content: &str) -> Option<i64> {
    content
        .lines()
        .find_map(|line| line.trim().strip_prefix("Your bet: "))
        .and_then(|rest| rest.split_whitespace().next())?
        .replace(',', "")
        .parse()
        .ok()
}

fn section_text(section: &Section) -> String {
    section
        .components
        .iter()
        .map(|component| match component {
            SectionComponent::TextDisplay(text) => text.content.as_str(),
            _ => "",
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub const PLAYER_HEADING: &str = "Your Hand";
pub const SPLIT_HEADING: &str = "Split Hand";
pub const DEALER_HEADING: &str = "Dealer Hand";

fn parse_hand(block: &str) -> Result<Vec<EmojiId>> {
    block
        .lines()
        .find(|line| line.contains("<:"))
        .ok_or_else(|| {
            GamblingError::Internal("hand block has no cards".to_string())
        })?
        .split(" - ")
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .map(|card| parse_emoji(card).map(|emoji| emoji.id))
        .collect::<Option<Vec<EmojiId>>>()
        .ok_or_else(|| {
            GamblingError::Internal("hand contains invalid emoji".to_string())
        })
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

fn divider<'a>() -> CreateContainerComponent<'a> {
    CreateContainerComponent::Separator(
        CreateSeparator::new().divider(true).spacing(SeparatorSpacingSize::Small),
    )
}

fn hand_section<'a>(
    id: BlackjackCustomId,
    heading: &str,
    cards: &str,
    value: u8,
    label: &'static str,
    style: ButtonStyle,
) -> CreateContainerComponent<'a> {
    CreateContainerComponent::Section(CreateSection::new(
        vec![CreateSectionComponent::TextDisplay(CreateTextDisplay::new(format!(
            "**{heading}**\n{cards}- {value}"
        )))],
        CreateSectionAccessory::Button(
            CreateButton::new(id.to_string())
                .label(label)
                .style(style)
                .disabled(true),
        ),
    ))
}

fn hand_id(index: usize, state: HandState) -> BlackjackCustomId {
    BlackjackCustomId::Hand { index: u8::try_from(index).unwrap_or(u8::MAX), state }
}

const fn hand_heading(index: usize, split: bool) -> &'static str {
    if split && index > 0 { SPLIT_HEADING } else { PLAYER_HEADING }
}

pub fn in_play_board<'a>(
    emojis: &EmojiCache,
    game: &GameDetails,
) -> Result<Vec<CreateContainerComponent<'a>>> {
    let dealer_card = game.dealer_card();
    let dealer_value = sum_cards(emojis, &[dealer_card])?;

    let card_to_num = get_card_values(emojis)?;

    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let card_back = emojis
        .emoji("card_back")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let dealer_num = card_to_num.get(&dealer_card).ok_or_else(|| {
        GamblingError::Internal("dealer card not in CARD_VALUES".to_string())
    })?;

    let stake = if game.is_split() { " (per hand)" } else { "" };

    let mut components = vec![
        CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
            "### Blackjack\nYour bet: {}{stake} <:coin:{coin}>",
            game.bet().format()
        ))),
        divider(),
    ];

    for index in 0..game.hands().len() {
        let value = game.hand_value(emojis, index)?;

        let (state, label, style) = match index.cmp(&game.active()) {
            Ordering::Equal => (HandState::Active, "Playing", ButtonStyle::Success),
            Ordering::Greater => {
                (HandState::Waiting, "Waiting", ButtonStyle::Secondary)
            },
            Ordering::Less if value > 21 => {
                (HandState::Done, "Bust", ButtonStyle::Danger)
            },
            Ordering::Less => (HandState::Done, "Stand", ButtonStyle::Secondary),
        };

        components.push(hand_section(
            hand_id(index, state),
            hand_heading(index, game.is_split()),
            &game.hand_str(emojis, index)?,
            value,
            label,
            style,
        ));
    }

    components.push(divider());
    components.push(hand_section(
        BlackjackCustomId::Dealer { state: HandState::Waiting },
        DEALER_HEADING,
        &format!("<:{dealer_num}:{dealer_card}> <:blank:{card_back}> "),
        dealer_value,
        "Waiting",
        ButtonStyle::Secondary,
    ));

    Ok(components)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandOutcome {
    Won,
    Push,
    Lost,
    Bust,
}

impl HandOutcome {
    #[must_use]
    pub const fn settle(value: u8, dealer_value: u8) -> Self {
        if value > 21 {
            Self::Bust
        } else if dealer_value > 21 || value > dealer_value {
            Self::Won
        } else if value == dealer_value {
            Self::Push
        } else {
            Self::Lost
        }
    }

    #[must_use]
    pub const fn payout(self, bet: i64) -> i64 {
        match self {
            Self::Won => bet.saturating_mul(2),
            Self::Push => bet,
            Self::Lost | Self::Bust => 0,
        }
    }

    const fn badge(self) -> (&'static str, ButtonStyle) {
        match self {
            Self::Won => ("Won", ButtonStyle::Success),
            Self::Push => ("Push", ButtonStyle::Secondary),
            Self::Lost => ("Lost", ButtonStyle::Danger),
            Self::Bust => ("Bust", ButtonStyle::Danger),
        }
    }
}

pub struct SettledHand {
    pub cards: String,
    pub value: u8,
    pub outcome: HandOutcome,
}

#[must_use]
pub fn final_board<'a>(
    title: &str,
    header: &str,
    hands: &[SettledHand],
    dealer: (&str, u8),
    summary: &str,
) -> Vec<CreateContainerComponent<'a>> {
    let (dealer_cards, dealer_value) = dealer;
    let split = hands.len() > 1;

    let mut components = vec![
        CreateContainerComponent::TextDisplay(CreateTextDisplay::new(format!(
            "### Blackjack - {title}\n{header}"
        ))),
        divider(),
    ];

    for (index, hand) in hands.iter().enumerate() {
        let (label, style) = hand.outcome.badge();

        components.push(hand_section(
            hand_id(index, HandState::Done),
            hand_heading(index, split),
            &hand.cards,
            hand.value,
            label,
            style,
        ));
    }

    let (dealer_label, dealer_style) = if dealer_value > 21 {
        ("Bust", ButtonStyle::Danger)
    } else {
        ("Stand", ButtonStyle::Secondary)
    };

    components.push(divider());
    components.push(hand_section(
        BlackjackCustomId::Dealer { state: HandState::Done },
        DEALER_HEADING,
        dealer_cards,
        dealer_value,
        dealer_label,
        dealer_style,
    ));
    components.push(divider());
    components.push(CreateContainerComponent::TextDisplay(CreateTextDisplay::new(
        summary.to_string(),
    )));

    components
}

pub fn final_response(
    components: Vec<CreateContainerComponent<'_>>,
    colour: Colour,
) -> EditInteractionResponse<'_> {
    EditInteractionResponse::new().flags(MessageFlags::IS_COMPONENTS_V2).components(
        vec![CreateComponent::Container(
            CreateContainer::new(components).accent_colour(colour),
        )],
    )
}

pub fn hit_button<'a>() -> CreateButton<'a> {
    CreateButton::new(BlackjackCustomId::Hit.to_string())
        .emoji('🎯')
        .label("Hit")
        .style(ButtonStyle::Secondary)
}

pub fn stand_button<'a>() -> CreateButton<'a> {
    CreateButton::new(BlackjackCustomId::Stand.to_string())
        .emoji('🛑')
        .label("Stand")
        .style(ButtonStyle::Secondary)
}

pub fn double_button<'a>() -> CreateButton<'a> {
    CreateButton::new(BlackjackCustomId::Double.to_string())
        .emoji('⏫')
        .label("Double Down")
        .style(ButtonStyle::Secondary)
}

pub fn split_button<'a>() -> CreateButton<'a> {
    CreateButton::new(BlackjackCustomId::Split.to_string())
        .emoji(ReactionType::Unicode(FixedString::from_static_trunc("✂️")))
        .label("Split")
        .style(ButtonStyle::Secondary)
}

pub fn surrender_button<'a>() -> CreateButton<'a> {
    CreateButton::new(BlackjackCustomId::Surrender.to_string())
        .emoji(ReactionType::Unicode(FixedString::from_static_trunc("🏳️")))
        .label("Surrender")
        .style(ButtonStyle::Danger)
}

struct GameOutcome {
    bet: i64,
    payout: i64,
    win: Option<bool>,
}

async fn game_end_common(
    ctx: &Context,
    pool: &PgPool,
    emojis: &EmojiCache,
    user_id: UserId,
    channel_id: GenericChannelId,
    outcome: GameOutcome,
) -> Result<(i64, i64, Vec<AppliedEffect>)> {
    let GameOutcome { bet, mut payout, win } = outcome;

    let mut row =
        GameRow::get(pool, user_id).await?.unwrap_or_else(|| GameRow::new(user_id));

    let before = row.clone();

    let dispatch = Dispatch::new(&ctx.http, pool, emojis);

    dispatch
        .fire(
            channel_id,
            &mut row,
            Event::Game(GameEvent::new(
                "blackjack",
                user_id,
                bet,
                payout,
                win == Some(true),
            )),
        )
        .await?;

    let payout_result =
        EffectsManager::payout(pool, user_id, "blackjack", bet, payout, win).await;
    payout = payout_result.payout;

    row.add_coins(payout);

    let delta = GameDelta::between(&before, &row);

    let coins = GameRow::commit(pool, user_id, &delta)
        .await?
        .ok_or(GamblingError::TransactionConflict)?
        .coins;

    Ok((payout, coins, payout_result.effects))
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

pub async fn game_end_draw<'a>(
    ctx: &Context,
    pool: &PgPool,
    emojis: &EmojiCache,
    user_id: UserId,
    channel_id: GenericChannelId,
    game: GameDetails,
    dealer_hand: &[EmojiId],
) -> Result<EditInteractionResponse<'a>> {
    let bet = game.bet();
    let dealer_value = sum_cards(emojis, dealer_hand)?;

    let (_, coins, _) =
        game_end_common(ctx, pool, emojis, user_id, channel_id, GameOutcome {
            bet,
            payout: bet,
            win: None,
        })
        .await?;

    let card_to_num = get_card_values(emojis)?;
    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let board = final_board(
        "Draw!",
        &format!("Your bet: {} <:coin:{coin}>", bet.format()),
        &[SettledHand {
            cards: game.player_hand_str(emojis)?,
            value: game.player_value(emojis)?,
            outcome: HandOutcome::Push,
        }],
        (&build_hand_str(card_to_num, dealer_hand)?, dealer_value),
        &format!(
            "Draw! Have your money back.\n\nYour coins: {} <:coin:{coin}>",
            coins.format()
        ),
    );

    Ok(final_response(board, Colour::DARKER_GREY))
}

pub async fn game_end_blackjack<'a>(
    ctx: &Context,
    pool: &PgPool,
    emojis: &EmojiCache,
    user_id: UserId,
    channel_id: GenericChannelId,
    game: GameDetails,
    dealer_hand: &[EmojiId],
) -> Result<EditInteractionResponse<'a>> {
    let bet = game.bet();
    let dealer_value = sum_cards(emojis, dealer_hand)?;

    let (payout, coins, effects) =
        game_end_common(ctx, pool, emojis, user_id, channel_id, GameOutcome {
            bet,
            payout: bet + (3 * bet) / 2,
            win: Some(true),
        })
        .await?;

    let card_to_num = get_card_values(emojis)?;
    let coin = emojis
        .emoji("heads")
        .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

    let board = final_board(
        "You Won!",
        &format!("Your bet: {} <:coin:{coin}>", bet.format()),
        &[SettledHand {
            cards: game.player_hand_str(emojis)?,
            value: game.player_value(emojis)?,
            outcome: HandOutcome::Won,
        }],
        (&build_hand_str(card_to_num, dealer_hand)?, dealer_value),
        &format!(
            "BLACKJACK!\n\nProfit: {} <:coin:{coin}>\nYour coins: {} <:coin:{coin}>{}",
            (payout - bet).format(),
            coins.format(),
            effects_summary(emojis, &effects),
        ),
    );

    Ok(final_response(board, Colour::DARK_GREEN))
}
