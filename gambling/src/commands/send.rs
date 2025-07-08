use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, EditInteractionResponse, Mentionable, ResolvedOption, ResolvedValue, UserId,
};
use sqlx::any::AnyQueryResult;
use sqlx::{Database, Pool};
use zayden_core::{FormatNum, parse_options};

use crate::events::{Dispatch, Event, SendEvent};
use crate::{
    COIN, Coins, Commands, Error, GamblingManager, Gems, GoalsManager, MaxBet, Prestige, Result,
    ShopCurrency, Stamina, StaminaManager,
};

pub struct SendRow {
    pub id: i64,
    pub coins: i64,
    pub gems: i64,
    pub stamina: i32,
    pub level: Option<i32>,
    pub prestige: i64,
}

impl SendRow {
    fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            id: id.get() as i64,
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

#[async_trait]
pub trait SendManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: impl Into<UserId> + Send) -> sqlx::Result<Option<SendRow>>;

    async fn save(pool: &Pool<Db>, row: SendRow) -> sqlx::Result<AnyQueryResult>;
}

impl Commands {
    pub async fn send<
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        StaminaHandler: StaminaManager<Db>,
        GoalHandler: GoalsManager<Db>,
        SendHandler: SendManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(ctx).await?;

        let mut options = parse_options(options);

        let Some(ResolvedValue::User(recipient, _)) = options.remove("recipient") else {
            unreachable!("recipient is required");
        };

        if recipient.id == interaction.user.id {
            return Err(Error::SelfSend);
        }

        let Some(ResolvedValue::Integer(amount)) = options.remove("amount") else {
            unreachable!("amount is required");
        };

        if amount < 0 {
            return Err(Error::NegativeAmount);
        }

        let mut row = match SendHandler::row(pool, interaction.user.id).await.unwrap() {
            Some(row) => row,
            None => SendRow::new(interaction.user.id),
        };

        row.verify_work::<Db, StaminaHandler>()?;

        if row.coins() < amount {
            return Err(Error::InsufficientFunds {
                required: amount - row.coins(),
                currency: ShopCurrency::Coins,
            });
        }

        let max_send = row.max_bet();
        if amount > max_send {
            return Err(Error::MaximumSendAmount(max_send));
        }

        *row.coins_mut() -= amount;

        let mut tx = pool.begin().await.unwrap();

        GamblingHandler::add_coins(&mut *tx, recipient.id, amount).await?;

        tx.commit().await.unwrap();

        row.done_work();

        let stamina = row.stamina_str();

        Dispatch::<Db, GoalHandler>::new(pool)
            .fire(
                &mut row,
                Event::Send(SendEvent::new(amount, interaction.user.id)),
            )
            .await
            .unwrap();

        SendHandler::save(pool, row).await?;

        let embed = CreateEmbed::new().description(format!(
            "You sent {} <:coin:{COIN}> to {}\nStamina: {stamina}",
            amount.format(),
            recipient.mention()
        ));

        interaction
            .edit_response(ctx, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_send() -> CreateCommand {
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
