use jiff::Timestamp;
use jiff_sqlx::{Timestamp as SqlxTimestamp, ToSqlx as _};
use serenity::all::{GuildId, UserId};
use sqlx::PgPool;
use zayden_core::{as_i64, as_u64};

use crate::error::{HostingError, Result};
use crate::pricing::Plan;

pub mod state {
    pub const PROVISIONING: &str = "provisioning";
    pub const TRIAL: &str = "trial";
    pub const ACTIVE: &str = "active";
    pub const SUSPENDED: &str = "suspended";
    pub const DELETED: &str = "deleted";
    pub const FAILED: &str = "failed";
}

pub mod billing {
    pub const KOFI: &str = "kofi";
    pub const ENTITLEMENT: &str = "entitlement";
    pub const TRIAL: &str = "trial";
}

#[derive(Debug, Clone)]
pub struct HostedServer {
    pub id: i64,
    pub owner_id: i64,
    pub guild_id: Option<i64>,
    pub game_key: String,
    pub plan: String,
    pub pelican_user_id: Option<i32>,
    pub pelican_server_id: Option<i32>,
    pub memory_mib: i32,
    pub cpu_percent: i32,
    pub disk_mib: i32,
    pub price_cents: i32,
    pub billing_source: String,
    pub claim_code: String,
    pub address: Option<String>,
    pub state: String,
    pub trial_ends_at: Option<jiff_sqlx::Timestamp>,
    pub paid_until: Option<jiff_sqlx::Timestamp>,
}

impl HostedServer {
    #[must_use]
    pub const fn owner(&self) -> UserId {
        UserId::new(as_u64(self.owner_id))
    }

    #[must_use]
    pub fn guild(&self) -> Option<GuildId> {
        self.guild_id.map(|id| GuildId::new(as_u64(id)))
    }

    pub fn plan(&self) -> Result<Plan> {
        Plan::from_key(&self.plan)
    }

    #[must_use]
    pub fn expires_at(&self) -> Option<Timestamp> {
        self.paid_until.or(self.trial_ends_at).map(jiff_sqlx::Timestamp::to_jiff)
    }

    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(
            self.state.as_str(),
            state::PROVISIONING | state::TRIAL | state::ACTIVE | state::SUSPENDED
        )
    }
}

