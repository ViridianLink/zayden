//! The `/good` rate limiters and the tier floors that bound them.
//!
//! The floors are the paid boundary: getting one wrong either hands a free
//! server the load profile a Pro server pays for, or refuses a paying one the
//! setting it bought.

use greetings::cooldown::{STATE_TTL, Verdict, verdict};
use greetings::{Cooldowns, GreetingsSettingsRow, parse_cooldown};
use jiff::{SignedDuration, Timestamp};
use zayden_app::entitlement::Tier;

const OFF: Cooldowns = Cooldowns { user_secs: 0, guild_secs: 0 };
const TEN_AND_FIVE: Cooldowns = Cooldowns { user_secs: 10, guild_secs: 5 };

/// Offsets from the epoch. The fallback cannot trigger for the small constants
/// used below, and if it somehow did every offset would collapse to the same
/// instant and the wait assertions would fail rather than pass wrongly.
fn at(secs: i64) -> Timestamp {
    Timestamp::UNIX_EPOCH
        .checked_add(SignedDuration::from_secs(secs))
        .unwrap_or(Timestamp::UNIX_EPOCH)
}

fn at_millis(millis: i64) -> Timestamp {
    Timestamp::UNIX_EPOCH
        .checked_add(SignedDuration::from_millis(millis))
        .unwrap_or(Timestamp::UNIX_EPOCH)
}

// region: verdict

#[test]
fn a_zero_cooldown_never_blocks() {
    assert_eq!(
        verdict(at(100), Some(at(100)), Some(at(100)), OFF),
        Verdict::Allowed,
        "zero means the limiter is off, even against a use in the same instant"
    );
}

#[test]
fn a_first_use_is_always_allowed() {
    assert_eq!(verdict(at(100), None, None, TEN_AND_FIVE), Verdict::Allowed);
}

#[test]
fn a_repeat_inside_the_window_reports_the_remaining_seconds() {
    assert_eq!(
        verdict(at(104), Some(at(100)), None, TEN_AND_FIVE),
        Verdict::UserWait(6)
    );
}

#[test]
fn a_partial_second_rounds_up() {
    // 6.5s left. Reporting 6 sends the user back a beat too early, which reads
    // as the bot lying about its own limit.
    assert_eq!(
        verdict(at_millis(103_500), Some(at(100)), None, TEN_AND_FIVE),
        Verdict::UserWait(7)
    );
}

#[test]
fn the_window_reopens_exactly_on_the_deadline() {
    assert_eq!(
        verdict(at(110), Some(at(100)), None, TEN_AND_FIVE),
        Verdict::Allowed
    );
}

#[test]
fn the_guild_limiter_catches_a_member_who_has_not_used_it_themselves() {
    assert_eq!(
        verdict(at(102), None, Some(at(100)), TEN_AND_FIVE),
        Verdict::GuildWait(3),
        "the server-wide limiter exists precisely for the second person"
    );
}

#[test]
fn the_member_limit_is_reported_ahead_of_the_server_one() {
    // Both are breached. The member limit is the one they can wait out by
    // themselves, so naming the server-wide one would be unactionable advice.
    assert_eq!(
        verdict(at(101), Some(at(100)), Some(at(100)), TEN_AND_FIVE),
        Verdict::UserWait(9)
    );
}

// endregion

// region: tier floors

#[test]
fn each_tier_floor_is_at_or_below_the_one_beneath_it() {
    let free = GreetingsSettingsRow::floors_for(Tier::Free);
    let pro = GreetingsSettingsRow::floors_for(Tier::Pro);
    let ultra = GreetingsSettingsRow::floors_for(Tier::Ultra);

    assert!(pro.user_secs < free.user_secs, "Pro must buy something");
    assert!(pro.guild_secs < free.guild_secs, "Pro must buy something");
    assert!(ultra.user_secs <= pro.user_secs);
    assert!(ultra.guild_secs <= pro.guild_secs);
}

