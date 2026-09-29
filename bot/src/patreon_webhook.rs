use std::sync::Arc;

use serenity::all::Context;
use tokio::sync::broadcast::error::RecvError;
use tracing::{error, info, warn};
use zayden_app::events::AppEvent;
use zayden_app::state::AppState;

pub fn spawn_patreon_listener(ctx: Context, app: Arc<AppState>) {
    tokio::spawn(async move {
        let mut rx = app.subscribe();

        loop {
            match rx.recv().await {
                Ok(AppEvent::PatreonPost(post_id)) => {
                    info!(post_id, "patreon: webhook post received, announcing");
                    announce(&ctx, &app).await;
                },
                Ok(AppEvent::Resync) => announce(&ctx, &app).await,
                Ok(
                    AppEvent::ConfigChanged(_)
                    | AppEvent::ModulesChanged(_)
                    | AppEvent::EntitlementChanged(_)
                    | AppEvent::YoutubeUpload(_)
                    | AppEvent::HostingPaid(_)
                    | AppEvent::ServingChanged(_)
                    | AppEvent::CustomBotsChanged(_)
                    | AppEvent::LoadoutsChanged,
                ) => {},
                Err(RecvError::Lagged(n)) => {
                    warn!(
                        n,
                        "patreon webhook listener lagged; the poll will catch up"
                    );
                },
                Err(RecvError::Closed) => {
                    error!("patreon webhook listener stopped: the event bus closed");
                    break;
                },
            }
        }
    });
}

async fn announce(ctx: &Context, app: &AppState) {
    if let Err(e) = patreon::announce_pending(&ctx.http, &app.http, &app.db).await {
        error!(error = ?e, "patreon: failed to announce pending posts");
    }
}
