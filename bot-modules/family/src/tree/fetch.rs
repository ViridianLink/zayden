use futures::TryStreamExt;
use serenity::all::{GuildId, UserId};
use sqlx::PgPool;
use zayden_core::as_i64;

use crate::Result;
use crate::tree::TreeQuota;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPerson {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawGraph {
    pub people: Vec<RawPerson>,
    pub partners: Vec<(i64, i64)>,
    pub parents: Vec<(i64, i64)>,
    pub truncated: bool,
}

impl RawGraph {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.people.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.people.is_empty()
    }

    pub async fn fetch(
        pool: &PgPool,
        guild_id: GuildId,
        focus: UserId,
        quota: TreeQuota,
    ) -> Result<Self> {
        let gid = as_i64(guild_id.get());
        let uid = as_i64(focus.get());

        let people = sqlx::query_as!(
            RawPerson,
            r#"
            WITH RECURSIVE edges AS (
                    SELECT user_id AS a, partner_id AS b
                    FROM family_partners WHERE guild_id = $1
                UNION ALL
                    SELECT partner_id, user_id
                    FROM family_partners WHERE guild_id = $1
                UNION ALL
                    SELECT parent_id, child_id
                    FROM family_parent_child WHERE guild_id = $1
                UNION ALL
                    SELECT child_id, parent_id
                    FROM family_parent_child WHERE guild_id = $1
            ),
            component AS (
                    SELECT $2::bigint AS id
                UNION
                    SELECT e.b FROM component c JOIN edges e ON e.a = c.id
            )
            SELECT c.id AS "id!", u.username AS "username!"
            FROM component c
            JOIN users u ON u.id = c.id
            ORDER BY c.id
            LIMIT $3
            "#,
            gid,
            uid,
            quota.fetch_limit,
        )
        .fetch_all(pool)
        .await?;

        let truncated =
            i64::try_from(people.len()).unwrap_or(i64::MAX) >= quota.fetch_limit;

        if people.is_empty() {
            return Ok(Self::default());
        }

        let ids: Vec<i64> = people.iter().map(|person| person.id).collect();

        let partners = sqlx::query!(
            "SELECT user_id, partner_id FROM family_partners \
             WHERE guild_id = $1 AND user_id = ANY($2) \
             ORDER BY user_id, partner_id",
            gid,
            &ids,
        )
        .fetch(pool)
        .map_ok(|row| (row.user_id, row.partner_id))
        .try_collect::<Vec<_>>()
        .await?;

        let parents = sqlx::query!(
            "SELECT parent_id, child_id FROM family_parent_child \
             WHERE guild_id = $1 AND parent_id = ANY($2) \
             ORDER BY parent_id, child_id",
            gid,
            &ids,
        )
        .fetch(pool)
        .map_ok(|row| (row.parent_id, row.child_id))
        .try_collect::<Vec<_>>()
        .await?;

        Ok(Self { people, partners, parents, truncated })
    }
}
