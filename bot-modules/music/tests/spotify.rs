use music::{parse_spotify_url, SpotifyKind};

#[test]
fn parses_track_url() {
    let (kind, id) =
        parse_spotify_url("https://open.spotify.com/track/6y0igZArWVi6Iz0rj35c1Y?si=abc").unwrap();
    assert_eq!(kind, SpotifyKind::Track);
    assert_eq!(id, "6y0igZArWVi6Iz0rj35c1Y");
}

#[test]
fn parses_album_url() {
    let (kind, id) = parse_spotify_url("https://open.spotify.com/album/1a2b3c").unwrap();
    assert_eq!(kind, SpotifyKind::Album);
    assert_eq!(id, "1a2b3c");
}

#[test]
fn parses_playlist_url() {
    let (kind, id) = parse_spotify_url("https://open.spotify.com/playlist/xyz").unwrap();
    assert_eq!(kind, SpotifyKind::Playlist);
    assert_eq!(id, "xyz");
}

#[test]
fn rejects_unknown_path_kind() {
    assert!(parse_spotify_url("https://open.spotify.com/artist/xyz").is_err());
}

#[test]
fn rejects_missing_id() {
    assert!(parse_spotify_url("https://open.spotify.com/track").is_err());
}

#[test]
fn rejects_unparseable_input() {
    assert!(parse_spotify_url("not a url").is_err());
}
