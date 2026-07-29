use axum::Json;
use axum::extract::{Extension, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use jiff::Timestamp;
use palworld::save::edit::apply_edits;
use tracing::warn;

use crate::WebState;
use crate::middleware::auth::AuthUser;

pub(crate) async fn export_handler(
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
    let level_path = save_dir.join("Level.sav");
    let built = tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&level_path)?;
        apply_edits(&bytes, &edits)
    })
    .await;

    let bytes = match built {
        Ok(Ok(bytes)) => bytes,
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

    let filename = export_filename(Timestamp::now());

    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        bytes,
    )
        .into_response()
}

fn export_filename(now: Timestamp) -> String {
    format!("Level_modified_{}.sav", now.strftime("%Y%m%d-%H%M%SZ"))
}
