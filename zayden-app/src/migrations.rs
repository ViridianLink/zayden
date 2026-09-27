use sqlx::migrate::Migrator;

#[must_use]
pub fn migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("../migrations");
    migrator.set_ignore_missing(true);
    migrator
}
