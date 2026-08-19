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
pub mod radio;
pub mod resolve;
pub mod track;
pub mod voice;

pub use commands::Command;
pub use embeds::{SeekTarget, parse_seek, parse_timestamp};
pub use error::{MusicError, Result};
pub use events::{InactivityCheck, TrackEndNotifier};
pub use manager::MusicManager;
pub use occupancy::VoiceOccupancy;
pub use player::{
    AdvanceAction,
    AnnounceConfig,
    GuildPlayer,
    NowPlaying,
    RadioSession,
    advance_action,
    records_history,
    volume_scalar,
};
pub use queue::{ClearMode, Queue};
pub use radio::RADIO_TIER;
pub use resolve::{
    AUTHED_STREAM_CLIENTS,
    COOKIE_UNSUPPORTED_CLIENTS,
    CompositeResolver,
    Cookie,
    CookieJar,
    CookieLease,
    EMBED_TRACK_LIMIT,
    EmbedPlaylist,
    EmbedTrack,
    JarStatus,
    PlaylistOrigin,
    RadioResolver,
    Resolution,
    STREAM_CLIENTS,
    STREAM_FORMAT,
    STREAM_READ_TIMEOUT,
    SourceKind,
    SourceQuery,
    SpotifyKind,
    SpotifyResolver,
    StreamFormat,
    TrackResolver,
    YT_DLP_PROBE_TIMEOUT,
    YT_DLP_STREAM_TIMEOUT,
    YT_DLP_TIMEOUT,
    YouTubeResolver,
    cookie_warning,
    embed_resolution,
    embed_url,
    fetch_embed_playlist,
    has_netscape_header,
    has_playlist,
    jar_status,
    next_retry_count,
    parse_embed_playlist,
    parse_netscape,
    parse_spotify_url,
    playlist_start_index,
    probe_stream,
    probe_yt_dlp,
    run_with_timeout,
    should_reconnect,
    station_track,
    stream_client,
    stream_client_with,
    stream_clients,
};
pub use track::{LoopMode, ResolvedTrack, TrackSource};
pub use zayden_app::config::{Genre, MusicSettingsRow, RadioStation};
