//! Locale → IANA timezone defaulting, and the activity catalog.
//!
//! Audit finding lfg #2 (`design-docs/audits/lfg.md`). `UserSettings::get`
//! resolves the mapped name with `tz::db().get(name).unwrap_or(TimeZone::UTC)`,
//! so a typo in the table does not fail — it silently drops that whole locale
//! back to UTC and every scheduled post in it lands hours off. The table is
//! therefore only correct if every entry actually resolves in the tzdb.

use jiff::tz::{self, TimeZone};
use lfg::models::timezone_manager::locale_to_timezone;
use lfg::{ACTIVITIES, ActivityCategory};

/// Every locale Discord can send, per the mapping's own arms.
const MAPPED_LOCALES: [&str; 31] = [
    "id", "da", "de", "en-GB", "es-ES", "es-419", "fr", "hr", "it", "lt", "hu",
    "nl", "no", "pl", "pt-BR", "ro", "fi", "sv-SE", "vi", "tr", "cs", "el", "bg",
    "ru", "uk", "hi", "th", "zh-CN", "ja", "zh-TW", "ko",
];

#[test]
fn every_mapped_timezone_resolves_in_the_tzdb() {
    for locale in MAPPED_LOCALES {
        let name = locale_to_timezone(locale);
        assert!(
            tz::db().get(name).is_ok(),
            "locale `{locale}` maps to `{name}`, which the tzdb cannot resolve \
             — it would silently fall back to UTC",
        );
    }
}

#[test]
fn no_mapped_locale_silently_degrades_to_utc() {
    // UTC is the *fallback*; no explicit arm should land on it, or the arm is
    // indistinguishable from an unmapped locale.
    for locale in MAPPED_LOCALES {
        assert_ne!(
            locale_to_timezone(locale),
            "UTC",
            "locale `{locale}` is mapped but resolves to the fallback",
        );
    }
}

#[test]
fn unknown_locales_fall_back_to_utc() {
    for locale in ["", "en-US", "xx-YY", "klingon"] {
        assert_eq!(locale_to_timezone(locale), "UTC");
    }
}

#[test]
fn fallback_is_itself_resolvable() {
    assert_eq!(
        tz::db().get(locale_to_timezone("en-US")).unwrap_or(TimeZone::UTC),
        TimeZone::UTC,
    );
}

#[test]
fn locale_mapping_is_stable_for_known_regions() {
    assert_eq!(locale_to_timezone("en-GB"), "Europe/London");
    assert_eq!(locale_to_timezone("ja"), "Asia/Tokyo");
    assert_eq!(locale_to_timezone("pt-BR"), "America/Sao_Paulo");
}

#[test]
fn every_activity_has_a_usable_fireteam_size() {
    // `fireteam_size` seeds the post's capacity; a zero or negative default
    // would make the post full on creation.
    for activity in &ACTIVITIES {
        assert!(
            activity.fireteam_size > 0,
            "`{}` has a non-positive fireteam size",
            activity.name,
        );
        assert!(
            activity.fireteam_size <= 6,
            "`{}` exceeds Destiny's six-guardian maximum",
            activity.name,
        );
    }
}

#[test]
fn activity_names_are_unique() {
    // Names are the autocomplete/choice key, so duplicates are unresolvable.
    let mut names = ACTIVITIES.iter().map(|a| a.name).collect::<Vec<_>>();
    let before = names.len();
    names.sort_unstable();
    names.dedup();

    assert_eq!(names.len(), before, "duplicate activity name in the catalog");
}

#[test]
fn raids_and_dungeons_carry_their_canonical_sizes() {
    for activity in &ACTIVITIES {
        match activity.category {
            ActivityCategory::Raid => assert_eq!(
                activity.fireteam_size, 6,
                "raid `{}` is not a six-stack",
                activity.name,
            ),
            ActivityCategory::Dungeon => assert_eq!(
                activity.fireteam_size, 3,
                "dungeon `{}` is not a three-stack",
                activity.name,
            ),
            ActivityCategory::ExoticMission
            | ActivityCategory::Vanguard
            | ActivityCategory::Pvp => {},
        }
    }
}
