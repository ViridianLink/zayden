mod build;
mod cradle;
mod faction;
mod map;
mod meta;
mod runner;
mod schedule;
mod weapon;

use std::hash::Hash;
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;

use crate::model::{
    BuildRecipe,
    Cradle,
    Faction,
    MarathonMap,
    MetaEntry,
    Runner,
    Weapon,
};
use crate::transport::{Fandom, MapGenie, MarathonDb, Mobalytics};

const LONG_TTL: Duration = Duration::from_hours(8);

fn ttl_cache<K, V>() -> Cache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    Cache::builder().time_to_live(LONG_TTL).max_capacity(512).build()
}

pub struct MarathonClient {
    mobalytics: Option<Mobalytics>,
    marathondb: MarathonDb,
    mapgenie: MapGenie,
    fandom: Fandom,

    weapon_cache: Cache<String, Arc<Weapon>>,
    weapon_list_cache: Cache<(), Arc<[Weapon]>>,
    runner_cache: Cache<String, Arc<Runner>>,
    runner_list_cache: Cache<(), Arc<[Runner]>>,
    map_cache: Cache<String, Arc<MarathonMap>>,
    map_list_cache: Cache<(), Arc<[MarathonMap]>>,
    faction_cache: Cache<String, Arc<Faction>>,
    faction_list_cache: Cache<(), Arc<[Faction]>>,
    build_cache: Cache<String, Arc<BuildRecipe>>,
    build_list_cache: Cache<(), Arc<[BuildRecipe]>>,
    cradle_cache: Cache<(), Arc<Cradle>>,
    meta_cache: Cache<(), Arc<[MetaEntry]>>,
}

impl MarathonClient {
    #[must_use]
    pub fn new(client: Client, flaresolverr_url: Option<String>) -> Self {
        let mobalytics =
            flaresolverr_url.map(|url| Mobalytics::new(client.clone(), url));

        Self {
            mobalytics,
            marathondb: MarathonDb::new(client.clone()),
            mapgenie: MapGenie::new(client.clone()),
            fandom: Fandom::new(client),
            weapon_cache: ttl_cache(),
            weapon_list_cache: ttl_cache(),
            runner_cache: ttl_cache(),
            runner_list_cache: ttl_cache(),
            map_cache: ttl_cache(),
            map_list_cache: ttl_cache(),
            faction_cache: ttl_cache(),
            faction_list_cache: ttl_cache(),
            build_cache: ttl_cache(),
            build_list_cache: ttl_cache(),
            cradle_cache: ttl_cache(),
            meta_cache: ttl_cache(),
        }
    }
}
