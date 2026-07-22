//! Regression coverage for `compendium::perk_entry` (destiny2 DS-2).
//!
//! The "gear perks" tab feeds `compendium::update`, which used to read cells
//! with `Vec::swap_remove(2)` / `swap_remove(0)`. Because the Google Sheets API
//! omits trailing empty cells, any short row (a blank section divider, or a
//! name-only row with no description column) has fewer than three cells, so
//! `swap_remove(2)` panicked — unwinding the interaction task, aborting the
//! `replace` transaction, and leaving `destiny2_compendium_perks` empty so every
//! subsequent `/destiny2 perk` re-ran `update` and re-panicked.
//!
//! `perk_entry` now guards the length and skips short rows via `None`.

use destiny2::compendium::perk_entry;

fn cells(values: &[&str]) -> Vec<Option<String>> {
    values.iter().map(|v| Some((*v).to_string())).collect()
}

#[test]
fn short_row_is_skipped_not_panicked() {
    // Fewer than three cells — the exact shape (a name-only / divider row) that
    // used to panic at `swap_remove(2)`. Must be skipped, never panic.
    assert_eq!(perk_entry(Vec::new()), None);
    assert_eq!(perk_entry(cells(&["Section Divider"])), None);
    assert_eq!(perk_entry(cells(&["Name", "middle"])), None);
}

#[test]
fn full_row_parses_name_and_description() {
    let entry = perk_entry(cells(&[
        "Rampage",
        "ignored middle",
        "Kills grant bonus damage.",
    ]))
    .expect("three populated cells is a valid perk row");

    // key is the lowercased display name; description comes from cell index 2.
    assert_eq!(entry.0, "rampage");
    assert_eq!(entry.1.name, "Rampage");
    assert_eq!(entry.1.description, "Kills grant bonus damage.");
}

#[test]
fn name_is_truncated_at_double_newline_and_flattened() {
    // Names carry a trailing blurb after a blank line, and single newlines
    // inside the name are flattened to spaces — behaviour preserved by the fix.
    let entry = perk_entry(vec![
        Some("Kill\nClip\n\nold flavour text".to_string()),
        None,
        Some("Reloading after a kill grants bonus damage.".to_string()),
    ])
    .expect("valid perk row");

    assert_eq!(entry.1.name, "Kill Clip");
    assert_eq!(entry.0, "kill clip");
}

#[test]
fn missing_name_or_description_cell_is_skipped() {
    // Three cells present, but the name (idx 0) or description (idx 2) cell has
    // no `formatted_value` — not a usable perk, so `None`.
    assert_eq!(perk_entry(vec![None, Some("m".into()), Some("desc".into())]), None);
    assert_eq!(perk_entry(vec![Some("Name".into()), Some("m".into()), None]), None);
}
