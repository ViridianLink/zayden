//! Settings forms read their urlencoded pairs strictly: each expected field
//! exactly once and nothing else.

use web::guild::GuildError;
use web::guild::kofi::KofiEmailForm;
use web::guild::modules::ModuleToggleForm;
use web::guild::settings::{AiSettingsForm, FamilySettingsForm};

fn refused<T>(result: Result<T, GuildError>) -> Option<GuildError> {
    result.err()
}

fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

#[test]
fn every_field_is_read_by_name_in_any_order() {
    let form = AiSettingsForm::from_pairs(pairs(&[
        ("channel_id", "11"),
        ("guild", "7"),
        ("enabled", "true"),
    ]))
    .unwrap();

    assert_eq!(form.guild, "7");
    assert_eq!(form.enabled, "true");
    assert_eq!(form.channel_id, "11");
}

#[test]
fn values_are_kept_verbatim() {
    let form = FamilySettingsForm::from_pairs(pairs(&[
        ("guild", " 7 "),
        ("max_partners", ""),
    ]))
    .unwrap();

    assert_eq!(form.guild, " 7 ");
    assert_eq!(form.max_partners, "");
}

#[test]
fn an_unknown_field_is_refused() {
    let err = refused(FamilySettingsForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("max_partners", "2"),
        ("extra", "x"),
    ])))
    .unwrap();

    assert_eq!(err, GuildError::UnknownField("extra".to_owned()));
    assert_eq!(err.to_string(), "unknown field `extra`");
    assert!(err.is_invalid_form());
}

#[test]
fn a_repeated_field_is_refused() {
    let err = refused(FamilySettingsForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("max_partners", "2"),
        ("max_partners", "3"),
    ])))
    .unwrap();

    assert_eq!(err.to_string(), "duplicate field `max_partners`");
    assert!(err.is_invalid_form());
}

#[test]
fn a_missing_field_is_refused() {
    let err =
        refused(FamilySettingsForm::from_pairs(pairs(&[("guild", "7")]))).unwrap();

    assert_eq!(err, GuildError::MissingField("max_partners"));
    assert_eq!(err.to_string(), "missing field `max_partners`");
    assert!(err.is_invalid_form());
}

#[test]
fn an_empty_body_names_the_first_missing_field() {
    let err = refused(KofiEmailForm::from_pairs(Vec::new())).unwrap();

    assert_eq!(err.to_string(), "missing field `email`");
}

#[test]
fn a_module_toggle_reads_only_true_or_false() {
    let toggle = |enabled: &str| {
        ModuleToggleForm::from_pairs(pairs(&[
            ("guild", "7"),
            ("module_id", "music"),
            ("enabled", enabled),
        ]))
        .unwrap()
        .enabled()
    };

    assert_eq!(toggle("true"), Ok(true));
    assert_eq!(toggle("false"), Ok(false));

    let err = toggle("on").unwrap_err();
    assert_eq!(err.to_string(), "invalid value for `enabled`");
    assert!(err.is_invalid_form());
}
