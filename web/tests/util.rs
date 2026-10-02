use web::util::{email_hash, hex_encode};

#[test]
fn hex_encode_is_lowercase_and_padded() {
    assert_eq!(hex_encode(&[0x00, 0x0a, 0xff]), "000aff");
}

#[test]
fn email_hash_normalizes_case_and_whitespace() {
    assert_eq!(
        email_hash("  Someone@Example.COM "),
        email_hash("someone@example.com")
    );
    assert_eq!(email_hash("someone@example.com").len(), 64);
}

#[test]
fn server_error_text_keeps_the_established_prefix() {
    assert_eq!(
        web::util::server_error_text("invalid guild id"),
        "error running server function: invalid guild id"
    );
}
