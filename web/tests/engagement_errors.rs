//! The messages engagement calls fail with, as the dashboard shows them. The
//! router tests in `engagement_data` pin the same texts end to end; these pin
//! the typed variants and the conversions.

use greetings::GreetingsError;
use reaction_roles::ReactionRoleError;
use web::auth::{AuthError, ForeignIdError};
use web::engagement::EngagementError;
use web::guild::GuildError;
use web::util::server_error_text;

#[test]
fn every_message_matches_what_the_dashboard_shows() {
    let cases = [
        (EngagementError::Server("boom".to_owned()), "boom"),
        (EngagementError::Invalid("channel"), "invalid channel"),
        (EngagementError::Invalid("role"), "invalid role"),
        (EngagementError::Invalid("message id"), "invalid message id"),
        (EngagementError::Invalid("custom emoji id"), "invalid custom emoji id"),
        (EngagementError::Invalid("channel id"), "invalid channel id"),
        (EngagementError::Invalid("image id"), "invalid image id"),
        (
            EngagementError::EmojiAlreadyMapped,
            "that emoji is already mapped on that message",
        ),
        (
            EngagementError::ChannelAlreadyListed,
            "that channel is already on the list",
        ),
        (
            EngagementError::ChannelListFull(90),
            "Discord allows at most 90 channels per command. Remove one \
             before adding another.",
        ),
        (
            EngagementError::ChannelNotListed,
            "that channel is not on this server's list",
        ),
        (EngagementError::ImageNotFound, "that image is not in this server's list"),
        (
            EngagementError::CooldownBelowFloor {
                tier: "Free",
                label: "per-member",
                floor: 15,
                upgrade: "Pro servers can go as low as 3s.".to_owned(),
            },
            "On the Free plan the per-member cooldown can't go below 15s. Pro \
             servers can go as low as 3s.",
        ),
        (
            EngagementError::CooldownBelowFloor {
                tier: "Ultra",
                label: "server-wide",
                floor: 1,
                upgrade: "That is as low as this command goes.".to_owned(),
            },
            "On the Ultra plan the server-wide cooldown can't go below 1s. That \
             is as low as this command goes.",
        ),
    ];

    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn form_shape_errors_name_the_field() {
    assert_eq!(
        EngagementError::UnknownField("extra".to_owned()).to_string(),
        "unknown field `extra`"
    );
    assert_eq!(
        EngagementError::DuplicateField("guild".to_owned()).to_string(),
        "duplicate field `guild`"
    );
    assert_eq!(
        EngagementError::MissingField("emoji").to_string(),
        "missing field `emoji`"
    );
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
        assert_eq!(EngagementError::from(error).to_string(), message);
    }
}

#[test]
fn command_permission_errors_keep_their_own_message() {
    let cases = [
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
        assert_eq!(EngagementError::from(error).to_string(), message);
    }
}

#[test]
fn greeting_errors_read_as_the_bot_words_them() {
    let cases = [
        (
            GreetingsError::InvalidUrl("ftp://x".to_owned()),
            "`ftp://x` isn't a usable image link. Links must start with `https://`.",
        ),
        (
            GreetingsError::TooManyImages(50),
            "This server already has the maximum of 50 images for that greeting. \
             Remove one before adding another.",
        ),
        (GreetingsError::DuplicateImage, "That image link is already in the list."),
        (
            GreetingsError::UnknownKind("noon".to_owned()),
            "Unknown greeting type `noon`.",
        ),
        (
            GreetingsError::MessageTooLong(1500),
            "Greeting messages are limited to 1500 characters so the reply still \
             fits once mentions are filled in.",
        ),
        (
            GreetingsError::InvalidCooldown("abc".to_owned()),
            "`abc` isn't a usable cooldown. Enter a whole number of seconds \
             between 0 and 86400.",
        ),
    ];

    for (error, message) in cases {
        assert_eq!(EngagementError::from(error).to_string(), message);
    }
}

#[test]
fn reaction_role_errors_read_as_the_bot_words_them() {
    assert_eq!(
        EngagementError::from(ReactionRoleError::UnsupportedEmoji("x".to_owned()))
            .to_string(),
        "Unsupported emoji: x"
    );
}

#[test]
fn pages_render_errors_with_the_server_error_prefix() {
    assert_eq!(
        server_error_text(EngagementError::ImageNotFound),
        "error running server function: that image is not in this server's list"
    );
}

#[test]
fn only_refusals_count_as_denied() {
    assert!(EngagementError::from(AuthError::Unauthenticated).is_denied());
    assert!(EngagementError::from(AuthError::Forbidden).is_denied());
    assert!(
        EngagementError::from(GuildError::from(AuthError::Forbidden)).is_denied()
    );
    assert!(!EngagementError::from(AuthError::InvalidGuildId).is_denied());
    assert!(!EngagementError::ImageNotFound.is_denied());
}

#[test]
fn only_form_shape_errors_are_invalid_forms() {
    assert!(EngagementError::UnknownField("x".to_owned()).is_invalid_form());
    assert!(EngagementError::DuplicateField("x".to_owned()).is_invalid_form());
    assert!(EngagementError::MissingField("x").is_invalid_form());
    assert!(EngagementError::GuildMismatch.is_invalid_form());
    assert!(!EngagementError::Invalid("channel").is_invalid_form());
    assert!(!EngagementError::from(AuthError::Forbidden).is_invalid_form());
}

#[test]
fn a_signed_out_caller_is_redirected_instead_of_shown_an_error() {
    assert!(
        EngagementError::from(AuthError::Unauthenticated)
            .redirect_unauthenticated()
            .is_err()
    );
    assert!(
        EngagementError::from(GuildError::from(AuthError::Unauthenticated))
            .redirect_unauthenticated()
            .is_err()
    );
}

#[test]
fn any_other_error_comes_back_to_be_rendered() {
    assert_eq!(
        EngagementError::from(AuthError::Forbidden)
            .redirect_unauthenticated()
            .unwrap(),
        EngagementError::Auth(AuthError::Forbidden)
    );
    assert_eq!(
        EngagementError::ImageNotFound.redirect_unauthenticated().unwrap(),
        EngagementError::ImageNotFound
    );
}
