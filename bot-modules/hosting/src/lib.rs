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
        pool: sqlx::PgPool,
        mut rx: Receiver<AppEvent>,
    ) {
        use tokio::sync::broadcast::error::RecvError;

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(AppEvent::HostingPaid(id)) => {
                        unsuspend_paid(&this, &pool, id).await;
                    },
                    Ok(_) => {},
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!(
                            n,
                            "hosting paid listener lagged; suspended servers \
                             will be picked up by the next payment or by hand"
                        );
                    },
                    Err(RecvError::Closed) => break,
                }
            }
        });
    }
}

async fn unsuspend_paid(runtime: &HostingRuntime, pool: &sqlx::PgPool, id: i64) {
    let row = match store::get(pool, id).await {
        Ok(row) => row,
        Err(e) => {
            tracing::error!(error = %e, row = id, "hosting: paid row vanished");
            return;
        },
    };

    let Some(server_id) = row.pelican_server_id else {
        return;
    };

    if let Err(e) = runtime.pelican.unsuspend(server_id).await {
        tracing::error!(
            error = %e,
            row = id,
            server_id,
            "hosting: unsuspend failed after payment; the owner has paid for a \
             server that is still down"
        );
    }
}
