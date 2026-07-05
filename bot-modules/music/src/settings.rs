use sqlx::PgPool;
use zayden_app::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MusicSettingsRow {
    pub guild_id: i64,
    pub dj_role_id: Option<i64>,
    pub default_volume: i16,
    pub auto_disconnect_secs: i32,
    pub stay_connected: bool,
    pub autoplay: bool,
    pub announce_now_playing: bool,
}

impl SettingsRow for MusicSettingsRow {
    const TABLE: &'static str = "music_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            dj_role_id: None,
            default_volume: 100,
            auto_disconnect_secs: 120,
            stay_connected: false,
            autoplay: false,
            announce_now_playing: true,
        }
    }

    async fn select(pool: &PgPool, guild_id: i64) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT
                guild_id,
                dj_role_id,
                default_volume,
                auto_disconnect_secs,
                stay_connected,
                autoplay,
                announce_now_playing
            FROM music_settings
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
            INSERT INTO music_settings (
                guild_id, dj_role_id, default_volume, auto_disconnect_secs,
                stay_connected, autoplay, announce_now_playing
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (guild_id) DO UPDATE SET
                dj_role_id = EXCLUDED.dj_role_id,
                default_volume = EXCLUDED.default_volume,
                auto_disconnect_secs = EXCLUDED.auto_disconnect_secs,
                stay_connected = EXCLUDED.stay_connected,
                autoplay = EXCLUDED.autoplay,
                announce_now_playing = EXCLUDED.announce_now_playing,
                updated_at = now()
            RETURNING
                guild_id,
                dj_role_id,
                default_volume,
                auto_disconnect_secs,
                stay_connected,
                autoplay,
                announce_now_playing
            "#,
            self.guild_id,
            self.dj_role_id,
            self.default_volume,
            self.auto_disconnect_secs,
            self.stay_connected,
            self.autoplay,
            self.announce_now_playing
        )
        .fetch_one(pool)
        .await
    }
}
