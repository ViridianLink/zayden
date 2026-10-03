//! A loadout posted as a flat form folds into the same `LoadoutForm` a JSON
//! body would carry, and any input the fold cannot place is refused.

use web::admin::{
    AspectForm,
    LoadoutFieldError,
    LoadoutForm,
    WeaponForm,
    blank,
    fold_loadout_form,
    loadout_pairs,
    row_name,
};

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

fn full() -> LoadoutForm {
    LoadoutForm {
        id: Some(12),
        name: "Pairs".to_owned(),
        tags: vec!["Raid".to_owned(), "Solo".to_owned()],
        super_name: "Thundercrash".to_owned(),
        artifact_perks: vec!["a_perk".to_owned(), "b_perk".to_owned()],
        aspects: vec![
            AspectForm {
                aspect: "knockout".to_owned(),
                fragments: vec![
                    "spark_of_shock".to_owned(),
                    "spark_of_ions".to_owned(),
                ],
            },
            AspectForm::default(),
        ],
        weapons: vec![WeaponForm {
            name: "W".to_owned(),
            affinity: "Arc".to_owned(),
            archetype: "Auto Rifle".to_owned(),
            icon_url: "https://www.bungie.net/w.jpg".to_owned(),
            perks: vec!["field_prep".to_owned()],
        }],
        how_it_works: "Line one\nline two".to_owned(),
        ..blank()
    }
}

#[test]
fn row_names_index_the_list() {
    assert_eq!(row_name("weapons", 2, "perks"), "weapons[2].perks");
}

#[test]
fn a_form_survives_the_flat_round_trip() {
    assert_eq!(fold_loadout_form(loadout_pairs(&full())).unwrap(), full());
    assert_eq!(fold_loadout_form(loadout_pairs(&blank())).unwrap(), blank());
}

#[test]
fn the_vec_conversion_is_the_fold() {
    let form = LoadoutForm::try_from(loadout_pairs(&full())).unwrap();
    assert_eq!(form, full());
}

#[test]
fn a_blank_id_is_a_new_loadout() {
    let form = fold_loadout_form(pairs(&[("id", " "), ("name", "N")])).unwrap();
    assert_eq!(form.id, None);
    assert_eq!(form.name, "N");
}

#[test]
fn rows_follow_their_index_and_gaps_close() {
    let form = fold_loadout_form(pairs(&[
        ("stats[5].stat", "weapons"),
        ("stats[0].stat", "health"),
        ("stats[0].value", "30"),
    ]))
    .unwrap();
    let stats: Vec<(&str, &str)> =
        form.stats.iter().map(|s| (s.stat.as_str(), s.value.as_str())).collect();
    assert_eq!(stats, [("health", "30"), ("weapons", "")]);
}

#[test]
fn unknown_or_repeated_fields_are_refused() {
    assert_eq!(
        fold_loadout_form(pairs(&[("colour", "red")])),
        Err(LoadoutFieldError::UnknownField("colour".to_owned()))
    );
    assert_eq!(
        fold_loadout_form(pairs(&[("weapons[0].colour", "red")])),
        Err(LoadoutFieldError::UnknownField("weapons[0].colour".to_owned()))
    );
    assert_eq!(
        fold_loadout_form(pairs(&[("name", "a"), ("name", "b")])),
        Err(LoadoutFieldError::RepeatedField("name".to_owned()))
    );
    assert_eq!(
        fold_loadout_form(pairs(&[
            ("armour[1].name", "a"),
            ("armour[1].name", "b")
        ])),
        Err(LoadoutFieldError::RepeatedField("armour[1].name".to_owned()))
    );
    assert_eq!(
        fold_loadout_form(pairs(&[("id", "1"), ("id", "1")])),
        Err(LoadoutFieldError::RepeatedField("id".to_owned()))
    );
}

#[test]
fn non_canonical_indexes_are_not_rows() {
    for key in ["weapons[01].name", "weapons[+1].name", "weapons[].name"] {
        assert_eq!(
            fold_loadout_form(pairs(&[(key, "W")])),
            Err(LoadoutFieldError::UnknownField(key.to_owned())),
            "{key}"
        );
    }
}

#[test]
fn a_non_numeric_id_is_refused() {
    assert_eq!(
        fold_loadout_form(pairs(&[("id", "seven")])),
        Err(LoadoutFieldError::InvalidId("seven".to_owned()))
    );
}
