mod common;

use common::{Mock, Reply, api, spawn_mock};
use deadlock::db::watch_state::{self, WatchKey};
use deadlock::db::{patches, snapshots};
use deadlock::patch_watch::{self, BUILD_GRACE, MAX_NOTE_AGE, WatchOutcome};
use jiff::{SignedDuration, Timestamp};
use serde_json::{Value, json};
use sqlx::PgPool;

const VERSIONS: &str = "/v1/assets/client-versions";
const HEROES: &str = "/v1/assets/heroes";
const ITEMS: &str = "/v1/assets/items";
const PATCHES: &str = "/v2/patches";

const FORUM_OLD: &str = "forum-old";
const FORUM_NEW: &str = "forum-new";
const STEAM_NEW: &str = "steam-new";

fn heroes(max_health: f64) -> Value {
    json!([{
        "id": 1,
        "class_name": "hero_inferno",
        "name": "Infernus",
        "player_selectable": true,
        "starting_stats": { "max_health": { "value": max_health } }
    }])
}

fn note(source: &str, guid: &str, published_at: Timestamp) -> Value {
    json!({
        "source": source,
        "title": format!("Update {guid}"),
        "pub_date": published_at.to_string(),
        "link": format!("https://example.com/{guid}"),
        "guid": { "text": guid }
    })
}

fn serve(mock: &Mock, path: &str, body: &Value) {
    mock.serve(path, Reply::ok(&body.to_string()));
}

fn fetched(mock: &Mock, needle: &str) -> usize {
    mock.requests.lock().map_or(0, |recorded| {
        recorded.iter().filter(|head| head.contains(needle)).count()
    })
}

macro_rules! setup {
    ($pool:expr, $versions:expr, $health:expr, $notes:expr) => {{
        let versions: &[u32] = $versions;
        let notes: &[Value] = $notes;
        let mock = spawn_mock(Reply {
            status: 404,
            headers: Vec::new(),
            body: String::new(),
        })
        .await
        .unwrap();

        serve(&mock, VERSIONS, &json!(versions));
        serve(&mock, HEROES, &heroes($health));
        serve(&mock, ITEMS, &json!([]));
        serve(&mock, PATCHES, &json!(notes));

        let api = api($pool, &mock.base).unwrap();

        (mock, api)
    }};
}

macro_rules! run_at {
    ($api:expr, $pool:expr, $now:expr) => {{
        $api.invalidate_unversioned().await.unwrap();

        patch_watch::run($api, $pool, $now).await.unwrap()
    }};
}

