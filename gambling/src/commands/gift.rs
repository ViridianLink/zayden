use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use serenity::all::{
    Colour, CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption, CreateEmbed,
    EditInteractionResponse, Http, Mentionable, ResolvedOption, ResolvedValue, UserId,
};
use sqlx::{Database, Pool, prelude::FromRow};
use zayden_core::FormatNum;

use crate::{
    Coins, Error, GamblingManager, Gems, GoalsManager, MaxBet, Prestige, Result, START_AMOUNT,
    events::{Dispatch, Event, SendEvent},
    tomorrow,
};

const GIFT_AMOUNT: i64 = (START_AMOUNT as f64 * 2.5) as i64;

use super::Commands;

#[async_trait]
pub trait GiftManager<Db: Database> {
    async fn sender(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<SenderRow>>;

    async fn save_sender(pool: &Pool<Db>, row: SenderRow) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct SenderRow {
    pub id: i64,
    pub coins: i64,
    pub gems: i64,
    pub gift: NaiveDate,
    pub level: Option<i32>,
    pub prestige: i64,
}

impl SenderRow {
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            id: id.get() as i64,
            coins: 0,
            gems: 0,
            gift: NaiveDate::default(),
            level: Some(0),
            prestige: 0,
        }
    }
}

impl Coins for SenderRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for SenderRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Prestige for SenderRow {
    fn prestige(&self) -> i64 {
        self.prestige
    }
}

impl MaxBet for SenderRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

#[derive(FromRow)]
pub struct RecipientRow {
    pub id: i64,
    pub coins: i64,
}

impl RecipientRow {
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            id: id.get() as i64,
            coins: 0,
        }
    }
}

impl Coins for RecipientRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Commands {
    pub async fn gift<
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        GoalsHandler: GoalsManager<Db>,
        GiftHandler: GiftManager<Db>,
    >(
        http: &Http,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await.unwrap();

        let ResolvedValue::User(recipient, _) = options[0].value else {
            unreachable!("recipient is required")
        };

        if recipient == &interaction.user {
            return Err(Error::SelfGift);
        }

        let mut user_row = GiftHandler::sender(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| SenderRow::new(interaction.user.id));

        let now = Utc::now();

        if user_row.gift == now.naive_utc().date() {
            return Err(Error::GiftUsed(tomorrow(Some(now))));
        }

        let amount = GIFT_AMOUNT * (user_row.prestige + 1);

        let mut tx = pool.begin().await.unwrap();

        GamblingHandler::add_coins(&mut *tx, recipient.id, amount)
            .await
            .unwrap();

        tx.commit().await.unwrap();

        Dispatch::<Db, GoalsHandler>::new(http, pool)
            .fire(
                interaction.channel_id,
                &mut user_row,
                Event::Send(SendEvent::new(amount, interaction.user.id)),
            )
            .await?;

        GiftHandler::save_sender(pool, user_row).await.unwrap();

        let embed = CreateEmbed::new()
            .description(format!(
                "🎁 You sent a gift of {} to {}",
                amount.format(),
                recipient.mention()
            ))
            .colour(Colour::GOLD);

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_gift<'a>() -> CreateCommand<'a> {
        CreateCommand::new("gift")
            .description("Send a free gift to a user!")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "recipient",
                    "The user to receive the free gift",
                )
                .required(true),
            )
    }
}
