use music::{SourceKind, SourceQuery};

#[test]
fn classifies_youtube_hosts() {
    for url in [
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtu.be/dQw4w9WgXcQ",
        "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://m.youtube.com/watch?v=dQw4w9WgXcQ",
        "  https://youtu.be/dQw4w9WgXcQ  ",
        "HTTPS://YOUTUBE.COM/watch?v=dQw4w9WgXcQ",
    ] {
        assert_eq!(SourceKind::classify(url), SourceKind::YouTubeUrl, "{url}");
    }
}

#[test]
fn classifies_spotify_hosts() {
    assert_eq!(
        SourceKind::classify("https://open.spotify.com/track/abc123"),
        SourceKind::SpotifyUrl
    );
}

#[test]
fn free_text_is_a_search() {
    for query in ["never gonna give you up", "artist - song title", ""] {
        assert_eq!(SourceKind::classify(query), SourceKind::Search, "{query}");
    }
}

#[test]
fn non_whitelisted_urls_are_treated_as_search_not_fetched() {
    // SSRF guard: an arbitrary URL must never be classified as a direct
    // link, even if it looks superficially plausible.
    for url in [
        "https://evil.example.com/youtube.com",
        "https://evil.example.com/?redirect=open.spotify.com",
        "http://169.254.169.254/latest/meta-data/",
        "file:///etc/passwd",
        "https://youtube.com.evil.example.com/watch?v=x",
    ] {
        assert_eq!(SourceKind::classify(url), SourceKind::Search, "{url}");
    }
}

#[test]
fn source_query_carries_its_classification() {
    let query = SourceQuery::new("https://youtu.be/dQw4w9WgXcQ");
    assert_eq!(query.kind, SourceKind::YouTubeUrl);
    assert_eq!(query.raw, "https://youtu.be/dQw4w9WgXcQ");
}
