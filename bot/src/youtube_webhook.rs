use std::sync::Arc;

use serenity::all::Context;
use tokio::sync::broadcast::error::RecvError;
use tracing::warn;
use youtube::YoutubeRuntime;
use zayden_app::events::AppEvent;
use zayden_app::state::AppState;

pub fn spawn_youtube_listener(
    ctx: Context,
    app: Arc<AppState>,
    runtime: YoutubeRuntime,
) {
    tokio::spawn(async move {
        let mut rx = app.subscribe();

        loop {
            match rx.recv().await {
                Ok(AppEvent::YoutubeUpload(channel_id)) => {
                    let ctx = ctx.clone();
                    let app = Arc::clone(&app);
                    let api_key = Arc::clone(&runtime.api_key);

                    tokio::spawn(async move {
                        youtube::announce::on_ping(
                            &ctx.http,
                            &app.http,
                            &app.db,
                            &api_key,
                            &channel_id,
                        )
                        .await;
                    });
                },
                Ok(
                    AppEvent::ConfigChanged(_)
                    | AppEvent::ModulesChanged(_)
                    | AppEvent::EntitlementChanged(_)
                    | AppEvent::PatreonPost(_)
                    | AppEvent::HostingPaid(_),
                ) => {},
                Err(RecvError::Lagged(n)) => {
                    warn!(
                        n,
                        "youtube webhook listener lagged; the poll will catch up"
                    );
                },
                Err(RecvError::Closed) => break,
            }
        }
    });
}
