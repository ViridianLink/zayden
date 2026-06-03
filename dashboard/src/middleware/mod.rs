pub mod auth;
pub mod guild_permission;
pub mod tier;

pub(crate) fn guild_id_from_path(path: &str) -> Option<u64> {
    // All guild-scoped routes have the form /guild/{id}[/...].
    path.split('/').nth(2)?.parse().ok()
}
