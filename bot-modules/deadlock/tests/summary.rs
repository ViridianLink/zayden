use deadlock::items::ItemIndex;
use deadlock::summary::{
    HeroAverages,
    MIN_ITEM_MATCHES,
    ability_sequence,
    summarize,
};
use deadlock_api::model::{
    Item,
    ItemStats,
    MatchMetadata,
    PerformancePoint,
    PlayerItem,
};
use serde_json::json;

const METADATA: &str = include_str!("fixtures/metadata_bulk.json");
const ITEMS: &str = include_str!("fixtures/items.json");

const ACCOUNT: u32 = 141_610_988;

const FIRST_ABILITIES: [u32; 3] = [4_111_222_521, 3_642_273_386, 1_247_583_368];
const SECOND_ABILITIES: [u32; 5] =
    [3_214_055_642, 629_925_354, 4_110_547_059, 2_001_589_780, 3_862_866_912];

const EXTENDED_MAGAZINE: u32 = 1_548_066_885;
const SPRINT_BOOTS: u32 = 3_399_065_363;

macro_rules! fixture_matches {
    () => {
        serde_json::from_str::<Vec<MatchMetadata>>(METADATA).unwrap()
    };
}

macro_rules! player_of {
    ($meta:expr) => {
        $meta
            .players
            .iter()
            .find(|player| player.account_id == ACCOUNT)
            .cloned()
            .unwrap()
    };
}

macro_rules! index {
    () => {{
        let ability = |id: u32| -> Item {
            serde_json::from_value(json!({
                "id": id,
                "class_name": format!("ability_{id}"),
                "type": "ability"
            }))
            .unwrap()
        };
        let mut items: Vec<Item> = serde_json::from_str(ITEMS).unwrap();
        items.extend(FIRST_ABILITIES.into_iter().map(ability));
        items.extend(SECOND_ABILITIES.into_iter().map(ability));

        ItemIndex::new(&items)
    }};
}

const fn point(game_time: u32, net_worth_avg: f64) -> PerformancePoint {
    PerformancePoint {
        game_time,
        net_worth_avg,
        net_worth_std: 0.0,
        kills_avg: 0.0,
        deaths_avg: 0.0,
        assists_avg: 0.0,
        creep_kills_avg: 0.0,
        denies_avg: 0.0,
    }
}

const fn stats(item_id: u32, matches: u64, avg_buy_time_s: f64) -> ItemStats {
    ItemStats {
        item_id,
        wins: matches / 2,
        losses: matches - matches / 2,
        matches,
        avg_buy_time_s: Some(avg_buy_time_s),
        avg_sell_time_s: None,
    }
}

macro_rules! summary_of {
    ($meta:expr, $averages:expr) => {
        summarize($meta, &player_of!($meta), $averages, &index!())
    };
}

#[test]
fn the_outcome_follows_the_players_team_and_the_winning_team() {
    let matches = fixture_matches!();
    let [first, second] = matches.as_slice() else {
        panic!("expected two fixture matches, got {}", matches.len());
    };
    let averages = HeroAverages::default();

    assert!(summary_of!(first, &averages).won);
    assert!(summary_of!(second, &averages).won);

    let mut lost = player_of!(first);
    lost.team = "Team1".to_owned();

    assert!(!summarize(first, &lost, &averages, &index!()).won);
}

#[test]
fn the_scoreline_is_copied_from_the_match() {
    let matches = fixture_matches!();
    let summary = summary_of!(&matches[0], &HeroAverages::default());

    assert_eq!(summary.match_id, matches[0].match_id);
    assert_eq!(summary.account_id, ACCOUNT);
    assert_eq!(summary.hero_id, 20);
    assert_eq!(summary.duration_s, 749);
    assert_eq!((summary.kills, summary.deaths, summary.assists), (1, 3, 13));
    assert_eq!(summary.net_worth, 14_039);
    assert_eq!(summary.level, 18);
}

#[test]
fn souls_only_cover_sampled_checkpoints_within_the_match() {
    let matches = fixture_matches!();
    let averages =
        HeroAverages { curve: vec![point(360, 5_000.0)], ..HeroAverages::default() };

    let first = summary_of!(&matches[0], &averages);
    let second = summary_of!(&matches[1], &averages);

    let first_times: Vec<u32> = first.souls.iter().map(|soul| soul.time_s).collect();
    let second_times: Vec<u32> =
        second.souls.iter().map(|soul| soul.time_s).collect();

    assert_eq!(first_times, vec![360]);
    assert_eq!(second_times, vec![360, 900]);
    assert_eq!(first.souls[0].net_worth, 5_486);
    assert_eq!(first.souls[0].average, Some(5_000.0));
    assert_eq!(second.souls[0].average, Some(5_000.0));
    assert_eq!(second.souls[1].net_worth, 18_960);
    assert_eq!(second.souls[1].average, None);
}

