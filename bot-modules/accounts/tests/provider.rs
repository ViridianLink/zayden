use accounts::AccountsError;
use accounts::provider::Provider;

#[test]
fn steam_names_round_trip() {
    assert_eq!(Provider::Steam.as_str(), "steam");
    assert_eq!(Provider::Steam.label(), "Steam");
    assert_eq!(Provider::Steam.to_string(), "Steam");
    assert_eq!("steam".parse::<Provider>().unwrap(), Provider::Steam);
    assert_eq!(Provider::ALL, [Provider::Steam]);
}

#[test]
fn unknown_names_are_rejected_exactly() {
    for name in ["Steam", "STEAM", " steam", "steam ", "epic", ""] {
        let err = name.parse::<Provider>().unwrap_err();
        assert!(
            matches!(&err, AccountsError::UnknownProvider(s) if s == name),
            "{name:?}: {err:?}"
        );
    }
}
