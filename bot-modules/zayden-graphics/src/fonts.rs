use std::path::PathBuf;
use std::sync::Arc;

use resvg::usvg::fontdb;
use tracing::{info, warn};

const PRIMARY_CANDIDATES: &[&str] = &[
    // docker/Dockerfile.bot
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/opentype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    // macOS
    "/System/Library/Fonts/Helvetica.ttc",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
    "/Library/Fonts/Arial.ttf",
];

const FALLBACK_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
    "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
];

const FONT_PATH_ENV: &str = "ZAYDEN_FONT_PATHS";

pub struct Fonts {
    pub db: Arc<fontdb::Database>,
    pub family: String,
}

fn env_paths() -> Vec<PathBuf> {
    std::env::var(FONT_PATH_ENV).map_or_else(
        |_| Vec::new(),
        |raw| {
            raw.split(':')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .collect()
        },
    )
}

fn first_family(db: &fontdb::Database) -> Option<String> {
    db.faces()
        .next()
        .and_then(|face| face.families.first().map(|(name, _)| name.clone()))
}

fn try_load(db: &mut fontdb::Database, path: &PathBuf) -> bool {
    if !path.is_file() {
        return false;
    }

    let before = db.len();
    match db.load_font_file(path) {
        Ok(()) => db.len() > before,
        Err(e) => {
            warn!(error = %e, path = %path.display(), "failed to load font file");
            false
        },
    }
}

pub fn discover() -> Option<Fonts> {
    let mut db = fontdb::Database::new();
    let mut family: Option<String> = None;

    let candidates =
        env_paths().into_iter().chain(PRIMARY_CANDIDATES.iter().map(PathBuf::from));

    for path in candidates {
        if try_load(&mut db, &path) {
            family = first_family(&db);
            if family.is_some() {
                info!(path = %path.display(), "resolved primary font");
                break;
            }
        }
    }

    let family = family.or_else(|| {
        warn!(
            "no usable font found; text rendering is unavailable. Install \
             fonts-noto-core or set {FONT_PATH_ENV}"
        );
        None
    })?;

    for path in FALLBACK_CANDIDATES.iter().map(PathBuf::from) {
        if try_load(&mut db, &path) {
            info!(path = %path.display(), "loaded fallback font");
        }
    }

    db.set_sans_serif_family(family.clone());

    Some(Fonts { db: Arc::new(db), family })
}
