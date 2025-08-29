use std::collections::HashSet;

use async_trait::async_trait;
use serenity::all::{ChannelId, UserId};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};

use crate::Result;

#[async_trait]
pub trait VoiceChannelManager<Db: Database> {
    async fn get(pool: &Pool<Db>, id: ChannelId) -> sqlx::Result<Option<VoiceChannelRow>>;
    async fn count_persistent_channels(pool: &Pool<Db>, user_id: UserId) -> sqlx::Result<i64>;
    async fn save(pool: &Pool<Db>, row: VoiceChannelRow) -> sqlx::Result<Db::QueryResult>;
    async fn delete(pool: &Pool<Db>, id: ChannelId) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct VoiceChannelRow {
    pub id: i64,
    pub owner_id: i64,
    pub trusted_ids: Vec<i64>,
    pub invites: Vec<i64>,
    pub password: Option<String>,
    pub persistent: bool,
    pub mode: VoiceChannelMode,
}

impl VoiceChannelRow {
    pub fn new(id: impl Into<ChannelId>, owner_id: impl Into<UserId>) -> Self {
        Self {
            id: id.into().get() as i64,
            owner_id: owner_id.into().get() as i64,
            trusted_ids: Vec::new(),
            invites: Vec::new(),
            password: None,
            persistent: false,
            mode: VoiceChannelMode::Open,
        }
    }

    pub fn new_persistent(id: impl Into<ChannelId>, owner_id: impl Into<UserId>) -> Self {
        Self {
            id: id.into().get() as i64,
            owner_id: owner_id.into().get() as i64,
            trusted_ids: Vec::new(),
            invites: Vec::new(),
            password: None,
            persistent: true,
            mode: VoiceChannelMode::Open,
        }
    }

    pub fn channel_id(&self) -> ChannelId {
        ChannelId::new(self.id as u64)
    }

    pub fn owner_id(&self) -> UserId {
        UserId::new(self.owner_id as u64)
    }

    pub fn trusted_ids(&self) -> HashSet<UserId> {
        self.trusted_ids
            .iter()
            .map(|id| UserId::new(*id as u64))
            .collect()
    }

    pub fn invites(&self) -> HashSet<UserId> {
        self.invites
            .iter()
            .map(|id| UserId::new(*id as u64))
            .collect()
    }

    pub fn is_owner(&self, user_id: impl Into<UserId>) -> bool {
        self.owner_id() == user_id.into()
    }

    pub fn set_owner(&mut self, id: impl Into<UserId>) {
        self.owner_id = id.into().get() as i64;
    }

    pub fn is_trusted(&self, user_id: impl Into<UserId>) -> bool {
        let user_id = user_id.into();

        self.trusted_ids().contains(&user_id) || self.owner_id() == user_id
    }

    pub fn verify_password(&self, pass: &str) -> bool {
        self.password.as_deref() == Some(pass)
    }

    pub fn is_persistent(&self) -> bool {
        self.persistent
    }

    pub fn toggle_persist(&mut self) {
        self.persistent = !self.persistent;
    }

    pub fn trust(&mut self, id: impl Into<UserId>) {
        self.trusted_ids.push(id.into().get() as i64);
    }

    pub fn untrust(&mut self, id: impl Into<UserId>) {
        let id = id.into();

        self.trusted_ids
            .retain(|trusted_id| *trusted_id != id.get() as i64);
    }

    pub fn create_invite(&mut self, id: impl Into<UserId>) {
        self.invites.push(id.into().get() as i64);
    }

    pub fn block(&mut self, id: impl Into<UserId>) {
        let id = id.into();

        self.trusted_ids
            .retain(|trusted_id| *trusted_id != id.get() as i64);
        self.invites.retain(|invite| *invite != id.get() as i64);
    }

    pub fn reset(&mut self) {
        self.trusted_ids.clear();
        self.invites.clear();
        self.password = None;
    }

    pub async fn save<Db: Database, Manager: VoiceChannelManager<Db>>(
        self,
        pool: &Pool<Db>,
    ) -> Result<()> {
        Manager::save(pool, self).await.unwrap();

        Ok(())
    }

    pub async fn delete<Db: Database, Manager: VoiceChannelManager<Db>>(
        self,
        pool: &Pool<Db>,
    ) -> Result<()> {
        Manager::delete(pool, self.channel_id()).await.unwrap();

        Ok(())
    }
}

#[derive(sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
pub enum VoiceChannelMode {
    Open,
    Spectator,
    Locked,
    Invisible,
}
