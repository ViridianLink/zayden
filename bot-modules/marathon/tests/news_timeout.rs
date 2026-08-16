//! Regression tests for the marathon news poll hanging on a wedged upstream.
//!
//! Reported failure: two consecutive 30-minute polls logged
//! `Reqwest(... url: "https://public.api.bsky.app/...getAuthorFeed", source:
//! TimedOut)`. The request carried no budget of its own, so it rode the shared
//! client's 30s *total-request* cap, got one shot at the upstream, and burned
//! the full 30s before failing — once per feed, in series, inside a cron job
//! `bot::cron::run_cron_jobs_loop` awaits inline. Four wedged feeds therefore
//! held the entire scheduler for two minutes.
//!
//! The tests below pin the two halves of the fix: a per-request budget tighter
//! than the client cap, and retries that only spend themselves on failures
//! worth retrying.

use std::future::pending;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use marathon::error::MarathonError;
use marathon::news::{self, BLUESKY_FEED_URL, BlueskyFeed};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use zayden_app::services::http::{ClientBuilderExt, HTTP_TIMEOUT};

/// The budgets the socket tests run `fetch_json_with` at.
///
/// **These tests must not use a paused clock.** They drive a real `reqwest`
/// client over a real loopback socket, and under `start_paused` tokio
/// auto-advances virtual time whenever the runtime goes idle — which is exactly
/// what it does while the OS completes the TCP handshake, so the budget would
/// elapse in virtual time before the connection finished in real time. The
/// clock is real and the budgets are scaled down to milliseconds instead, which
/// is the same trade `music`'s resolver timeout suite makes.
const TEST_TIMEOUT: Duration = Duration::from_millis(300);
const TEST_RETRY: zayden_core::RetryBudget =
    zayden_core::RetryBudget::new(3, Duration::from_millis(10));

fn http_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: \
         {len}\r\nconnection: close\r\n\r\n{body}",
        len = body.len()
    )
}

/// Reads one request off the socket and returns its start line.
async fn read_request_line(socket: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];

    loop {
        let n = match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let Some(received) = chunk.get(..n) else { break };
        buf.extend_from_slice(received);

        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }

    String::from_utf8_lossy(&buf).lines().next().unwrap_or_default().to_string()
}

struct MockUpstream {
    url: String,
    requests: Arc<Mutex<Vec<String>>>,
}

impl MockUpstream {
    /// The start line of every request the upstream has served, in order — the
    /// attempt counter the retry assertions read.
    fn request_lines(&self) -> Vec<String> {
        self.requests.lock().map(|requests| requests.clone()).unwrap_or_default()
    }
}

