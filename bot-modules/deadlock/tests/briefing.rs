use deadlock::briefing::{Recipient, recipients};
use deadlock::db::briefings::RecentHero;
use deadlock::db::patches::PatchRow;
use deadlock::diff::{Change, HeroDiff, PatchDiff, Presence};
use deadlock::embeds::briefing_dm;
use jiff::Timestamp;

const TITLE: &str = "Update 14 October";
const LINK: &str = "https://forums.playdeadlock.com/threads/update-14-october.1/";

macro_rules! recent {
    ($discord_id:expr, $hero_id:expr, $games:expr, $last_played:expr) => {
        RecentHero {
            discord_id: $discord_id,
            hero_id: $hero_id,
            games: $games,
            last_played: Timestamp::from_second($last_played).unwrap(),
        }
    };
}

fn hero(hero_id: u32, name: &str, presence: Presence) -> HeroDiff {
    HeroDiff {
        hero_id,
        name: name.to_owned(),
        presence,
        stats: Vec::new(),
        weapon: Vec::new(),
        abilities: Vec::new(),
    }
}

fn with_stat(hero: HeroDiff) -> HeroDiff {
    let stat = Change {
        label: "Max health".to_owned(),
        old: Some("830".to_owned()),
        new: Some("850".to_owned()),
    };

    HeroDiff { stats: vec![stat], ..hero }
}

const fn diff(heroes: Vec<HeroDiff>) -> PatchDiff {
    PatchDiff { heroes, items: Vec::new() }
}

fn touching(ids: &[u32]) -> PatchDiff {
    diff(ids.iter().map(|id| hero(*id, "Hero", Presence::Changed)).collect())
}

#[test]
fn the_top_three_heroes_are_ranked_by_games_then_recency_then_id() {
    let rows = [
        recent!(10, 1, 5, 100),
        recent!(10, 2, 9, 50),
        recent!(10, 3, 5, 200),
        recent!(10, 4, 5, 200),
        recent!(10, 5, 1, 900),
    ];

    assert_eq!(recipients(&rows, &touching(&[1, 2, 3, 4, 5])), vec![Recipient {
        discord_id: 10,
        hero_ids: vec![2, 3, 4],
    }]);
}

#[test]
fn only_touched_heroes_are_kept_and_a_fourth_favourite_is_not_promoted() {
    let rows = [
        recent!(10, 1, 9, 100),
        recent!(10, 2, 8, 100),
        recent!(10, 3, 7, 100),
        recent!(10, 4, 6, 100),
    ];

    assert_eq!(recipients(&rows, &touching(&[2, 4])), vec![Recipient {
        discord_id: 10,
        hero_ids: vec![2],
    }]);
}

#[test]
fn a_player_without_a_touched_hero_is_dropped() {
    let rows = [recent!(10, 1, 3, 100), recent!(11, 2, 3, 100)];

    assert_eq!(recipients(&rows, &touching(&[2])), vec![Recipient {
        discord_id: 11,
        hero_ids: vec![2],
    }]);
}

#[test]
fn a_removed_hero_still_counts() {
    let rows = [recent!(10, 1, 3, 100)];
    let removed = diff(vec![hero(1, "Infernus", Presence::Removed)]);

    assert_eq!(recipients(&rows, &removed), vec![Recipient {
        discord_id: 10,
        hero_ids: vec![1],
    }]);
}

#[test]
fn recipients_are_ordered_by_discord_id() {
    let rows =
        [recent!(12, 1, 3, 100), recent!(10, 1, 3, 100), recent!(11, 1, 3, 100)];

    let ids: Vec<i64> = recipients(&rows, &touching(&[1]))
        .iter()
        .map(|recipient| recipient.discord_id)
        .collect();

    assert_eq!(ids, vec![10, 11, 12]);
}

#[test]
fn an_empty_diff_has_no_recipients() {
    let rows = [recent!(10, 1, 3, 100)];

    assert_eq!(recipients(&rows, &PatchDiff::default()), Vec::<Recipient>::new());
}

#[test]
fn the_briefing_lists_only_the_given_heroes_and_the_patch_link() {
    let patch = PatchRow {
        guid: "forum-1".to_owned(),
        title: TITLE.to_owned(),
        link: LINK.to_owned(),
        published_at: Timestamp::UNIX_EPOCH,
        detected_at: Timestamp::UNIX_EPOCH,
        diff: Some(diff(vec![
            with_stat(hero(1, "Infernus", Presence::Changed)),
            with_stat(hero(2, "Haze", Presence::Changed)),
        ])),
    };

    let json = serde_json::to_value(briefing_dm(&patch, &[1])).unwrap().to_string();

    assert!(json.contains("Infernus"));
    assert!(!json.contains("Haze"));
    assert!(json.contains(LINK));
}
