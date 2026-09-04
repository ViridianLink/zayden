pub mod choice;
mod entry;
mod policy;

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::{DashMap, DashSet};
use futures::StreamExt;
use reqwest::Client;
use serenity::all::{AutocompleteChoice, GuildId};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tracing::{debug, warn};
use zayden_app::events::AppEvent;

pub use crate::faq::index::choice::Target;
use crate::faq::index::choice::{MAX_CHOICES, ask, jump};
use crate::faq::index::entry::{Entry, score};
use crate::faq::index::policy::Policy;
use crate::faq::render;
use crate::wiki::{self, WikiConfig};

const PROBE_BUDGET: Duration = Duration::from_millis(1500);
const POLL: Duration = Duration::from_millis(100);
const MAX_HEADINGS: usize = 5000;
const HEADING_LEVELS: std::ops::RangeInclusive<usize> = 1..=3;

pub struct WikiIndex {
    guilds: DashMap<GuildId, Arc<Snapshot>>,
    building: DashSet<GuildId>,
    fetching: DashSet<(GuildId, i32)>,
    client: Client,
}

struct Snapshot {
    built_at: Instant,
    policy: Policy,
    pages: Vec<Entry>,
    headings: Vec<Entry>,
}

impl Snapshot {
    fn is_fresh(&self) -> bool {
        self.built_at.elapsed() < self.policy.ttl
    }

    fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.pages.iter().chain(self.headings.iter())
    }
}

