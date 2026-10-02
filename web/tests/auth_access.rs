//! Role literals, guild access capabilities and the error text pages render.

use web::auth::{
    AuthError,
    FORBIDDEN,
    ForeignIdError,
    GuildAccess,
    SessionUser,
    UNAUTHENTICATED,
    WebRole,
};
use web::util::server_error_text;

/// `web_user_roles.role` is plain text with no CHECK constraint, so a typo
/// here matches no row and silently denies access. The migrations seed these
/// literals.
#[test]
fn web_roles_map_to_the_literals_stored_in_the_database() {
    assert_eq!(WebRole::Admin.as_str(), "admin");
    assert_eq!(WebRole::Operator.as_str(), "operator");
}

#[test]
fn the_operator_role_is_distinct_from_the_admin_role() {
    assert_ne!(WebRole::Admin.as_str(), WebRole::Operator.as_str());
}

/// Discord only accepts a command-permission overwrite from a bearer token of
/// a member with Manage Server, which an operator viewing a guild lacks.
#[test]
fn only_member_access_may_write_command_permissions() {
    assert!(GuildAccess::Member.can_write_command_permissions());
    assert!(!GuildAccess::Operator.can_write_command_permissions());
}

#[test]
fn error_messages_match_the_rendered_text() {
    assert_eq!(AuthError::Unauthenticated.to_string(), UNAUTHENTICATED);
    assert_eq!(AuthError::Forbidden.to_string(), FORBIDDEN);
    assert_eq!(UNAUTHENTICATED, "unauthenticated");
    assert_eq!(FORBIDDEN, "forbidden");
    assert_eq!(AuthError::InvalidGuildId.to_string(), "invalid guild id");
    assert_eq!(AuthError::BotNotInGuild.to_string(), "Zayden isn't in that server");
    assert_eq!(
        AuthError::from(ForeignIdError::Channel).to_string(),
        "that channel is not in this server"
    );
    assert_eq!(
        AuthError::from(ForeignIdError::Role).to_string(),
        "that role is not in this server"
    );
    assert_eq!(
        AuthError::BotCannotPost {
            bot: "Zayden".to_owned(),
            channel: "announcements".to_owned(),
        }
        .to_string(),
        "Zayden can't post in #announcements: it needs View Channel and Send \
         Messages there. Allow them for the bot's role in the channel's \
         permissions, then save again."
    );
}

#[test]
fn inline_error_text_carries_the_display_prefix() {
    assert_eq!(
        server_error_text(AuthError::Forbidden),
        "error running server function: forbidden"
    );
}

#[test]
fn only_unauthenticated_becomes_a_login_redirect() {
    assert!(AuthError::Unauthenticated.redirect_unauthenticated().is_err());
    assert_eq!(
        AuthError::Forbidden.redirect_unauthenticated().ok(),
        Some(AuthError::Forbidden)
    );
    assert_eq!(
        AuthError::Database("down".to_owned()).redirect_unauthenticated().ok(),
        Some(AuthError::Database("down".to_owned()))
    );
}

#[test]
fn only_refusals_are_denials() {
    assert!(AuthError::Unauthenticated.is_denied());
    assert!(AuthError::Forbidden.is_denied());
    assert!(!AuthError::InvalidGuildId.is_denied());
    assert!(!AuthError::BotNotInGuild.is_denied());
    assert!(!AuthError::Database("down".to_owned()).is_denied());
    assert!(!AuthError::Discord("401".to_owned()).is_denied());
}

fn user(name: &str, avatar: Option<&str>) -> SessionUser {
    SessionUser {
        id: "41".to_owned(),
        name: name.to_owned(),
        avatar: avatar.map(str::to_owned),
    }
}

#[test]
fn the_avatar_url_points_at_the_discord_cdn() {
    assert_eq!(
        user("Zed", Some("abc")).avatar_url().as_deref(),
        Some("https://cdn.discordapp.com/avatars/41/abc.png?size=64")
    );
    assert_eq!(user("Zed", None).avatar_url(), None);
}

#[test]
fn the_initial_falls_back_to_a_hash() {
    assert_eq!(user("Zed", None).initial(), "Z");
    assert_eq!(user("", None).initial(), "#");
}
