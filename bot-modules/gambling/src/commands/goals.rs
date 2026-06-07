use std::fmt::Write as _;

use async_trait::async_trait;
use serenity::all::{
    CommandInteraction,
    Context,
    CreateCommand,
    CreateEmbed,
    EditInteractionResponse,
    UserId,
};
use sqlx::{Database, FromRow, Pool};
use tokio::sync::RwLock;
use zayden_core::EmojiCacheData;

use super::Commands;
use crate::{
    Coins,
    GamblingError,
    GamblingGoalsRow,
    Gems,
    GoalHandler,
    MaxBet,
    Prestige,
    Result,
    tomorrow,
};

#[async_trait]
pub trait GoalsManager<Db: Database> {
    async fn row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<GoalsRow>>;

    async fn full_rows(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>>;

    async fn update(
        pool: &Pool<Db>,
        rows: &[GamblingGoalsRow],
    ) -> sqlx::Result<Vec<GamblingGoalsRow>>;
}

#[derive(FromRow, Default)]
pub struct GoalsRow {
    pub coins: i64,
    pub gems: i64,
    pub level: Option<i32>,
    pub prestige: Option<i64>,
}

impl Coins for GoalsRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for GoalsRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Prestige for GoalsRow {
    fn prestige(&self) -> i64 {
        self.prestige.unwrap_or_default()
    }
}

impl MaxBet for GoalsRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

impl Commands {
    pub async fn goals<
        Data: EmojiCacheData,
        Db: Database,
        Manager: GoalsManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let row = Manager::row(pool, interaction.user.id).await?.unwrap_or_default();

        let mut desc = GoalHandler::get_user_progress::<Db, Manager>(
            pool,
            interaction.user.id,
            &row,
        )
        .await?
        .into_iter()
        .fold(String::new(), |mut acc, goal| {
            let _ = write!(acc, "{}\n\n", goal.description());
            acc
        });

        let (coin, reset_ts) = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            let coin = data.emojis().emoji("heads").map_err(|n| {
                GamblingError::Internal(format!("emoji '{n}' not in cache"))
            })?;
            drop(data);
            let reset_ts = tomorrow(None)?;

            (coin, reset_ts)
        };

        let _ = write!(
            desc,
            "Reward for completing __**each goals**__: 5,000 <:coin:{coin}>\nReward for completing __**all goals**__: 1 💎\n\nGoals reset <t:{reset_ts}:R>",
        );

        let embed = CreateEmbed::new().title("Daily Goals 📋").description(desc);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register_goals<'a>() -> CreateCommand<'a> {
        CreateCommand::new("goals").description("Show your daily goal progress")
    }
}
