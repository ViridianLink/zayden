use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct FaqArticle {
    pub id: i32,
    pub guild_id: i64,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub source_thread_id: Option<i64>,
    pub generated: bool,
    pub updated_at: jiff_sqlx::Timestamp,
}

impl FaqArticle {
    pub async fn list(
        pool: &PgPool,
        guild_id: i64,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT id, guild_id, title, summary, content, category, tags,
                   source_thread_id, generated,
                   updated_at AS "updated_at: jiff_sqlx::Timestamp"
            FROM faq_articles
            WHERE guild_id = $1
            ORDER BY updated_at DESC
            LIMIT $2
            "#,
            guild_id,
            limit
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get(
        pool: &PgPool,
        guild_id: i64,
        id: i32,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT id, guild_id, title, summary, content, category, tags,
                   source_thread_id, generated,
                   updated_at AS "updated_at: jiff_sqlx::Timestamp"
            FROM faq_articles
            WHERE guild_id = $1 AND id = $2
            "#,
            guild_id,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn search(
        pool: &PgPool,
        guild_id: i64,
        query: &str,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT id, guild_id, title, summary, content, category, tags,
                   source_thread_id, generated,
                   updated_at AS "updated_at: jiff_sqlx::Timestamp"
            FROM faq_articles
            WHERE guild_id = $1
              AND search @@ websearch_to_tsquery('english', $2)
            ORDER BY ts_rank(search, websearch_to_tsquery('english', $2)) DESC,
                     updated_at DESC
            LIMIT $3
            "#,
            guild_id,
            query,
            limit
        )
        .fetch_all(pool)
        .await
    }

    pub async fn best_match_rank(
        pool: &PgPool,
        guild_id: i64,
        title: &str,
    ) -> sqlx::Result<Option<f32>> {
        sqlx::query_scalar!(
            r#"
            SELECT ts_rank(search, websearch_to_tsquery('english', $2))
                       AS "rank!"
            FROM faq_articles
            WHERE guild_id = $1
              AND search @@ websearch_to_tsquery('english', $2)
            ORDER BY 1 DESC
            LIMIT 1
            "#,
            guild_id,
            title
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn insert_generated(
        pool: &PgPool,
        guild_id: i64,
        thread_id: i64,
        draft: NewArticle<'_>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            INSERT INTO faq_articles (guild_id, title, summary, content, category,
                                      tags, source_thread_id, generated)
            VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE)
            ON CONFLICT DO NOTHING
            RETURNING id, guild_id, title, summary, content, category, tags,
                      source_thread_id, generated,
                      updated_at AS "updated_at: jiff_sqlx::Timestamp"
            "#,
            guild_id,
            draft.title,
            draft.summary,
            draft.content,
            draft.category,
            draft.tags,
            thread_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(
        pool: &PgPool,
        guild_id: i64,
        article: NewArticle<'_>,
    ) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Self,
            r#"
            INSERT INTO faq_articles (guild_id, title, summary, content, category,
                                      tags, generated)
            VALUES ($1, $2, $3, $4, $5, $6, FALSE)
            RETURNING id, guild_id, title, summary, content, category, tags,
                      source_thread_id, generated,
                      updated_at AS "updated_at: jiff_sqlx::Timestamp"
            "#,
            guild_id,
            article.title,
            article.summary,
            article.content,
            article.category,
            article.tags
        )
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &PgPool,
        guild_id: i64,
        id: i32,
        article: NewArticle<'_>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            UPDATE faq_articles
            SET title = $3, summary = $4, content = $5, category = $6, tags = $7,
                updated_at = now()
            WHERE guild_id = $1 AND id = $2
            RETURNING id, guild_id, title, summary, content, category, tags,
                      source_thread_id, generated,
                      updated_at AS "updated_at: jiff_sqlx::Timestamp"
            "#,
            guild_id,
            id,
            article.title,
            article.summary,
            article.content,
            article.category,
            article.tags
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(
        pool: &PgPool,
        guild_id: i64,
        id: i32,
    ) -> sqlx::Result<bool> {
        let result = sqlx::query!(
            "DELETE FROM faq_articles WHERE guild_id = $1 AND id = $2",
            guild_id,
            id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NewArticle<'a> {
    pub title: &'a str,
    pub summary: &'a str,
    pub content: &'a str,
    pub category: Option<&'a str>,
    pub tags: &'a [String],
}
