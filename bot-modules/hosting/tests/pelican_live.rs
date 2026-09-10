//! Live integration against the real panel. Ignored by default: it creates and
//! destroys a real server, so it needs credentials and an empty node.
//!
//! ```text
//! doppler run -- cargo test -p hosting --test pelican_live -- --ignored
//! ```
//! Requires `PELICAN_BASE_URL`, `PELICAN_API_KEY` and `HOSTING_TEST_EGG_ID`.

use std::collections::BTreeMap;
use std::env;
use std::time::Duration;

use hosting::pelican::PelicanApp;
use hosting::pricing::Plan;

fn client() -> Option<PelicanApp> {
    let base = env::var("PELICAN_BASE_URL").ok()?;
    let key = env::var("PELICAN_API_KEY").ok()?;

    Some(PelicanApp::new(reqwest::Client::new(), &base, &key))
}

fn egg_id() -> Option<i32> {
    env::var("HOSTING_TEST_EGG_ID").ok()?.parse().ok()
}

#[tokio::test]
#[ignore = "creates a real server on the live panel"]
async fn a_small_server_provisions_and_then_releases_its_allocation() {
    let (Some(client), Some(egg_id)) = (client(), egg_id()) else {
        // Nothing to assert without credentials; the harness reports the skip
        // by way of this test being ignored in the first place.
        return;
    };

    let egg = client.egg(egg_id).await;
    assert!(egg.is_ok(), "the configured egg id does not resolve on the panel");

    let Some(egg) = egg.ok() else {
        return;
    };

    let env: BTreeMap<String, String> = egg.default_env().into_iter().collect();
    let plan = Plan::Small;

    let owner = env::var("HOSTING_TEST_USER_ID")
        .ok()
        .and_then(|id| id.parse().ok())
        .unwrap_or(1);

    let created = client
        .create_server(
            "zayden-hosting-live-test",
            owner,
            &egg,
            env,
            plan.memory_mib(),
            plan.cpu_percent(),
            plan.disk_mib(),
            &[],
        )
        .await
        .ok();

    assert!(created.is_some(), "the panel refused to create the test server");

    let Some(created) = created else {
        return;
    };

    let installed = client
        .await_install(created.id, Duration::from_secs(5), Duration::from_secs(600))
        .await
        .ok();

    assert!(
        installed.and_then(|s| s.primary_address()).is_some(),
        "a provisioned server must advertise a reachable address"
    );

    assert!(
        client.delete_server(created.id).await.is_ok(),
        "the test must not leave an allocation behind"
    );
}
