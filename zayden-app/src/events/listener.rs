use sqlx::PgPool;
use sqlx::postgres::PgListener;
use tokio::sync::broadcast;
use tracing::warn;

use super::AppEvent;
use crate::entitlement::EntitlementScope;

pub struct EventListener;

impl EventListener {
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

    pub fn spawn(pool: PgPool, events: broadcast::Sender<AppEvent>) {
        tokio::spawn(async move {
            Self::listen(&pool, events).await;
        });
    }
}
