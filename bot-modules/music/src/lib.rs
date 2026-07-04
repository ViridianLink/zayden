pub mod error;
pub mod events;
pub mod manager;
pub mod occupancy;
pub mod player;
pub mod queue;
pub mod resolve;
pub mod track;
pub mod voice;

pub use error::{MusicError, Result};
pub use events::{InactivityCheck, TrackEndNotifier};
pub use manager::MusicManager;
pub use occupancy::VoiceOccupancy;
pub use player::{GuildPlayer, NowPlaying};
pub use queue::Queue;
pub use resolve::{PlaylistOrigin, Resolution, SourceKind, SourceQuery, TrackResolver};
pub use track::{LoopMode, RequestedBy, ResolvedTrack, TrackSource};
