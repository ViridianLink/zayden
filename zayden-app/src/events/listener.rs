use std::time::Duration;

use sqlx::PgPool;
use sqlx::postgres::{PgListener, PgNotification};
use tokio::sync::broadcast::Sender;
use tokio::time::sleep;
use tracing::{info, warn};

use super::{AppEvent, Channel};

const MIN_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(60);

pub struct EventListener;

impl EventListener {
    pub async fn listen(pool: &PgPool, events: Sender<AppEvent>) {
        let mut backoff = MIN_BACKOFF;
        let mut missed = false;

        loop {
            let error = match Self::subscribe(pool).await {
                Ok(mut listener) => {
                    backoff = MIN_BACKOFF;
                    if missed {
                        info!("EventListener: reconnected; resyncing caches");
                        let _ = events.send(AppEvent::Resync);
                    }
                    Self::pump(&mut listener, &events).await
                },
                Err(e) => Some(e),
            };

            match error {
                Some(sqlx::Error::PoolClosed) => return,
                Some(e) => warn!(?backoff, "EventListener: connection failed: {e}"),
                None => warn!(?backoff, "EventListener: connection lost"),
            }

            missed = true;
            sleep(backoff).await;
            backoff = backoff.saturating_mul(2).min(MAX_BACKOFF);
        }
    }

    async fn subscribe(pool: &PgPool) -> sqlx::Result<PgListener> {
        let mut listener = PgListener::connect_with(pool).await?;
        listener.listen_all(Channel::ALL.map(Channel::as_str)).await?;
        Ok(listener)
    }

    async fn pump(
        listener: &mut PgListener,
        events: &Sender<AppEvent>,
    ) -> Option<sqlx::Error> {
        loop {
            match listener.try_recv().await {
                Ok(Some(notification)) => Self::dispatch(&notification, events),
                Ok(None) => return None,
                Err(e) => return Some(e),
            }
        }
    }

    fn dispatch(notification: &PgNotification, events: &Sender<AppEvent>) {
        let name = notification.channel();
        let payload = notification.payload();

        let Some(channel) = Channel::from_name(name) else {
            warn!("EventListener: unexpected channel: {name}");
            return;
        };

        match channel.decode(payload) {
            Some(event) => {
                let _ = events.send(event);
            },
            None => warn!("EventListener: unparseable {name} payload: {payload}"),
        }
    }

    pub fn spawn(pool: PgPool, events: Sender<AppEvent>) {
        tokio::spawn(async move {
            Self::listen(&pool, events).await;
        });
    }
}
