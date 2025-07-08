use async_trait::async_trait;
use gambling::{GamblingManager, GameManager, GameRow};
use serenity::all::{Context, CreateCommand, UserId};
use sqlx::{PgConnection, PgPool, Postgres, postgres::PgQueryResult};
use zayden_core::SlashCommand;

mod blackjack;
mod coinflip;
mod craft;
mod daily;
mod dig;
mod effects;
mod gift;
mod goals;
mod higher_lower;
mod inventory;
mod leaderboard;
mod lotto;
mod mine;
mod prestige;
mod profile;
mod roll;
mod rps;
mod send;
mod shop;
mod stamina;
mod tictactoe;
mod work;

pub use blackjack::Blackjack;
pub use coinflip::Coinflip;
pub use craft::Craft;
pub use daily::Daily;
pub use dig::Dig;
pub use effects::EffectsTable;
pub use gift::Gift;
pub use goals::{Goals, GoalsTable};
pub use higher_lower::HigherLower;
pub use inventory::Inventory;
pub use leaderboard::Leaderboard;
pub use lotto::{Lotto, LottoTable};
pub use mine::{Mine, MineTable};
pub use prestige::Prestige;
pub use profile::Profile;
pub use roll::Roll;
pub use rps::RockPaperScissors;
pub use send::Send;
pub use shop::Shop;
pub use stamina::StaminaTable;
pub use tictactoe::TicTacToe;
pub use work::Work;

pub fn register(ctx: &Context) -> [CreateCommand; 20] {
    [
        Blackjack::register(ctx).unwrap(),
        Coinflip::register(ctx).unwrap(),
        Craft::register(ctx).unwrap(),
        Daily::register(ctx).unwrap(),
        Dig::register(ctx).unwrap(),
        Gift::register(ctx).unwrap(),
        Goals::register(ctx).unwrap(),
        HigherLower::register(ctx).unwrap(),
        Inventory::register(ctx).unwrap(),
        Leaderboard::register(ctx).unwrap(),
        Lotto::register(ctx).unwrap(),
        Mine::register(ctx).unwrap(),
        Prestige::register(ctx).unwrap(),
        Profile::register(ctx).unwrap(),
        Roll::register(ctx).unwrap(),
        RockPaperScissors::register(ctx).unwrap(),
        Send::register(ctx).unwrap(),
        Shop::register(ctx).unwrap(),
        TicTacToe::register(ctx).unwrap(),
        Work::register(ctx).unwrap(),
    ]
}

pub struct GamblingTable;

#[async_trait]
impl GamblingManager<Postgres> for GamblingTable {
    async fn coins(
        conn: &mut PgConnection,
        id: impl Into<UserId> + std::marker::Send,
    ) -> sqlx::Result<i64> {
        let id = id.into();

        sqlx::query_file_scalar!("./sql/gambling/GamblingManager/coins.sql", id.get() as i64)
            .fetch_one(conn)
            .await
    }

    async fn max_bet(
        conn: &mut PgConnection,
        id: impl Into<UserId> + std::marker::Send,
    ) -> sqlx::Result<i64> {
        let id = id.into();

        sqlx::query_scalar!(
            r#"
            SELECT
                (
                    GREATEST(l.level * 10000, 10000)
                    * (COALESCE(m.prestige, 0) + 10)
                ) / 10
            FROM
                levels l
            LEFT JOIN
                gambling_mine m ON l.id = m.id
            WHERE
                l.id = $1
            "#,
            id.get() as i64
        )
        .fetch_one(conn)
        .await
        .map(|r| r.unwrap())
    }

    //region: Update
    async fn bet(
        pool: &PgPool,
        id: impl Into<UserId> + std::marker::Send,
        bet: i64,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query_file!(
            "./sql/gambling/GamblingManager/bet.sql",
            id.get() as i64,
            bet
        )
        .execute(pool)
        .await
    }

    async fn add_coins(
        conn: &mut PgConnection,
        id: impl Into<UserId> + std::marker::Send,
        amount: i64,
    ) -> sqlx::Result<PgQueryResult> {
        let id = id.into();

        sqlx::query_file!(
            "./sql/gambling/GamblingManager/add_coins.sql",
            id.get() as i64,
            amount
        )
        .execute(conn)
        .await
    }
}

pub struct GameTable;

#[async_trait]
impl GameManager<Postgres> for GameTable {
    async fn row(
        pool: &PgPool,
        id: impl Into<UserId> + std::marker::Send,
    ) -> sqlx::Result<Option<GameRow>> {
        let id = id.into();

        sqlx::query_file_as!(
            GameRow,
            "./sql/gambling/GameManager/row.sql",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn save(pool: &PgPool, row: GameRow) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO gambling (id, coins, gems)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems;",
            row.id,
            row.coins,
            row.gems,
        )
        .execute(pool)
        .await
    }
}
