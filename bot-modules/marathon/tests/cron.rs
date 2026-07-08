use std::sync::Arc;

use marathon::client::MarathonClient;
use marathon::cron::MarathonAnnounceCron;

#[test]
fn schedule_expression_parses() {
    let client = Arc::new(MarathonClient::new(reqwest::Client::new(), None));

    let job = MarathonAnnounceCron::cron_job(client);

    assert!(job.is_ok(), "cron schedule failed to parse: {:?}", job.err());
}
