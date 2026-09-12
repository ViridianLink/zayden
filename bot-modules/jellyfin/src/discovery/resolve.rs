use std::sync::Arc;

use sqlx::PgPool;

use crate::error::{JellyfinError, Result};
use crate::index::LibraryItemRow;
use crate::runtime::JellyfinRuntime;
use crate::transport::jellyseerr::model::{MediaType, SearchResult};

#[derive(Debug, Clone)]
pub enum Resolved {
    OnServer { local: Box<LibraryItemRow>, remote: Option<Box<SearchResult>> },
    Remote(Box<SearchResult>),
}

impl Resolved {
    #[must_use]
    pub fn title(&self) -> String {
        match self {
            Self::OnServer { local, .. } => local.name.clone(),
            Self::Remote(remote) => remote.display_title().to_owned(),
        }
    }

    #[must_use]
    pub fn tmdb_id(&self) -> Option<i32> {
        match self {
            Self::OnServer { local, remote } => {
                local.tmdb_id.or_else(|| remote.as_ref().map(|r| r.id))
            },
            Self::Remote(remote) => Some(remote.id),
        }
    }

    #[must_use]
    pub fn kind(&self) -> MediaType {
        match self {
            Self::OnServer { local, .. } => {
                if local.item_type == "Movie" {
                    MediaType::Movie
                } else {
                    MediaType::Tv
                }
            },
            Self::Remote(remote) => remote.kind().unwrap_or(MediaType::Movie),
        }
    }

    #[must_use]
    pub const fn is_on_server(&self) -> bool {
        matches!(self, Self::OnServer { .. })
    }
}

pub async fn search_remote(
    runtime: &Arc<JellyfinRuntime>,
    query: &str,
) -> Result<Vec<SearchResult>> {
    let key = query.trim().to_lowercase();

    if let Some(hit) = runtime.caches.search.get(&key).await {
        return Ok(hit);
    }

    let results = runtime.seer.search(query).await.map_err(JellyfinError::from)?;
    runtime.caches.search.insert(key, results.clone()).await;

    Ok(results)
}

pub async fn resolve(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
    query: &str,
) -> Result<Resolved> {
    // An autocomplete pick round-trips the Jellyfin item id, so try that first
    // and skip the search entirely.
    if let Some(local) = LibraryItemRow::by_id(pool, query).await? {
        return Ok(Resolved::OnServer { local: Box::new(local), remote: None });
    }

    let remote = search_remote(runtime, query).await?;

    let best = remote
        .iter()
        .find(|r| matches!(r.kind(), Some(MediaType::Movie | MediaType::Tv)))
        .cloned();

    if let Some(candidate) = best.clone() {
        let item_type = match candidate.kind() {
            Some(MediaType::Tv) => "Series",
            _ => "Movie",
        };

        if let Some(local) =
            LibraryItemRow::by_tmdb(pool, item_type, candidate.id).await?
        {
            return Ok(Resolved::OnServer {
                local: Box::new(local),
                remote: Some(Box::new(candidate)),
            });
        }
    }

    // Nothing on TMDB: fall back to a local title match so a server-only item
    // with no provider ids is still findable.
    if best.is_none()
        && let Some(local) =
            LibraryItemRow::search(pool, query, None, 1).await?.into_iter().next()
    {
        return Ok(Resolved::OnServer { local: Box::new(local), remote: None });
    }

    best.map(|remote| Resolved::Remote(Box::new(remote)))
        .ok_or_else(|| JellyfinError::NoSuchTitle(query.to_owned()))
}

pub async fn resolve_local(
    pool: &PgPool,
    query: &str,
    item_type: Option<&str>,
) -> Result<LibraryItemRow> {
    if let Some(local) = LibraryItemRow::by_id(pool, query).await? {
        return Ok(local);
    }

    LibraryItemRow::search(pool, query, item_type, 1)
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| JellyfinError::NotOnServer(query.to_owned()))
}
