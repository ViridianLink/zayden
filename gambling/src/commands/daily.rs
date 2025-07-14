use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use serenity::all::{
    Colour, CommandInteraction, CreateCommand, CreateEmbed, EditInteractionResponse, Http, UserId,
};
use sqlx::{Database, Pool, prelude::FromRow};
use zayden_core::FormatNum;

use crate::{COIN, Coins, Error, Result, START_AMOUNT, tomorrow};

use super::Commands;

#[async_trait]
pub trait DailyManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: impl Into<UserId> + Send) -> sqlx::Result<Option<DailyRow>>;

    async fn save(pool: &Pool<Db>, row: DailyRow) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct DailyRow {
    pub id: i64,
    pub coins: i64,
    pub daily: NaiveDate,
    pub prestige: Option<i64>,
}

impl DailyRow {
    pub fn new(id: impl Into<UserId>) -> Self {
        let id = id.into();

        Self {
            id: id.get() as i64,
            coins: 0,
            daily: NaiveDate::default(),
            prestige: Some(0),
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

impl Commands {
    pub async fn daily<Db: Database, Manager: DailyManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer(http).await.unwrap();

        let mut row = Manager::row(pool, interaction.user.id)
            .await
            .unwrap()
            .unwrap_or_else(|| DailyRow::new(interaction.user.id));

        let now = Utc::now();
        let today = now.naive_utc().date();

        if row.daily == today {
            return Err(Error::DailyClaimed(tomorrow(Some(now))));
        }

        let amount = START_AMOUNT * (row.prestige.unwrap_or_default() + 1);

        *row.coins_mut() += amount;

        Manager::save(pool, row).await.unwrap();

        let embed = CreateEmbed::new()
            .description(format!("Collected {} <:coin:{COIN}>", amount.format()))
            .colour(Colour::GOLD);

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await
            .unwrap();

        Ok(())
    }

    pub fn register_daily<'a>() -> CreateCommand<'a> {
        CreateCommand::new("daily").description("Collect your daily coins")
    }
}
