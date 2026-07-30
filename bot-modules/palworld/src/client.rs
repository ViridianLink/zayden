use std::collections::HashMap;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, UNIX_EPOCH};

use moka::future::Cache;
use reqwest::Client;
use tokio::sync::Mutex;

use crate::breeding::BreedingIndex;
use crate::error::{PalworldError, Result};
use crate::model::{Element, Item, Pal, PassiveSkill, PlayerName, WorldRoster};
use crate::save::dps::StoredPals;
use crate::save::player::PlayerRecord;
use crate::source::SourceId;
use crate::transport::{Fandom, PalCalc, PalDb, Paldex, PalworldGg, Pelican};
use crate::{merge, parse, save};

const LONG_TTL: Duration = Duration::from_hours(8);
const SAVE_FRESHNESS: Duration = Duration::from_mins(5);
const LEVEL_SAVE: &str = "Level.sav";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKey {
    Shared,
    User(i64),
}

fn ttl_cache<K, V>() -> Cache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    Cache::builder().time_to_live(LONG_TTL).max_capacity(512).build()
}

pub struct PalworldClient {
    palcalc: PalCalc,
    paldex: Paldex,
    paldb: PalDb,
    palworldgg: PalworldGg,
    fandom: Fandom,

    save_dir: Option<PathBuf>,
    uploads_dir: PathBuf,
    pelican: Option<Pelican>,
    refresh_lock: Mutex<Option<Instant>>,

    pal_basic_cache: Cache<(), Arc<[Pal]>>,
    elements_cache: Cache<(), Arc<HashMap<String, Vec<Element>>>>,
    pal_list_cache: Cache<(), Arc<[Pal]>>,
    pal_cache: Cache<String, Arc<Pal>>,
    item_list_cache: Cache<(), Arc<[Item]>>,
    passive_list_cache: Cache<(), Arc<[PassiveSkill]>>,
    breeding_cache: Cache<(), Arc<BreedingIndex>>,
    roster_cache: Cache<(SourceKey, u64), Arc<WorldRoster>>,
    names_cache: Cache<(SourceKey, u64), Arc<[PlayerName]>>,
    dps_cache: Cache<(PathBuf, u64), Arc<StoredPals>>,
    record_cache: Cache<(SourceKey, String, u64), Arc<PlayerRecord>>,
}

impl PalworldClient {
    #[must_use]
    pub fn new(
        client: Client,
        flaresolverr_url: Option<String>,
        paldex_base: Option<String>,
        palcalc_base: Option<String>,
        save_dir: Option<PathBuf>,
        uploads_dir: PathBuf,
        pelican: Option<Pelican>,
    ) -> Self {
        Self {
            palcalc: PalCalc::new(client.clone(), palcalc_base),
            paldex: Paldex::new(client.clone(), paldex_base),
            paldb: PalDb::new(client.clone(), flaresolverr_url.clone()),
            palworldgg: PalworldGg::new(client.clone(), flaresolverr_url),
            fandom: Fandom::new(client),
            save_dir,
            uploads_dir,
            pelican,
            refresh_lock: Mutex::new(None),
            pal_basic_cache: ttl_cache(),
            elements_cache: ttl_cache(),
            pal_list_cache: ttl_cache(),
            pal_cache: ttl_cache(),
            item_list_cache: ttl_cache(),
            passive_list_cache: ttl_cache(),
            breeding_cache: ttl_cache(),
            roster_cache: ttl_cache(),
            names_cache: ttl_cache(),
            dps_cache: ttl_cache(),
            record_cache: ttl_cache(),
        }
    }

    #[must_use]
    pub fn save_dir(&self) -> Option<&Path> {
        self.save_dir.as_deref()
    }

    pub async fn pals_basic(&self) -> Result<Arc<[Pal]>> {
        self.pal_basic_cache
            .try_get_with((), async {
                let raw = self.palcalc.pals().await?;
                Ok::<_, PalworldError>(
                    raw.into_iter().map(parse::pal_from_palcalc).collect(),
                )
            })
            .await
            .map_err(|e| PalworldError::from_shared(&e))
    }

