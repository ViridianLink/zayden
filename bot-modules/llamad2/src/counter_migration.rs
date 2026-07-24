//! TEMPORARY one-time data migration.
//!
//! The counter state used to live in on-disk JSON files (`countingFails.json`,
//! `dumbCount.json`) written from the process CWD. Finding llamad2 #1/#2 moved
//! the counters into the `llamad2_counters` table, so on the first deploy of the
//! new code the accumulated on-disk counts must be seeded into the DB before the
//! files are discarded — otherwise the counters silently reset to zero.
//!
//! This module reads any surviving legacy file, seeds the DB with `GREATEST`
//! (so a re-run, or a DB value that has already advanced, can never *lower* the
//! count), and deletes the file only on a successful seed. It is resilient: any
//! error is logged and the file is left in place to retry on the next startup;
//! it never aborts boot.
//!
//! Once this has run in production, delete this module, its `mod`/`pub use` in
//! `lib.rs`, the `migrate_json_counters` call in `bot/src/main.rs`, and the
//! `serde_json` dependency from `Cargo.toml`.

use std::path::Path;

use sqlx::PgPool;
use tracing::{error, info};

/// `(file name, JSON key, DB counter name)` for each legacy counter.
const LEGACY_COUNTERS: [(&str, &str, &str); 2] = [
    ("countingFails.json", "countingFails", "counting_fails"),
    ("dumbCount.json", "dumbCount", "dumb_count"),
];

/// Seed `llamad2_counters` from any legacy JSON counter files, then delete them.
pub async fn migrate_json_counters(pool: &PgPool) {
    for (file, key, counter) in LEGACY_COUNTERS {
        let path = Path::new(file);
        if !path.exists() {
            continue;
        }

        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(e) => {
                error!("counter migration: failed to read {file}: {e}");
                continue;
            },
        };

        let value = match serde_json::from_str::<serde_json::Value>(&contents) {
            Ok(value) => value,
            Err(e) => {
                error!("counter migration: failed to parse {file}: {e}");
                continue;
            },
        };

        let Some(count) = value.get(key).and_then(serde_json::Value::as_i64)
        else {
            error!(
                "counter migration: {file} missing integer key '{key}'; leaving file in place"
            );
            continue;
        };

        // GREATEST so re-running (or a DB value that already advanced) never lowers the count.
        let seeded = sqlx::query!(
            "INSERT INTO llamad2_counters (name, count)
                 VALUES ($1, $2)
             ON CONFLICT (name)
                 DO UPDATE SET count = GREATEST(llamad2_counters.count, EXCLUDED.count)",
            counter,
            count,
        )
        .execute(pool)
        .await;

        if let Err(e) = seeded {
            error!(
                "counter migration: failed to seed {counter} from {file}: {e}; leaving file in place to retry"
            );
            continue;
        }

        match std::fs::remove_file(path) {
            Ok(()) => info!(
                "counter migration: seeded {counter}={count} from {file} and deleted the file"
            ),
            Err(e) => error!(
                "counter migration: seeded {counter}={count} but failed to delete {file}: {e}"
            ),
        }
    }
}
