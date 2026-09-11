use jellyfin::transport::http::encode_query;

const RESERVED: &str = ":/?#[]@!$&'()*+,;=";

#[test]
fn spaces_become_percent_twenty_not_plus() {
    assert_eq!(
        encode_query(&[("query", "the matrix"), ("page", "1")]),
        "query=the%20matrix&page=1"
    );
}

#[test]
fn reserved_characters_never_survive_encoding() {
    let encoded = encode_query(&[("query", RESERVED)]);
    let value = encoded.strip_prefix("query=").unwrap();

    assert!(!value.chars().any(|c| RESERVED.contains(c)), "{value}");
}

#[test]
fn unreserved_characters_pass_through() {
    assert_eq!(encode_query(&[("q", "abcXYZ019-._~")]), "q=abcXYZ019-._~");
}

#[test]
fn non_ascii_is_percent_encoded_as_utf8() {
    assert_eq!(encode_query(&[("q", "Amélie")]), "q=Am%C3%A9lie");
}

#[test]
fn empty_params_produce_an_empty_string() {
    assert_eq!(encode_query(&[]), "");
}