    pub async fn elements(&self) -> Option<Arc<HashMap<String, Vec<Element>>>> {
        self.elements_cache
            .try_get_with((), async {
                self.palworldgg
                    .elements_index()
                    .await
                    .map(Arc::new)
                    .ok_or(PalworldError::SourceUnavailable)
            })
            .await
            .ok()
    }

    pub async fn pals(&self) -> Result<Arc<[Pal]>> {
        self.pal_list_cache
            .try_get_with((), async {
                let mut pals: Vec<Pal> = self.pals_basic().await?.to_vec();

                if let Some(index) = self.elements().await {
                    for pal in &mut pals {
                        if let Some(elements) = index.get(&parse::gg_slug(&pal.name))
                        {
                            pal.elements.clone_from(elements);
                        }
                    }
                }

                Ok::<_, PalworldError>(pals.into())
            })
            .await
            .map_err(|e| PalworldError::from_shared(&e))
    }

    pub async fn pal(&self, key: &str) -> Result<Arc<Pal>> {
        self.pal_cache
            .try_get_with(key.to_string(), async {
                let pals = self.pals().await?;
                let base = pals.iter().find(|p| p.key == key).cloned().ok_or_else(
                    || PalworldError::NotFound {
                        entity: "pal",
                        query: key.to_string(),
                    },
                )?;
                Ok::<_, PalworldError>(Arc::new(self.enrich_pal(base).await))
            })
            .await
            .map_err(|e| PalworldError::from_shared(&e))
    }

    async fn enrich_pal(&self, base: Pal) -> Pal {
        let name = base.name.clone();
        let key = base.key.clone();
        let slug = parse::gg_slug(&name);

        let (fandom, paldb, palworldgg) = tokio::join!(
            self.fandom.description(&name),
            self.paldb.pal_details(&name),
            self.palworldgg.pal_description(&slug),
        );

        let mut candidates = vec![(SourceId::PalCalc, base)];
        candidates.push((SourceId::PalDb, Pal {
            key: key.clone(),
            name: name.clone(),
            description: paldb.description,
            image_url: paldb.image_url,
            ..Pal::default()
        }));
        for (source, desc) in
            [(SourceId::Fandom, fandom), (SourceId::PalworldGg, palworldgg)]
        {
            if let Some(description) = desc {
                candidates.push((source, Pal {
                    key: key.clone(),
                    name: name.clone(),
                    description: Some(description),
                    ..Pal::default()
                }));
            }
        }

        merge::pal(&candidates).unwrap_or_else(|| Pal {
            key,
            name,
            ..Pal::default()
        })
    }

    pub async fn items(&self) -> Result<Arc<[Item]>> {
        self.item_list_cache
            .try_get_with((), async {
                let raw = self.paldex.items().await?;
                Ok::<_, PalworldError>(
                    raw.into_iter().map(parse::item_from_raw).collect(),
                )
            })
            .await
            .map_err(|e| PalworldError::from_shared(&e))
    }

    pub async fn item(&self, key: &str) -> Result<Item> {
        let items = self.items().await?;
        let candidates: Vec<(SourceId, Item)> = items
            .iter()
            .filter(|i| i.key == key)
            .map(|i| (SourceId::Paldex, i.clone()))
            .collect();
        merge::item(&candidates).ok_or_else(|| PalworldError::NotFound {
            entity: "item",
            query: key.to_string(),
        })
    }

    pub async fn passives(&self) -> Result<Arc<[PassiveSkill]>> {
        self.passive_list_cache
            .try_get_with((), async {
                let raw = self.paldex.passives().await?;
                let mut passives: Vec<PassiveSkill> = raw
                    .into_iter()
                    .map(|(key, value)| parse::passive_from_raw(key, value))
                    .collect();
                passives.sort_by_key(|p| p.name.to_lowercase());
                Ok::<_, PalworldError>(passives.into())
            })
            .await
            .map_err(|e| PalworldError::from_shared(&e))
    }

