use levels::{LeaderboardScope, LevelsCustomId, LevelsError, level_up_xp};

#[test]
fn level_up_xp_matches_curve() {
    // 3*l^2 + 50*l + 100
    assert_eq!(level_up_xp(0), 100);
    assert_eq!(level_up_xp(1), 153);
    assert_eq!(level_up_xp(2), 212);
    assert_eq!(level_up_xp(5), 425);
    assert_eq!(level_up_xp(10), 900);
}

#[test]
fn level_up_xp_is_strictly_increasing() {
    for level in 0..100 {
        assert!(
            level_up_xp(level) < level_up_xp(level + 1),
            "curve must be strictly increasing at level {level}"
        );
    }
}

#[test]
fn custom_id_round_trips() {
    for id in [LevelsCustomId::Previous, LevelsCustomId::User, LevelsCustomId::Next]
    {
        assert_eq!(id.as_str().parse::<LevelsCustomId>().unwrap(), id);
    }
}

#[test]
fn custom_id_as_str_is_namespaced() {
    assert_eq!(LevelsCustomId::Previous.as_str(), "levels_previous");
    assert_eq!(LevelsCustomId::User.as_str(), "levels_user");
    assert_eq!(LevelsCustomId::Next.as_str(), "levels_next");
}

#[test]
fn custom_id_rejects_unknown() {
    let err = "levels_bogus".parse::<LevelsCustomId>().unwrap_err();
    let LevelsError::Internal(msg) = err else {
        panic!("expected Internal error, got {err:?}");
    };
    assert!(msg.contains("levels_bogus"), "message was: {msg}");
}

#[test]
fn leaderboard_scope_footer_tag_round_trips() {
    for scope in [LeaderboardScope::Guild, LeaderboardScope::Global] {
        assert_eq!(
            LeaderboardScope::from_footer_tag(scope.footer_tag()),
            scope,
            "footer tag must round-trip so the pager recovers the right scope"
        );
    }
}

#[test]
fn leaderboard_scope_from_global_flag() {
    assert_eq!(LeaderboardScope::from_global_flag(true), LeaderboardScope::Global);
    assert_eq!(LeaderboardScope::from_global_flag(false), LeaderboardScope::Guild);
}

#[test]
fn leaderboard_scope_unknown_footer_defaults_to_guild() {
    // Legacy leaderboards (pre-scope) carried a footer without a scope tag; the
    // pager must treat those as the server board, never silently as global.
    assert_eq!(
        LeaderboardScope::from_footer_tag("nonsense"),
        LeaderboardScope::Guild
    );
}
