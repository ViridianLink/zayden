//! How settings saves read the strings their forms submit.

use web::guild::GuildError;
use web::guild::parse::{
    opt_str,
    parse_answer_max_tokens,
    parse_answer_temperature,
    parse_archive_secs,
    parse_flag,
    parse_id,
    parse_idle_secs,
    parse_link,
    parse_max_partners,
    parse_max_results,
    parse_optional,
    parse_role,
    parse_user,
    parse_wiki_locale,
    parse_wiki_url,
};

#[test]
fn a_blank_or_unparsable_id_clears_the_setting() {
    assert_eq!(parse_id(" 42 "), Some(42));
    assert_eq!(parse_id(""), None);
    assert_eq!(parse_id("   "), None);
    assert_eq!(parse_id("abc"), None);
    assert_eq!(parse_id("-5"), Some(-5));
}

#[test]
fn optional_text_is_trimmed_and_blank_is_none() {
    assert_eq!(parse_optional("  key "), Some("key".to_owned()));
    assert_eq!(parse_optional(" "), None);
}

#[test]
fn only_true_switches_on() {
    assert!(parse_flag("true"));
    assert!(parse_flag(" true "));
    assert!(!parse_flag("True"));
    assert!(!parse_flag("on"));
    assert!(!parse_flag(""));
}

#[test]
fn stored_ids_render_as_strings() {
    assert_eq!(opt_str(Some(9)), Some("9".to_owned()));
    assert_eq!(opt_str(None), None);
}

#[test]
fn the_wiki_url_loses_its_trailing_slash() {
    assert_eq!(parse_wiki_url(""), Ok(None));
    assert_eq!(
        parse_wiki_url(" https://wiki.example.com/ "),
        Ok(Some("https://wiki.example.com".to_owned()))
    );
    assert_eq!(
        parse_wiki_url("http://wiki.example.com/docs/"),
        Ok(Some("http://wiki.example.com/docs".to_owned()))
    );
}

#[test]
fn the_wiki_url_must_be_http_or_https() {
    assert_eq!(parse_wiki_url("not a url"), Err(GuildError::InvalidWikiUrl));
    assert_eq!(
        parse_wiki_url("ftp://wiki.example.com"),
        Err(GuildError::WikiUrlScheme)
    );
    assert_eq!(GuildError::InvalidWikiUrl.to_string(), "invalid wiki URL");
    assert_eq!(
        GuildError::WikiUrlScheme.to_string(),
        "the wiki URL must start with http:// or https://"
    );
}

#[test]
fn roles_and_users_must_be_snowflakes() {
    assert_eq!(parse_role(" 5 "), Ok(5));
    assert_eq!(parse_role("x").unwrap_err().to_string(), "invalid role");
    assert_eq!(parse_role("-1").unwrap_err().to_string(), "invalid role");
    assert_eq!(parse_role(&u64::MAX.to_string()), Err(GuildError::InvalidRole));

    assert_eq!(parse_user("6"), Ok(6));
    assert_eq!(parse_user("").unwrap_err().to_string(), "invalid user id");
    assert_eq!(parse_user(&u64::MAX.to_string()), Err(GuildError::InvalidUserId));
}

#[test]
fn a_helper_link_is_normalized() {
    assert_eq!(
        parse_link(" https://example.com "),
        Ok("https://example.com/".to_owned())
    );
}

#[test]
fn a_helper_link_is_refused_with_the_reason() {
    assert_eq!(
        parse_link("example.com").unwrap_err().to_string(),
        "invalid link: relative URL without a base"
    );
    assert_eq!(
        parse_link("ftp://example.com").unwrap_err().to_string(),
        "link must be an http:// or https:// address"
    );
    assert_eq!(
        parse_link("https://user:pw@example.com").unwrap_err().to_string(),
        "link must not embed credentials"
    );
    assert_eq!(
        parse_link("https://user@example.com").unwrap_err(),
        GuildError::LinkCredentials
    );
}

#[test]
fn a_helper_link_is_at_most_200_characters_once_normalized() {
    let base = "https://example.com/";
    let fits = format!("{base}{}", "a".repeat(200 - base.len()));
    let over = format!("{fits}a");

    assert_eq!(parse_link(&fits), Ok(fits.clone()));
    assert_eq!(parse_link(&over).unwrap_err().to_string(), "link is too long");
}

#[test]
fn a_negative_archive_delay_never_archives() {
    assert_eq!(parse_archive_secs("-5"), -1);
    assert_eq!(parse_archive_secs("0"), 0);
    assert_eq!(parse_archive_secs(" 3600 "), 3600);
    assert_eq!(parse_archive_secs("soon"), 60);
}

#[test]
fn idle_delays_clamp_between_an_hour_and_a_month() {
    assert_eq!(parse_idle_secs("", 172_800), 172_800);
    assert_eq!(parse_idle_secs("10", 172_800), 3_600);
    assert_eq!(parse_idle_secs("99999999", 86_400), 2_592_000);
    assert_eq!(parse_idle_secs("7200", 86_400), 7_200);
}

#[test]
fn max_partners_is_at_least_one() {
    assert_eq!(parse_max_partners(""), 1);
    assert_eq!(parse_max_partners("0"), 1);
    assert_eq!(parse_max_partners(" 4 "), 4);
}

#[test]
fn the_wiki_locale_defaults_to_english() {
    assert_eq!(parse_wiki_locale(" "), "en");
    assert_eq!(parse_wiki_locale(" de "), "de");
}

#[test]
fn faq_tuning_defaults_and_clamps() {
    assert_eq!(parse_max_results(""), 5);
    assert_eq!(parse_max_results("0"), 1);
    assert_eq!(parse_max_results("99"), 25);

    assert_eq!(parse_answer_max_tokens("x"), 500);
    assert_eq!(parse_answer_max_tokens("1"), 64);
    assert_eq!(parse_answer_max_tokens("10000"), 4096);

    assert!((parse_answer_temperature("") - 0.2).abs() < f32::EPSILON);
    assert!((parse_answer_temperature("-1") - 0.0).abs() < f32::EPSILON);
    assert!((parse_answer_temperature("3.5") - 2.0).abs() < f32::EPSILON);
    assert!((parse_answer_temperature(" 0.7 ") - 0.7).abs() < f32::EPSILON);
}