#[test]
fn items_are_upgrades_in_purchase_order_without_repeats() {
    let matches = fixture_matches!();
    let summary = summary_of!(&matches[0], &HeroAverages::default());

    let bought: Vec<(u32, u32)> = summary
        .items
        .iter()
        .map(|timing| (timing.item_id, timing.bought_at_s))
        .collect();

    assert_eq!(bought, vec![
        (EXTENDED_MAGAZINE, 117),
        (SPRINT_BOOTS, 275),
        (2_356_412_290, 458),
        (2_603_935_618, 673),
        (2_447_176_615, 674),
    ]);
}

#[test]
fn only_the_first_purchase_of_an_item_counts() {
    let matches = fixture_matches!();
    let mut player = player_of!(&matches[0]);
    player.items.push(PlayerItem {
        game_time_s: 5,
        item_id: SPRINT_BOOTS,
        upgrade_id: 0,
        sold_time_s: 0,
    });
    player.items.push(PlayerItem {
        game_time_s: 700,
        item_id: EXTENDED_MAGAZINE,
        upgrade_id: 0,
        sold_time_s: 0,
    });

    let summary =
        summarize(&matches[0], &player, &HeroAverages::default(), &index!());

    let ids: Vec<u32> = summary.items.iter().map(|timing| timing.item_id).collect();

    assert_eq!(ids.first(), Some(&SPRINT_BOOTS));
    assert_eq!(summary.items[0].bought_at_s, 5);
    assert_eq!(ids.iter().filter(|id| **id == SPRINT_BOOTS).count(), 1);
    assert_eq!(ids.iter().filter(|id| **id == EXTENDED_MAGAZINE).count(), 1);
}

#[test]
fn at_most_six_items_are_shown_in_ascending_time() {
    let matches = fixture_matches!();
    let summary = summary_of!(&matches[1], &HeroAverages::default());

    let times: Vec<u32> =
        summary.items.iter().map(|timing| timing.bought_at_s).collect();

    assert_eq!(summary.items.len(), 6);
    assert!(times.is_sorted());
    assert_eq!(summary.items[0].item_id, 7_409_189);
}

#[test]
fn item_averages_need_enough_matches() {
    let matches = fixture_matches!();
    let averages = HeroAverages {
        items: vec![
            stats(EXTENDED_MAGAZINE, MIN_ITEM_MATCHES, 120.0),
            stats(SPRINT_BOOTS, MIN_ITEM_MATCHES - 1, 250.0),
        ],
        ..HeroAverages::default()
    };

    let summary = summary_of!(&matches[0], &averages);

    assert_eq!(summary.items[0].average_s, Some(120.0));
    assert_eq!(summary.items[1].average_s, None);
    assert_eq!(summary.items[2].average_s, None);
}

#[test]
fn the_ability_sequence_keeps_only_ability_entries_in_order() {
    let matches = fixture_matches!();
    let [a, b, c] = FIRST_ABILITIES;

    let sequence = ability_sequence(&player_of!(&matches[0]), &index!());

    assert_eq!(sequence, vec![a, a, b, b, c, c, a, a, c]);
    assert_ne!(
        ability_sequence(&player_of!(&matches[1]), &index!()),
        Vec::<u32>::new()
    );
}

#[test]
fn followed_is_the_common_prefix_with_the_most_played_order() {
    let matches = fixture_matches!();
    let sequence = ability_sequence(&player_of!(&matches[0]), &index!());
    let mut order = sequence[..5].to_vec();
    order.extend([u32::MAX, u32::MAX]);
    let averages =
        HeroAverages { most_played_order: Some(order), ..HeroAverages::default() };

    let abilities = summary_of!(&matches[0], &averages).abilities.unwrap();

    assert_eq!(abilities.followed, 5);
    assert_eq!(abilities.total, 7);
}

#[test]
fn abilities_are_absent_without_a_known_order() {
    let matches = fixture_matches!();

    let summary = summary_of!(&matches[0], &HeroAverages::default());

    assert_eq!(summary.abilities, None);
}

#[test]
fn missing_data_degrades_to_empty_sections() {
    let matches = fixture_matches!();
    let mut player = player_of!(&matches[0]);
    player.items.clear();
    player.stats.clear();
    let averages = HeroAverages {
        most_played_order: Some(vec![1, 2, 3]),
        ..HeroAverages::default()
    };

    let summary = summarize(&matches[0], &player, &averages, &index!());

    assert_eq!(summary.souls, Vec::new());
    assert_eq!(summary.items, Vec::new());
    assert_eq!(summary.abilities, None);
}

#[test]
fn unknown_items_are_skipped() {
    let matches = fixture_matches!();

    let summary = summarize(
        &matches[0],
        &player_of!(&matches[0]),
        &HeroAverages::default(),
        &ItemIndex::new(&[]),
    );

    assert_eq!(summary.items, Vec::new());
    assert_eq!(summary.abilities, None);
}
