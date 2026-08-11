use std::time::Duration;

use music::{
    MusicError,
    PlaylistOrigin,
    TrackSource,
    embed_resolution,
    embed_url,
    parse_embed_playlist,
};
use serenity::all::UserId;

const REQUESTER: UserId = UserId::new(7);

/// Captured from the public embed page for playlist `6ZyXZyvEQvANMOmB1ur7S4`,
/// truncated to three tracks.
const PLAYLIST: &str = include_str!("fixtures/spotify_embed_playlist.html");

#[test]
fn parses_tracks_from_embed_page() {
    let playlist = parse_embed_playlist(PLAYLIST).unwrap();

    assert_eq!(playlist.name, "Steven Universe (all songs)");
    assert_eq!(playlist.tracks.len(), 3);

    let first = playlist.tracks.first().unwrap();
    assert_eq!(first.id, "2HD2g1syRNaqanstTj8zfJ");
    assert_eq!(first.title, "Love Like You (End Credits)");
    assert_eq!(first.duration, Some(Duration::from_millis(143_629)));
}

#[test]
fn normalises_non_breaking_spaces_between_artists() {
    // Spotify separates artists with U+00A0, which reads as a stray glyph in
    // Discord embeds and skews the YouTube search query.
    let playlist = parse_embed_playlist(PLAYLIST).unwrap();

    let first = playlist.tracks.first().unwrap();
    let second = playlist.tracks.get(1).unwrap();

    assert_eq!(first.artists, "Steven Universe, Rebecca Sugar");
    assert!(!second.artists.contains('\u{a0}'));
}

#[test]
fn reads_cover_art_from_the_playlist() {
    let playlist = parse_embed_playlist(PLAYLIST).unwrap();

    let cover = playlist.cover_url.as_deref().unwrap();
    assert!(cover.starts_with("https://"), "unexpected cover url: {cover}");
}

#[test]
fn skips_entries_that_are_not_tracks() {
    let html = embed_html(
        r#"[
            {"uri":"spotify:episode:aaa","title":"An Episode",
             "subtitle":"A Show","duration":1000,"entityType":"episode"},
            {"uri":"spotify:track:bbb","title":"A Track",
             "subtitle":"An Artist","duration":2000,"entityType":"track"}
        ]"#,
    );

    let playlist = parse_embed_playlist(&html).unwrap();

    assert_eq!(playlist.tracks.len(), 1);
    assert_eq!(playlist.tracks.first().unwrap().id, "bbb");
}

#[test]
fn errors_when_the_page_has_no_embedded_data() {
    let err = parse_embed_playlist("<html><body>nope</body></html>").unwrap_err();

    assert!(matches!(err, MusicError::Resolve(_)), "got {err:?}");
}

#[test]
fn errors_when_the_playlist_has_no_playable_tracks() {
    let err = parse_embed_playlist(&embed_html("[]")).unwrap_err();

    assert!(matches!(err, MusicError::NoResults), "got {err:?}");
}

#[tokio::test]
async fn resolution_plays_the_first_track_and_defers_the_rest() {
    let playlist = parse_embed_playlist(PLAYLIST).unwrap();

    let resolution = embed_resolution(playlist, REQUESTER);

    assert_eq!(resolution.origin, PlaylistOrigin::SpotifyPlaylist);
    assert_eq!(resolution.head.len(), 1);
    assert_eq!(
        resolution.head.first().unwrap().title,
        "Steven Universe - Love Like You (End Credits)"
    );

    let tail = resolution.tail.unwrap().await.unwrap();
    assert_eq!(tail.len(), 2);
    assert_eq!(
        tail.first().unwrap().title,
        "Steven Universe - Here Comes a Thought"
    );
}

#[test]
fn resolved_tracks_carry_spotify_metadata() {
    let playlist = parse_embed_playlist(PLAYLIST).unwrap();

    let resolution = embed_resolution(playlist, REQUESTER);
    let track = resolution.head.first().unwrap();

    assert_eq!(track.source, TrackSource::Spotify);
    assert_eq!(track.source_id, "2HD2g1syRNaqanstTj8zfJ");
    assert_eq!(track.url, "https://open.spotify.com/track/2HD2g1syRNaqanstTj8zfJ");
    assert_eq!(track.duration, Some(Duration::from_millis(143_629)));
    assert_eq!(track.requested_by, REQUESTER);
    assert!(!track.is_live);
    assert!(track.thumbnail_url.is_some());
}

#[tokio::test]
async fn titles_use_only_the_lead_artist_so_youtube_search_stays_tight() {
    // The third fixture track credits six artists; carrying all of them into
    // the search query is what breaks the YouTube match.
    let playlist = parse_embed_playlist(PLAYLIST).unwrap();

    let resolution = embed_resolution(playlist, REQUESTER);
    let tail = resolution.tail.unwrap().await.unwrap();

    assert_eq!(
        tail.get(1).unwrap().title,
        "Steven Universe - Peace and Love on the Planet Earth"
    );
}

/// Guards the one thing the fixture cannot: that Spotify still serves this
/// shape. Ignored by default so the suite stays offline and deterministic.
#[tokio::test]
#[ignore = "hits open.spotify.com"]
async fn fetches_a_live_playlist_from_spotify() {
    let client = music::stream_client().unwrap();

    let playlist = music::fetch_embed_playlist(&client, "6ZyXZyvEQvANMOmB1ur7S4")
        .await
        .unwrap();

    assert_eq!(playlist.name, "Steven Universe (all songs)");
    assert!(playlist.tracks.len() > 1, "got {} tracks", playlist.tracks.len());
    assert!(playlist.tracks.iter().all(|track| !track.title.is_empty()));
    assert!(playlist.tracks.iter().all(|track| track.duration.is_some()));
}

#[test]
fn builds_the_embed_url_for_a_playlist_id() {
    assert_eq!(
        embed_url("6ZyXZyvEQvANMOmB1ur7S4"),
        "https://open.spotify.com/embed/playlist/6ZyXZyvEQvANMOmB1ur7S4"
    );
}

fn embed_html(track_list: &str) -> String {
    format!(
        r#"<html><body><script id="__NEXT_DATA__" type="application/json">
        {{"props":{{"pageProps":{{"state":{{"data":{{"entity":{{
            "name":"Test Playlist",
            "coverArt":{{"sources":[{{"url":"https://example.invalid/a.jpg"}}]}},
            "trackList":{track_list}
        }}}}}}}}}}}}</script></body></html>"#
    )
}
