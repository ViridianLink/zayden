//! The messages guild calls fail with, as the dashboard shows them.

use web::auth::{AuthError, ForeignIdError};
use web::guild::GuildError;
use web::util::server_error_text;

#[test]
fn every_message_matches_what_the_dashboard_shows() {
    let cases = [
        (GuildError::Server("boom".to_owned()), "boom"),
        (GuildError::InvalidWikiUrl, "invalid wiki URL"),
        (
            GuildError::WikiUrlScheme,
            "the wiki URL must start with http:// or https://",
        ),
        (GuildError::InvalidRole, "invalid role"),
        (GuildError::DuplicateSupportRole, "that role is already a support role"),
        (GuildError::InvalidUserId, "invalid user id"),
        (
            GuildError::InvalidLink("empty host".to_owned()),
            "invalid link: empty host",
        ),
        (GuildError::LinkScheme, "link must be an http:// or https:// address"),
        (GuildError::LinkCredentials, "link must not embed credentials"),
        (GuildError::LinkTooLong, "link is too long"),
        (GuildError::NoCategory, "select a category first"),
        (GuildError::InvalidChannelId, "invalid channel id"),
        (GuildError::InvalidEmail, "invalid email"),
        (
            GuildError::KofiEmailTaken,
            "This Ko-fi email is already linked to an account.",
        ),
        (GuildError::UnknownModule, "unknown module"),
        (
            GuildError::NoSettingsSwitch("music".to_owned()),
            "module music has no settings switch",
        ),
        (
            GuildError::DerivedModule("Patreon"),
            "Patreon is switched on from its own settings page, not from this toggle.",
        ),
        (
            GuildError::CommandNotRegistered("good".to_owned()),
            "/good isn't registered for this server yet",
        ),
        (
            GuildError::OperatorCommandPermissions("good".to_owned()),
            "Discord only lets a member with Manage Server change command \
             permissions, so /good can't be changed through operator access.",
        ),
        (
            GuildError::PermissionUpdateRejected {
                name: "good".to_owned(),
                reason: "Missing Access".to_owned(),
            },
            "Discord rejected the permission update for /good: Missing Access",
        ),
    ];

    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn access_errors_keep_their_own_message() {
    let cases = [
        (AuthError::Unauthenticated, "unauthenticated"),
        (AuthError::Forbidden, "forbidden"),
        (AuthError::InvalidGuildId, "invalid guild id"),
        (AuthError::BotNotInGuild, "Zayden isn't in that server"),
        (ForeignIdError::Channel.into(), "that channel is not in this server"),
        (ForeignIdError::Role.into(), "that role is not in this server"),
    ];

    for (error, message) in cases {
        assert_eq!(GuildError::from(error).to_string(), message);
    }
}

#[test]
fn pages_render_errors_with_the_server_error_prefix() {
    assert_eq!(
        server_error_text(GuildError::InvalidEmail),
        "error running server function: invalid email"
    );
}

#[test]
fn only_refusals_count_as_denied() {
    assert!(GuildError::from(AuthError::Unauthenticated).is_denied());
    assert!(GuildError::from(AuthError::Forbidden).is_denied());
    assert!(!GuildError::from(AuthError::InvalidGuildId).is_denied());
    assert!(!GuildError::InvalidRole.is_denied());
}

#[test]
fn only_form_shape_errors_are_invalid_forms() {
    assert!(GuildError::InvalidField("enabled").is_invalid_form());
    assert!(!GuildError::InvalidRole.is_invalid_form());
    assert!(!GuildError::from(AuthError::Forbidden).is_invalid_form());
}

#[test]
fn a_signed_out_caller_is_redirected_instead_of_shown_an_error() {
    assert!(
        GuildError::from(AuthError::Unauthenticated)
            .redirect_unauthenticated()
            .is_err()
    );
}

#[test]
fn any_other_error_comes_back_to_be_rendered() {
    assert_eq!(
        GuildError::from(AuthError::Forbidden).redirect_unauthenticated().unwrap(),
        GuildError::Auth(AuthError::Forbidden)
    );
    assert_eq!(
        GuildError::InvalidRole.redirect_unauthenticated().unwrap(),
        GuildError::InvalidRole
    );
}
