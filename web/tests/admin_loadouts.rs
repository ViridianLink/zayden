//! Loadout reads and writes against Postgres: a saved form reads back as the
//! editor form, the list is ordered for the list page, and missing or
//! duplicate loadouts fail with the text the pages show.

use std::future::ready;
use std::sync::atomic::{AtomicBool, Ordering};

use destiny2::endgame_analysis::sheet::Affinity;
use destiny2::loadouts::{
    Archetype,
    ArmourSlot,
    Class,
    Element,
    Mode,
    RawArmour,
    RawAspect,
    RawLoadout,
    RawWeapon,
    StatKind,
};
use sqlx::PgPool;
use web::admin::{
    AdminError,
    EmojiInfo,
    blank,
    catalog,
    draft,
    editor_form,
    loadout_form,
    options,
    remove_unchecked,
    stored_form,
    summaries,
    write_unchecked,
};

fn raw(name: &str) -> RawLoadout {
    RawLoadout {
        name: name.into(),
        class: Class::Warlock,
        element: Element::Prismatic,
        mode: Mode::PvE,
        tags: vec!["Raid".into()],
        super_name: "Song of Flame".into(),
        super_emoji: "song_of_flame".into(),
        class_ability: "healing_rift".into(),
        jump: "burst_glide".into(),
        melee: "arcane_needle".into(),
        grenade: "storm_grenade".into(),
        aspects: vec![RawAspect {
            aspect: "feed_the_void".into(),
            fragments: vec!["facet_of_dawn".into()],
        }],
        weapons: vec![RawWeapon {
            name: "Admin Test Rifle".into(),
            affinity: Affinity::Solar,
            archetype: Archetype::PulseRifle,
            icon_url: "https://www.bungie.net/p.jpg".into(),
            perks: vec!["admin_test_perk".into()],
        }],
        armour: vec![RawArmour {
            slot: ArmourSlot::Chest,
            name: "Admin Test Robes".into(),
            icon_url: "https://www.bungie.net/r.jpg".into(),
            mods: vec!["recuperation".into(), "recuperation".into()],
        }],
        stats: vec![(StatKind::Grenade, 200), (StatKind::Health, 70)],
        artifact_name: "Admin Artifact".into(),
        artifact_perks: vec!["radiant_shrapnel".into()],
        author: "Oscar".into(),
        dim_link: "https://dim.gg/admin".into(),
        video_url: String::new(),
        how_it_works: "Throw storm grenades.".into(),
    }
}

async fn save_new(pool: &PgPool, name: &str) -> Result<i32, AdminError> {
    let form = loadout_form(None, raw(name));
    write_unchecked(pool, None, &draft(&form)?).await
}

#[sqlx::test(migrations = "../migrations")]
async fn a_saved_loadout_reads_back_as_the_editor_form(pool: PgPool) {
    let id = save_new(&pool, "Round Trip").await.unwrap();

    let form = stored_form(&pool, id).await.unwrap();

    assert_eq!(form, editor_form(Some(id), raw("Round Trip")));
}

