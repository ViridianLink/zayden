use music::permissions::{can_manage_track, is_privileged, vote_threshold};
use serenity::all::{Permissions, RoleId, UserId};

#[test]
fn no_dj_role_means_everyone_privileged() {
    assert!(is_privileged(&[], None, None));
}

#[test]
fn manage_guild_is_always_privileged() {
    assert!(is_privileged(&[], Some(Permissions::MANAGE_GUILD), Some(1)));
}

#[test]
fn dj_role_required_when_configured() {
    let dj = RoleId::new(42);
    assert!(is_privileged(&[dj], None, Some(42)));
    assert!(!is_privileged(&[RoleId::new(1)], None, Some(42)));
}

#[test]
fn requester_can_always_manage_own_track() {
    let user = UserId::new(1);
    assert!(can_manage_track(false, user, user));
    assert!(!can_manage_track(false, user, UserId::new(2)));
    assert!(can_manage_track(true, user, UserId::new(2)));
}

#[test]
fn vote_threshold_rounds_up_and_has_floor_of_one() {
    assert_eq!(vote_threshold(0), 1);
    assert_eq!(vote_threshold(1), 1);
    assert_eq!(vote_threshold(2), 1);
    assert_eq!(vote_threshold(3), 2);
    assert_eq!(vote_threshold(4), 2);
    assert_eq!(vote_threshold(5), 3);
}
