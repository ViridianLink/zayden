//! Regression tests for audit `music` #2 — resolver calls could hang forever.
//!
//! Two independent hang surfaces, one per half of the finding:
//! * the `reqwest` client songbird streams audio through, built with `Client::new()`
//!   (reqwest's default: **no** timeout of any kind);
//! * the `yt-dlp` child process metadata resolution shells out to, awaited with no
//!   budget.

use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn drain_request(socket: &mut TcpStream) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];

    loop {
        let n = match socket.read(&mut chunk).await {
            Ok(0) | Err(_) => return,
            Ok(n) => n,
        };
        let Some(received) = chunk.get(..n) else { return };
        buf.extend_from_slice(received);

        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            return;
        }
    }
}

/// Accepts the connection, reads the request, then never answers and never
/// hangs up — the silent upstream a streaming timeout has to defend against.
async fn spawn_black_hole_server() -> Option<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };

            tokio::spawn(async move {
                drain_request(&mut socket).await;
                // Hold the connection open, silently, forever.
                std::future::pending::<()>().await;
            });
        }
    });

    Some(format!("http://{addr}"))
}

/// Answers, then dribbles the body one byte per `gap` — a healthy but slow
/// live stream. `gap * body_len` is deliberately longer than any total-request
/// budget the workspace uses elsewhere.
async fn spawn_dribbling_server(body_len: usize, gap: Duration) -> Option<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;

    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };

            tokio::spawn(async move {
                drain_request(&mut socket).await;

                let headers = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: audio/mpeg\r\ncontent-length: {body_len}\r\n\r\n"
                );
                if socket.write_all(headers.as_bytes()).await.is_err() {
                    return;
                }

                for _ in 0..body_len {
                    tokio::time::sleep(gap).await;
                    if socket.write_all(b"x").await.is_err() {
                        return;
                    }
                }
                let _ = socket.shutdown().await;
            });
        }
    });

    Some(format!("http://{addr}"))
}

/// The budgets the two socket tests below run the real client at.
///
/// **These tests must not use a paused clock.** They drive a real `reqwest`
/// client over a real loopback socket, and under `start_paused` tokio
/// auto-advances virtual time whenever the runtime goes idle — which is exactly
/// what it does while the OS completes the TCP handshake. The connect budget
/// then elapses in virtual time before the connect finishes in real time, and
/// the assertion never reaches the read budget it exists to measure. So the
/// clock is real and the budgets are scaled down to milliseconds instead
/// (`music::stream_client_with`), which keeps both tests under a second.
const TEST_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TEST_READ_TIMEOUT: Duration = Duration::from_millis(300);

/// Fails-before: `Client::new()` carried no timeout, so the request sat on the
/// silent upstream until the backstop fired — in production that is a track
/// that never starts, never errors, and never advances the queue.
#[tokio::test]
async fn stream_client_gives_up_on_a_silent_upstream_instead_of_waiting_forever() {
    let url = spawn_black_hole_server().await.expect("start mock server");

    let client = music::stream_client_with(TEST_CONNECT_TIMEOUT, TEST_READ_TIMEOUT)
        .expect("build streaming client");

    let started = Instant::now();

    let result = tokio::time::timeout(
        TEST_CONNECT_TIMEOUT + TEST_READ_TIMEOUT * 10,
        client.get(&url).send(),
    )
    .await
    .expect("the client must give up on its own rather than wait forever");

    let err = result.expect_err("a silent upstream must surface as an error");
    let elapsed = started.elapsed();

    assert!(err.is_timeout(), "expected a timeout error, got {err:?}");
    // A loopback connect that succeeded and then went silent is the scenario;
    // if this ever fires as a *connect* timeout the test has stopped measuring
    // the read budget — which is precisely how the paused-clock version of this
    // suite went red (audit `music` #4).
    assert!(
        !err.is_connect(),
        "the upstream accepted the connection, so this must be the read budget \
         firing, not the connect budget: {err:?}",
    );
    assert!(
        elapsed < TEST_CONNECT_TIMEOUT,
        "gave up after {elapsed:?}, which is the connect budget rather than the \
         {TEST_READ_TIMEOUT:?} read budget",
    );
}

