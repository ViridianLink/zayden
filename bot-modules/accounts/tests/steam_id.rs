use accounts::AccountsError;
use accounts::steam::id::STEAM_ID64_BASE;
use accounts::steam::{AccountId, SteamId64};

const SAMPLE: u64 = 76_561_197_960_287_930;

#[test]
fn a_steam_id_converts_to_its_account_id_and_back() {
    let id = SteamId64::new(SAMPLE).unwrap();

    assert_eq!(id.account_id(), AccountId::new(22_202));
    assert_eq!(id.account_id().steam_id64(), id);
    assert_eq!(id.account_id().to_string(), "[U:1:22202]");
    assert_eq!(id.to_string(), "76561197960287930");
    assert_eq!(id.get(), SAMPLE);
}

#[test]
fn ids_outside_the_individual_range_are_rejected() {
    assert!(SteamId64::new(STEAM_ID64_BASE - 1).is_none());
    assert!(SteamId64::new(STEAM_ID64_BASE + u64::from(u32::MAX) + 1).is_none());
    assert!(SteamId64::new(STEAM_ID64_BASE).is_some());
    assert!(SteamId64::new(STEAM_ID64_BASE + u64::from(u32::MAX)).is_some());
}

#[test]
fn claimed_ids_must_have_the_exact_steam_form() {
    let prefix = "https://steamcommunity.com/openid/id/";

    assert_eq!(
        SteamId64::from_claimed_id(&format!("{prefix}{SAMPLE}")),
        SteamId64::new(SAMPLE)
    );

    for claimed in [
        format!("http://steamcommunity.com/openid/id/{SAMPLE}"),
        format!("{prefix}{SAMPLE}/"),
        format!("{prefix}{}", SAMPLE / 10),
        format!("{prefix}{SAMPLE}0"),
        format!("{prefix}{SAMPLE}/extra"),
        format!("{prefix}7656119796028793a"),
        format!("{prefix}+7656119796028793"),
        prefix.to_owned(),
        SAMPLE.to_string(),
    ] {
        assert!(SteamId64::from_claimed_id(&claimed).is_none(), "{claimed}");
    }
}

#[test]
fn parsing_a_stored_id_needs_seventeen_digits_in_range() {
    assert_eq!(
        "76561197960287930".parse::<SteamId64>().unwrap(),
        SteamId64::new(SAMPLE).unwrap()
    );

    for raw in ["", "1234", "7656119796028793x", "76561197960265727"] {
        let err = raw.parse::<SteamId64>().unwrap_err();
        assert!(
            matches!(err, AccountsError::InvalidAssertion("steam id")),
            "{raw:?}: {err:?}"
        );
    }
}
