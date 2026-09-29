//! Discord rejects a whole autocomplete response when any choice name exceeds
//! 100 characters, which would hide every build of that class.

use destiny2::endgame_analysis::sheet::Affinity;
use destiny2::loadouts::{
    Archetype,
    Class,
    Element,
    LoadoutRecord,
    Mode,
    WeaponRecord,
    limits,
};

fn record(name: String) -> LoadoutRecord {
    LoadoutRecord {
        id: 1,
        name,
        class: Class::Warlock,
        element: Element::Prismatic,
        mode: Mode::All,
        tags: vec![],
        super_name: "Song of Flame".into(),
        super_emoji: "song_of_flame".into(),
        class_ability: "phoenix_dive".into(),
        jump: "burst_glide".into(),
        melee: "arcane_needle".into(),
        grenade: "storm_grenade".into(),
        aspects: vec![],
        weapons: vec![WeaponRecord {
            name: "Test".into(),
            affinity: Affinity::Solar,
            archetype: Archetype::Glaive,
            icon_url: "https://www.bungie.net/w.jpg".into(),
            perks: vec![],
        }],
        armour: vec![],
        stats: vec![],
        artifact_name: None,
        artifact_perks: vec![],
        author: "Oscar".into(),
        dim_link: "https://dim.gg/x".into(),
        video_url: None,
        how_it_works: None,
    }
}

#[test]
fn a_maximum_length_name_fits_discords_choice_limit() {
    let label = record("é".repeat(limits::NAME)).choice_label();
    assert_eq!(label.chars().count(), 100);
    assert!(label.starts_with("Prismatic | é"));
}

#[test]
fn a_short_label_is_unchanged() {
    assert_eq!(record("Hellion".into()).choice_label(), "Prismatic | Hellion");
}
