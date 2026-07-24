use jiff::tz;
use jiff::tz::TimeZone;
use serenity::all::UserId;
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use zayden_core::as_i64;

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

pub struct UserSettings;

impl UserSettings {
    pub async fn get(
        pool: &PgPool,
        id: UserId,
        locale: &str,
    ) -> sqlx::Result<TimeZone> {
        let row = sqlx::query!(
            "SELECT timezone FROM lfg_user_settings WHERE id = $1",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await?;

        let tz_name = match row {
            Some(row) => row.timezone,
            None => locale_to_timezone(locale).to_string(),
        };

        Ok(tz::db().get(&tz_name).unwrap_or(TimeZone::UTC))
    }

    pub async fn save(
        pool: &PgPool,
        id: UserId,
        tz_name: &str,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO lfg_user_settings (id, timezone) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET timezone = $2",
            as_i64(id.get()),
            tz_name
        )
        .execute(pool)
        .await
    }
}
