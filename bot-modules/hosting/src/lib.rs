pub mod catalog;
pub mod commands;
pub mod components;
pub mod cron;
pub mod error;
pub mod kofi;
pub mod modal;
pub mod pelican;
pub mod pricing;
pub mod provision;
pub mod store;
pub mod sweep;

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::broadcast::Receiver;
use zayden_app::config::HostingConfig;
use zayden_app::events::AppEvent;

pub use crate::catalog::Catalog;
pub use crate::error::{HostingError, Result};
pub use crate::pelican::PelicanApp;
pub use crate::pricing::Plan;

#[derive(Debug, Clone)]
pub struct HostingRuntime {
    pub pelican: PelicanApp,
    pub catalog: Catalog,
    pub config: Arc<HostingConfig>,
}

impl HostingRuntime {
    #[must_use]
    pub fn new(
        http: reqwest::Client,
        pelican_base_url: &str,
        pelican_api_key: &str,
        config: HostingConfig,
    ) -> Self {
        Self {
            pelican: PelicanApp::new(http, pelican_base_url, pelican_api_key),
            catalog: Catalog::new(&config.games),
            config: Arc::new(config),
        }
    }
}

pub const COMMAND_NAME: &str = "server";

impl HostingRuntime {
    pub fn spawn_paid_listener(
        this: Arc<Self>,
        pool: PgPool,
        mut rx: Receiver<AppEvent>,
    ) {
        use tokio::sync::broadcast::error::RecvError;

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(AppEvent::HostingPaid(_) | AppEvent::Resync)
                    | Err(RecvError::Lagged(_)) => {
                        this.unsuspend_due(&pool).await;
                    },
                    Ok(_) => {},
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }

    pub async fn unsuspend_due(&self, pool: &PgPool) {
        const BATCH: i64 = 50;

        let rows = match sweep::claim_unsuspends(pool, BATCH).await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(error = %e, "hosting: unsuspend sweep failed");
                return;
            },
        };

        for row in rows {
            if let Some(server_id) = row.pelican_server_id
                && let Err(e) = self.pelican.unsuspend(server_id).await
            {
                // The lease runs out and the next sweep retries.
                tracing::error!(
                    error = %e,
                    row = row.id,
                    server_id,
                    "hosting: unsuspend failed after payment; will retry"
                );
                continue;
            }

            if let Err(e) = store::clear_unsuspend(pool, row.id).await {
                tracing::error!(
                    error = %e,
                    row = row.id,
                    "hosting: could not clear the unsuspend marker"
                );
            }
        }
    }
}
