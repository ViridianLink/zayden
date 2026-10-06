//! Pure helpers behind the loadout editor page: the emoji index, the
//! missing-emoji warning, draft keys, id parsing, the inline JSON data block and
//! the JSON reply shape the editor script reads.

use web::admin::editor::{
    CatalogIndex,
    NewEmoji,
    Reply,
    draft_key,
    editor_data,
    editor_id,
    emoji_keys,
    missing_emojis,
};
use web::admin::{
    AspectForm,
    EmojiInfo,
    EmojiSource,
    LoadoutCatalog,
    LoadoutCheck,
    LoadoutForm,
    WeaponForm,
    blank,
    options,
};

fn catalog(emojis: &[(&str, &str)]) -> LoadoutCatalog {
    LoadoutCatalog {
        emojis: emojis
            .iter()
            .map(|(name, id)| EmojiInfo {
                name: (*name).to_owned(),
                id: (*id).to_owned(),
            })
            .collect(),
        options: options(),
        blank: blank(),
        ..LoadoutCatalog::default()
    }
}

fn filled() -> LoadoutForm {
    LoadoutForm {
        super_emoji: "thundercrash".to_owned(),
        class_ability: " ".to_owned(),
        jump: "catapult_lift".to_owned(),
        aspects: vec![AspectForm {
            aspect: "knockout".to_owned(),
            fragments: vec!["spark_of_ions".to_owned(), "thundercrash".to_owned()],
        }],
        weapons: vec![WeaponForm {
            name: "Choir of One".to_owned(),
            perks: vec!["field_prep".to_owned()],
            ..WeaponForm::default()
        }],
        artifact_perks: vec!["anti_barrier".to_owned()],
        ..blank()
    }
}

#[test]
fn the_index_maps_emoji_names_to_cdn_images_and_reserves_enum_labels() {
    let index = CatalogIndex::new(&catalog(&[("arc", "11"), ("knockout", "22")]));

    assert_eq!(
        index.image("knockout"),
        Some("https://cdn.discordapp.com/emojis/22.webp?size=64")
    );
    assert_eq!(
        index.enum_image("Arc"),
        Some("https://cdn.discordapp.com/emojis/11.webp?size=64")
    );
    assert_eq!(index.image("missing"), None);
    assert!(index.has_any_emoji());
    assert!(index.is_reserved("auto_rifle"));
    assert!(index.is_reserved("class_item"));
    assert!(index.is_reserved("pve"));
    assert!(!index.is_reserved("knockout"));
    assert!(!CatalogIndex::new(&catalog(&[])).has_any_emoji());
}

#[test]
fn emoji_keys_cover_every_key_field_and_skip_blanks() {
    assert_eq!(emoji_keys(&filled()), [
        "thundercrash",
        "catapult_lift",
        "knockout",
        "spark_of_ions",
        "thundercrash",
        "field_prep",
        "anti_barrier",
    ]);
}

#[test]
fn missing_emojis_are_sorted_deduped_and_hidden_when_no_emoji_loaded() {
    let index =
        CatalogIndex::new(&catalog(&[("knockout", "1"), ("field_prep", "2")]));
    assert_eq!(missing_emojis(&index, &filled()), [
        "anti_barrier",
        "catapult_lift",
        "spark_of_ions",
        "thundercrash",
    ]);
    assert_eq!(
        missing_emojis(&CatalogIndex::new(&catalog(&[])), &filled()),
        Vec::<String>::new()
    );
}

#[test]
fn drafts_are_keyed_by_id_and_unparsable_ids_open_a_new_loadout() {
    assert_eq!(draft_key(None), "zayden:loadout-draft:new");
    assert_eq!(draft_key(Some(7)), "zayden:loadout-draft:7");
    assert_eq!(editor_id("12"), Some(12));
    assert_eq!(editor_id("abc"), None);
    assert_eq!(editor_id("new"), None);
    assert_eq!(editor_id("99999999999"), None);
}

#[test]
fn editor_data_round_trips_and_cannot_close_its_script_element() {
    let form = LoadoutForm {
        id: Some(3),
        name: "</script><!-- x".to_owned(),
        how_it_works: "a < b & c > d".to_owned(),
        ..blank()
    };
    let data = editor_data(&catalog(&[]), &form).expect("data");

    assert!(!data.contains('<'));
    let parsed: serde_json::Value = serde_json::from_str(&data).expect("json");
    assert_eq!(parsed["draftKey"], "zayden:loadout-draft:3");
    assert_eq!(parsed["form"]["name"], "</script><!-- x");
    let back: LoadoutForm =
        serde_json::from_value(parsed["form"].clone()).expect("form");
    assert_eq!(back, form);
    assert!(parsed["catalog"]["options"]["classes"].is_array());
}

#[test]
fn replies_are_externally_tagged_ok_or_error() {
    let ok = serde_json::to_value(Reply::Ok(LoadoutCheck {
        components: 12,
        max_components: 40,
        text: None,
        max_text: 4000,
        error: Some("name is required".to_owned()),
    }))
    .expect("json");
    assert_eq!(ok["ok"]["components"], 12);
    assert_eq!(ok["ok"]["text"], serde_json::Value::Null);
    assert_eq!(ok["ok"]["error"], "name is required");

    let error = serde_json::to_value(Reply::<i32>::Error("forbidden".to_owned()))
        .expect("json");
    assert_eq!(error, serde_json::json!({ "error": "forbidden" }));
}

#[test]
fn new_emoji_requests_use_externally_tagged_sources() {
    let link: NewEmoji = serde_json::from_str(
        r#"{"name":"spark","source":{"Url":"https://x/a.png"}}"#,
    )
    .expect("url");
    assert_eq!(link.source, EmojiSource::Url("https://x/a.png".to_owned()));
    let file: NewEmoji = serde_json::from_str(
        r#"{"name":"spark","source":{"DataUri":"data:image/png;base64,AA=="}}"#,
    )
    .expect("data uri");
    assert_eq!(
        file.source,
        EmojiSource::DataUri("data:image/png;base64,AA==".to_owned())
    );
}
