use sqlx::PgPool;
use sqlx::postgres::PgListener;
use tokio::sync::broadcast;
use tracing::warn;

use super::AppEvent;
use crate::entitlement::EntitlementScope;

/// Postgres `LISTEN/NOTIFY` listener that forwards `config_changed` and
/// `entitlement_changed` payloads onto the in-process `AppEvent` broadcast channel.
///
/// Wraps `sqlx::postgres::PgListener`, which reconnects automatically on
/// transient network interruptions. Only a fatal connection error will cause
/// the loop to exit.
pub struct EventListener;

impl EventListener {
    /// Connect to Postgres, subscribe to both notification channels, and forward
    /// each notification as the appropriate `AppEvent` variant.
    ///
    /// Runs indefinitely; intended to be spawned with `tokio::spawn` or via
    /// [`Self::spawn`].
    pub async fn listen(pool: &PgPool, events: broadcast::Sender<AppEvent>) {
        let mut listener = match PgListener::connect_with(pool).await {
            Ok(l) => l,
            Err(e) => {
                warn!("EventListener: failed to connect: {e}");
                return;
            },
        };

        if let Err(e) =
            listener.listen_all(["config_changed", "entitlement_changed"]).await
        {
            warn!("EventListener: LISTEN failed: {e}");
            return;
        }

        loop {
            match listener.recv().await {
                Ok(notification) => match notification.channel() {
                    "config_changed" => {
                        if let Ok(guild_id) = notification.payload().parse::<u64>() {
                            let _ = events.send(AppEvent::ConfigChanged(guild_id));
                        } else {
                            warn!(
                                "EventListener: unparseable config_changed payload: {}",
                                notification.payload()
                            );
                        }
                    },
                    "entitlement_changed" => {
                        match EntitlementScope::from_notify_payload(
                            notification.payload(),
                        ) {
                            Ok(scope) => {
                                let _ =
                                    events.send(AppEvent::EntitlementChanged(scope));
                            },
                            Err(e) => {
                                warn!(
                                    "EventListener: unparseable entitlement_changed payload: {e}"
                                );
                            },
                        }
                    },
                    other => {
                        warn!("EventListener: unexpected channel: {other}");
                    },
                },
                Err(e) => {
                    warn!("EventListener: fatal recv error: {e}");
                    break;
                },
            }
        }
    }

    /// Spawn the listener as a background `tokio` task.
    pub fn spawn(pool: PgPool, events: broadcast::Sender<AppEvent>) {
        tokio::spawn(async move {
            Self::listen(&pool, events).await;
        });
    }
}