macro_rules! state {
    ($pool:expr, $key:expr) => {
        watch_state::get($pool, $key).await.unwrap()
    };
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_first_run_seeds_state_and_stores_existing_notes_as_announced(
    pool: PgPool,
) {
    let now = Timestamp::now();
    let (mock, api) =
        setup!(&pool, &[1, 2], 830.0, &[note("forum", FORUM_NEW, now)]);

    let outcome = run_at!(&api, &pool, now);

    assert_eq!(outcome, WatchOutcome { seeded: true, ..WatchOutcome::default() });
    assert_eq!(state!(&pool, WatchKey::Latest), Some(2));
    assert_eq!(state!(&pool, WatchKey::Baseline), Some(2));
    assert!(patches::known_guids(&pool).await.unwrap().contains(FORUM_NEW));
    assert!(patches::ready(&pool).await.unwrap().is_empty());
    assert!(patches::awaiting_diff(&pool).await.unwrap().is_empty());
    assert!(snapshots::load(&pool, 2).await.unwrap().is_some());
    assert_eq!(fetched(&mock, "client_version=2"), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_client_version_alone_stores_a_snapshot_and_moves_only_latest(
    pool: PgPool,
) {
    let now = Timestamp::now();
    let (mock, api) = setup!(&pool, &[1], 830.0, &[]);
    run_at!(&api, &pool, now);

    serve(&mock, VERSIONS, &json!([1, 2]));
    serve(&mock, HEROES, &heroes(850.0));
    let outcome = run_at!(&api, &pool, now);

    assert_eq!(outcome, WatchOutcome {
        new_version: Some(2),
        ..WatchOutcome::default()
    });
    assert_eq!(state!(&pool, WatchKey::Latest), Some(2));
    assert_eq!(state!(&pool, WatchKey::Baseline), Some(1));
    assert!(snapshots::load(&pool, 1).await.unwrap().is_some());
    assert!(snapshots::load(&pool, 2).await.unwrap().is_some());
    assert!(patches::ready(&pool).await.unwrap().is_empty());
    assert_eq!(fetched(&mock, "client_version=2"), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_new_note_after_a_version_bump_is_diffed_against_the_baseline(
    pool: PgPool,
) {
    let now = Timestamp::now();
    let (mock, api) = setup!(&pool, &[1], 830.0, &[]);
    run_at!(&api, &pool, now);

    serve(&mock, VERSIONS, &json!([1, 2]));
    serve(&mock, HEROES, &heroes(850.0));
    serve(&mock, PATCHES, &json!([note("forum", FORUM_NEW, now)]));
    let outcome = run_at!(&api, &pool, now);

    assert_eq!(outcome, WatchOutcome {
        seeded: false,
        new_version: Some(2),
        new_notes: 1,
        diffed: 1,
    });
    assert_eq!(state!(&pool, WatchKey::Baseline), Some(2));

    let ready = patches::ready(&pool).await.unwrap();
    let [patch] = ready.as_slice() else {
        panic!("expected one ready patch, got {}", ready.len());
    };
    let diff = patch.diff.as_ref().unwrap();

    assert_eq!(patch.guid, FORUM_NEW);
    assert!(!diff.is_empty());
    assert_eq!(diff.hero_ids(), vec![1]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_note_without_a_version_bump_waits_for_the_build_then_gets_an_empty_diff(
    pool: PgPool,
) {
    let now = Timestamp::now();
    let (mock, api) = setup!(&pool, &[1], 830.0, &[]);
    run_at!(&api, &pool, now);

    serve(&mock, PATCHES, &json!([note("forum", FORUM_NEW, now)]));
    let waiting = run_at!(&api, &pool, now);

    assert_eq!(waiting, WatchOutcome { new_notes: 1, ..WatchOutcome::default() });
    assert!(patches::ready(&pool).await.unwrap().is_empty());
    assert_eq!(patches::awaiting_diff(&pool).await.unwrap().len(), 1);

    let later = now.checked_add(BUILD_GRACE + SignedDuration::from_mins(1)).unwrap();
    let settled = run_at!(&api, &pool, later);

    assert_eq!(settled, WatchOutcome { diffed: 1, ..WatchOutcome::default() });
    assert_eq!(state!(&pool, WatchKey::Baseline), Some(1));

    let ready = patches::ready(&pool).await.unwrap();
    let [patch] = ready.as_slice() else {
        panic!("expected one ready patch, got {}", ready.len());
    };

    assert!(patch.diff.as_ref().unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_old_note_is_stored_announced_and_other_sources_are_ignored(
    pool: PgPool,
) {
    let now = Timestamp::now();
    let (mock, api) = setup!(&pool, &[1], 830.0, &[]);
    run_at!(&api, &pool, now);

    let old = now.checked_sub(MAX_NOTE_AGE + SignedDuration::from_hours(1)).unwrap();
    serve(
        &mock,
        PATCHES,
        &json!([note("forum", FORUM_OLD, old), note("steam", STEAM_NEW, now)]),
    );
    let outcome = run_at!(&api, &pool, now);
    let known = patches::known_guids(&pool).await.unwrap();

    assert_eq!(outcome, WatchOutcome::default());
    assert!(known.contains(FORUM_OLD));
    assert!(!known.contains(STEAM_NEW));
    assert!(patches::awaiting_diff(&pool).await.unwrap().is_empty());
    assert!(patches::ready(&pool).await.unwrap().is_empty());
}
