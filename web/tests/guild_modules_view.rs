//! How a module's card reads its state.

use std::collections::HashMap;

use web::guild::modules::view;
use zayden_app::modules::{self, ModuleStates};

const DERIVED_LOCK: &str = "This module is switched on from its own settings page \u{2014} use Configure below.";

fn flags() -> HashMap<&'static str, bool> {
    HashMap::from([("ai", true), ("patreon", false), ("youtube", true)])
}

#[test]
fn a_command_module_reads_its_stored_state() {
    let music = modules::find("music").unwrap();
    let states = ModuleStates::from([("music".to_owned(), false)]);

    let card = view(music, &states, &flags());

    assert_eq!(card.id, "music");
    assert_eq!(card.label, music.label);
    assert_eq!(card.description, music.description);
    assert_eq!(card.enabled, Some(false));
    assert_eq!(card.locked, None);
}

#[test]
fn a_command_module_without_a_row_is_unknown() {
    let music = modules::find("music").unwrap();

    assert_eq!(view(music, &ModuleStates::new(), &flags()).enabled, None);
}

#[test]
fn a_settings_module_reads_its_settings_flag() {
    let ai = modules::find("ai").unwrap();
    let states = ModuleStates::from([("ai".to_owned(), false)]);

    let card = view(ai, &states, &flags());

    assert_eq!(card.enabled, Some(true));
    assert_eq!(card.locked, None);
}

#[test]
fn a_derived_module_reads_its_flag_and_is_locked() {
    for (id, enabled) in [("patreon", false), ("youtube", true)] {
        let card = view(modules::find(id).unwrap(), &ModuleStates::new(), &flags());

        assert_eq!(card.enabled, Some(enabled), "{id}");
        assert_eq!(card.locked.as_deref(), Some(DERIVED_LOCK), "{id}");
    }
}
