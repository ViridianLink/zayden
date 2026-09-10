use std::sync::Arc;

use zayden_app::config::HostingGame;

use crate::error::{HostingError, Result};

#[derive(Debug, Clone)]
pub struct Catalog {
    games: Arc<[HostingGame]>,
}

impl Catalog {
    #[must_use]
    pub fn new(games: &[HostingGame]) -> Self {
        Self { games: games.into() }
    }

    #[must_use]
    pub fn games(&self) -> &[HostingGame] {
        &self.games
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.games.is_empty()
    }

    pub fn get(&self, key: &str) -> Result<&HostingGame> {
        self.games
            .iter()
            .find(|g| g.key == key)
            .ok_or_else(|| HostingError::UnknownGame(key.to_owned()))
    }

    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> Vec<&HostingGame> {
        let query = query.trim().to_lowercase();

        if query.is_empty() {
            return self.games.iter().take(limit).collect();
        }

        let mut hits: Vec<(u8, &HostingGame)> = self
            .games
            .iter()
            .filter_map(|game| {
                let name = game.name.to_lowercase();
                let key = game.key.to_lowercase();

                if name.starts_with(&query) || key.starts_with(&query) {
                    Some((0, game))
                } else if name.contains(&query) || key.contains(&query) {
                    Some((1, game))
                } else {
                    None
                }
            })
            .collect();

        hits.sort_by_key(|&(rank, game)| (rank, game.name.clone()));
        hits.into_iter().map(|(_, game)| game).take(limit).collect()
    }
}
