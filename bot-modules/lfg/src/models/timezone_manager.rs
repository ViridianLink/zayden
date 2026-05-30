use async_trait::async_trait;
use jiff::tz::TimeZone;
use serenity::all::UserId;
use sqlx::{Database, Pool};

#[must_use]
pub fn locale_to_timezone(locale: &str) -> &'static str {
    match locale {
        "id" => "Asia/Jakarta",
        "da" => "Europe/Copenhagen",
        "de" => "Europe/Berlin",
        "en-GB" => "Europe/London",
        "es-ES" => "Europe/Madrid",
        "es-419" => "America/Mexico_City",
        "fr" => "Europe/Paris",
        "hr" => "Europe/Zagreb",
        "it" => "Europe/Rome",
        "lt" => "Europe/Vilnius",
        "hu" => "Europe/Budapest",
        "nl" => "Europe/Amsterdam",
        "no" => "Europe/Oslo",
        "pl" => "Europe/Warsaw",
        "pt-BR" => "America/Sao_Paulo",
        "ro" => "Europe/Bucharest",
        "fi" => "Europe/Helsinki",
        "sv-SE" => "Europe/Stockholm",
        "vi" => "Asia/Ho_Chi_Minh",
        "tr" => "Europe/Istanbul",
        "cs" => "Europe/Prague",
        "el" => "Europe/Athens",
        "bg" => "Europe/Sofia",
        "ru" => "Europe/Moscow",
        "uk" => "Europe/Kyiv",
        "hi" => "Asia/Kolkata",
        "th" => "Asia/Bangkok",
        "zh-CN" => "Asia/Shanghai",
        "ja" => "Asia/Tokyo",
        "zh-TW" => "Asia/Taipei",
        "ko" => "Asia/Seoul",
        _ => "UTC", // Default fallback
    }
}

#[async_trait]
pub trait TimezoneManager<Db: Database> {
    async fn get(pool: &Pool<Db>, id: UserId, local: &str)
    -> sqlx::Result<TimeZone>;

    async fn save(
        pool: &Pool<Db>,
        id: impl Into<UserId> + Send,
        tz_name: &str,
    ) -> sqlx::Result<Db::QueryResult>;
}
