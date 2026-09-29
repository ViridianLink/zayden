use std::sync::{Arc, OnceLock};

use tokio::sync::broadcast::error::RecvError;
use zayden_app::events::AppEvent;
use zayden_app::state::AppState;

static SPAWNED: OnceLock<()> = OnceLock::new();

pub fn spawn_cache_listener(app: Arc<AppState>) {
    // `ready` fires again on every gateway reconnect.
    if SPAWNED.set(()).is_err() {
        return;
    }

    tokio::spawn(async move {
        let mut rx = app.subscribe();

        loop {
            match rx.recv().await {
                Ok(AppEvent::LoadoutsChanged | AppEvent::Resync)
                | Err(RecvError::Lagged(_)) => {
                    destiny2::loadouts::invalidate_cache().await;
                },
                Ok(
                    AppEvent::ConfigChanged(_)
                    | AppEvent::ModulesChanged(_)
                    | AppEvent::EntitlementChanged(_)
                    | AppEvent::PatreonPost(_)
                    | AppEvent::YoutubeUpload(_)
                    | AppEvent::HostingPaid(_)
                    | AppEvent::ServingChanged(_)
                    | AppEvent::CustomBotsChanged(_),
                ) => {},
                Err(RecvError::Closed) => break,
            }
        }
    });
}
