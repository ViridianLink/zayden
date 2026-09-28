use secrecy::{ExposeSecret, SecretString};
use zayden_app::custom_bots::{CustomBotError, Keyring};

const KEY_1: &str = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=";
const KEY_2: &str = "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=";
const APPLICATION: i64 = 1_385_991_157_762_359_417;
const TOKEN: &str = "not.a-real.token";

#[expect(
    clippy::unwrap_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
fn keyring(active: &str) -> Keyring {
    Keyring::parse(&format!("1:{KEY_1}, 2:{KEY_2}"), active).unwrap()
}

#[test]
fn a_sealed_token_opens_to_the_original() {
    let keys = keyring("1");
    let sealed =
        keys.seal(APPLICATION, &SecretString::from(TOKEN.to_owned())).unwrap();

    assert_eq!(sealed.key_id, 1);
    assert_eq!(keys.open(APPLICATION, &sealed).unwrap().expose_secret(), TOKEN);
}

#[test]
fn sealing_twice_uses_fresh_nonces() {
    let keys = keyring("1");
    let token = SecretString::from(TOKEN.to_owned());

    let first = keys.seal(APPLICATION, &token).unwrap();
    let second = keys.seal(APPLICATION, &token).unwrap();

    assert_ne!(first.nonce, second.nonce);
    assert_ne!(first.ciphertext, second.ciphertext);
}

#[test]
fn a_token_moved_to_another_application_fails_to_open() {
    let keys = keyring("1");
    let sealed =
        keys.seal(APPLICATION, &SecretString::from(TOKEN.to_owned())).unwrap();

    assert!(matches!(
        keys.open(APPLICATION + 1, &sealed),
        Err(CustomBotError::Decrypt(_))
    ));
}

#[test]
fn a_tampered_ciphertext_fails_to_open() {
    let keys = keyring("1");
    let mut sealed =
        keys.seal(APPLICATION, &SecretString::from(TOKEN.to_owned())).unwrap();
    if let Some(byte) = sealed.ciphertext.first_mut() {
        *byte ^= 1;
    }

    assert!(matches!(
        keys.open(APPLICATION, &sealed),
        Err(CustomBotError::Decrypt(_))
    ));
}

#[test]
fn a_token_sealed_under_a_retired_key_still_opens_after_rotation() {
    let sealed = keyring("1")
        .seal(APPLICATION, &SecretString::from(TOKEN.to_owned()))
        .unwrap();

    let rotated = keyring("2");

    assert_eq!(rotated.active_key_id(), 2);
    assert_eq!(rotated.open(APPLICATION, &sealed).unwrap().expose_secret(), TOKEN);
}

#[test]
fn a_token_sealed_under_an_unloaded_key_is_refused() {
    let sealed = keyring("2")
        .seal(APPLICATION, &SecretString::from(TOKEN.to_owned()))
        .unwrap();

    let only_first = Keyring::parse(&format!("1:{KEY_1}"), "1").unwrap();

    assert!(matches!(
        only_first.open(APPLICATION, &sealed),
        Err(CustomBotError::UnknownKey(2))
    ));
}

#[test]
fn a_different_key_under_the_same_id_fails_to_open() {
    let sealed = Keyring::parse(&format!("1:{KEY_1}"), "1")
        .unwrap()
        .seal(APPLICATION, &SecretString::from(TOKEN.to_owned()))
        .unwrap();

    let swapped = Keyring::parse(&format!("1:{KEY_2}"), "1").unwrap();

    assert!(matches!(
        swapped.open(APPLICATION, &sealed),
        Err(CustomBotError::Decrypt(_))
    ));
}

#[test]
fn an_active_key_that_is_not_loaded_is_rejected() {
    assert!(matches!(
        Keyring::parse(&format!("1:{KEY_1}"), "3"),
        Err(CustomBotError::UnknownKey(3))
    ));
}

#[test]
fn a_short_key_is_rejected() {
    assert!(matches!(
        Keyring::parse("1:AQID", "1"),
        Err(CustomBotError::KeyLength { id: 1, .. })
    ));
}

#[test]
fn a_duplicated_key_id_is_rejected() {
    assert!(matches!(
        Keyring::parse(&format!("1:{KEY_1},1:{KEY_2}"), "1"),
        Err(CustomBotError::InvalidKeySpec(_))
    ));
}

#[test]
fn debug_output_never_contains_secrets() {
    let token = SecretString::from(TOKEN.to_owned());
    let keys = keyring("1");

    let printed = format!("{token:?} {keys:?}");

    assert!(!printed.contains(TOKEN));
    assert!(!printed.contains(KEY_1));
    assert!(!printed.contains(KEY_2));
}
