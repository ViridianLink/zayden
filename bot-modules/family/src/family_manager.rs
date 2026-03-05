use async_trait::async_trait;
use serenity::all::{User, UserId};
use sqlx::{Database, FromRow, Pool};
use std::collections::HashMap;

use crate::relationships::Relationships;
use crate::Result;

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
}

#[derive(Debug, Default, Clone, PartialEq, FromRow)]
pub struct FamilyRow {
    pub id: i64,
    pub username: String,
    pub partner_ids: Vec<i64>,
    pub parent_ids: Vec<i64>,
    pub children_ids: Vec<i64>,
    pub blocked_ids: Vec<i64>,
}

impl FamilyRow {
    pub fn new(id: i64, username: String) -> Self {
        Self {
            id,
            username,
            ..Default::default()
        }
    }

    pub fn add_blocked(&mut self, user_id: UserId) {
        self.blocked_ids.push(user_id.get() as i64);
    }

    pub fn remove_blocked(&mut self, user_id: UserId) {
        self.blocked_ids.retain(|id| *id != user_id.get() as i64);
    }

    pub fn add_child(&mut self, child: &FamilyRow) {
        self.children_ids.push(child.id);
    }

    pub fn add_partner(&mut self, partner: &FamilyRow) {
        self.partner_ids.push(partner.id);
    }

    pub fn add_parent(&mut self, parent: &FamilyRow) {
        self.parent_ids.push(parent.id);
    }

    pub fn relationship(&self, user_id: UserId) -> Relationships {
        let user_id = user_id.get() as i64;

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

    // async fn long_relationship<'a, Db: Database, Manager: FamilyManager<Db>>(
    //     &self,
    //     pool: &Pool<Db>,
    //     target_user: &FamilyRow,
    //     mut working_relationship: Vec<&'a str>,
    //     added_members: &mut HashSet<i64>,
    // ) -> Result<Vec<&'a str>> {
    //     if added_members.contains(&self.id) {
    //         return Ok(working_relationship);
    //     }

    //     if target_user.id == self.id {
    //         return Ok(working_relationship);
    //     }

    //     added_members.insert(self.id);

    //     for parent_id in self.parent_ids.iter() {
    //         if added_members.contains(parent_id) {
    //             continue;
    //         }

    //         let parent = match Manager::get_row(pool, self.id).await? {
    //             Some(parent) => parent,
    //             None => FamilyRow::new(*parent_id, "Unknown".to_string()),
    //         };

    //         working_relationship.push("parent");

    //         let old_len = working_relationship.len();

    //         working_relationship = Box::pin(parent.long_relationship::<Db, Manager>(
    //             pool,
    //             target_user,
    //             working_relationship,
    //             added_members,
    //         ))
    //         .await?;

    //         if working_relationship.len() != old_len {
    //             return Ok(working_relationship);
    //         }
    //     }

    //     for partner in self.partner_ids.iter() {
    //         if added_members.contains(partner) {
    //             continue;
    //         }

    //         let partner = match Manager::get_row(pool, *partner).await? {
    //             Some(partner) => partner,
    //             None => FamilyRow::new(*partner, "Unknown".to_string()),
    //         };

    //         working_relationship.push("partner");

    //         let old_len = working_relationship.len();

    //         working_relationship = Box::pin(partner.long_relationship::<Db, Manager>(
    //             pool,
    //             target_user,
    //             working_relationship,
    //             added_members,
    //         ))
    //         .await?;

    //         if working_relationship.len() != old_len {
    //             return Ok(working_relationship);
    //         }
    //     }

    //     for child in self.children_ids.iter() {
    //         if added_members.contains(child) {
    //             continue;
    //         }

    //         let child = match Manager::get_row(pool, *child).await? {
    //             Some(child) => child,
    //             None => FamilyRow::new(*child, "Unknown".to_string()),
    //         };

    //         working_relationship.push("child");

    //         let old_len = working_relationship.len();

    //         working_relationship = Box::pin(child.long_relationship::<Db, Manager>(
    //             pool,
    //             target_user,
    //             working_relationship,
    //             added_members,
    //         ))
    //         .await?;

    //         if working_relationship.len() != old_len {
    //             return Ok(working_relationship);
    //         }
    //     }

    //     Ok(working_relationship)
    // }

    pub async fn tree<Db: Database, Manager: FamilyManager<Db>>(
        self,
        pool: &Pool<Db>,
    ) -> Result<HashMap<i32, Vec<FamilyRow>>> {
        let tree = Manager::tree(
            pool,
            UserId::new(self.id as u64),
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
            id: user.id.get() as i64,
            username: user.display_name().to_string(),
            ..Default::default()
        }
    }
}

// struct RelationshipSimplifier {}

// impl RelationshipSimplifier {
//     pub fn cousin_string() {}

//     pub fn simplify(string: &str) -> Relationships {
//         Relationships::None
//     }
// }
