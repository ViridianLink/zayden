use serenity::all::{
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
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, as_i64, parse_options};

use crate::events::{Dispatch, Event, SendEvent};
use crate::{
    Coins,
    Commands,
    GamblingError,
    GamblingManager,
    Gems,
    MaxBet,
    Prestige,
    Result,
    ShopCurrency,
    Stamina,
};

pub struct SendRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub stamina: i32,
    pub level: Option<i32>,
    pub prestige: i64,
}

impl SendRow {
    const fn new(id: UserId) -> Self {
        Self {
            user_id: as_i64(id.get()),
            coins: 0,
            gems: 0,
            stamina: 0,
            level: Some(0),
            prestige: 0,
        }
    }
}

impl Coins for SendRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for SendRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl MaxBet for SendRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

impl Stamina for SendRow {
    fn stamina(&self) -> i32 {
        self.stamina
    }

    fn stamina_mut(&mut self) -> &mut i32 {
        &mut self.stamina
    }
}

impl Prestige for SendRow {
    fn prestige(&self) -> i64 {
        self.prestige
    }
}

pub struct SendManager;

impl SendManager {
    pub async fn row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<SendRow>> {
        sqlx::query_as!(
            SendRow,
            r#"SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.stamina,
                COALESCE(l.level, 0) AS "level!: i32",
                COALESCE(m.prestige, 0) AS "prestige!: i64"
                FROM gambling g
                LEFT JOIN levels l ON g.user_id = l.user_id
                LEFT JOIN gambling_mine m on g.user_id = m.user_id
                WHERE g.user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn transfer(
        pool: &PgPool,
        sender: UserId,
        recipient: UserId,
        amount: i64,
    ) -> sqlx::Result<bool> {
        let mut tx = pool.begin().await?;

        let debit = sqlx::query!(
            "UPDATE gambling
            SET coins = coins - $2, stamina = GREATEST(stamina - 1, 0)
            WHERE user_id = $1 AND coins >= $2",
            as_i64(sender.get()),
            amount
        )
        .execute(&mut *tx)
        .await?;

        if debit.rows_affected() == 0 {
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

impl Commands {
    pub async fn send<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::User(recipient, _)) = options.remove("recipient")
        else {
            return Err(GamblingError::InvalidAmount);
        };

        if recipient.id == interaction.user.id {
            return Err(GamblingError::SelfSend);
        }

        let Some(ResolvedValue::Integer(amount)) = options.remove("amount") else {
            return Err(GamblingError::InvalidAmount);
        };

        if amount < 0 {
            return Err(GamblingError::NegativeAmount);
        }

        let mut row = SendManager::row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| SendRow::new(interaction.user.id));

        row.verify_work()?;

        if row.coins() < amount {
            return Err(GamblingError::InsufficientFunds {
                required: amount - row.coins(),
                currency: ShopCurrency::Coins,
            });
        }

        let max_send = row.max_bet();
        if amount > max_send {
            return Err(GamblingError::MaximumSendAmount(max_send));
        }

        if !SendManager::transfer(pool, interaction.user.id, recipient.id, amount)
            .await?
        {
            return Err(GamblingError::InsufficientFunds {
                required: amount,
                currency: ShopCurrency::Coins,
            });
        }

        *row.coins_mut() -= amount;
        row.done_work();

        let stamina = row.stamina_str();

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        let coins_before = row.coins();
        let gems_before = row.gems();

        Dispatch::new(&ctx.http, pool, &emojis)
            .fire(
                interaction.channel_id,
                &mut row,
                Event::Send(SendEvent::new(amount, interaction.user.id)),
            )
            .await?;

        let coin_reward = row.coins() - coins_before;
        let gem_reward = row.gems() - gems_before;
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

        let coin = emojis.emoji("heads").map_err(|n| {
            GamblingError::Internal(format!("emoji '{n}' not in cache"))
        })?;

        let embed = CreateEmbed::new().description(format!(
            "You sent {} <:coin:{coin}> to {}\nStamina: {stamina}",
            amount.format(),
            recipient.mention()
        ));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register_send<'a>() -> CreateCommand<'a> {
        CreateCommand::new("send")
            .description("Send another player some of your coins")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "recipient",
                    "The player recieving the coins",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "amount",
                    "The amount to send",
                )
                .required(true),
            )
    }
}
