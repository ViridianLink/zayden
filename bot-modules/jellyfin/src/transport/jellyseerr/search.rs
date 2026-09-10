use super::SeerClient;
use super::model::{
    KeywordPage,
    MediaType,
    MovieDetails,
    SearchPage,
    SearchResult,
    TvDetails,
};
use crate::transport::http::{ApiResult, fetch_json};

impl SeerClient {
    pub async fn search(&self, query: &str) -> ApiResult<Vec<SearchResult>> {
        let page: SearchPage = fetch_json(super::SERVICE, "search", || {
            self.get("search").query(&[("query", query), ("page", "1")])
        })
        .await?;

        Ok(page.results)
    }

    pub async fn movie(&self, tmdb_id: i32) -> ApiResult<MovieDetails> {
        fetch_json(super::SERVICE, "movie details", || {
            self.get(&format!("movie/{tmdb_id}"))
        })
        .await
    }

    pub async fn tv(&self, tmdb_id: i32) -> ApiResult<TvDetails> {
        fetch_json(super::SERVICE, "tv details", || {
            self.get(&format!("tv/{tmdb_id}"))
        })
        .await
    }

    pub async fn keyword_id(&self, name: &str) -> ApiResult<Option<i32>> {
        let page: KeywordPage = fetch_json(super::SERVICE, "keyword search", || {
            self.get("search/keyword").query(&[("query", name), ("page", "1")])
        })
        .await?;

        let lowered = name.to_lowercase();
        Ok(page
            .results
            .iter()
            .find(|k| k.name.to_lowercase() == lowered)
            .or_else(|| page.results.first())
            .map(|k| k.id))
    }

    pub async fn details_title(
        &self,
        kind: MediaType,
        tmdb_id: i32,
    ) -> ApiResult<String> {
        Ok(match kind {
            MediaType::Movie => self.movie(tmdb_id).await?.title,
            MediaType::Tv => self.tv(tmdb_id).await?.name,
        })
    }
}
