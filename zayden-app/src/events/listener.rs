use sqlx::PgPool;
use sqlx::postgres::PgListener;
use tokio::sync::broadcast;
use tracing::warn;

use super::AppEvent;

/// Postgres `LISTEN/NOTIFY` listener that forwards `config_changed` payloads
/// onto the in-process `AppEvent` broadcast channel.
///
/// Wraps `sqlx::postgres::PgListener`, which reconnects automatically on
/// transient network interruptions. Only a fatal connection error will cause
/// the loop to exit.
pub struct ConfigListener;

impl ConfigListener {
    /// Connect to Postgres, subscribe to `config_changed`, and forward each
    /// notification as `AppEvent::ConfigChanged(guild_id)`.
    ///
    /// Runs indefinitely; intended to be spawned with `tokio::spawn`.
    pub async fn listen(pool: &PgPool, events: broadcast::Sender<AppEvent>) {
        let mut listener = match PgListener::connect_with(pool).await {
            Ok(l) => l,
            Err(e) => {
                warn!("ConfigListener: failed to connect: {e}");
                return;
            },
        };

        if let Err(e) = listener.listen("config_changed").await {
            warn!("ConfigListener: LISTEN failed: {e}");
            return;
        }

        loop {
            match listener.recv().await {
                Ok(notification) => {
                    if let Ok(guild_id) = notification.payload().parse::<u64>() {
                        let _ = events.send(AppEvent::ConfigChanged(guild_id));
                    } else {
                        warn!(
                            "ConfigListener: unparseable payload: {}",
                            notification.payload()
                        );
                    }
                },
                Err(e) => {
                    warn!("ConfigListener: fatal recv error: {e}");
                    break;
                },
            }
        }
    }
}
