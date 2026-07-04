use async_trait::async_trait;
use songbird::input::Input;

use crate::error::Result;
use crate::track::ResolvedTrack;

#[async_trait]
pub trait TrackResolver: Send + Sync {
    async fn stream(&self, track: &ResolvedTrack) -> Result<Input>;
}
