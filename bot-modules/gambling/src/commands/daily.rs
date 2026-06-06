use async_trait::async_trait;
use jiff::tz::TimeZone;
use jiff_sqlx::{Date, ToSqlx};
use serenity::all::{
    Colour,
    CommandInteraction,
    Context,
    CreateCommand,
    CreateEmbed,
    EditInteractionResponse,
    UserId,
};
use sqlx::{Database, FromRow, Pool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, as_i64};

use super::Commands;
use crate::{
    Coins,
    GamblingError,
    GamblingGoalsRow,
    Gems,
    GoalHandler,
    GoalsManager,
    MaxBet,
    Prestige,
    Result,
    START_AMOUNT,
    tomorrow,
};

#[async_trait]
pub trait DailyManager<Db: Database> {
    async fn daily_row(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<DailyRow>>;
    async fn goal_rows(
        pool: &Pool<Db>,
        id: UserId,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>>;
    async fn save(pool: &Pool<Db>, row: DailyRow) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct DailyRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub daily: Date,
    pub prestige: Option<i64>,
    pub level: Option<i32>,
}

impl DailyRow {
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            user_id: as_i64(id.get()),
            coins: 0,
            gems: 0,
            daily: jiff::civil::Date::default().to_sqlx(),
            prestige: Some(0),
            level: Some(0),
        }
    }
}

impl Coins for DailyRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for DailyRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Prestige for DailyRow {
    fn prestige(&self) -> i64 {
        self.prestige.unwrap_or_default()
    }
}

impl MaxBet for DailyRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

impl Commands {
    pub async fn daily<
        Data: EmojiCacheData,
        Db: Database,
        Manager: DailyManager<Db> + GoalsManager<Db>,
    >(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut row = Manager::daily_row(pool, interaction.user.id)
            .await
            .expect("async call")
            .unwrap_or_else(|| DailyRow::new(interaction.user.id));

        let now = jiff::Timestamp::now();
        let today = now.to_zoned(TimeZone::UTC).date();

        if row.daily.to_jiff() == today {
            return Err(GamblingError::DailyClaimed(tomorrow(Some(now))));
        }

        let amount = START_AMOUNT * (row.prestige.unwrap_or_default() + 1);

        *row.coins_mut() += amount;
        let mut goals = Manager::goal_rows(pool, interaction.user.id).await?;
        if goals.is_empty() || !goals.first().is_some_and(GamblingGoalsRow::is_today)
        {
            goals = GoalHandler::daily_reset::<Db, Manager>(
                pool,
                interaction.user.id,
                &row,
            )
            .await?;
        }

        let goals_str = goals
            .iter()
            .map(|goal| {
                if goal.is_complete() {
                    format!("\n- ~~{}~~", goal.title())
                } else {
                    format!(
                        "\n- {} (`{}/{}`)",
                        goal.title(),
                        goal.progress.format(),
                        goal.target.format()
                    )
                }
            })
            .collect::<String>();

        Manager::save(pool, row).await?;

        let coin = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis().emoji("heads").expect("emoji 'heads' in cache")
        };

        let embed = CreateEmbed::new()
            .description(format!(
                "**Collected {} <:coin:{coin}>**\n\n__Daily Goals__: {goals_str}",
                amount.format()
            ))
            .colour(Colour::GOLD);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register_daily<'a>() -> CreateCommand<'a> {
        CreateCommand::new("daily").description("Collect your daily coins")
    }
}
