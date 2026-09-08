use std::sync::Arc;
use std::time::Duration;

use serenity::all::{EditThread, Http, HttpError, JsonErrorCode, ThreadId};
use tokio::time::sleep;
use tracing::warn;

pub(crate) fn archive_in(http: Arc<Http>, thread_id: ThreadId, delay: Duration) {
    tokio::spawn(async move {
        sleep(delay).await;

        match thread_id.edit(&http, EditThread::new().archived(true)).await {
            Ok(_) => {},
            // The thread can be deleted while the archive is pending.
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(resp)))
                if resp.error.code == JsonErrorCode::UnknownChannel => {},
            Err(e) => warn!(?thread_id, "failed to archive solved thread: {e}"),
        }
    });
}