/// The guard against the *wrong* fix. Chaining `zayden_app`'s
/// `ClientBuilderExt::with_timeouts()` here would look like the obvious
/// symmetry with the `ai` #1 fix, but its cap is a **total request** budget and
/// songbird streams the whole track body through this client — every track
/// longer than the cap would be cut off mid-playback.
///
/// So the body here takes longer to arrive *in total* than the read budget,
/// while no single read ever exceeds it: a per-read cap passes this, a
/// total-request cap of the same size fails it. That relationship is the
/// property under test, which is why it holds at millisecond scale exactly as
/// it does at the production constants.
#[tokio::test]
async fn stream_client_does_not_cap_a_slow_but_healthy_stream() {
    const BODY_LEN: usize = 10;
    const GAP: Duration = Duration::from_millis(50);

    assert!(
        GAP < TEST_READ_TIMEOUT,
        "the dribble gap must stay inside the per-read budget, or this test \
         stops measuring what it claims to"
    );
    assert!(
        GAP * u32::try_from(BODY_LEN).expect("body length fits a u32")
            > TEST_READ_TIMEOUT,
        "the whole body must take longer than the read budget, or a \
         total-request cap would pass this test too"
    );

    let url = spawn_dribbling_server(BODY_LEN, GAP).await.expect("start server");

    let client = music::stream_client_with(TEST_CONNECT_TIMEOUT, TEST_READ_TIMEOUT)
        .expect("build streaming client");

    let body = tokio::time::timeout(TEST_CONNECT_TIMEOUT, async {
        client.get(&url).send().await?.bytes().await
    })
    .await
    .expect("the dribbling stream must not stall the test")
    .expect("a healthy slow stream must not be cut off by the client");

    assert_eq!(body.len(), BODY_LEN, "the whole body must arrive");
}

/// The production entry point still builds, and still carries the constants the
/// two tests above stand in for. `reqwest` exposes no getter for a built
/// client's timeouts, so this pins what is observable: the real budgets are a
/// per-read one, and the read budget is the larger of the two — the ordering
/// that makes a silent upstream fail on the read rather than the connect.
#[tokio::test]
async fn production_stream_client_builds_with_a_read_budget() {
    music::stream_client().expect("build the production streaming client");

    assert!(
        music::STREAM_READ_TIMEOUT
            > zayden_app::services::http::HTTP_CONNECT_TIMEOUT,
        "the read budget must outlast the connect budget",
    );
}

/// Fails-before: `Command::output()` was awaited with no budget, so a `yt-dlp`
/// that hangs (throttled, waiting on a stuck JS runtime, prompting) wedges the
/// resolve future for as long as the child lives.
#[tokio::test]
async fn run_with_timeout_gives_up_on_a_child_that_outlives_its_budget() {
    let started = Instant::now();

    let err = music::run_with_timeout("sleep", &["30"], Duration::from_millis(200))
        .await
        .expect_err("a child that outruns its budget must surface as an error");

    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "run_with_timeout waited {elapsed:?} on a 200ms budget",
    );
    assert!(
        err.contains("did not finish within"),
        "expected a timeout message, got {err:?}",
    );
}

/// `kill_on_drop` is what turns the budget above into an actual kill rather
/// than an abandoned child. Skipped where `pgrep` is unavailable.
#[tokio::test]
async fn run_with_timeout_kills_the_child_it_gave_up_on() {
    const MARKER: &str = "31.41592";

    let pgrep_available = tokio::process::Command::new("pgrep")
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .is_ok();
    if !pgrep_available {
        eprintln!("skipping: `pgrep` is not available on this host");
        return;
    }

    music::run_with_timeout("sleep", &[MARKER], Duration::from_millis(200))
        .await
        .expect_err("the child must outrun its budget");

    // The kill is asynchronous with respect to the dropped future; give the
    // reaper a moment before looking.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let found = tokio::process::Command::new("pgrep")
        .args(["-f", &format!("sleep {MARKER}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .expect("run pgrep");

    assert!(
        !found.success(),
        "the abandoned `sleep {MARKER}` child is still running; the timeout \
         walked away instead of killing it",
    );
}

#[tokio::test]
async fn run_with_timeout_returns_output_for_a_child_that_finishes() {
    let output = music::run_with_timeout("echo", &["ok"], Duration::from_secs(10))
        .await
        .expect("a fast child must succeed");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ok");
}
