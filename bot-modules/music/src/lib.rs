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
pub use resolve::{
    parse_spotify_url, CompositeResolver, PlaylistOrigin, Resolution, SourceKind, SourceQuery,
    SpotifyKind, SpotifyResolver, TrackResolver, YouTubeResolver,
};
pub use track::{LoopMode, RequestedBy, ResolvedTrack, TrackSource};
