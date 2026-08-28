use axum::Json;
use axum::extract::{Extension, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use jiff::Timestamp;
use palworld::save::edit::apply_edits;
use tracing::warn;

use crate::WebState;
use crate::middleware::auth::AuthUser;

pub(super) async fn export_handler(
    State(state): State<WebState>,
    Extension(user): Extension<AuthUser>,
    Json(edits): Json<dashboard::dto::SaveEdits>,
) -> Response {
    let Ok(user_id) = user.id.parse::<i64>() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let granted = sqlx::query_scalar!(
        "SELECT 1 FROM web_user_roles WHERE discord_user_id = $1 AND role = 'admin'",
        user_id,
    )
    .fetch_optional(&state.app.db)
    .await;

    match granted {
        Ok(Some(_)) => {},
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            warn!(?e, "failed to check web_user_roles for save export");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        },
    }

    let Some(save_dir) = state.palworld.save_dir().map(std::path::Path::to_path_buf)
    else {
        return (
            StatusCode::CONFLICT,
            "no world save is configured for this deployment",
        )
            .into_response();
    };

    let edits = palworld::save::edit::SaveEdits::from(edits);
    let built =
        tokio::task::spawn_blocking(move || build_export(&save_dir, &edits)).await;

    let export = match built {
        Ok(Ok(export)) => export,
        Ok(Err(e)) => {
            warn!(error = %e, "save export failed");
            return (StatusCode::UNPROCESSABLE_ENTITY, e.to_string())
                .into_response();
        },
        Err(e) => {
            warn!(?e, "save export task panicked");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        },
    };

    let (content_type, filename, bytes) = match export {
        Export::Level(bytes) => {
            ("application/octet-stream", level_filename(Timestamp::now()), bytes)
        },
        Export::Bundle(bytes) => {
            ("application/zip", bundle_filename(Timestamp::now()), bytes)
        },
    };

    (
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response()
}

enum Export {
    Level(Vec<u8>),
    Bundle(Vec<u8>),
}

fn build_export(
    save_dir: &std::path::Path,
    edits: &palworld::save::edit::SaveEdits,
) -> Result<Export, palworld::error::PalworldError> {
    use palworld::save::edit_player::grant_tech_points;

    let level_bytes = std::fs::read(save_dir.join("Level.sav"))?;
    let edited = apply_edits(&level_bytes, edits)?;

    if edited.level_deltas.is_empty() {
        return Ok(Export::Level(edited.level));
    }

    let mut players = Vec::new();
    for (uid, delta) in &edited.level_deltas {
        let path =
            palworld::save::player_save_path(save_dir, uid).ok_or_else(|| {
                palworld::error::PalworldError::Edit(format!(
                    "player uid {uid} is not a valid save filename"
                ))
            })?;
        let raw = std::fs::read(&path).map_err(|e| {
            palworld::error::PalworldError::Edit(format!(
                "cannot read the save for player {uid} ({}): {e}",
                path.display()
            ))
        })?;
        let stem = path.file_name().map_or_else(
            || format!("{uid}.sav"),
            |n| n.to_string_lossy().to_string(),
        );
        players.push((stem, grant_tech_points(&raw, *delta)?));
    }

    zip_bundle(&edited.level, &players).map(Export::Bundle)
}

fn zip_bundle(
    level: &[u8],
    players: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, palworld::error::PalworldError> {
    use std::io::Write;

    use zip::write::SimpleFileOptions;

    let zip_err = |e: zip::result::ZipError| {
        palworld::error::PalworldError::Edit(format!("building the archive: {e}"))
    };

    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    writer.start_file("Level.sav", options).map_err(zip_err)?;
    writer.write_all(level)?;

    for (stem, bytes) in players {
        writer.start_file(format!("Players/{stem}"), options).map_err(zip_err)?;
        writer.write_all(bytes)?;
    }

    Ok(writer.finish().map_err(zip_err)?.into_inner())
}

fn level_filename(now: Timestamp) -> String {
    format!("Level_modified_{}.sav", now.strftime("%Y%m%d-%H%M%SZ"))
}

fn bundle_filename(now: Timestamp) -> String {
    format!("palworld_save_edit_{}.zip", now.strftime("%Y%m%d-%H%M%SZ"))
}
