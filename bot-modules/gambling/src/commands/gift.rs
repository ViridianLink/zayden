use jiff::tz::TimeZone;
use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::{
    Colour,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    Mentionable,
    ResolvedOption,
    ResolvedValue,
    UserId,
};
use sqlx::PgPool;
use sqlx::prelude::FromRow;
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, as_i64};

use crate::events::{Dispatch, Event, SendEvent};
use crate::{
    Coins,
    GamblingError,
    GamblingManager,
    Gems,
    MaxBet,
    Prestige,
    Result,
    START_AMOUNT,
    tomorrow,
};

const GIFT_AMOUNT: i64 = START_AMOUNT * 5 / 2;

use super::Commands;

pub struct GiftManager;

impl GiftManager {
    pub async fn sender(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<Option<SenderRow>> {
        sqlx::query_as!(
            SenderRow,
            r#"SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.gift as "gift: jiff_sqlx::Timestamp",

                COALESCE(l.level, 0) AS "level!",
                
                m.prestige

                FROM gambling g
                LEFT JOIN levels l ON g.user_id = l.user_id
                LEFT JOIN gambling_mine m on g.user_id = m.user_id
                WHERE g.user_id = $1;"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn claim(
        pool: &PgPool,
        sender: UserId,
        recipient: UserId,
        amount: i64,
    ) -> sqlx::Result<bool> {
        let mut tx = pool.begin().await?;

        let claim = sqlx::query!(
            "INSERT INTO gambling (user_id, gift)
            VALUES ($1, CURRENT_DATE)
            ON CONFLICT (user_id) DO UPDATE SET gift = CURRENT_DATE
            WHERE gambling.gift < CURRENT_DATE",
            as_i64(sender.get())
        )
        .execute(&mut *tx)
        .await?;

        if claim.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query_file!(
            "sql/GamblingManager/add_coins.sql",
            as_i64(recipient.get()),
            amount
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(true)
    }
}

#[derive(FromRow)]
pub struct SenderRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub gift: Timestamp,
    pub level: Option<i32>,
    pub prestige: Option<i64>,
}

impl SenderRow {
    #[must_use]
    pub fn new(id: UserId) -> Self {
        Self {
            user_id: as_i64(id.get()),
            coins: 0,
            gems: 0,
            gift: jiff::Timestamp::default().to_sqlx(),
            level: Some(0),
            prestige: Some(0),
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
        self.prestige.unwrap_or_default()
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
    #[must_use]
    pub const fn new(id: UserId) -> Self {
        Self { id: as_i64(id.get()), coins: 0 }
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
    pub async fn gift<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let Some(option) = options.first() else {
            return Err(GamblingError::InvalidAmount);
        };
        let ResolvedValue::User(recipient, _) = option.value else {
            return Err(GamblingError::InvalidAmount);
        };

        if recipient == &interaction.user {
            return Err(GamblingError::SelfGift);
        }

        let mut user_row = GiftManager::sender(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| SenderRow::new(interaction.user.id));

        let now = jiff::Timestamp::now().to_zoned(TimeZone::UTC);

        if user_row.gift.to_jiff().to_zoned(TimeZone::UTC).date() == now.date() {
            return Err(GamblingError::GiftUsed(tomorrow(Some(now.timestamp()))?));
        }

        let amount = GIFT_AMOUNT * (user_row.prestige() + 1);

        if !GiftManager::claim(pool, interaction.user.id, recipient.id, amount)
            .await?
        {
            return Err(GamblingError::GiftUsed(tomorrow(Some(now.timestamp()))?));
        }

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let coins_before = user_row.coins();
        let gems_before = user_row.gems();

        Dispatch::new(&ctx.http, pool, &emojis)
            .fire(
                interaction.channel_id,
                &mut user_row,
                Event::Send(SendEvent::new(amount, interaction.user.id)),
            )
            .await?;

        let coin_reward = user_row.coins() - coins_before;
        let gem_reward = user_row.gems() - gems_before;
        if coin_reward != 0 || gem_reward != 0 {
            let mut tx = pool.begin().await?;
            if coin_reward != 0 {
                GamblingManager::add_coins(
                    &mut tx,
                    interaction.user.id,
                    coin_reward,
                )
                .await?;
            }
            if gem_reward != 0 {
                GamblingManager::add_gems(&mut tx, interaction.user.id, gem_reward)
                    .await?;
            }
            tx.commit().await?;
        }

        let embed = CreateEmbed::new()
            .description(format!(
                "🎁 You sent a gift of {} to {}",
                amount.format(),
                recipient.mention()
            ))
            .colour(Colour::GOLD);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

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
