use sqlx::PgPool;

use crate::transport::jellyfin::model::Item;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LibraryItemRow {
    pub item_id: String,
    pub item_type: String,
    pub name: String,
    pub sort_name: String,
    pub production_year: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<i32>,
    pub runtime_ticks: Option<i64>,
    pub community_rating: Option<f32>,
    pub genres: Vec<String>,
    pub parent_path: Option<String>,
}

impl LibraryItemRow {
    #[must_use]
    pub fn library_root(&self) -> Option<&str> {
        let path = self.parent_path.as_deref()?;

        if self.item_type == "Movie" {
            let cut = path.rfind('/')?;
            // `rfind` lands on a char boundary, so this slice is always valid.
            (cut > 0).then(|| path.get(..cut)).flatten()
        } else {
            Some(path)
        }
    }

    #[must_use]
    pub fn collection_type(&self) -> &'static str {
        if self.item_type == "Movie" { "movies" } else { "tvshows" }
    }

    #[must_use]
    pub fn from_item(item: &Item, item_type: &str) -> Self {
        let providers = item.provider_ids.clone().unwrap_or_default();

        Self {
            item_id: item.id.clone(),
            item_type: item_type.to_owned(),
            name: item.name.clone(),
            sort_name: item
                .sort_name
                .clone()
                .unwrap_or_else(|| item.name.to_lowercase()),
            production_year: item.production_year,
            tmdb_id: providers.tmdb.as_deref().and_then(|v| v.parse().ok()),
            imdb_id: providers.imdb.clone(),
            tvdb_id: providers.tvdb.as_deref().and_then(|v| v.parse().ok()),
            runtime_ticks: item.run_time_ticks,
            community_rating: item.community_rating,
            genres: item.genres.clone().unwrap_or_default(),
            parent_path: item.path.clone(),
        }
    }

    pub async fn upsert(&self, pool: &PgPool) -> sqlx::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO jellyfin_library_items
                (item_id, item_type, name, sort_name, production_year, tmdb_id,
                 imdb_id, tvdb_id, runtime_ticks, community_rating, genres,
                 parent_path, refreshed_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, now())
            ON CONFLICT (item_id) DO UPDATE SET
                item_type = EXCLUDED.item_type,
                name = EXCLUDED.name,
                sort_name = EXCLUDED.sort_name,
                production_year = EXCLUDED.production_year,
                tmdb_id = EXCLUDED.tmdb_id,
                imdb_id = EXCLUDED.imdb_id,
                tvdb_id = EXCLUDED.tvdb_id,
                runtime_ticks = EXCLUDED.runtime_ticks,
                community_rating = EXCLUDED.community_rating,
                genres = EXCLUDED.genres,
                parent_path = EXCLUDED.parent_path,
                refreshed_at = now()
            "#,
            self.item_id,
            self.item_type,
            self.name,
            self.sort_name,
            self.production_year,
            self.tmdb_id,
            self.imdb_id,
            self.tvdb_id,
            self.runtime_ticks,
            self.community_rating,
            &self.genres,
            self.parent_path,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn count(pool: &PgPool, item_type: &str) -> sqlx::Result<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM jellyfin_library_items WHERE item_type = $1",
            item_type
        )
        .fetch_one(pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    pub async fn by_tmdb(
        pool: &PgPool,
        item_type: &str,
        tmdb_id: i32,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT item_id, item_type, name, sort_name, production_year, tmdb_id,
                   imdb_id, tvdb_id, runtime_ticks, community_rating,
                   genres AS "genres!: Vec<String>", parent_path
            FROM jellyfin_library_items
            WHERE item_type = $1 AND tmdb_id = $2
            "#,
            item_type,
            tmdb_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn by_id(pool: &PgPool, item_id: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT item_id, item_type, name, sort_name, production_year, tmdb_id,
                   imdb_id, tvdb_id, runtime_ticks, community_rating,
                   genres AS "genres!: Vec<String>", parent_path
            FROM jellyfin_library_items
            WHERE item_id = $1
            "#,
            item_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn search(
        pool: &PgPool,
        query: &str,
        item_type: Option<&str>,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let pattern = format!("%{}%", query.to_lowercase());

        sqlx::query_as!(
            Self,
            r#"
            SELECT item_id, item_type, name, sort_name, production_year, tmdb_id,
                   imdb_id, tvdb_id, runtime_ticks, community_rating,
                   genres AS "genres!: Vec<String>", parent_path
            FROM jellyfin_library_items
            WHERE ($2::text IS NULL OR item_type = $2)
              AND (lower(name) LIKE $1 OR lower(sort_name) LIKE $1)
            ORDER BY
                CASE WHEN lower(name) LIKE $3 THEN 0 ELSE 1 END,
                sort_name
            LIMIT $4
            "#,
            pattern,
            item_type,
            format!("{}%", query.to_lowercase()),
            limit
        )
        .fetch_all(pool)
        .await
    }

    pub async fn hidden_gems(
        pool: &PgPool,
        min_rating: f32,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT item_id, item_type, name, sort_name, production_year, tmdb_id,
                   imdb_id, tvdb_id, runtime_ticks, community_rating,
                   genres AS "genres!: Vec<String>", parent_path
            FROM jellyfin_library_items
            WHERE item_type = 'Movie' AND community_rating >= $1
            ORDER BY community_rating DESC
            LIMIT $2
            "#,
            min_rating,
            limit
        )
        .fetch_all(pool)
        .await
    }

    pub async fn tmdb_ids(pool: &PgPool, item_type: &str) -> sqlx::Result<Vec<i32>> {
        sqlx::query_scalar!(
            r#"
            SELECT tmdb_id AS "tmdb_id!"
            FROM jellyfin_library_items
            WHERE item_type = $1 AND tmdb_id IS NOT NULL
            "#,
            item_type
        )
        .fetch_all(pool)
        .await
    }

    #[expect(
        trivial_casts,
        reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
    )]
    pub async fn prune_stale(
        pool: &PgPool,
        started_at: jiff::Timestamp,
    ) -> sqlx::Result<u64> {
        let cutoff = jiff_sqlx::Timestamp::from(started_at);

        let result = sqlx::query!(
            "DELETE FROM jellyfin_library_items WHERE refreshed_at < $1",
            cutoff as jiff_sqlx::Timestamp
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
