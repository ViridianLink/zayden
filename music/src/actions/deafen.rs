use songbird::Call;
use songbird::error::JoinResult;
use tokio::sync::MutexGuard;

pub async fn deafen(handler: &mut MutexGuard<'_, Call>) -> JoinResult<()> {
    handler.deafen(true).await
}
