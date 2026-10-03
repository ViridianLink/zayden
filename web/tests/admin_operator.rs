//! The operator's "go to server ID" box and the role literals that gate the
//! operator and loadout pages.

use web::admin::parse_guild_id;
use web::auth::WebRole;

#[test]
fn the_loadout_builder_role_is_the_seeded_admin_literal() {
    assert_eq!(WebRole::Admin.as_str(), "admin");
}

#[test]
fn the_server_list_role_is_the_seeded_operator_literal() {
    assert_eq!(WebRole::Operator.as_str(), "operator");
}

#[test]
fn a_snowflake_is_accepted_as_a_server_id() {
    assert_eq!(parse_guild_id("98765432109876543"), Some(98_765_432_109_876_543));
    assert_eq!(
        parse_guild_id("  98765432109876543  "),
        Some(98_765_432_109_876_543)
    );
}

/// Anything else would navigate to a guild page that cannot load.
#[test]
fn non_snowflakes_are_rejected_as_server_ids() {
    assert_eq!(parse_guild_id(""), None);
    assert_eq!(parse_guild_id("   "), None);
    assert_eq!(parse_guild_id("0"), None);
    assert_eq!(parse_guild_id("not-an-id"), None);
    assert_eq!(parse_guild_id("123abc"), None);
    assert_eq!(parse_guild_id("-1"), None);
    assert_eq!(parse_guild_id("+5"), None);
    assert_eq!(parse_guild_id("https://discord.com/channels/123"), None);
    assert_eq!(parse_guild_id("99999999999999999999999"), None);
}