#[test]
fn only_the_top_tier_can_switch_the_limiters_off() {
    assert!(
        GreetingsSettingsRow::floors_for(Tier::Free).guild_secs > 0,
        "a free server must not be able to reach an unmetered /good"
    );
    assert!(GreetingsSettingsRow::floors_for(Tier::Pro).guild_secs > 0);
    assert_eq!(GreetingsSettingsRow::floors_for(Tier::Ultra), OFF);
}

#[test]
fn clamping_raises_below_floor_values_and_leaves_the_rest_alone() {
    let floor = GreetingsSettingsRow::floors_for(Tier::Free);

    assert_eq!(
        Cooldowns { user_secs: 0, guild_secs: 0 }.clamp_to(floor),
        floor,
        "a lapsed subscription has to take the sub-floor setting with it"
    );
    assert_eq!(
        Cooldowns { user_secs: 60, guild_secs: 30 }.clamp_to(floor),
        Cooldowns { user_secs: 60, guild_secs: 30 },
        "a server is always free to be stricter than its floor"
    );
}

#[test]
fn clamping_caps_at_the_column_constraint() {
    let max = GreetingsSettingsRow::MAX_COOLDOWN_SECS;
    let clamped = Cooldowns { user_secs: max + 1, guild_secs: max + 1 }
        .clamp_to(GreetingsSettingsRow::floors_for(Tier::Free));

    assert_eq!(
        clamped,
        Cooldowns { user_secs: max, guild_secs: max },
        "anything above the CHECK constraint would fail the insert instead"
    );
}

// endregion

// region: drift guards

/// `empty()` supplies the row for a guild Postgres has never seen, and the
/// column defaults supply it for one it has. A mismatch means a guild's
/// cooldown silently changes the first time any greetings setting is saved.
#[test]
fn the_column_defaults_match_the_free_floor() {
    let migration =
        include_str!("../../../migrations/0026_greetings_cooldowns.up.sql");
    let free = GreetingsSettingsRow::FREE_FLOORS;

    assert!(
        migration.contains(&format!(
            "user_cooldown_secs integer NOT NULL DEFAULT {}",
            free.user_secs
        )),
        "0026 must default user_cooldown_secs to the free floor",
    );
    assert!(
        migration.contains(&format!(
            "guild_cooldown_secs integer NOT NULL DEFAULT {}",
            free.guild_secs
        )),
        "0026 must default guild_cooldown_secs to the free floor"
    );
}

/// The limiter's state is evicted on a fixed TTL. Were it shorter than the
/// longest configurable cooldown, a long cooldown would quietly stop applying
/// once its entry aged out.
#[test]
fn the_state_ttl_covers_the_longest_cooldown() {
    assert_eq!(
        i64::try_from(STATE_TTL.as_secs()).unwrap_or(-1),
        i64::from(GreetingsSettingsRow::MAX_COOLDOWN_SECS),
    );
}

// endregion

// region: parsing

#[test]
fn a_blank_box_falls_back_to_the_floor_not_to_zero() {
    // Zero is the paid setting. An empty field must not be a back door to it.
    assert_eq!(parse_cooldown("", 15).unwrap(), 15);
    assert_eq!(parse_cooldown("   ", 15).unwrap(), 15);
}

#[test]
fn a_number_is_taken_as_written() {
    assert_eq!(parse_cooldown(" 42 ", 15).unwrap(), 42);
    assert_eq!(
        parse_cooldown("0", 15).unwrap(),
        0,
        "parsing accepts zero; the floor check is what refuses it"
    );
}

#[test]
fn nonsense_and_out_of_range_values_are_refused() {
    assert!(parse_cooldown("soon", 15).is_err());
    assert!(parse_cooldown("-1", 15).is_err());
    assert!(
        parse_cooldown(
            &(GreetingsSettingsRow::MAX_COOLDOWN_SECS + 1).to_string(),
            15
        )
        .is_err(),
        "the CHECK constraint would reject this at the database anyway"
    );
}

// endregion
