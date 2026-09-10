use super::SeerClient;
use super::model::{MediaType, SearchPage, SearchResult};
use crate::transport::http::{ApiResult, fetch_json};

pub const MIN_VOTE_COUNT: i64 = 150;

#[derive(Debug, Clone, Default)]
pub struct DiscoverFilters {
    pub keywords: Vec<i32>,
    pub genres: Vec<i32>,
    pub exclude_genres: Vec<i32>,
    pub year_from: Option<i32>,
    pub year_to: Option<i32>,
    pub min_votes: Option<i64>,
    pub sort_by: Option<String>,
}

impl DiscoverFilters {
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = vec![("page", "1".to_owned())];

        if !self.keywords.is_empty() {
            q.push(("keywords", join_ids(&self.keywords)));
        }
        if !self.genres.is_empty() {
            q.push(("genre", join_ids(&self.genres)));
        }
        if let Some(from) = self.year_from {
            q.push(("primaryReleaseDateGte", format!("{from}-01-01")));
        }
        if let Some(to) = self.year_to {
            q.push(("primaryReleaseDateLte", format!("{to}-12-31")));
        }
        q.push((
            "voteCountGte",
            self.min_votes.unwrap_or(MIN_VOTE_COUNT).to_string(),
        ));
        q.push((
            "sortBy",
            self.sort_by.clone().unwrap_or_else(|| "popularity.desc".to_owned()),
        ));

        q
    }

    #[must_use]
    pub fn relaxed(&self) -> Option<Self> {
        let mut next = self.clone();

        if next.keywords.len() > 1 {
            next.keywords.pop();
        } else if next.year_from.is_some() || next.year_to.is_some() {
            next.year_from = None;
            next.year_to = None;
        } else if !next.keywords.is_empty() {
            next.keywords.clear();
        } else if next.genres.len() > 1 {
            next.genres.pop();
        } else {
            return None;
        }

        Some(next)
    }
}

fn join_ids(ids: &[i32]) -> String {
    ids.iter().map(ToString::to_string).collect::<Vec<_>>().join(",")
}

impl SeerClient {
    pub async fn discover(
        &self,
        kind: MediaType,
        filters: &DiscoverFilters,
    ) -> ApiResult<Vec<SearchResult>> {
        let path = match kind {
            MediaType::Movie => "discover/movies",
            MediaType::Tv => "discover/tv",
        };
        let query = filters.query();

        let page: SearchPage =
            fetch_json(super::SERVICE, "discover", || self.get(path).query(&query))
                .await?;

        Ok(page.results)
    }

    pub async fn trending(&self) -> ApiResult<Vec<SearchResult>> {
        let page: SearchPage = fetch_json(super::SERVICE, "trending", || {
            self.get("discover/trending").query(&[("page", "1")])
        })
        .await?;

        Ok(page.results)
    }

    pub async fn recommendations(
        &self,
        kind: MediaType,
        tmdb_id: i32,
    ) -> ApiResult<Vec<SearchResult>> {
        let path = format!("{}/{tmdb_id}/recommendations", kind.as_str());

        let page: SearchPage = fetch_json(super::SERVICE, "recommendations", || {
            self.get(&path).query(&[("page", "1")])
        })
        .await?;

        Ok(page.results)
    }

    pub async fn similar(
        &self,
        kind: MediaType,
        tmdb_id: i32,
    ) -> ApiResult<Vec<SearchResult>> {
        let path = format!("{}/{tmdb_id}/similar", kind.as_str());

        let page: SearchPage = fetch_json(super::SERVICE, "similar", || {
            self.get(&path).query(&[("page", "1")])
        })
        .await?;

        Ok(page.results)
    }
}
