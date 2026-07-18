use std::collections::HashMap;

use async_trait::async_trait;
use serenity::all::{User, UserId};
use sqlx::{Database, FromRow, Pool};
use zayden_core::{as_i64, as_u64};

use crate::Result;
use crate::relationships::Relationships;

#[async_trait]
pub trait FamilyManager<Db: Database> {
    async fn row(
        pool: &Pool<Db>,
        user_id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<FamilyRow>>;

    async fn tree<'a>(
        pool: &Pool<Db>,
        user_id: impl Into<UserId> + Send,
        tree: HashMap<i32, Vec<FamilyRow>>,
        depth: i32,
        add_parents: bool,
        add_partners: bool,
    ) -> sqlx::Result<HashMap<i32, Vec<FamilyRow>>>;

    async fn reset(pool: &Pool<Db>) -> sqlx::Result<()>;

    async fn save(pool: &Pool<Db>, row: &FamilyRow) -> sqlx::Result<()>;

    async fn remove_partner(
        pool: &Pool<Db>,
        user_id: UserId,
        partner_id: UserId,
    ) -> sqlx::Result<()>;

    async fn remove_block(
        pool: &Pool<Db>,
        user_id: UserId,
        blocked_id: UserId,
    ) -> sqlx::Result<()>;
}

#[derive(Debug, Default, Clone, PartialEq, Eq, FromRow)]
pub struct FamilyRow {
    pub id: i64,
    pub username: String,
    pub partner_ids: Vec<i64>,
    pub parent_ids: Vec<i64>,
    pub children_ids: Vec<i64>,
    pub blocked_ids: Vec<i64>,
}

impl FamilyRow {
    #[must_use]
    pub fn new(id: i64, username: String) -> Self {
        Self { id, username, ..Default::default() }
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

impl From<&User> for FamilyRow {
    fn from(user: &User) -> Self {
        Self {
            id: as_i64(user.id.get()),
            username: user.display_name().to_string(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_detects_partner() {
        let mut row = FamilyRow::new(1, "Alice".to_string());
        let partner = FamilyRow::new(2, "Bob".to_string());
        row.add_partner(&partner);

        assert_eq!(row.relationship(UserId::new(2)), Relationships::Partner);
    }

    #[test]
    fn relationship_detects_parent() {
        let mut row = FamilyRow::new(1, "Alice".to_string());
        let parent = FamilyRow::new(2, "Bob".to_string());
        row.add_parent(&parent);

        assert_eq!(row.relationship(UserId::new(2)), Relationships::Parent);
    }

    #[test]
    fn relationship_detects_child() {
        let mut row = FamilyRow::new(1, "Alice".to_string());
        let child = FamilyRow::new(2, "Bob".to_string());
        row.add_child(&child);

        assert_eq!(row.relationship(UserId::new(2)), Relationships::Child);
    }

    #[test]
    fn relationship_none_for_unrelated_user() {
        let row = FamilyRow::new(1, "Alice".to_string());

        assert_eq!(row.relationship(UserId::new(2)), Relationships::None);
    }

    #[test]
    fn divorce_removes_partner_relationship() {
        let mut row = FamilyRow::new(1, "Alice".to_string());
        let partner = FamilyRow::new(2, "Bob".to_string());
        row.add_partner(&partner);
        assert_eq!(row.relationship(UserId::new(2)), Relationships::Partner);

        // Mirrors `FamilyTable::remove_partner`, which deletes the `family_partners`
        // row.
        row.partner_ids.retain(|id| *id != partner.id);

        assert_eq!(row.relationship(UserId::new(2)), Relationships::None);
    }
}