    pub async fn passive(&self, key: &str) -> Result<PassiveSkill> {
        let passives = self.passives().await?;
        let candidates: Vec<(SourceId, PassiveSkill)> = passives
            .iter()
            .filter(|p| p.key == key)
            .map(|p| (SourceId::Paldex, p.clone()))
            .collect();
        merge::passive(&candidates).ok_or_else(|| PalworldError::NotFound {
            entity: "passive skill",
            query: key.to_string(),
        })
    }

    pub async fn breeding_index(&self) -> Result<Arc<BreedingIndex>> {
        self.breeding_cache
            .try_get_with((), async {
                let map = self.palcalc.breeding().await?;
                Ok::<_, PalworldError>(Arc::new(BreedingIndex::from_map(map)))
            })
            .await
            .map_err(|e| PalworldError::from_shared(&e))
    }

    pub async fn roster(&self) -> Result<Arc<WorldRoster>> {
        let save_dir = self.source_dir(SourceKey::Shared)?;
        self.refresh_shared_if_stale().await;
        self.roster_from(SourceKey::Shared, &save_dir).await
    }

    pub async fn user_roster(&self, discord_id: i64) -> Result<Arc<WorldRoster>> {
        self.roster_from(SourceKey::User(discord_id), &self.user_dir(discord_id))
            .await
    }

    pub async fn player_names(
        &self,
        source: SourceKey,
    ) -> Result<Arc<[PlayerName]>> {
        let dir = self.source_dir(source)?;
        let mtime = level_mtime(&dir).await?;

        self.names_cache
            .try_get_with((source, mtime), async {
                let load_dir = dir.clone();
                let names = tokio::task::spawn_blocking(move || {
                    save::load_player_names(&load_dir)
                })
                .await
                .map_err(|e| {
                    PalworldError::Save(format!("name index task failed: {e}"))
                })?
                .inspect_err(|e| {
                    tracing::error!(
                        error = %e,
                        dir = %dir.display(),
                        "failed to read player names",
                    );
                })?;
                Ok::<_, PalworldError>(names.into())
            })
            .await
            .map_err(|e| PalworldError::from_shared(&e))
    }

    fn source_dir(&self, source: SourceKey) -> Result<PathBuf> {
        match source {
            SourceKey::Shared => self.save_dir.clone().ok_or(PalworldError::NoWorld),
            SourceKey::User(discord_id) => Ok(self.user_dir(discord_id)),
        }
    }

    pub async fn player_record(
        &self,
        source: SourceKey,
        uid: &str,
    ) -> Result<Option<Arc<PlayerRecord>>> {
        let dir = match source {
            SourceKey::Shared => {
                self.refresh_shared_if_stale().await;
                self.save_dir.clone().ok_or(PalworldError::NoWorld)?
            },
            SourceKey::User(discord_id) => self.user_dir(discord_id),
        };

        let Some(path) = save::player_save_path(&dir, uid) else {
            return Ok(None);
        };
        let Some(mtime) = file_mtime(&path) else { return Ok(None) };

        let key = (source, uid.to_string(), mtime);
        if let Some(cached) = self.record_cache.get(&key).await {
            return Ok(Some(cached));
        }

        let uid_owned = uid.to_string();
        let record = tokio::task::spawn_blocking(move || {
            save::player::load_player(&dir, &uid_owned)
        })
        .await
        .map_err(|e| {
            PalworldError::Save(format!("player save parse task failed: {e}"))
        })??;

        let Some(record) = record else { return Ok(None) };
        let record = Arc::new(record);
        self.record_cache.insert(key, Arc::clone(&record)).await;
        Ok(Some(record))
    }

    fn user_dir(&self, discord_id: i64) -> PathBuf {
        self.uploads_dir.join(discord_id.to_string())
    }

