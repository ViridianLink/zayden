use std::collections::HashMap;

use async_trait::async_trait;
use serenity::all::{GuildId, User, UserId};
use sqlx::{Database, FromRow, Pool};
use zayden_core::{as_i64, as_u64};

use crate::Result;
use crate::relationships::Relationships;

#[async_trait]
pub trait FamilyManager<Db: Database> {
    async fn row(
        pool: &Pool<Db>,
        guild_id: GuildId,
        user_id: UserId,
    ) -> sqlx::Result<Option<FamilyRow>>;

    async fn tree<'a>(
        pool: &Pool<Db>,
        guild_id: GuildId,
        user_id: UserId,
        tree: HashMap<i32, Vec<FamilyRow>>,
        depth: i32,
        add_parents: bool,
        add_partners: bool,
    ) -> sqlx::Result<HashMap<i32, Vec<FamilyRow>>>;

    async fn reset(pool: &Pool<Db>, guild_id: GuildId) -> sqlx::Result<()>;

    async fn save(pool: &Pool<Db>, row: &FamilyRow) -> sqlx::Result<()>;

    async fn remove_partner(
        pool: &Pool<Db>,
        guild_id: GuildId,
        user_id: UserId,
        partner_id: UserId,
    ) -> sqlx::Result<()>;

    async fn remove_block(
        pool: &Pool<Db>,
        guild_id: GuildId,
        user_id: UserId,
        blocked_id: UserId,
    ) -> sqlx::Result<()>;

    async fn settings(
        pool: &Pool<Db>,
        guild_id: GuildId,
    ) -> sqlx::Result<FamilySettings>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilySettings {
    pub max_partners: i32,
}

impl Default for FamilySettings {
    fn default() -> Self {
        Self { max_partners: 1 }
    }
}

impl FamilySettings {
    #[must_use]
    pub const fn new(max_partners: i32) -> Self {
        Self { max_partners }
    }

    #[must_use]
    pub fn max_partners(&self) -> usize {
        usize::try_from(self.max_partners).unwrap_or(0)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, FromRow)]
pub struct FamilyRow {
    pub guild_id: i64,
    pub id: i64,
    pub username: String,
    pub partner_ids: Vec<i64>,
    pub parent_ids: Vec<i64>,
    pub children_ids: Vec<i64>,
    pub blocked_ids: Vec<i64>,
}

impl FamilyRow {
    #[must_use]
    pub fn new(guild_id: GuildId, user_id: UserId, username: String) -> Self {
        Self {
            guild_id: as_i64(guild_id.get()),
            id: as_i64(user_id.get()),
            username,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn from_user(guild_id: GuildId, user: &User) -> Self {
        Self::new(guild_id, user.id, user.display_name().to_string())
    }

    pub fn add_blocked(&mut self, user_id: UserId) {
        self.blocked_ids.push(as_i64(user_id.get()));
    }

    #[must_use]
    pub fn is_blocked(&self, user_id: UserId) -> bool {
        self.blocked_ids.contains(&as_i64(user_id.get()))
    }

    pub fn add_child(&mut self, child: &Self) {
        self.children_ids.push(child.id);
    }

    pub fn add_partner(&mut self, partner: &Self) {
        self.partner_ids.push(partner.id);
    }

    pub fn add_parent(&mut self, parent: &Self) {
        self.parent_ids.push(parent.id);
    }

    #[must_use]
    pub const fn at_partner_limit(&self, max_partners: usize) -> bool {
        self.partner_ids.len() >= max_partners
    }

    #[must_use]
    pub const fn is_adopted(&self) -> bool {
        !self.parent_ids.is_empty()
    }

    #[must_use]
    pub fn relationship(&self, user_id: UserId) -> Relationships {
        let user_id = as_i64(user_id.get());

        if self.partner_ids.contains(&user_id) {
            Relationships::Partner
        } else if self.parent_ids.contains(&user_id) {
            Relationships::Parent
        } else if self.children_ids.contains(&user_id) {
            Relationships::Child
        } else {
            Relationships::None
        }
    }

    pub async fn tree<Db: Database, Manager: FamilyManager<Db>>(
        self,
        pool: &Pool<Db>,
    ) -> Result<HashMap<i32, Vec<Self>>> {
        let tree = Manager::tree(
            pool,
            GuildId::new(as_u64(self.guild_id)),
            UserId::new(as_u64(self.id)),
            HashMap::new(),
            0,
            true,
            true,
        )
        .await?;

        Ok(tree)
    }

    pub async fn save<Db: Database, Manager: FamilyManager<Db>>(
        &self,
        pool: &Pool<Db>,
    ) -> Result<()> {
        Manager::save(pool, self).await?;
        Ok(())
    }
}
