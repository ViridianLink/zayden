use async_trait::async_trait;
use jiff_sqlx::{Timestamp, ToSqlx};
use sqlx::{Database, FromRow, Pool};

#[async_trait]
pub trait GoldStarManager<Db: Database> {
    async fn get_row(
        pool: &Pool<Db>,
        user_id: impl Into<i64> + Send,
    ) -> sqlx::Result<Option<GoldStarRow>>;

    async fn save_row(pool: &Pool<Db>, row: &GoldStarRow) -> sqlx::Result<()>;
}

#[derive(FromRow)]
pub struct GoldStarRow {
    pub id: i64,
    pub number_of_stars: i32,
    pub given_stars: i32,
    pub received_stars: i32,
    pub last_free_star: Timestamp,
}

impl GoldStarRow {
    pub fn new(user_id: impl Into<i64>) -> Self {
        Self {
            id: user_id.into(),
            number_of_stars: 0,
            given_stars: 0,
            received_stars: 0,
            last_free_star: jiff::Timestamp::default().to_sqlx(),
        }
    }

    pub fn give_star(&mut self, reciever: &mut GoldStarRow) {
        self.given_stars += 1;
        self.number_of_stars -= 1;

        reciever.number_of_stars += 1;
        reciever.received_stars += 1;
    }

    pub fn give_free_star(&mut self, reciever: &mut GoldStarRow) {
        self.given_stars += 1;
        self.last_free_star = jiff::Timestamp::now().to_sqlx();

        reciever.number_of_stars += 1;
        reciever.received_stars += 1;
    }

    pub async fn save<Db: Database, Manager: GoldStarManager<Db>>(
        &self,
        pool: &Pool<Db>,
    ) -> sqlx::Result<()> {
        Manager::save_row(pool, self).await?;
        Ok(())
    }
}