    #[must_use]
    pub fn uploads_dir(&self) -> &Path {
        &self.uploads_dir
    }

    async fn roster_from(
        &self,
        key: SourceKey,
        dir: &Path,
    ) -> Result<Arc<WorldRoster>> {
        let mtime = level_mtime(dir).await?;

        self.roster_cache
            .try_get_with((key, mtime), async {
                let stored = self.storage_pals(dir).await;

                let load_dir = dir.to_path_buf();
                let roster = tokio::task::spawn_blocking(move || {
                    save::load_world_with(&load_dir, stored)
                })
                .await
                .map_err(|e| {
                    PalworldError::Save(format!("save parse task failed: {e}"))
                })?
                .inspect_err(|e| {
                    tracing::error!(
                        error = %e,
                        dir = %dir.display(),
                        "failed to read world save",
                    );
                })?;

                Ok::<_, PalworldError>(Arc::new(roster))
            })
            .await
            .map_err(|e| PalworldError::from_shared(&e))
    }

    async fn storage_pals(&self, dir: &Path) -> StoredPals {
        let mut out = StoredPals::new();
        let mut merge = |pals: &StoredPals| {
            for (uid, owned) in pals {
                out.entry(uid.clone()).or_default().extend_from_slice(owned);
            }
        };

        let mut pending = Vec::new();
        for path in save::dps::list_files(dir) {
            let Some(mtime) = file_mtime(&path) else { continue };

            if let Some(cached) = self.dps_cache.get(&(path.clone(), mtime)).await {
                merge(&cached);
                continue;
            }

            let load_path = path.clone();
            let handle = tokio::task::spawn_blocking(move || {
                save::dps::load_file(&load_path)
            });
            pending.push((path, mtime, handle));
        }

        for (path, mtime, handle) in pending {
            let parsed = match handle.await {
                Ok(Ok(pals)) => Arc::new(pals),
                Ok(Err(e)) => {
                    tracing::warn!(
                        error = %e,
                        file = %path.display(),
                        "palworld: skipping unreadable Pal storage save",
                    );
                    continue;
                },
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        file = %path.display(),
                        "palworld: Pal storage parse task failed",
                    );
                    continue;
                },
            };

