use std::io;
use std::process::ExitCode;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::net::TcpListener;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::Router;
use tracing::{error, info, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, fmt};
use web::error::StartupError;
use web::state::WebState;
use web::{config, router, session_pruning};
use zayden_app::config::BotConfig;
use zayden_app::events::listener::EventListener;
use zayden_app::state::AppState as ZaydenAppState;

#[tokio::main]
async fn main() -> ExitCode {
    logging();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            ExitCode::FAILURE
        },
    }
}

async fn run() -> Result<(), StartupError> {
    if rustls::crypto::aws_lc_rs::default_provider().install_default().is_err() {
        warn!("Rustls CryptoProvider was already installed");
    }

    let _ = dotenvy::dotenv();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    let config = BotConfig::load(&pool).await?;

    let app = Arc::new(ZaydenAppState::new(pool, &config));
    let state = WebState::new(Arc::clone(&app), &config)?;
    let assets = AssetBundle::load().map_err(StartupError::AssetBundle)?;

    let addr = config::bind_addr(&config)?;
    let listener = TcpListener::bind(addr).await.map_err(StartupError::Listen)?;

    EventListener::spawn(app.db.clone(), app.events.clone());
    session_pruning::spawn(app.db.clone());

    info!("Dashboard listening on {addr}");
    let base = Router::builder().app_context(state).assets(assets);
    topcoat::serve(listener, router(base)).await.map_err(StartupError::Serve)
}

fn logging() {
    let stdout_log =
        fmt::layer().with_writer(io::stdout).with_filter(LevelFilter::INFO);

    Registry::default().with(stdout_log).init();
}
