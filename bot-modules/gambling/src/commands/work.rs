use jiff_sqlx::{Timestamp, ToSqlx};
use serenity::all::{
    Colour,
    CommandInteraction,
    Context,
    CreateCommand,
    CreateEmbed,
    EditInteractionResponse,
    UserId,
};
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use sqlx::prelude::FromRow;
use tokio::sync::RwLock;
use zayden_core::{EmojiCacheData, FormatNum, as_i64};

use super::Commands;
use crate::events::{Dispatch, Event};
use crate::models::MineAmount;
use crate::{
    Coins,
    GamblingError,
    Gems,
    MaxBet,
    MineHourly,
    Prestige,
    Result,
    Stamina,
};

#[derive(Debug, FromRow)]
pub struct WorkRow {
    pub user_id: i64,
    pub coins: i64,
    pub gems: i64,
    pub stamina: i32,
    pub level: Option<i32>,
    pub miners: Option<i64>,
    pub prestige: Option<i64>,
    pub mine_activity: Option<Timestamp>,
}

impl WorkRow {
    fn new(id: UserId) -> Self {
        Self {
            user_id: as_i64(id.get()),
            coins: 0,
            gems: 0,
            stamina: 3,
            level: Some(0),
            miners: Some(0),
            prestige: Some(0),
            mine_activity: Some(jiff::Timestamp::now().to_sqlx()),
        }
    }
}

impl Coins for WorkRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

impl Gems for WorkRow {
    fn gems(&self) -> i64 {
        self.gems
    }

    fn gems_mut(&mut self) -> &mut i64 {
        &mut self.gems
    }
}

impl Stamina for WorkRow {
    fn stamina(&self) -> i32 {
        self.stamina
    }

    fn stamina_mut(&mut self) -> &mut i32 {
        &mut self.stamina
    }
}

impl MaxBet for WorkRow {
    fn level(&self) -> i32 {
        self.level.unwrap_or_default()
    }
}

impl MineHourly for WorkRow {
    fn miners(&self) -> i64 {
        self.miners.unwrap_or_default()
    }
}

impl MineAmount for WorkRow {
    fn mine_activity(&self) -> jiff::Timestamp {
        self.mine_activity.map_or_else(jiff::Timestamp::now, Timestamp::to_jiff)
    }
}

impl Prestige for WorkRow {
    fn prestige(&self) -> i64 {
        self.prestige.unwrap_or_default()
    }
}

pub struct WorkManager;

impl WorkManager {
    pub async fn row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<WorkRow>> {
        sqlx::query_as!(
            WorkRow,
            r#"SELECT
                g.user_id,
                g.coins,
                g.gems,
                g.stamina,

                COALESCE(l.level, 0) AS level,
                
                COALESCE(m.miners, 0) AS miners,
                COALESCE(m.prestige, 0) AS prestige,
                COALESCE(m.mine_activity, now()::TIMESTAMP) AS "mine_activity: jiff_sqlx::Timestamp"

                FROM gambling g
                LEFT JOIN levels l ON g.user_id = l.user_id
                LEFT JOIN gambling_mine m on g.user_id = m.user_id
                WHERE g.user_id = $1;"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    #[expect(
        trivial_casts,
        reason = "sqlx requires explicit type for jiff_sqlx TIMESTAMPTZ mapping"
    )]
    pub async fn save(pool: &PgPool, row: WorkRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems, stamina)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems, stamina = EXCLUDED.stamina;",
            row.user_id,
            row.coins,
            row.gems,
            row.stamina
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            "INSERT INTO gambling_mine (user_id, mine_activity)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET
            mine_activity = EXCLUDED.mine_activity;",
            row.user_id,
            row.mine_activity as Option<Timestamp>,
        )
        .execute(&mut *tx)
        .await?;

        result.extend([result2]);

        tx.commit().await?;

        Ok(result)
    }
}

impl Commands {
    pub async fn work<Data: EmojiCacheData>(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let mut row = WorkManager::row(pool, interaction.user.id)
            .await?
            .unwrap_or_else(|| WorkRow::new(interaction.user.id));

        row.verify_work()?;

        let base_amount = rand::random_range(100..=500);
        let mine_amount = row.mine_amount()?;
        let total_amount = base_amount + mine_amount;

        *row.coins_mut() += total_amount;

        let gem_desc = if rand::random_bool(1.0 / 100.0) {
            row.add_gems(1);
            "\n💎 You found a GEM!"
        } else {
            ""
        };

        let coins = row.coins_str();

        let emojis = {
            let data_lock = ctx.data::<RwLock<Data>>();
            let data = data_lock.read().await;
            data.emojis()
        };

        Dispatch::new(&ctx.http, pool, &emojis)
            .fire(interaction.channel_id, &mut row, Event::Work(interaction.user.id))
            .await?;

        row.done_work();
        row.mine_activity = Some(jiff::Timestamp::now().to_sqlx());

        let stamina = row.stamina_str();

        WorkManager::save(pool, row).await?;

        let coin = emojis.emoji("heads").map_err(|n| {
            GamblingError::Internal(format!("emoji '{n}' not in cache"))
        })?;

        let embed = CreateEmbed::new()
            .description(format!(
                "Collected {} <:coin:{coin}> for working{gem_desc}\nYour coins: {coins}\nStamina: {stamina}", total_amount.format()
            ))
            .colour(Colour::GOLD);

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register_work<'a>() -> CreateCommand<'a> {
        CreateCommand::new("work")
            .description("Do some work and get some quick coins")
    }
}
