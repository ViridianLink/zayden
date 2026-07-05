pub mod commands;
pub mod components;
pub mod embeds;
pub mod error;
pub mod events;
pub mod manager;
pub mod occupancy;
pub mod permissions;
pub mod player;
pub mod queue;
pub mod resolve;
pub mod settings;
pub mod track;
pub mod voice;

pub use commands::Command;
pub use error::{MusicError, Result};
pub use events::{InactivityCheck, TrackEndNotifier};
pub use manager::MusicManager;
pub use occupancy::VoiceOccupancy;
pub use player::{GuildPlayer, NowPlaying};
pub use queue::Queue;
pub use resolve::{
    CompositeResolver,
    PlaylistOrigin,
    Resolution,
    SourceKind,
    SourceQuery,
    SpotifyKind,
    SpotifyResolver,
    TrackResolver,
    YouTubeResolver,
    parse_spotify_url,
};
pub use settings::MusicSettingsRow;
pub use track::{LoopMode, RequestedBy, ResolvedTrack, TrackSource};