impl WikiIndex {
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            guilds: DashMap::new(),
            building: DashSet::new(),
            fetching: DashSet::new(),
            client,
        }
    }

    pub fn invalidate(&self, guild_id: GuildId) {
        self.guilds.remove(&guild_id);
    }

    pub fn spawn_invalidator(index: Arc<Self>, mut rx: Receiver<AppEvent>) {
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(AppEvent::ConfigChanged(guild_id)) => {
                        if guild_id != 0 {
                            index.invalidate(GuildId::new(guild_id));
                        }
                    },
                    Ok(AppEvent::EntitlementChanged(_)) => {},
                    Err(RecvError::Lagged(n)) => {
                        warn!(n, "wiki index invalidator lagged; dropping all");
                        index.guilds.clear();
                    },
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }

    pub async fn choices(
        self: &Arc<Self>,
        guild_id: GuildId,
        config: &WikiConfig,
        query: &str,
    ) -> Vec<AutocompleteChoice<'static>> {
        let mut choices = vec![ask(query)];

        let Some(snapshot) = self.snapshot(guild_id, config).await else {
            return choices;
        };

        let mut ranked = snapshot
            .entries()
            .map(|entry| (score(entry, query), entry))
            .filter(|(score, _entry)| *score > 0)
            .collect::<Vec<_>>();

        ranked.sort_by(|(left, a), (right, b)| {
            right.cmp(left).then_with(|| a.label.cmp(&b.label))
        });

        choices.extend(
            ranked
                .iter()
                .take(MAX_CHOICES - 1)
                .map(|(_score, entry)| jump(&entry.label, &entry.target())),
        );

        self.demand_headings(guild_id, config, &snapshot, &ranked);

        choices
    }

    async fn snapshot(
        self: &Arc<Self>,
        guild_id: GuildId,
        config: &WikiConfig,
    ) -> Option<Arc<Snapshot>> {
        if let Some(snapshot) = self.fresh(guild_id) {
            return Some(snapshot);
        }

        self.spawn_build(guild_id, config);

        let deadline = Instant::now() + PROBE_BUDGET;

        while Instant::now() < deadline {
            tokio::time::sleep(POLL).await;

            if let Some(snapshot) = self.fresh(guild_id) {
                return Some(snapshot);
            }
        }

        None
    }

    fn fresh(&self, guild_id: GuildId) -> Option<Arc<Snapshot>> {
        let snapshot = Arc::clone(self.guilds.get(&guild_id)?.value());

        snapshot.is_fresh().then_some(snapshot)
    }

    fn spawn_build(self: &Arc<Self>, guild_id: GuildId, config: &WikiConfig) {
        if !self.building.insert(guild_id) {
            return;
        }

        let index = Arc::clone(self);
        let config = config.clone();

        tokio::spawn(async move {
            index.build(guild_id, &config).await;
            index.building.remove(&guild_id);
        });
    }

    async fn build(self: &Arc<Self>, guild_id: GuildId, config: &WikiConfig) {
        let listed = match wiki::list(&self.client, config).await {
            Ok(listed) => listed,
            Err(e) => {
                warn!(error = ?e, %guild_id, "wiki page list failed");
                return;
            },
        };

        let policy = Policy::for_size(listed.len());

        let pages = listed
            .into_iter()
            .map(|page| {
                let title = page.title.filter(|title| !title.trim().is_empty());

                Entry::page(
                    page.id,
                    page.path.clone(),
                    title.unwrap_or(page.path),
                    page.description.as_deref().unwrap_or_default(),
                )
            })
            .collect::<Vec<_>>();

        debug!(%guild_id, pages = pages.len(), crawl = policy.crawl, "wiki index built");

        self.guilds.insert(
            guild_id,
            Arc::new(Snapshot {
                built_at: Instant::now(),
                policy,
                pages,
                headings: Vec::new(),
            }),
        );

        if policy.crawl {
            let index = Arc::clone(self);
            let config = config.clone();

            tokio::spawn(async move { index.crawl(guild_id, &config).await });
        }
    }

    async fn crawl(self: &Arc<Self>, guild_id: GuildId, config: &WikiConfig) {
        let Some(snapshot) = self.guilds.get(&guild_id).map(|s| Arc::clone(&s))
        else {
            return;
        };

        let policy = snapshot.policy;

        let headings = futures::stream::iter(snapshot.pages.clone())
            .map(|page| async move {
                tokio::time::sleep(policy.batch_pause).await;
                self.headings(config, &page).await
            })
            .buffer_unordered(policy.concurrency)
            .flat_map(futures::stream::iter)
            .take(MAX_HEADINGS)
            .collect::<Vec<_>>()
            .await;

        debug!(%guild_id, headings = headings.len(), "wiki heading crawl finished");

        self.guilds.insert(
            guild_id,
            Arc::new(Snapshot {
                built_at: snapshot.built_at,
                policy,
                pages: snapshot.pages.clone(),
                headings,
            }),
        );
    }

    fn demand_headings(
        self: &Arc<Self>,
        guild_id: GuildId,
        config: &WikiConfig,
        snapshot: &Arc<Snapshot>,
        ranked: &[(usize, &Entry)],
    ) {
        if snapshot.policy.crawl || snapshot.headings.len() >= MAX_HEADINGS {
            return;
        }

        let Some((_score, page)) =
            ranked.iter().find(|(_score, entry)| entry.anchor.is_none())
        else {
            return;
        };

        if snapshot.headings.iter().any(|entry| entry.id == page.id) {
            return;
        }

        // Autocomplete fires per keystroke, so without this every character
        // typed would start another fetch of the same page.
        if !self.fetching.insert((guild_id, page.id)) {
            return;
        }

        let index = Arc::clone(self);
        let config = config.clone();
        let page = (*page).clone();
        let snapshot = Arc::clone(snapshot);

        tokio::spawn(async move {
            let found = index.headings(&config, &page).await;
            index.fetching.remove(&(guild_id, page.id));

            if found.is_empty() {
                return;
            }

            let mut headings = snapshot.headings.clone();
            headings.extend(found);

            index.guilds.insert(
                guild_id,
                Arc::new(Snapshot {
                    built_at: snapshot.built_at,
                    policy: snapshot.policy,
                    pages: snapshot.pages.clone(),
                    headings,
                }),
            );
        });
    }

    async fn headings(&self, config: &WikiConfig, page: &Entry) -> Vec<Entry> {
        let fetched = match wiki::page(&self.client, config, &page.path).await {
            Ok(fetched) => fetched,
            Err(e) => {
                debug!(error = ?e, path = page.path, "wiki page skipped while indexing");
                return Vec::new();
            },
        };

        render::split_sections(&fetched.content)
            .into_iter()
            .filter(|section| {
                HEADING_LEVELS.contains(&section.level) && !section.anchor.is_empty()
            })
            .map(|section| Entry::heading(page, &section.title, section.anchor))
            .collect()
    }
}
