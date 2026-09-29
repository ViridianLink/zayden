//! The website editor writes loadouts through `save`/`delete`; the bot reads
//! them back through `all`/`by_id`. A save must be lossless (what the admin typed
//! is what `/destiny2 builds` renders), must fully replace an edited loadout's
//! child rows, and must reuse catalog rows by name.

use std::time::Duration;

use destiny2::SaveError;
use destiny2::db::{loadout_catalog, loadout_writes, loadouts};
use destiny2::endgame_analysis::sheet::Affinity;
use destiny2::loadouts::{
    Archetype,
    ArmourSlot,
    Class,
    Element,
    LoadoutDraft,
    Mode,
    RawArmour,
    RawAspect,
    RawLoadout,
    RawWeapon,
    StatKind,
};
use sqlx::PgPool;
use sqlx::postgres::PgListener;
use tokio::time::timeout;

fn raw(name: &str) -> RawLoadout {
    RawLoadout {
        name: name.into(),
        class: Class::Warlock,
        element: Element::Prismatic,
        mode: Mode::All,
        tags: vec!["Raid".into(), "GM".into()],
        super_name: "Song of Flame".into(),
        super_emoji: "song_of_flame".into(),
        class_ability: "phoenix_dive".into(),
        jump: "burst_glide".into(),
        melee: "arcane_needle".into(),
        grenade: "storm_grenade".into(),
        aspects: vec![
            RawAspect {
                aspect: "hellion".into(),
                fragments: vec!["facet_of_purpose".into(), "facet_of_dawn".into()],
            },
            RawAspect { aspect: "feed_the_void".into(), fragments: vec![] },
        ],
        weapons: vec![RawWeapon {
            name: "Test Weapon".into(),
            affinity: Affinity::Solar,
            archetype: Archetype::Glaive,
            icon_url: "https://www.bungie.net/w.jpg".into(),
            perks: vec!["field_prep".into(), "brand_new_perk".into()],
        }],
        armour: vec![RawArmour {
            slot: ArmourSlot::Chest,
            name: "Test Robes".into(),
            icon_url: "https://www.bungie.net/c.jpg".into(),
            mods: vec!["solar_siphon".into(), "solar_siphon".into()],
        }],
        stats: vec![(StatKind::Grenade, 200), (StatKind::Super, 120)],
        artifact_name: "Tablet".into(),
        artifact_perks: vec!["radiant_shrapnel".into()],
        author: "Oscar".into(),
        dim_link: "https://dim.gg/x".into(),
        video_url: String::new(),
        how_it_works: "Throw grenades.".into(),
    }
}

// A macro so `expect` expands inside the `#[sqlx::test]` that clippy exempts.
macro_rules! save {
    ($pool:expr, $id:expr, $raw:expr) => {{
        let draft =
            LoadoutDraft::try_from($raw).expect("test data should be a valid draft");
        let mut tx = $pool.begin().await.expect("transaction should begin");
        let result = loadout_writes::save(&mut tx, $id, &draft).await;
        if result.is_ok() {
            tx.commit().await.expect("transaction should commit");
        }
        result
    }};
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_created_loadout_reads_back_unchanged(pool: PgPool) {
    let id = save!(&pool, None, raw("Round Trip")).unwrap();

    let record = loadouts::by_id(&pool, id).await.unwrap().unwrap();

    assert_eq!(RawLoadout::from(record), raw("Round Trip"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn an_edit_replaces_every_child_row(pool: PgPool) {
    let id = save!(&pool, None, raw("Edit Me")).unwrap();
    let mut edited = raw("Edit Me");
    edited.aspects.truncate(1);
    edited.tags = vec![];
    edited.stats = vec![(StatKind::Health, 50)];

    assert_eq!(save!(&pool, Some(id), edited.clone()).unwrap(), id);

    let record = loadouts::by_id(&pool, id).await.unwrap().unwrap();
    assert_eq!(RawLoadout::from(record), edited);
}

#[sqlx::test(migrations = "../../migrations")]
async fn catalog_rows_are_reused_by_name(pool: PgPool) {
    let before = loadout_catalog::perks(&pool).await.unwrap().len();

    save!(&pool, None, raw("First")).unwrap();
    save!(&pool, None, raw("Second")).unwrap();

    let perks = loadout_catalog::perks(&pool).await.unwrap();
    assert_eq!(perks.len(), before + 1, "only brand_new_perk is new");
    let weapons = loadout_catalog::weapons(&pool).await.unwrap();
    assert_eq!(weapons.iter().filter(|w| w.name == "Test Weapon").count(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_duplicate_class_element_name_is_a_readable_error(pool: PgPool) {
    save!(&pool, None, raw("Twin")).unwrap();

    assert!(matches!(
        save!(&pool, None, raw("Twin")),
        Err(SaveError::DuplicateName)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn updating_a_missing_loadout_is_not_found(pool: PgPool) {
    assert!(matches!(
        save!(&pool, Some(999_999), raw("Ghost")),
        Err(SaveError::NotFound(999_999))
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn delete_removes_the_loadout_and_its_children(pool: PgPool) {
    let id = save!(&pool, None, raw("Doomed")).unwrap();

    assert!(loadout_writes::delete(&pool, id).await.unwrap());
    assert!(loadouts::by_id(&pool, id).await.unwrap().is_none());
    assert!(!loadout_writes::delete(&pool, id).await.unwrap());
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_seeded_loadouts_still_load(pool: PgPool) {
    assert_eq!(loadouts::all(&pool).await.unwrap().len(), 11);
}

#[sqlx::test(migrations = "../../migrations")]
async fn emoji_keys_in_use_include_seeded_keys(pool: PgPool) {
    let keys = loadout_catalog::emoji_keys_in_use(&pool).await.unwrap();
    assert!(keys.iter().any(|k| k == "thundercrash"));
    assert!(keys.iter().any(|k| k == "spark_of_shock"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_save_raises_one_loadouts_changed_notification(pool: PgPool) {
    let mut listener = PgListener::connect_with(&pool).await.unwrap();
    listener.listen("loadouts_changed").await.unwrap();

    save!(&pool, None, raw("Noisy")).unwrap();

    let first = timeout(Duration::from_secs(5), listener.recv()).await;
    assert!(first.is_ok_and(|n| n.is_ok()), "a committed save must notify");
    let second = timeout(Duration::from_millis(500), listener.recv()).await;
    assert!(
        second.is_err(),
        "Postgres folds identical notifications within one transaction"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn every_seeded_loadout_is_an_editable_draft(pool: PgPool) {
    for record in loadouts::all(&pool).await.unwrap() {
        let name = record.name.clone();
        let result = LoadoutDraft::try_from(RawLoadout::from(record));
        assert_eq!(result.err(), None, "seeded loadout {name}");
    }
}
