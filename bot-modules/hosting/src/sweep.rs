use jiff::Timestamp;
use serenity::all::UserId;
use sqlx::PgPool;
use zayden_core::as_u64;

use crate::error::Result;
use crate::pricing::Plan;

#[derive(Debug, Clone)]
pub struct Claimed {
    pub id: i64,
    pub owner_id: i64,
    pub game_key: String,
    pub plan: String,
    pub pelican_server_id: Option<i32>,
    pub price_cents: i32,
    pub billing_source: String,
    pub claim_code: String,
    pub has_paid: bool,
    pub expires_at: Option<jiff_sqlx::Timestamp>,
}

impl Claimed {
    #[must_use]
    pub const fn owner(&self) -> UserId {
        UserId::new(as_u64(self.owner_id))
    }

    pub fn plan(&self) -> Result<Plan> {
        Plan::from_key(&self.plan)
    }

    #[must_use]
    pub fn expires_at(&self) -> Option<Timestamp> {
        self.expires_at.map(jiff_sqlx::Timestamp::to_jiff)
    }
}

pub async fn claim_expired(
    pool: &PgPool,
    grace_days: i32,
    limit: i64,
) -> Result<Vec<Claimed>> {
    let rows = sqlx::query_as!(
        Claimed,
        r#"
        WITH due AS (
            SELECT s.id
            FROM hosted_servers s
            WHERE s.state IN ('trial', 'active')
              AND s.price_cents > 0
              AND coalesce(s.paid_until, s.trial_ends_at) < now()
            ORDER BY coalesce(s.paid_until, s.trial_ends_at)
            LIMIT $1
            FOR UPDATE OF s SKIP LOCKED
        )
        UPDATE hosted_servers s
        SET state = 'suspended',
            suspended_at = now(),
            delete_after = CASE
                WHEN s.paid_until IS NULL THEN now()
                ELSE now() + make_interval(days => $2)
            END,
            updated_at = now()
        FROM due
        WHERE s.id = due.id
        RETURNING
            s.id, s.owner_id, s.game_key, s.plan, s.pelican_server_id,
            s.price_cents, s.billing_source, s.claim_code,
            s.paid_until IS NOT NULL AS "has_paid!",
            coalesce(s.paid_until, s.trial_ends_at) AS "expires_at: jiff_sqlx::Timestamp"
        "#,
        limit,
        grace_days
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn convert_included_trials(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<Claimed>> {
    let rows = sqlx::query_as!(
        Claimed,
        r#"
        WITH due AS (
            SELECT s.id
            FROM hosted_servers s
            WHERE s.state = 'trial'
              AND s.price_cents = 0
              AND s.trial_ends_at < now()
            ORDER BY s.trial_ends_at
            LIMIT $1
            FOR UPDATE OF s SKIP LOCKED
        )
        UPDATE hosted_servers s
        SET state = 'active',
            billing_source = 'entitlement',
            reminded_at = NULL,
            updated_at = now()
        FROM due
        WHERE s.id = due.id
        RETURNING
            s.id, s.owner_id, s.game_key, s.plan, s.pelican_server_id,
            s.price_cents, s.billing_source, s.claim_code,
            s.paid_until IS NOT NULL AS "has_paid!",
            coalesce(s.paid_until, s.trial_ends_at) AS "expires_at: jiff_sqlx::Timestamp"
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn entitlement_backed(pool: &PgPool, limit: i64) -> Result<Vec<Claimed>> {
    let rows = sqlx::query_as!(
        Claimed,
        r#"
        SELECT
            s.id, s.owner_id, s.game_key, s.plan, s.pelican_server_id,
            s.price_cents, s.billing_source, s.claim_code,
            s.paid_until IS NOT NULL AS "has_paid!",
            coalesce(s.paid_until, s.trial_ends_at) AS "expires_at: jiff_sqlx::Timestamp"
        FROM hosted_servers s
        WHERE s.state = 'active' AND s.billing_source = 'entitlement'
        ORDER BY s.id
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn suspend(pool: &PgPool, id: i64, grace_days: i32) -> Result<()> {
    sqlx::query!(
        "UPDATE hosted_servers \
         SET state = 'suspended', suspended_at = now(), \
             delete_after = CASE \
                 WHEN paid_until IS NULL THEN now() \
                 ELSE now() + make_interval(days => $2) \
             END, \
             updated_at = now() \
         WHERE id = $1 AND state IN ('trial', 'active')",
        id,
        grace_days
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn claim_reminders(
    pool: &PgPool,
    reminder_days: i32,
    limit: i64,
) -> Result<Vec<Claimed>> {
    let rows = sqlx::query_as!(
        Claimed,
        r#"
        WITH due AS (
            SELECT s.id
            FROM hosted_servers s
            WHERE s.state IN ('trial', 'active')
              AND s.reminded_at IS NULL
              AND s.paid_until IS NOT NULL
              AND s.paid_until < now() + make_interval(days => $2)
            ORDER BY s.paid_until
            LIMIT $1
            FOR UPDATE OF s SKIP LOCKED
        )
        UPDATE hosted_servers s
        SET reminded_at = now()
        FROM due
        WHERE s.id = due.id
        RETURNING
            s.id, s.owner_id, s.game_key, s.plan, s.pelican_server_id,
            s.price_cents, s.billing_source, s.claim_code,
            s.paid_until IS NOT NULL AS "has_paid!",
            coalesce(s.paid_until, s.trial_ends_at) AS "expires_at: jiff_sqlx::Timestamp"
        "#,
        limit,
        reminder_days
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn claim_deletions(pool: &PgPool, limit: i64) -> Result<Vec<Claimed>> {
    let rows = sqlx::query_as!(
        Claimed,
        r#"
        WITH due AS (
            SELECT s.id
            FROM hosted_servers s
            WHERE s.state = 'suspended'
              AND s.delete_after IS NOT NULL
              AND s.delete_after <= now()
            ORDER BY s.delete_after
            LIMIT $1
            FOR UPDATE OF s SKIP LOCKED
        )
        UPDATE hosted_servers s
        SET delete_after = now() + interval '1 hour',
            updated_at = now()
        FROM due
        WHERE s.id = due.id
        RETURNING
            s.id, s.owner_id, s.game_key, s.plan, s.pelican_server_id,
            s.price_cents, s.billing_source, s.claim_code,
            s.paid_until IS NOT NULL AS "has_paid!",
            coalesce(s.paid_until, s.trial_ends_at) AS "expires_at: jiff_sqlx::Timestamp"
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