            merge(&parsed);
            self.dps_cache.insert((path, mtime), parsed).await;
        }

        out
    }

    pub async fn refresh_shared_save(&self) {
        self.refresh_shared_if_stale().await;
    }

    pub async fn warm(&self) {
        let (pals, items, passives, breeding) = tokio::join!(
            self.pals(),
            self.items(),
            self.passives(),
            self.breeding_index(),
        );

        for (source, result) in [
            ("pals", pals.err()),
            ("items", items.err()),
            ("passives", passives.err()),
            ("breeding", breeding.err()),
        ] {
            if let Some(e) = result {
                tracing::warn!(error = %e, source, "palworld: cache warm failed");
            }
        }

        self.warm_player_names().await;
    }

    pub async fn has_shared_save(&self) -> bool {
        let Some(dir) = self.save_dir.as_deref() else { return false };
        tokio::fs::try_exists(dir.join(LEVEL_SAVE)).await.unwrap_or(false)
    }

    pub async fn warm_player_names(&self) {
        if self.save_dir.is_none() {
            return;
        }

        self.refresh_shared_if_stale().await;

        if !self.has_shared_save().await {
            tracing::debug!("palworld: no shared save on disk yet, skipping warm");
            return;
        }

        match self.player_names(SourceKey::Shared).await {
            Ok(_) | Err(PalworldError::NoWorld) => {},
            Err(e) => {
                tracing::warn!(error = %e, "palworld: player name warm failed");
            },
        }
    }

    async fn refresh_shared_if_stale(&self) {
        let (Some(save_dir), Some(pelican)) =
            (self.save_dir.clone(), self.pelican.clone())
        else {
            return;
        };

        let mut last = self.refresh_lock.lock().await;
        if last.is_some_and(|t| t.elapsed() < SAVE_FRESHNESS) {
            return;
        }

        if let Err(e) = Self::refresh_shared(&pelican, &save_dir).await {
            tracing::warn!(
                error = %e,
                "palworld shared save refresh failed; using last local save"
            );
        }

        *last = Some(Instant::now());
    }

    async fn refresh_shared(pelican: &Pelican, save_dir: &Path) -> Result<()> {
        let level = Self::refresh_shared_level(pelican, save_dir).await;

        if let Err(e) = Self::refresh_shared_players(pelican, save_dir).await {
            tracing::warn!(
                error = %e,
                "palworld: player save refresh failed; using last local copies"
            );
        }

        level
    }

    async fn refresh_shared_level(pelican: &Pelican, save_dir: &Path) -> Result<()> {
        let remote_modified = pelican.level_modified().await?;

        let level_path = save_dir.join(LEVEL_SAVE);
        let local_modified =
            tokio::task::spawn_blocking(move || local_mtime_secs(&level_path))
                .await
                .map_err(|e| {
                    PalworldError::Pelican(format!("mtime task failed: {e}"))
                })?;

        if remote_modified <= local_modified {
            return Ok(());
        }

        let bytes = pelican.download_level().await?;

        let save_dir = save_dir.to_path_buf();
        tokio::task::spawn_blocking(move || {
            save::validate_level(&bytes).map_err(|e| {
                PalworldError::Pelican(format!(
                    "downloaded save failed validation, keeping last local save: {e}"
                ))
            })?;
            save::write_level_atomic(&save_dir, &bytes)
        })
        .await
        .map_err(|e| {
            PalworldError::Pelican(format!("save write task failed: {e}"))
        })??;

        Ok(())
    }

    async fn refresh_shared_players(
        pelican: &Pelican,
        save_dir: &Path,
    ) -> Result<()> {
        let players_dir = save_dir.join("Players");

        for remote in pelican.list_players().await? {
            let local = players_dir.join(format!("{}.sav", remote.stem));
            let local_modified =
                tokio::task::spawn_blocking(move || local_mtime_secs(&local))
                    .await
                    .map_err(|e| {
                        PalworldError::Pelican(format!("mtime task failed: {e}"))
                    })?;
            if remote.modified <= local_modified {
                continue;
            }

            let bytes = match pelican.download_player(&remote.stem).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        player = remote.stem,
                        "palworld: player save download failed"
                    );
                    continue;
                },
            };

            let save_dir = save_dir.to_path_buf();
            let stem = remote.stem.clone();
            let is_storage = remote.is_storage;
            let written = tokio::task::spawn_blocking(move || {
                validate_player_file(&bytes, is_storage).map_err(|e| {
                    PalworldError::Pelican(format!(
                        "downloaded player save failed validation: {e}"
                    ))
                })?;
                save::write_raw_player(&save_dir, &stem, &bytes)
            })
            .await
            .map_err(|e| {
                PalworldError::Pelican(format!("save write task failed: {e}"))
            })?;

            if let Err(e) = written {
                tracing::warn!(
                    error = %e,
                    player = remote.stem,
                    "palworld: keeping last local player save"
                );
            }
        }

        Ok(())
    }
}

fn validate_player_file(bytes: &[u8], is_storage: bool) -> Result<()> {
    if is_storage {
        save::dps::parse(bytes).map(|_| ())
    } else {
        save::player::parse_player_uid(bytes).map(|_| ())
    }
}

fn local_mtime_secs(level_path: &Path) -> i64 {
    std::fs::metadata(level_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

async fn level_mtime(dir: &Path) -> Result<u64> {
    let meta = tokio::fs::metadata(dir.join("Level.sav")).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PalworldError::NoWorld
        } else {
            PalworldError::Io(e)
        }
    })?;
    Ok(mtime_nanos(&meta))
}

fn mtime_nanos(meta: &std::fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX))
}

fn file_mtime(path: &Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| mtime_nanos(&m))
}
