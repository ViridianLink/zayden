use sqlx::PgPool;

use crate::config::SettingsRow;
use crate::entitlement::Tier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cooldowns {
    pub user_secs: i32,
    pub guild_secs: i32,
}

impl Cooldowns {
    #[must_use]
    pub const fn clamp_to(self, floor: Self) -> Self {
        Self {
            user_secs: GreetingsSettingsRow::clamp_cooldown(
                self.user_secs,
                floor.user_secs,
            ),
            guild_secs: GreetingsSettingsRow::clamp_cooldown(
                self.guild_secs,
                floor.guild_secs,
            ),
        }
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GreetingsSettingsRow {
    pub guild_id: i64,
    pub morning_message: Option<String>,
    pub night_message: Option<String>,
    pub user_cooldown_secs: i32,
    pub guild_cooldown_secs: i32,
}

impl From<&GreetingsSettingsRow> for Cooldowns {
    fn from(row: &GreetingsSettingsRow) -> Self {
        Self {
            user_secs: row.user_cooldown_secs,
            guild_secs: row.guild_cooldown_secs,
        }
    }
}

impl GreetingsSettingsRow {
    pub const FREE_FLOORS: Cooldowns = Cooldowns { user_secs: 15, guild_secs: 3 };
    pub const MAX_COOLDOWN_SECS: i32 = 24 * 60 * 60;
    pub const PRO_FLOORS: Cooldowns = Cooldowns { user_secs: 3, guild_secs: 1 };
    pub const ULTRA_FLOORS: Cooldowns = Cooldowns { user_secs: 0, guild_secs: 0 };

    #[must_use]
    pub const fn floors_for(tier: Tier) -> Cooldowns {
        match tier {
            Tier::Free => Self::FREE_FLOORS,
            Tier::Pro => Self::PRO_FLOORS,
            Tier::Ultra => Self::ULTRA_FLOORS,
        }
    }

    #[must_use]
    pub const fn clamp_cooldown(requested: i32, floor: i32) -> i32 {
        if requested < floor {
            floor
        } else if requested > Self::MAX_COOLDOWN_SECS {
            Self::MAX_COOLDOWN_SECS
        } else {
            requested
        }
    }
}

impl SettingsRow for GreetingsSettingsRow {
    const TABLE: &'static str = "greetings_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            morning_message: None,
            night_message: None,
            user_cooldown_secs: Self::FREE_FLOORS.user_secs,
            guild_cooldown_secs: Self::FREE_FLOORS.guild_secs,
        }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT
                guild_id,
                morning_message,
                night_message,
                user_cooldown_secs,
                guild_cooldown_secs
            FROM greetings_settings
            WHERE guild_id = $1
            "#,
            guild_id
        )
        .fetch_optional(pool)
        .await
    }

    async fn upsert(&self, pool: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            INSERT INTO greetings_settings (
                guild_id, morning_message, night_message,
                user_cooldown_secs, guild_cooldown_secs
            )
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (guild_id) DO UPDATE SET
                morning_message = EXCLUDED.morning_message,
                night_message = EXCLUDED.night_message,
                user_cooldown_secs = EXCLUDED.user_cooldown_secs,
                guild_cooldown_secs = EXCLUDED.guild_cooldown_secs,
                updated_at = now()
            RETURNING
                guild_id,
                morning_message,
                night_message,
                user_cooldown_secs,
                guild_cooldown_secs
            "#,
            self.guild_id,
            self.morning_message.as_deref(),
            self.night_message.as_deref(),
            self.user_cooldown_secs,
            self.guild_cooldown_secs
        )
        .fetch_one(pool)
        .await
    }
}