#[sqlx::test(migrations = "../migrations")]
async fn saving_an_existing_id_replaces_it(pool: PgPool) {
    let id = save_new(&pool, "Before").await.unwrap();

    let mut form = stored_form(&pool, id).await.unwrap();
    form.name = "After".into();
    form.weapons.clear();
    assert_eq!(
        write_unchecked(&pool, form.id, &draft(&form).unwrap()).await.unwrap(),
        id
    );

    let mut expected = raw("After");
    expected.weapons.clear();
    assert_eq!(
        stored_form(&pool, id).await.unwrap(),
        editor_form(Some(id), expected)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn the_list_is_ordered_by_class_then_element(pool: PgPool) {
    let id = save_new(&pool, "Listed").await.unwrap();

    let list = summaries(&pool).await.unwrap();

    let row = list.iter().find(|s| s.id == id).unwrap();
    assert_eq!(
        (
            row.name.as_str(),
            row.class.as_str(),
            row.element.as_str(),
            row.mode.as_str()
        ),
        ("Listed", "Warlock", "Prismatic", "PvE")
    );
    assert_eq!(row.author, "Oscar");
    assert_eq!(row.tags, ["Raid"]);

    let rank = |label: &str, all: &[String]| all.iter().position(|l| l == label);
    let opts = options();
    let keys: Vec<_> = list
        .iter()
        .map(|s| (rank(&s.class, &opts.classes), rank(&s.element, &opts.elements)))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_duplicate_class_element_and_name_is_refused(pool: PgPool) {
    save_new(&pool, "Twice").await.unwrap();

    let form = loadout_form(None, raw("Twice"));
    let error =
        write_unchecked(&pool, None, &draft(&form).unwrap()).await.unwrap_err();

    assert_eq!(
        error.to_string(),
        "a loadout with this class, element and name already exists"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn missing_loadouts_name_their_id(pool: PgPool) {
    let form = loadout_form(None, raw("Ghost"));
    let d = draft(&form).unwrap();

    assert_eq!(
        write_unchecked(&pool, Some(987_654), &d).await.unwrap_err().to_string(),
        "loadout 987654 does not exist"
    );
    assert_eq!(
        stored_form(&pool, 987_654).await.unwrap_err().to_string(),
        "loadout 987654 does not exist"
    );
    assert_eq!(
        remove_unchecked(&pool, 987_654).await.unwrap_err().to_string(),
        "loadout 987654 does not exist"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_deleted_loadout_is_gone(pool: PgPool) {
    let id = save_new(&pool, "Doomed").await.unwrap();

    remove_unchecked(&pool, id).await.unwrap();

    assert!(summaries(&pool).await.unwrap().iter().all(|s| s.id != id));
    assert_eq!(
        remove_unchecked(&pool, id).await.unwrap_err().to_string(),
        format!("loadout {id} does not exist")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn the_catalog_offers_what_saved_loadouts_use(pool: PgPool) {
    save_new(&pool, "Catalogued").await.unwrap();
    let emoji = EmojiInfo { name: "song_of_flame".into(), id: "42".into() };

    let c = catalog(&pool, ready(vec![emoji.clone()])).await.unwrap();

    assert_eq!(c.emojis, [emoji]);
    assert_eq!(c.blank, blank());
    assert_eq!(c.options, options());

    let rifle = c.weapons.iter().find(|w| w.name == "Admin Test Rifle").unwrap();
    assert_eq!(
        (rifle.affinity.as_str(), rifle.archetype.as_str()),
        ("Solar", "Pulse Rifle")
    );
    assert!(rifle.known_perks.iter().any(|p| p == "admin_test_perk"));
    assert!(c.perks.iter().any(|p| p == "admin_test_perk"));

    assert!(c.usage.iter().any(|u| {
        u.field == "super"
            && u.key == "song_of_flame"
            && u.class == "Warlock"
            && u.element == "Prismatic"
            && u.uses == 1
    }));
    assert!(
        c.usage.iter().any(|u| u.field == "armour_mod" && u.key == "recuperation")
    );
    assert!(
        c.super_names
            .iter()
            .any(|(emoji, name)| emoji == "song_of_flame" && name == "Song of Flame")
    );
    assert!(c.armour.iter().any(|a| {
        a.slot == "Chest" && a.class == "Warlock" && a.name == "Admin Test Robes"
    }));
}

/// The bot is asked for its emojis only after every catalog query succeeds.
#[sqlx::test(migrations = "../migrations")]
async fn the_catalog_skips_discord_when_the_database_fails(pool: PgPool) {
    let polled = AtomicBool::new(false);
    pool.close().await;
    let emojis = async {
        polled.store(true, Ordering::SeqCst);
        Vec::new()
    };

    assert!(catalog(&pool, emojis).await.is_err());
    assert!(!polled.load(Ordering::SeqCst));
}
