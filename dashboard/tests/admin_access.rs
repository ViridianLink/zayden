#![cfg(feature = "ssr")]
//! The loadout builder is gated on `web_user_roles.role = 'admin'`, a plain
//! text column; the literal must match what migration 0021 seeds.

use dashboard::server::auth::WebRole;

#[test]
fn the_loadout_builder_role_is_the_seeded_admin_literal() {
    assert_eq!(WebRole::Admin.as_str(), "admin");
}
