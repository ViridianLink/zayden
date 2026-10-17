use std::sync::Arc;

use deadlock::DeadlockApi;
use deadlock::cron::{DeadlockMatchCron, DeadlockPatchCron};
use deadlock_api::DeadlockClient;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn schedule_expression_parses(pool: PgPool) {
    let client = DeadlockClient::builder().build().unwrap();
    let api = Arc::new(DeadlockApi::from_client(client, pool));

    let job = DeadlockPatchCron::cron_job(api, 1);

    assert!(job.is_ok(), "cron schedule failed to parse: {:?}", job.err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn match_poll_schedule_expression_parses(pool: PgPool) {
    let client = DeadlockClient::builder().build().unwrap();
    let api = Arc::new(DeadlockApi::from_client(client, pool));

    let job = DeadlockMatchCron::cron_job(api, 1);

    assert!(job.is_ok(), "cron schedule failed to parse: {:?}", job.err());
}
