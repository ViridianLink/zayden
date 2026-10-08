//! Settings forms read their urlencoded pairs strictly: each expected field
//! exactly once and nothing else.

use web::guild::GuildError;
use web::guild::faq::WikiSettingsForm;
use web::guild::kofi::KofiEmailForm;
use web::guild::modules::ModuleToggleForm;
use web::guild::settings::{AiSettingsForm, FamilySettingsForm, ServerSettingsForm};
use web::guild::support::TicketSettingsForm;

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

/// The merged forms read every field of the sections they replace, so a stale
/// page that posts only one of the old forms is refused before any write.
#[test]
fn merged_forms_need_every_field_of_their_sections() {
    let server = ServerSettingsForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("rules_channel_id", "1"),
        ("general_channel_id", ""),
        ("spoiler_channel_id", ""),
        ("artist_role_id", "2"),
        ("sleep_role_id", ""),
        ("verified_role_id", ""),
    ]))
    .unwrap();
    assert_eq!(
        (server.rules_channel_id.as_str(), server.artist_role_id.as_str()),
        ("1", "2")
    );

    let channels_only = refused(ServerSettingsForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("rules_channel_id", ""),
        ("general_channel_id", ""),
        ("spoiler_channel_id", ""),
    ])))
    .unwrap();
    assert_eq!(channels_only, GuildError::MissingField("artist_role_id"));

    let idle_only = refused(TicketSettingsForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("idle_enabled", "true"),
        ("idle_after_secs", "3600"),
        ("idle_close_enabled", "false"),
        ("idle_close_after_secs", "86400"),
    ])))
    .unwrap();
    assert_eq!(idle_only, GuildError::MissingField("support_channel_id"));

    let tuning_only = refused(WikiSettingsForm::from_pairs(pairs(&[
        ("guild", "7"),
        ("max_results", "5"),
        ("answer_max_tokens", "500"),
        ("answer_temperature", "0.2"),
    ])))
    .unwrap();
    assert_eq!(tuning_only, GuildError::MissingField("enabled"));
}

#[test]
fn a_partial_write_names_what_was_and_was_not_saved() {
    let err = GuildError::PartlySaved {
        saved: "The channels",
        unsaved: "the roles",
        reason: "pool timed out".to_owned(),
    };

    assert_eq!(
        err.to_string(),
        "The channels were saved, but the roles were not: pool timed out"
    );
    assert!(!err.is_invalid_form());
}
