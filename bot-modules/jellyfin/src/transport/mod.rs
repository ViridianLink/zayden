pub mod http;
pub mod jellyfin;
pub mod jellyseerr;
pub mod playback;
pub mod segments;

pub use http::{ApiError, ApiResult};
pub use jellyfin::JellyfinClient;
pub use jellyseerr::SeerClient;
pub use playback::PlaybackClient;
pub use segments::{SegmentStats, segment_stats};
