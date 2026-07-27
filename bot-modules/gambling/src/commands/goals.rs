use std::fmt::Write as _;

use jiff_sqlx::Date;
use serenity::all::{
    CommandInteraction,
    Context,
    CreateCommand,
    CreateEmbed,
    EditInteractionResponse,
    UserId,
};
use sqlx::{FromRow, PgPool};
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, as_i64};

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

pub struct GoalsManager;

impl GoalsManager {
    pub async fn row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<GoalsRow>> {
        sqlx::query_as!(
            GoalsRow,
            "SELECT
                g.coins,
                g.gems,

                COALESCE(l.level, 0) AS level,
                
                COALESCE(m.prestige, 0) AS prestige

                FROM gambling g
                LEFT JOIN levels l ON g.user_id = l.user_id
                LEFT JOIN gambling_mine m on g.user_id = m.user_id
                WHERE g.user_id = $1;",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn full_rows(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        sqlx::query_as!(
            GamblingGoalsRow,
            r#"SELECT user_id, goal_id, day as "day: jiff_sqlx::Date", progress, target FROM gambling_goals WHERE user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        rows: &[GamblingGoalsRow],
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        let user_id = match rows.first() {
            Some(row) => row.user_id,
            None => return Ok(Vec::new()),
        };

        let mut tx = pool.begin().await?;

        sqlx::query!("DELETE FROM gambling_goals WHERE user_id = $1", user_id)
            .execute(&mut *tx)
            .await?;

        let num_rows = rows.len();
        let mut user_ids: Vec<i64> = Vec::with_capacity(num_rows);
        let mut goal_ids: Vec<String> = Vec::with_capacity(num_rows);
        let mut days: Vec<Date> = Vec::with_capacity(num_rows);
        let mut progresses: Vec<i64> = Vec::with_capacity(num_rows);
        let mut targets: Vec<i64> = Vec::with_capacity(num_rows);

        for row in rows {
            user_ids.push(row.user_id);
            goal_ids.push(row.goal_id.clone());
            days.push(row.day);
            progresses.push(row.progress);
            targets.push(row.target);
        }

        #[expect(
            trivial_casts,
            reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because DATE[] has no built-in jiff mapping"
        )]
        let rows = sqlx::query_as!(
            GamblingGoalsRow,
            r#"INSERT INTO gambling_goals (user_id, goal_id, day, progress, target)
            SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::date[], $4::bigint[], $5::bigint[])
            RETURNING user_id, goal_id, day as "day: jiff_sqlx::Date", progress, target;"#,
            &user_ids,
            &goal_ids,
            &days as &[Date],
            &progresses,
            &targets
        )
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(rows)
    }
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
    pub async fn goals<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let row =
            GoalsManager::row(pool, interaction.user.id).await?.unwrap_or_default();

        let mut desc =
            GoalHandler::get_user_progress(pool, interaction.user.id, &row)
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
