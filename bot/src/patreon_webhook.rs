use std::sync::Arc;

use serenity::all::Context;
use tokio::sync::broadcast::error::RecvError;
use tracing::{error, warn};
use zayden_app::events::AppEvent;
use zayden_app::state::AppState;

pub fn spawn_patreon_listener(ctx: Context, app: Arc<AppState>) {
    tokio::spawn(async move {
        let mut rx = app.subscribe();

        loop {
            match rx.recv().await {
                Ok(AppEvent::PatreonPost(post_id)) => {
                    if let Err(e) =
                        patreon::announce_pending(&ctx.http, &app.http, &app.db)
                            .await
                    {
                        error!(
                            error = ?e,
                            post_id,
                            "patreon: failed to announce a webhook post"
                        );
                    }
                },
                Ok(AppEvent::ConfigChanged(_) | AppEvent::EntitlementChanged(_)) => {
                },
                Err(RecvError::Lagged(n)) => {
                    warn!(
                        n,
                        "patreon webhook listener lagged; the poll will catch up"
                    );
                },
                Err(RecvError::Closed) => break,
            }
        }
    });
}