pub async fn ensure_user(
    pool: &PgPool,
    user_id: UserId,
    username: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO users (id, username) VALUES ($1, $2) \
         ON CONFLICT (id) DO NOTHING",
        as_i64(user_id.get()),
        username
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn ensure_guild(pool: &PgPool, guild_id: GuildId) -> Result<()> {
    sqlx::query!(
        "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
        as_i64(guild_id.get())
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn panel_user_id(pool: &PgPool, user_id: UserId) -> Result<Option<i32>> {
    let id = sqlx::query_scalar!(
        "SELECT pelican_user_id FROM hosting_pelican_users WHERE user_id = $1",
        as_i64(user_id.get())
    )
    .fetch_optional(pool)
    .await?;

    Ok(id)
}

pub async fn link_panel_user(
    pool: &PgPool,
    user_id: UserId,
    pelican_user_id: i32,
    username: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO hosting_pelican_users (user_id, pelican_user_id, pelican_username) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (user_id) DO UPDATE \
         SET pelican_user_id = EXCLUDED.pelican_user_id, \
             pelican_username = EXCLUDED.pelican_username",
        as_i64(user_id.get()),
        pelican_user_id,
        username
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn live_count(pool: &PgPool, user_id: UserId) -> Result<i64> {
    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM hosted_servers \
         WHERE owner_id = $1 \
           AND state IN ('provisioning', 'trial', 'active', 'suspended')",
        as_i64(user_id.get())
    )
    .fetch_one(pool)
    .await?;

    Ok(count.unwrap_or(0))
}

pub async fn global_usage(pool: &PgPool) -> Result<(i64, i64)> {
    let row = sqlx::query!(
        r#"
        SELECT
            count(*) AS "servers!",
            coalesce(sum(memory_mib), 0)::bigint AS "memory!"
        FROM hosted_servers
        WHERE state IN ('provisioning', 'trial', 'active', 'suspended')
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok((row.servers, row.memory))
}

pub struct NewServer<'a> {
    pub owner_id: UserId,
    pub guild_id: Option<GuildId>,
    pub game_key: &'a str,
    pub plan: Plan,
    pub pelican_user_id: Option<i32>,
    pub price_cents: i64,
    pub billing_source: &'a str,
    pub claim_code: &'a str,
}

pub async fn insert_provisioning(pool: &PgPool, new: &NewServer<'_>) -> Result<i64> {
    let price: i32 = new
        .price_cents
        .try_into()
        .map_err(|_e| HostingError::internal("price does not fit in an i32"))?;

    let id = sqlx::query_scalar!(
        "INSERT INTO hosted_servers ( \
            owner_id, guild_id, game_key, plan, pelican_user_id, \
            memory_mib, cpu_percent, disk_mib, price_cents, billing_source, \
            claim_code \
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) \
         RETURNING id",
        as_i64(new.owner_id.get()),
        new.guild_id.map(|g| as_i64(g.get())),
        new.game_key,
        new.plan.key(),
        new.pelican_user_id,
        new.plan.memory_mib(),
        new.plan.cpu_percent(),
        new.plan.disk_mib(),
        price,
        new.billing_source,
        new.claim_code
    )
    .fetch_one(pool)
    .await?;

    Ok(id)
}

pub async fn get(pool: &PgPool, id: i64) -> Result<HostedServer> {
    let row = sqlx::query_as!(
        HostedServer,
        r#"
        SELECT
            id, owner_id, guild_id, game_key, plan, pelican_user_id,
            pelican_server_id, memory_mib, cpu_percent, disk_mib, price_cents,
            billing_source, claim_code, address, state,
            trial_ends_at AS "trial_ends_at: jiff_sqlx::Timestamp",
            paid_until AS "paid_until: jiff_sqlx::Timestamp"
        FROM hosted_servers
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(HostingError::ServerNotFound)?;

    Ok(row)
}

pub async fn get_owned(
    pool: &PgPool,
    id: i64,
    owner_id: UserId,
) -> Result<HostedServer> {
    let row = get(pool, id).await?;

    if row.owner_id != as_i64(owner_id.get()) {
        return Err(HostingError::ServerNotFound);
    }

    Ok(row)
}

pub async fn list_for_owner(
    pool: &PgPool,
    owner_id: UserId,
) -> Result<Vec<HostedServer>> {
    let rows = sqlx::query_as!(
        HostedServer,
        r#"
        SELECT
            id, owner_id, guild_id, game_key, plan, pelican_user_id,
            pelican_server_id, memory_mib, cpu_percent, disk_mib, price_cents,
            billing_source, claim_code, address, state,
            trial_ends_at AS "trial_ends_at: jiff_sqlx::Timestamp",
            paid_until AS "paid_until: jiff_sqlx::Timestamp"
        FROM hosted_servers
        WHERE owner_id = $1 AND state <> 'deleted'
        ORDER BY created_at
        "#,
        as_i64(owner_id.get())
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn mark_provisioned(
    pool: &PgPool,
    id: i64,
    pelican_user_id: i32,
    pelican_server_id: i32,
    uuid: &str,
    address: Option<&str>,
    trial_ends_at: Timestamp,
) -> Result<()> {
    #[expect(
        trivial_casts,
        reason = "not a cast: `as T` is sqlx's bind-param type-override \
                  syntax, required because TIMESTAMPTZ has no built-in jiff \
                  mapping"
    )]
    sqlx::query!(
        "UPDATE hosted_servers \
         SET pelican_user_id = $2, pelican_server_id = $3, \
             pelican_server_uuid = $4, address = $5, state = 'trial', \
             billing_source = 'trial', trial_ends_at = $6, updated_at = now() \
         WHERE id = $1",
        id,
        pelican_user_id,
        pelican_server_id,
        uuid,
        address,
        trial_ends_at.to_sqlx() as SqlxTimestamp
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_failed(pool: &PgPool, id: i64, reason: &str) -> Result<()> {
    sqlx::query!(
        "UPDATE hosted_servers \
         SET state = 'failed', failure_reason = $2, updated_at = now() \
         WHERE id = $1",
        id,
        reason
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_deleted(pool: &PgPool, id: i64) -> Result<()> {
    sqlx::query!(
        "UPDATE hosted_servers \
         SET state = 'deleted', delete_after = NULL, updated_at = now() \
         WHERE id = $1",
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn record_payment(
    pool: &PgPool,
    id: i64,
    paid_until: Timestamp,
) -> Result<()> {
    #[expect(
        trivial_casts,
        reason = "not a cast: `as T` is sqlx's bind-param type-override \
                  syntax, required because TIMESTAMPTZ has no built-in jiff \
                  mapping"
    )]
    sqlx::query!(
        "UPDATE hosted_servers \
         SET paid_until = $2, state = 'active', billing_source = 'kofi', \
             suspended_at = NULL, delete_after = NULL, reminded_at = NULL, \
             updated_at = now() \
         WHERE id = $1",
        id,
        paid_until.to_sqlx() as SqlxTimestamp
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub struct NewPayment<'a> {
    pub kofi_message_id: &'a str,
    pub server_id: Option<i64>,
    pub email: Option<&'a str>,
    pub tier_name: Option<&'a str>,
    pub amount_cents: i32,
    pub currency: &'a str,
    pub matched_by: Option<&'a str>,
}

pub async fn insert_payment(
    pool: &PgPool,
    payment: &NewPayment<'_>,
) -> Result<bool> {
    let inserted = sqlx::query!(
        "INSERT INTO hosting_payments ( \
            kofi_message_id, server_id, kofi_email, tier_name, amount_cents, \
            currency, matched_by \
         ) VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (kofi_message_id) DO NOTHING",
        payment.kofi_message_id,
        payment.server_id,
        payment.email,
        payment.tier_name,
        payment.amount_cents,
        payment.currency,
        payment.matched_by
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(inserted > 0)
}

pub async fn payable_for_owner(
    pool: &PgPool,
    owner_id: UserId,
) -> Result<Vec<HostedServer>> {
    let rows = sqlx::query_as!(
        HostedServer,
        r#"
        SELECT
            id, owner_id, guild_id, game_key, plan, pelican_user_id,
            pelican_server_id, memory_mib, cpu_percent, disk_mib, price_cents,
            billing_source, claim_code, address, state,
            trial_ends_at AS "trial_ends_at: jiff_sqlx::Timestamp",
            paid_until AS "paid_until: jiff_sqlx::Timestamp"
        FROM hosted_servers
        WHERE owner_id = $1
          AND state IN ('trial', 'active', 'suspended')
        ORDER BY coalesce(paid_until, trial_ends_at) NULLS FIRST, id
        "#,
        as_i64(owner_id.get())
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn find_by_claim_code(
    pool: &PgPool,
    claim_code: &str,
) -> Result<Option<HostedServer>> {
    let row = sqlx::query_as!(
        HostedServer,
        r#"
        SELECT
            id, owner_id, guild_id, game_key, plan, pelican_user_id,
            pelican_server_id, memory_mib, cpu_percent, disk_mib, price_cents,
            billing_source, claim_code, address, state,
            trial_ends_at AS "trial_ends_at: jiff_sqlx::Timestamp",
            paid_until AS "paid_until: jiff_sqlx::Timestamp"
        FROM hosted_servers
        WHERE claim_code = $1 AND state <> 'deleted'
        "#,
        claim_code
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn discord_id_for_email_hash(
    pool: &PgPool,
    email_hash: &str,
) -> Result<Option<i64>> {
    let id = sqlx::query_scalar!(
        "SELECT discord_user_id FROM kofi_links WHERE email_hash = $1",
        email_hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(id)
}

pub async fn attach_payment(
    pool: &PgPool,
    kofi_message_id: &str,
    server_id: i64,
    matched_by: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE hosting_payments SET server_id = $2, matched_by = $3 \
         WHERE kofi_message_id = $1",
        kofi_message_id,
        server_id,
        matched_by
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn notify_paid(pool: &PgPool, id: i64) -> Result<()> {
    sqlx::query!("SELECT pg_notify('hosting_paid', $1)", id.to_string())
        .execute(pool)
        .await?;

    Ok(())
}
