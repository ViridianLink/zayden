use std::collections::HashMap;

use futures::TryStreamExt;
use serenity::all::{GuildId, User, UserId};
use sqlx::{FromRow, PgPool};
use zayden_core::{as_i64, as_u64};

use crate::relationships::Relationships;

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

    pub async fn get(pool: &PgPool, guild_id: GuildId) -> sqlx::Result<Self> {
        let gid = as_i64(guild_id.get());

        let max_partners: Option<i32> = sqlx::query_scalar!(
            "SELECT max_partners FROM family_settings WHERE guild_id = $1",
            gid
        )
        .fetch_optional(pool)
        .await?;

        Ok(max_partners.map_or_else(Self::default, Self::new))
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

    pub async fn get(
        pool: &PgPool,
        guild_id: GuildId,
        user_id: UserId,
    ) -> sqlx::Result<Option<Self>> {
        let gid = as_i64(guild_id.get());
        let uid = as_i64(user_id.get());

        let username: Option<String> = sqlx::query_scalar!(
            "SELECT u.username FROM family f \
             JOIN users u ON u.id = f.user_id \
             WHERE f.guild_id = $1 AND f.user_id = $2",
            gid,
            uid
        )
        .fetch_optional(pool)
        .await?;

        let Some(username) = username else {
            return Ok(None);
        };

        // family_partners stores (LEAST, GREATEST) so we query both columns.
        let partner_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT partner_id FROM family_partners WHERE guild_id = $1 AND user_id = $2 \
             UNION ALL \
             SELECT user_id FROM family_partners WHERE guild_id = $1 AND partner_id = $2",
            gid,
            uid
        )
        .fetch(pool)
        .try_filter_map(|x| std::future::ready(Ok(x)))
        .try_collect()
        .await?;

        let parent_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT parent_id FROM family_parent_child WHERE guild_id = $1 AND child_id = $2",
            gid,
            uid
        )
        .fetch_all(pool)
        .await?;

        let children_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT child_id FROM family_parent_child WHERE guild_id = $1 AND parent_id = $2",
            gid,
            uid
        )
        .fetch_all(pool)
        .await?;

        let blocked_ids: Vec<i64> = sqlx::query_scalar!(
            "SELECT blocked_id FROM family_blocks WHERE guild_id = $1 AND user_id = $2",
            gid,
            uid
        )
        .fetch_all(pool)
        .await?;

        Ok(Some(Self {
            guild_id: gid,
            id: uid,
            username,
            partner_ids,
            parent_ids,
            children_ids,
            blocked_ids,
        }))
    }

    async fn build_tree(
        pool: &PgPool,
        guild_id: GuildId,
        user_id: UserId,
        mut tree: HashMap<i32, Vec<Self>>,
        depth: i32,
        add_parents: bool,
        add_partners: bool,
    ) -> sqlx::Result<HashMap<i32, Vec<Self>>> {
        let signed_id = as_i64(user_id.get());

        // Cycle prevention: skip if already in tree.
        if tree.values().flatten().any(|row| row.id == signed_id) {
            return Ok(tree);
        }

        let Some(row) = Self::get(pool, guild_id, user_id).await? else {
            return Ok(tree);
        };

        let partner_ids = row.partner_ids.clone();
        let parent_ids = row.parent_ids.clone();
        let children_ids = row.children_ids.clone();

        tree.entry(depth).or_default().push(row);

        if add_parents {
            for parent_id in parent_ids {
                let pid = UserId::new(as_u64(parent_id));
                tree = Box::pin(Self::build_tree(
                    pool,
                    guild_id,
                    pid,
                    tree,
                    depth - 1,
                    true,
                    add_partners,
                ))
                .await?;
            }
        }

        if add_partners {
            for partner_id in partner_ids {
                let pid = UserId::new(as_u64(partner_id));
                // Don't recurse into partners' partners to prevent runaway
                // expansion.
                tree = Box::pin(Self::build_tree(
                    pool, guild_id, pid, tree, depth, false, false,
                ))
                .await?;
            }
        }

        for child_id in children_ids {
            let cid = UserId::new(as_u64(child_id));
            tree = Box::pin(Self::build_tree(
                pool,
                guild_id,
                cid,
                tree,
                depth + 1,
                false,
                add_partners,
            ))
            .await?;
        }

        Ok(tree)
    }

    pub async fn tree(self, pool: &PgPool) -> sqlx::Result<HashMap<i32, Vec<Self>>> {
        Self::build_tree(
            pool,
            GuildId::new(as_u64(self.guild_id)),
            UserId::new(as_u64(self.id)),
            HashMap::new(),
            0,
            true,
            true,
        )
        .await
    }

    pub async fn reset(pool: &PgPool, guild_id: GuildId) -> sqlx::Result<()> {
        let gid = as_i64(guild_id.get());

        sqlx::query!("DELETE FROM family WHERE guild_id = $1", gid)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn save(&self, pool: &PgPool) -> sqlx::Result<()> {
        let gid = self.guild_id;

        sqlx::query!(
            "INSERT INTO users (id, username) VALUES ($1, $2) \
             ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username",
            self.id,
            &self.username
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            "INSERT INTO family (guild_id, user_id) VALUES ($1, $2) \
             ON CONFLICT DO NOTHING",
            gid,
            self.id
        )
        .execute(pool)
        .await?;

        // Sync partners. The schema enforces user_id < partner_id via CHECK.
        for &partner_id in &self.partner_ids {
            ensure_family_member(pool, gid, partner_id).await?;
            let (uid, pid) = if self.id < partner_id {
                (self.id, partner_id)
            } else {
                (partner_id, self.id)
            };
            sqlx::query!(
                "INSERT INTO family_partners (guild_id, user_id, partner_id) \
                 VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                gid,
                uid,
                pid
            )
            .execute(pool)
            .await?;
        }

        // Sync children (this user is the parent).
        for &child_id in &self.children_ids {
            ensure_family_member(pool, gid, child_id).await?;
            sqlx::query!(
                "INSERT INTO family_parent_child (guild_id, parent_id, child_id) \
                 VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                gid,
                self.id,
                child_id
            )
            .execute(pool)
            .await?;
        }

        // Sync parents (this user is the child).
        for &parent_id in &self.parent_ids {
            ensure_family_member(pool, gid, parent_id).await?;
            sqlx::query!(
                "INSERT INTO family_parent_child (guild_id, parent_id, child_id) \
                 VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                gid,
                parent_id,
                self.id
            )
            .execute(pool)
            .await?;
        }

        // Sync blocked users.
        for &blocked_id in &self.blocked_ids {
            ensure_family_member(pool, gid, blocked_id).await?;
            sqlx::query!(
                "INSERT INTO family_blocks (guild_id, user_id, blocked_id) \
                 VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
                gid,
                self.id,
                blocked_id
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    pub async fn remove_partner(
        pool: &PgPool,
        guild_id: GuildId,
        user_id: UserId,
        partner_id: UserId,
    ) -> sqlx::Result<()> {
        let gid: i64 = as_i64(guild_id.get());
        let uid: i64 = as_i64(user_id.get());
        let pid: i64 = as_i64(partner_id.get());
        let (lo, hi) = if uid < pid { (uid, pid) } else { (pid, uid) };

        sqlx::query!(
            "DELETE FROM family_partners \
             WHERE guild_id = $1 AND user_id = $2 AND partner_id = $3",
            gid,
            lo,
            hi
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn remove_block(
        pool: &PgPool,
        guild_id: GuildId,
        user_id: UserId,
        blocked_id: UserId,
    ) -> sqlx::Result<()> {
        let gid: i64 = as_i64(guild_id.get());
        let uid: i64 = as_i64(user_id.get());
        let bid: i64 = as_i64(blocked_id.get());

        sqlx::query!(
            "DELETE FROM family_blocks \
             WHERE guild_id = $1 AND user_id = $2 AND blocked_id = $3",
            gid,
            uid,
            bid
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}

async fn ensure_family_member(
    pool: &PgPool,
    guild_id: i64,
    id: i64,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO users (id, username) VALUES ($1, 'Unknown') ON CONFLICT DO NOTHING",
        id
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "INSERT INTO family (guild_id, user_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
        guild_id,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}
