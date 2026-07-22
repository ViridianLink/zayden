use std::collections::HashSet;

use async_trait::async_trait;
use serenity::all::{ChannelId, UserId};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use zayden_core::{as_i64, as_u64};

use crate::Result;

#[async_trait]
pub trait VoiceChannelManager<Db: Database> {
    async fn get(
        pool: &Pool<Db>,
        id: ChannelId,
    ) -> sqlx::Result<Option<VoiceChannelRow>>;

    async fn count_persistent_channels(
        pool: &Pool<Db>,
        user_id: UserId,
    ) -> sqlx::Result<i64>;

    async fn save(
        pool: &Pool<Db>,
        row: VoiceChannelRow,
    ) -> sqlx::Result<Db::QueryResult>;

    async fn claim(
        pool: &Pool<Db>,
        id: ChannelId,
        expected_owner: UserId,
        new_owner: UserId,
    ) -> sqlx::Result<bool>;

    async fn delete(pool: &Pool<Db>, id: ChannelId)
    -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow, Clone)]
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
    #[must_use]
    pub const fn new(id: ChannelId, owner_id: UserId) -> Self {
        Self {
            id: as_i64(id.get()),
            owner_id: as_i64(owner_id.get()),
            trusted_ids: Vec::new(),
            invites: Vec::new(),
            password: None,
            persistent: false,
            mode: VoiceChannelMode::Open,
        }
    }

    #[must_use]
    pub const fn new_persistent(id: ChannelId, owner_id: UserId) -> Self {
        Self {
            id: as_i64(id.get()),
            owner_id: as_i64(owner_id.get()),
            trusted_ids: Vec::new(),
            invites: Vec::new(),
            password: None,
            persistent: true,
            mode: VoiceChannelMode::Open,
        }
    }

    #[must_use]
    pub const fn channel_id(&self) -> ChannelId {
        ChannelId::new(as_u64(self.id))
    }

    #[must_use]
    pub const fn owner_id(&self) -> UserId {
        UserId::new(as_u64(self.owner_id))
    }

    #[must_use]
    pub fn trusted_ids(&self) -> HashSet<UserId> {
        self.trusted_ids.iter().map(|id| UserId::new(as_u64(*id))).collect()
    }

    #[must_use]
    pub fn invites(&self) -> HashSet<UserId> {
        self.invites.iter().map(|id| UserId::new(as_u64(*id))).collect()
    }

    pub fn is_owner(&self, user_id: impl Into<UserId>) -> bool {
        self.owner_id() == user_id.into()
    }

    pub fn set_owner(&mut self, id: impl Into<UserId>) {
        self.owner_id = as_i64(id.into().get());
    }

    pub fn is_trusted(&self, user_id: impl Into<UserId>) -> bool {
        let user_id = user_id.into();

        self.trusted_ids().contains(&user_id) || self.owner_id() == user_id
    }

    #[must_use]
    pub fn verify_password(&self, pass: &str) -> bool {
        self.password.as_deref() == Some(pass)
    }

    #[must_use]
    pub const fn is_persistent(&self) -> bool {
        self.persistent
    }

    pub const fn toggle_persist(&mut self) {
        self.persistent = !self.persistent;
    }

    pub fn trust(&mut self, id: impl Into<UserId>) {
        self.trusted_ids.push(as_i64(id.into().get()));
    }

    pub fn untrust(&mut self, id: impl Into<UserId>) {
        let id = as_i64(id.into().get());

        self.trusted_ids.retain(|trusted_id| *trusted_id != id);
    }

    pub fn create_invite(&mut self, id: impl Into<UserId>) {
        self.invites.push(as_i64(id.into().get()));
    }

    pub fn block(&mut self, id: impl Into<UserId>) {
        let id = as_i64(id.into().get());

        self.trusted_ids.retain(|trusted_id| *trusted_id != id);
        self.invites.retain(|invite| *invite != id);
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
        Manager::save(pool, self).await?;
        Ok(())
    }

    pub async fn delete<Db: Database, Manager: VoiceChannelManager<Db>>(
        self,
        pool: &Pool<Db>,
    ) -> Result<()> {
        Manager::delete(pool, self.channel_id()).await?;
        Ok(())
    }
}

#[derive(sqlx::Type, Clone)]
#[sqlx(rename_all = "lowercase")]
pub enum VoiceChannelMode {
    Open,
    Spectator,
    Locked,
    Invisible,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_persistent_registers_caller_as_owner_of_an_open_channel() {
        let channel_id = ChannelId::new(123);
        let owner_id = UserId::new(456);

        let row = VoiceChannelRow::new_persistent(channel_id, owner_id);

        assert_eq!(row.channel_id(), channel_id);
        assert_eq!(row.owner_id(), owner_id);
        assert!(row.is_owner(owner_id));
        assert!(row.is_persistent());
        assert!(matches!(row.mode, VoiceChannelMode::Open));
        assert!(row.trusted_ids.is_empty());
        assert!(row.invites.is_empty());
        assert!(row.password.is_none());
    }
}
