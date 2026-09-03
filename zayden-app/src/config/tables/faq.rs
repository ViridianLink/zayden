use sqlx::PgPool;

use crate::config::SettingsRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FaqSettingsRow {
    pub guild_id: i64,
    pub enabled: bool,
    pub auto_triage: bool,
    pub wiki_url: Option<String>,
    pub wiki_api_key: Option<String>,
    pub wiki_locale: String,
    pub max_results: i32,
    pub answer_max_tokens: i32,
    pub answer_temperature: f32,
}

impl SettingsRow for FaqSettingsRow {
    const TABLE: &'static str = "faq_settings";

    fn empty(guild_id: i64) -> Self {
        Self {
            guild_id,
            enabled: false,
            auto_triage: false,
            wiki_url: None,
            wiki_api_key: None,
            wiki_locale: String::from("en"),
            max_results: 5,
            answer_max_tokens: 500,
            answer_temperature: 0.2,
        }
    }

    async fn select(
        pool: &PgPool,
        guild_id: i64,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, enabled, auto_triage, wiki_url, wiki_api_key,
                   wiki_locale, max_results, answer_max_tokens, answer_temperature
            FROM faq_settings
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
            INSERT INTO faq_settings (guild_id, enabled, auto_triage, wiki_url,
                                      wiki_api_key, wiki_locale, max_results,
                                      answer_max_tokens, answer_temperature)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (guild_id) DO UPDATE SET
                enabled = EXCLUDED.enabled,
                auto_triage = EXCLUDED.auto_triage,
                wiki_url = EXCLUDED.wiki_url,
                wiki_api_key = EXCLUDED.wiki_api_key,
                wiki_locale = EXCLUDED.wiki_locale,
                max_results = EXCLUDED.max_results,
                answer_max_tokens = EXCLUDED.answer_max_tokens,
                answer_temperature = EXCLUDED.answer_temperature,
                updated_at = now()
            RETURNING guild_id, enabled, auto_triage, wiki_url, wiki_api_key,
                      wiki_locale, max_results, answer_max_tokens,
                      answer_temperature
            "#,
            self.guild_id,
            self.enabled,
            self.auto_triage,
            self.wiki_url.as_deref(),
            self.wiki_api_key.as_deref(),
            self.wiki_locale,
            self.max_results,
            self.answer_max_tokens,
            self.answer_temperature
        )
        .fetch_one(pool)
        .await
    }
}
