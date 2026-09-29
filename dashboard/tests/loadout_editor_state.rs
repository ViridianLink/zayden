#![cfg(feature = "ssr")]
//! The editor holds one signal per field so typing never re-renders a row and
//! steals focus. Loading a form into signals and snapshotting it back must be
//! lossless, and row keys must stay unique across add/remove so `<For>` never
//! reuses a removed row's inputs.

use dashboard::dto::destiny2::{AspectForm, LoadoutForm, StatForm, WeaponForm};
use dashboard::ui::pages::destiny2_loadouts::state::EditorState;
use leptos::prelude::*;

fn form() -> LoadoutForm {
    LoadoutForm {
        id: Some(3),
        name: "State Trip".into(),
        class: "Titan".into(),
        tags: vec!["Raid".into()],
        super_emoji: "thundercrash".into(),
        aspects: vec![AspectForm {
            aspect: "knockout".into(),
            fragments: vec!["spark_of_shock".into()],
        }],
        weapons: vec![WeaponForm {
            name: "W".into(),
            perks: vec!["field_prep".into()],
            ..WeaponForm::default()
        }],
        stats: vec![StatForm { stat: "weapons".into(), value: "200".into() }],
        ..LoadoutForm::default()
    }
}

#[test]
fn a_form_survives_the_signal_round_trip() {
    let owner = Owner::new();
    owner.with(|| {
        assert_eq!(EditorState::from_form(form()).to_form(), form());
    });
}

#[test]
fn row_keys_stay_unique_after_remove_and_add() {
    let owner = Owner::new();
    owner.with(|| {
        let state = EditorState::from_form(form());
        let first = state.aspects.get_untracked()[0].key;
        state.remove_aspect(first);
        state.add_aspect();
        assert_ne!(state.aspects.get_untracked()[0].key, first);
    });
}

#[test]
fn emoji_keys_collects_every_key_field() {
    let owner = Owner::new();
    owner.with(|| {
        let keys = EditorState::from_form(form()).emoji_keys();
        for key in ["thundercrash", "knockout", "spark_of_shock", "field_prep"] {
            assert!(keys.iter().any(|k| k == key), "{key} missing");
        }
    });
}

#[test]
fn the_tracked_snapshot_matches_the_untracked_form() {
    let owner = Owner::new();
    owner.with(|| {
        let state = EditorState::from_form(form());
        assert_eq!(state.snapshot(), state.to_form());
    });
}

/// A row added from a picker callback or a `<Show>`-guarded button is created
/// while a short-lived owner is current. Its signals must belong to the editor,
/// or disposing that owner kills the row and the next read aborts the page.
#[test]
fn rows_added_under_a_short_lived_owner_outlive_it() {
    let owner = Owner::new();
    owner.with(|| {
        let state = EditorState::from_form(form());
        let transient = Owner::new();
        transient.with(|| {
            state.add_weapon(WeaponForm::default());
            state.add_aspect();
        });
        transient.cleanup();

        let weapons = state.weapons.get_untracked();
        let aspects = state.aspects.get_untracked();
        assert!(
            weapons.last().is_some_and(|w| w.perks.try_get_untracked().is_some())
        );
        assert!(
            aspects
                .last()
                .is_some_and(|a| a.fragments.try_get_untracked().is_some())
        );
    });
}

#[test]
fn move_item_reorders_and_ignores_out_of_range() {
    use dashboard::ui::pages::destiny2_loadouts::state::move_item;
    let mut v = vec!['a', 'b', 'c', 'd'];
    move_item(&mut v, 0, 2);
    assert_eq!(v, ['b', 'c', 'a', 'd']);
    move_item(&mut v, 3, 0);
    assert_eq!(v, ['d', 'b', 'c', 'a']);
    move_item(&mut v, 1, 9);
    assert_eq!(v, ['d', 'b', 'c', 'a']);
}

#[test]
fn applying_a_draft_replaces_every_field() {
    let owner = Owner::new();
    owner.with(|| {
        let state = EditorState::from_form(LoadoutForm::default());
        state.apply(form());
        assert_eq!(state.to_form(), form());
    });
}
