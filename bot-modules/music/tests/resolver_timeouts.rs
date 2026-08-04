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

/// Fails-before: `Client::new()` carries no timeout, so the request sits on the
/// silent upstream until the backstop fires — in production that is a track
/// that never starts, never errors, and never advances the queue.
///
/// Virtual time, so the client's real 20s budget costs the suite nothing.
#[tokio::test(start_paused = true)]
async fn stream_client_gives_up_on_a_silent_upstream_instead_of_waiting_forever() {
    let url = spawn_black_hole_server().await.expect("start mock server");

    let client = music::stream_client().expect("build streaming client");

    let result =
        tokio::time::timeout(Duration::from_secs(120), client.get(&url).send())
            .await
            .expect(
                "the client must give up on its own; it was still waiting on a \
                 silent upstream after 120s",
            );

    let err = result.expect_err("a silent upstream must surface as an error");

    assert!(err.is_timeout(), "expected a timeout error, got {err:?}");
}

/// The guard against the *wrong* fix. Chaining `zayden_app`'s
/// `ClientBuilderExt::with_timeouts()` here would look like the obvious
/// symmetry with the `ai` #1 fix, but its 30s cap is a **total request** budget
/// and songbird streams the whole track body through this client — every track
/// longer than 30s would be cut off mid-playback. This stream stays healthy for
/// 50 virtual seconds; a total-request cap fails it, a per-read cap does not.
#[tokio::test(start_paused = true)]
async fn stream_client_does_not_cap_a_slow_but_healthy_stream() {
    const BODY_LEN: usize = 5;
    const GAP: Duration = Duration::from_secs(10);

    assert!(
        GAP < music::STREAM_READ_TIMEOUT,
        "the dribble gap must stay inside the per-read budget, or this test \
         stops measuring what it claims to"
    );

    let url = spawn_dribbling_server(BODY_LEN, GAP).await.expect("start server");

    let client = music::stream_client().expect("build streaming client");

    let body = tokio::time::timeout(Duration::from_secs(300), async {
        client.get(&url).send().await?.bytes().await
    })
    .await
    .expect("the 50s stream must not stall the test")
    .expect("a healthy slow stream must not be cut off by the client");

    assert_eq!(body.len(), BODY_LEN, "the whole body must arrive");
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
