//! An older binary restarting against a database a newer binary has already
//! migrated must start instead of failing on the unknown version.

use sqlx::PgPool;
use zayden_app::migrations::migrator;

#[sqlx::test(migrations = false)]
async fn a_version_applied_by_a_newer_binary_is_tolerated(pool: PgPool) {
    let migrator = migrator();
    migrator.run(&pool).await.unwrap();

    sqlx::query!(
        "INSERT INTO _sqlx_migrations
             (version, description, success, checksum, execution_time)
         VALUES (99999999, 'applied by a newer binary', TRUE, '\\x00', 0)"
    )
    .execute(&pool)
    .await
    .unwrap();

    migrator.run(&pool).await.unwrap();
}
