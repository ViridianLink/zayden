use music::{has_playlist, playlist_start_index};

const MIX_RADIO: &str = "https://www.youtube.com/watch?v=L9hjMCJbGIg&list=RDL9hjMCJbGIg&start_radio=1&rv=L9hjMCJbGIg";
const MIX: &str = "https://www.youtube.com/watch?v=L9hjMCJbGIg&list=RDMMpnYf3w4aSZ0";
const MIX_INDEXED: &str =
    "https://www.youtube.com/watch?v=L9hjMCJbGIg&list=RDMMpnYf3w4aSZ0&index=27";

#[test]
fn detects_playlist_and_mix_urls() {
    assert!(has_playlist(MIX_RADIO));
    assert!(has_playlist(MIX));
    assert!(has_playlist(MIX_INDEXED));
    assert!(has_playlist(
        "https://www.youtube.com/playlist?list=PLBCF2DAC6FFB574DE"
    ));
}

#[test]
fn plain_video_is_not_a_playlist() {
    assert!(!has_playlist("https://www.youtube.com/watch?v=L9hjMCJbGIg"));
    assert!(!has_playlist("https://youtu.be/L9hjMCJbGIg"));
    assert!(!has_playlist("not a url"));
}

#[test]
fn start_index_honours_index_param() {
    assert_eq!(playlist_start_index(MIX_INDEXED), 27);
}

#[test]
fn start_index_defaults_to_one() {
    assert_eq!(playlist_start_index(MIX), 1);
    assert_eq!(playlist_start_index(MIX_RADIO), 1);
    assert_eq!(playlist_start_index("not a url"), 1);
}

#[test]
fn start_index_ignores_invalid_values() {
    assert_eq!(
        playlist_start_index("https://www.youtube.com/watch?v=x&list=PL1&index=0"),
        1
    );
    assert_eq!(
        playlist_start_index("https://www.youtube.com/watch?v=x&list=PL1&index=abc"),
        1
    );
}
