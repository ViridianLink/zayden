//! Unit coverage for the `WeaponStat` key enum used by the `MarathonDB` weapon
//! parser (`parse/marathondb/weapon.rs`).

use marathon::parse::WeaponStat;

#[test]
fn parses_every_known_stat_key() {
    assert_eq!("damage".parse(), Ok(WeaponStat::Damage));
    assert_eq!("rate_of_fire".parse(), Ok(WeaponStat::RateOfFire));
    assert_eq!("magazine_size".parse(), Ok(WeaponStat::MagazineSize));
    assert_eq!("reload_speed".parse(), Ok(WeaponStat::ReloadSpeed));
    assert_eq!("range_meters".parse(), Ok(WeaponStat::Range));
}

#[test]
fn rejects_unknown_and_mismatched_keys() {
    // Keys that are collected into `Weapon::stats` but have no dedicated field.
    assert_eq!("handling".parse::<WeaponStat>(), Err(()));
    assert_eq!("range".parse::<WeaponStat>(), Err(())); // must be `range_meters`
    assert_eq!("".parse::<WeaponStat>(), Err(()));
    assert_eq!("Damage".parse::<WeaponStat>(), Err(())); // case-sensitive
}
