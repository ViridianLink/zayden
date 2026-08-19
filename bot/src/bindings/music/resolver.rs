use std::sync::Arc;

use jiff::Timestamp;
use music::{
    CompositeResolver,
    CookieJar,
    Genre,
    JarStatus,
    RadioResolver,
    SpotifyResolver,
    TrackResolver,
    YouTubeResolver,
    probe_yt_dlp,
};
use tracing::{error, info, warn};
use zayden_app::config::BotConfig;

use crate::{BotError, Result};

pub async fn build_resolver(config: &BotConfig) -> Result<Arc<dyn TrackResolver>> {
    let youtube = YouTubeResolver::new().map_err(BotError::from)?;
    let youtube = match open_cookie_jar(config) {
        Some(jar) => youtube.with_cookies(jar),
        None => youtube,
    };

    match probe_yt_dlp().await {
        Ok(version) => info!("yt-dlp available (version {version})"),
        Err(e) => error!(
            "yt-dlp is unavailable ({e}); YouTube playback will NOT work. \
             Install yt-dlp on this host (and ideally a JS runtime such as \
             deno) and restart."
        ),
    }

    let spotify = match &config.spotify {
        Some(creds) => {
            let resolver = SpotifyResolver::new(
                creds.client_id.clone(),
                creds.client_secret.clone(),
            )
            .await
            .map_err(BotError::from)?;
            Some(resolver)
        },
        None => {
            warn!(
                "Spotify credentials not configured; Spotify links will be unsupported"
            );
            None
        },
    };

    let stations = Arc::clone(&config.radio_stations);
    if stations.is_empty() {
        warn!("no radio stations configured; /music radio will be unavailable");
    } else {
        info!("loaded {} radio station(s)", stations.len());

        let unbacked = zayden_app::config::radio::unbacked(&stations);
        if !unbacked.is_empty() {
            let names: Vec<&str> = unbacked.into_iter().map(Genre::label).collect();
            warn!(
                "no radio stations configured for: {}; those choices will error",
                names.join(", ")
            );
        }
    }
    let radio = RadioResolver::new(stations).map_err(BotError::from)?;

    Ok(Arc::new(CompositeResolver::new(youtube, spotify, radio)))
}

fn open_cookie_jar(config: &BotConfig) -> Option<Arc<CookieJar>> {
    let path = config.youtube_cookies.clone()?;
    let shown = path.display().to_string();

    let jar = match CookieJar::open(path) {
        Ok(jar) => jar,
        Err(e) => {
            error!(
                "the YouTube cookie file at {shown} is unusable ({e}); \
                 YouTube requests will stay anonymous"
            );
            return None;
        },
    };

    let status = jar.status();

    match status {
        JarStatus::Authenticated { .. } => {
            match status.expires_in(Timestamp::now().as_second()) {
                Some(secs) if secs <= 0 => warn!(
                    "the YouTube cookies in {shown} have already expired; \
                     playback will fail until they are re-exported"
                ),
                Some(secs) => {
                    info!(
                        "YouTube cookies loaded from {shown}; the session \
                         expires in {} day(s)",
                        secs / 86_400
                    );
                },
                None => info!("YouTube cookies loaded from {shown}"),
            }
            Some(Arc::new(jar))
        },
        JarStatus::Anonymous => {
            warn!(
                "the YouTube cookie file at {shown} carries no signed-in \
                 session (LOGIN_INFO plus SAPISID are both required); export it \
                 again from a logged-in browser. Continuing without cookies"
            );
            None
        },
    }
}