/// Serves `script[n]` to the n-th request, holding at the last entry once the
/// script runs out. A `None` entry reads the request and then answers nothing,
/// ever, without hanging up — the wedged upstream a request budget exists to
/// defend against.
async fn spawn_scripted_upstream(
    script: Vec<Option<String>>,
) -> Option<MockUpstream> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;

    let requests = Arc::new(Mutex::new(Vec::new()));
    let served = Arc::new(AtomicUsize::new(0));
    let script = Arc::new(script);

    let accepted = Arc::clone(&requests);
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else { break };

            let script = Arc::clone(&script);
            let served = Arc::clone(&served);
            let accepted = Arc::clone(&accepted);

            tokio::spawn(async move {
                let request_line = read_request_line(&mut socket).await;
                if let Ok(mut requests) = accepted.lock() {
                    requests.push(request_line);
                }

                let n = served.fetch_add(1, Ordering::SeqCst);
                let last = script.len().saturating_sub(1);
                let Some(entry) = script.get(n.min(last)) else { return };
                let Some(response) = entry.as_ref() else {
                    // Hold the connection open, silently, forever.
                    pending::<()>().await;
                    return;
                };

                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    Some(MockUpstream { url: format!("http://{addr}"), requests })
}

/// The production client the poll actually runs on: its 30s total-request cap
/// is the backstop the per-request budget has to beat.
fn shared_client() -> Option<reqwest::Client> {
    reqwest::Client::builder().with_timeouts().build().ok()
}

/// Fails-before: the poll carried no request budget, so it rode the shared
/// client's 30s cap, tried exactly once, and stalled the cron loop for the full
/// 30s per wedged feed.
#[tokio::test]
async fn a_news_poll_gives_up_on_a_wedged_upstream_and_retries_it() {
    let upstream =
        spawn_scripted_upstream(vec![None]).await.expect("start the mock upstream");
    let client = shared_client().expect("build the production client");

    let started = Instant::now();
    let error = news::fetch_json_with(
        || client.get(&upstream.url),
        TEST_TIMEOUT,
        TEST_RETRY,
    )
    .await
    .expect_err("a silent upstream must surface as an error");
    let elapsed = started.elapsed();

    let MarathonError::Reqwest(e) = &error else {
        panic!("expected a reqwest error, got {error:?}")
    };
    assert!(e.is_timeout(), "expected a timeout, got {e:?}");
    assert!(
        !e.is_connect(),
        "the upstream accepted the connection, so this must be the request \
         budget firing rather than the connect budget: {e:?}",
    );

    assert!(
        elapsed < HTTP_TIMEOUT,
        "the poll waited {elapsed:?}, which means it rode the shared client's \
         {HTTP_TIMEOUT:?} cap instead of its own budget",
    );

    let attempts = upstream.request_lines().len();
    assert_eq!(
        u32::try_from(attempts).expect("attempt count fits a u32"),
        TEST_RETRY.attempts,
        "a wedged upstream must be retried up to the budget, not abandoned \
         after one shot",
    );
}

/// A 5xx is the upstream saying it is having a bad minute, which is precisely
/// what the retry budget is for.
#[tokio::test]
async fn a_news_poll_retries_a_transient_upstream_failure() {
    let upstream = spawn_scripted_upstream(vec![
        Some(http_response("503 Service Unavailable", "{}")),
        Some(http_response("200 OK", r#"{"feed":[]}"#)),
    ])
    .await
    .expect("start the mock upstream");
    let client = shared_client().expect("build the production client");

    let body = news::fetch_json_with(
        || client.get(&upstream.url),
        TEST_TIMEOUT,
        TEST_RETRY,
    )
    .await
    .expect("the retry must reach the healthy second response");

    assert!(body.get("feed").is_some(), "the second response must be decoded");
    assert_eq!(
        upstream.request_lines().len(),
        2,
        "one failure and one success, and no attempts past the success",
    );
}

/// The other half of a useful budget: a 404 is a bad request, and asking three
/// times does not make it a good one.
#[tokio::test]
async fn a_news_poll_does_not_retry_a_client_error() {
    let upstream =
        spawn_scripted_upstream(vec![Some(http_response("404 Not Found", "{}"))])
            .await
            .expect("start the mock upstream");
    let client = shared_client().expect("build the production client");

    news::fetch_json_with(|| client.get(&upstream.url), TEST_TIMEOUT, TEST_RETRY)
        .await
        .expect_err("a 404 must surface as an error rather than an empty feed");

    assert_eq!(
        upstream.request_lines().len(),
        1,
        "a 4xx is not transient, so the retry budget must be left unspent",
    );
}

/// The bluesky feed reaches the endpoint it is handed, with the actor attached,
/// and decodes a real-shaped payload.
#[tokio::test]
async fn the_bluesky_feed_queries_its_endpoint_for_the_requested_actor() {
    const ACTOR: &str = "marathonteam.bungie.net";

    let feed = r#"{"feed":[{"post":{
        "uri":"at://did:plc:abc123/app.bsky.feed.post/3lxyz",
        "record":{"text":"Servers are back up."}
    }}]}"#;

    let upstream =
        spawn_scripted_upstream(vec![Some(http_response("200 OK", feed))])
            .await
            .expect("start the mock upstream");
    let client = shared_client().expect("build the production client");

    let items = BlueskyFeed::fetch_actor(&client, &upstream.url, ACTOR)
        .await
        .expect("a healthy feed must decode");

    let item = items.first().expect("the one post must survive filtering");
    assert_eq!(item.feed_key, format!("bluesky:{ACTOR}"));
    assert_eq!(item.title, "Servers are back up.");
    let expected_url = format!("https://bsky.app/profile/{ACTOR}/post/3lxyz");
    assert_eq!(item.url.as_deref(), Some(expected_url.as_str()));

    let requests = upstream.request_lines();
    let line = requests.first().expect("the endpoint must have been hit");
    assert!(
        line.contains(&format!("actor={ACTOR}")),
        "the actor must travel as a query parameter, got {line:?}",
    );
}

/// The production budgets the socket tests stand in for. `reqwest` exposes no
/// getter for a request's timeout, so this pins the two relationships that make
/// the fix a fix.
#[test]
fn the_production_news_budget_stays_inside_the_cron_loop_it_blocks() {
    assert!(
        news::NEWS_TIMEOUT < HTTP_TIMEOUT,
        "a per-request budget at or above the shared client's {HTTP_TIMEOUT:?} \
         cap would never fire, leaving the poll exactly where it was",
    );
    const {
        assert!(
            news::NEWS_RETRY.attempts >= 2,
            "a budget of one attempt is not a retry"
        );
    }

    let backoff: Duration = (0..news::NEWS_RETRY.attempts.saturating_sub(1))
        .map(|n| news::NEWS_RETRY.backoff * 2u32.saturating_pow(n))
        .sum();
    let worst_case = news::NEWS_TIMEOUT * news::NEWS_RETRY.attempts + backoff;

    // `bot::cron::run_cron_jobs_loop` awaits its due jobs inline and the
    // shortest job in the workspace runs every minute, so a fully wedged poll
    // has to unblock the loop before the next minute's jobs are due. The feeds
    // are polled concurrently, so this is the whole job's worst case, not one
    // feed's.
    assert!(
        worst_case < Duration::from_secs(60),
        "a fully wedged poll would hold the cron loop for {worst_case:?}, past \
         the minute-granularity jobs queued behind it",
    );
}

/// The endpoint the production poll is wired to, kept honest against a typo
/// that would send every actor's feed to the wrong host.
#[test]
fn the_production_bluesky_endpoint_is_the_public_appview() {
    assert_eq!(
        BLUESKY_FEED_URL,
        "https://public.api.bsky.app/xrpc/app.bsky.feed.getAuthorFeed",
    );
}
