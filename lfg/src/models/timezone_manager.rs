use std::collections::HashMap;
use std::sync::LazyLock;

use async_trait::async_trait;
use chrono_tz::{America, Asia, Europe, Tz};
use serenity::all::UserId;
use sqlx::{Database, Pool};

pub static LOCALE_TO_TIMEZONE: LazyLock<HashMap<&'static str, chrono_tz::Tz>> =
    LazyLock::new(|| {
        HashMap::from([
            ("id", Asia::Jakarta),
            ("da", Europe::Copenhagen),
            ("de", Europe::Berlin),
            ("en-GB", Europe::London),
            ("es-ES", Europe::Madrid),
            ("es-419", America::Mexico_City),
            ("fr", Europe::Paris),
            ("hr", Europe::Zagreb),
            ("it", Europe::Rome),
            ("lt", Europe::Vilnius),
            ("hu", Europe::Budapest),
            ("nl", Europe::Amsterdam),
            ("no", Europe::Oslo),
            ("pl", Europe::Warsaw),
            ("pt-BR", America::Sao_Paulo),
            ("ro", Europe::Bucharest),
            ("fi", Europe::Helsinki),
            ("sv-SE", Europe::Stockholm),
            ("vi", Asia::Ho_Chi_Minh),
            ("tr", Europe::Istanbul),
            ("cs", Europe::Prague),
            ("el", Europe::Athens),
            ("bg", Europe::Sofia),
            ("ru", Europe::Moscow),
            ("uk", Europe::Kiev),
            ("hi", Asia::Kolkata),
            ("th", Asia::Bangkok),
            ("zh-CN", Asia::Shanghai),
            ("ja", Asia::Tokyo),
            ("zh-TW", Asia::Taipei),
            ("ko", Asia::Seoul),
        ])
    });

#[async_trait]
pub trait TimezoneManager<Db: Database> {
    async fn get(pool: &Pool<Db>, id: impl Into<UserId> + Send, local: &str) -> sqlx::Result<Tz>;

    async fn save(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
        tz: Tz,
    ) -> sqlx::Result<Db::QueryResult>;
}
