use std::collections::BTreeMap;
use std::time::Duration;

use jiff::{Timestamp, ToSpan as _};
use serenity::all::UserId;
use sqlx::PgPool;
use tracing::{error, warn};

use crate::error::{HostingError, Result};
use crate::{HostingRuntime, store};

const POLL_INTERVAL: Duration = Duration::from_secs(5);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(600);
const MAX_SERVER_NAME: usize = 60;

pub struct Provisioned {
    pub address: Option<String>,
    pub panel_username: String,
    pub password: Option<String>,
}

pub async fn provision(
    runtime: &HostingRuntime,
    pool: &PgPool,
    row_id: i64,
    owner: UserId,
    display_name: &str,
) -> Result<Provisioned> {
    let row = store::get(pool, row_id).await?;
    let game = runtime.catalog.get(&row.game_key)?;
    let plan = row.plan()?;

    let (panel_user_id, panel_username, password) =
        ensure_panel_user(runtime, pool, owner, display_name).await?;

    let egg = runtime.pelican.egg(game.egg_id).await?;

    let mut env: BTreeMap<String, String> = egg.default_env().into_iter().collect();
    for (key, value) in &game.env {
        env.insert(key.clone(), value.clone());
    }

    let name = server_name(&game.name, display_name);

    let server = runtime
        .pelican
        .create_server(
            &name,
            panel_user_id,
            &egg,
            env,
            plan.memory_mib(),
            plan.cpu_percent(),
            plan.disk_mib(),
            &runtime.config.location_ids,
        )
        .await
        .inspect_err(|e| {
            error!(error = %e, row_id, "hosting: panel refused server creation");
        })?;

    let installed = match runtime
        .pelican
        .await_install(server.id, POLL_INTERVAL, INSTALL_TIMEOUT)
        .await
    {
        Ok(installed) => installed,
        Err(e) => {
            roll_back(runtime, pool, row_id, server.id, &e).await;
            return Err(e);
        },
    };

    let address = installed.primary_address();
    let trial_ends_at = trial_deadline(runtime);

    store::mark_provisioned(
        pool,
        row_id,
        panel_user_id,
        installed.id,
        &installed.uuid,
        address.as_deref(),
        trial_ends_at,
    )
    .await?;

    Ok(Provisioned { address, panel_username, password })
}

#[must_use]
pub fn trial_deadline(runtime: &HostingRuntime) -> Timestamp {
    let hours = runtime.config.trial_hours.max(0);
    Timestamp::now().checked_add(hours.hours()).unwrap_or_else(|_e| Timestamp::now())
}

async fn roll_back(
    runtime: &HostingRuntime,
    pool: &PgPool,
    row_id: i64,
    pelican_server_id: i32,
    cause: &HostingError,
) {
    if let Err(e) = runtime.pelican.delete_server(pelican_server_id).await {
        warn!(
            error = %e,
            row_id,
            pelican_server_id,
            "hosting: rollback delete failed; the panel server is orphaned"
        );
    }

    if let Err(e) = store::mark_failed(pool, row_id, &cause.to_string()).await {
        error!(error = %e, row_id, "hosting: could not mark row failed");
    }
}

async fn ensure_panel_user(
    runtime: &HostingRuntime,
    pool: &PgPool,
    owner: UserId,
    display_name: &str,
) -> Result<(i32, String, Option<String>)> {
    let username = format!("discord{}", owner.get());
    let email = synthetic_email(&runtime.config.panel_url, owner);

    if let Some(id) = store::panel_user_id(pool, owner).await? {
        return Ok((id, username, None));
    }

    if let Some(existing) = runtime.pelican.find_user_by_email(&email).await? {
        store::link_panel_user(pool, owner, existing.id, &existing.username).await?;
        return Ok((existing.id, existing.username, None));
    }

    let (created, password) =
        runtime.pelican.create_user(&email, &username, display_name).await?;

    store::link_panel_user(pool, owner, created.id, &created.username).await?;

    Ok((created.id, created.username, Some(password)))
}

fn synthetic_email(panel_url: &str, owner: UserId) -> String {
    let host = panel_url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|h| h.contains('.'))
        .unwrap_or("panel.invalid");

    format!("discord-{}@{host}", owner.get())
}

fn server_name(game: &str, display_name: &str) -> String {
    let mut name = format!("{game} - {display_name}");

    if name.chars().count() > MAX_SERVER_NAME {
        name = name.chars().take(MAX_SERVER_NAME).collect();
    }

    name
}
