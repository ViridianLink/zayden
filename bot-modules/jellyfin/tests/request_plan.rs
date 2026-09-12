//! Regression tests for what `/jellyfin request` sends to Jellyseerr.
//!
//! Two things used to go wrong. `seasons` was left as the literal string
//! `"latest"`, which is not one of the two shapes the API accepts, and no
//! quality profile was named at all, so Jellyseerr fell back to its own
//! `activeAnimeProfileId` — a stale id Sonarr rejected with
//! `QualityProfileExistsValidator`.

use jellyfin::requests::{
    Seasons,
    all_seasons,
    is_anime,
    profile_name,
    season_payload,
};
use jellyfin::transport::jellyseerr::model::{
    Keyword,
    MediaType,
    Season,
    TvDetails,
};
use serde_json::json;

fn tv(seasons: &[(i32, i32)]) -> TvDetails {
    TvDetails {
        seasons: seasons
            .iter()
            .map(|(season_number, episode_count)| Season {
                season_number: *season_number,
                episode_count: *episode_count,
            })
            .collect(),
        ..TvDetails::default()
    }
}

fn keywords(ids: &[i32]) -> Vec<Keyword> {
    ids.iter().map(|id| Keyword { id: *id, name: "k".to_owned() }).collect()
}

#[test]
fn anime_asks_for_the_anime_profile() {
    assert!(is_anime(&keywords(&[210_024])));
    assert_eq!(profile_name(true), "Anime 1080p");
}

#[test]
fn everything_else_asks_for_the_compact_profile() {
    assert!(!is_anime(&keywords(&[9840, 4344])));
    assert_eq!(profile_name(false), "1080p Compact");
}

#[test]
fn every_season_is_requested_by_default() {
    let details = tv(&[(0, 12), (1, 25), (2, 25), (3, 25), (4, 25), (5, 20)]);
    let available = all_seasons(&details);

    assert_eq!(available, vec![1, 2, 3, 4, 5]);
    assert_eq!(
        season_payload(MediaType::Tv, Seasons::All, &available),
        Some(json!([1, 2, 3, 4, 5]))
    );
}

#[test]
fn an_unaired_season_is_still_requested() {
    let details = tv(&[(1, 25), (2, 0)]);

    assert_eq!(all_seasons(&details), vec![1, 2]);
}

#[test]
fn specials_are_left_to_sonarr() {
    let details = tv(&[(0, 12), (1, 25)]);

    assert!(!all_seasons(&details).contains(&0));
}

#[test]
fn latest_resolves_to_a_season_number_not_the_word() {
    assert_eq!(
        season_payload(MediaType::Tv, Seasons::Latest, &[1, 2, 3]),
        Some(json!([3]))
    );
}

#[test]
fn first_resolves_to_the_lowest_season() {
    assert_eq!(
        season_payload(MediaType::Tv, Seasons::First, &[2, 3, 4]),
        Some(json!([2]))
    );
}

#[test]
fn unreadable_details_fall_back_to_all() {
    assert_eq!(season_payload(MediaType::Tv, Seasons::All, &[]), Some(json!("all")));
}

#[test]
fn a_film_carries_no_seasons() {
    assert_eq!(season_payload(MediaType::Movie, Seasons::All, &[1]), None);
}

#[test]
fn an_unknown_season_choice_means_everything() {
    assert_eq!(Seasons::parse(None), Seasons::All);
    assert_eq!(Seasons::parse(Some("all")), Seasons::All);
    assert_eq!(Seasons::parse(Some("nonsense")), Seasons::All);
    assert_eq!(Seasons::parse(Some("first")), Seasons::First);
    assert_eq!(Seasons::parse(Some("latest")), Seasons::Latest);
}
