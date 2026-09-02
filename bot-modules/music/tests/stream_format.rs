//! Regression tests for the `YouTube` stream path — the half of the resolver
//! that turns a track URL into bytes songbird can play.
//!
//! Every track in a queue failing with
//! `input creation [failed to create audio: failed with http status code: 403
//! Forbidden]` came from handing songbird a media URL nobody had checked: the
//! URL is only fetched once the mixer wants audio, by which point the track can
//! only error out. The resolver now asks the audio host first, and tries the
//! next player client when the answer is not a success.

use std::time::Duration;

use music::StreamFormat;
use reqwest::header::{HeaderMap, HeaderValue};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(5);

async fn read_request(socket: &mut TcpStream) -> String {
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

    String::from_utf8_lossy(&buf).into_owned()
}

/// Answers every request with `status`, and hands back the request line and
/// headers of the first one so the probe's shape can be asserted on.
async fn spawn_server(
    status: &'static str,
) -> Option<(String, oneshot::Receiver<String>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;
    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut tx = Some(tx);

        while let Ok((mut socket, _)) = listener.accept().await {
            let request = read_request(&mut socket).await;
            if let Some(tx) = tx.take() {
                let _ = tx.send(request);
            }

            let response = format!("HTTP/1.1 {status}\r\ncontent-length: 0\r\n\r\n");
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    Some((format!("http://{addr}"), rx))
}

fn format_at(url: String, filesize: Option<u64>) -> StreamFormat {
    let mut headers = HeaderMap::new();
    headers.insert("user-agent", HeaderValue::from_static("zayden-test"));

    StreamFormat { url, headers, filesize, protocol: Some(String::from("https")) }
}

/// Fails-before: nothing looked at the media URL, so a `403` only appeared once
/// the mixer asked for audio — as a dead track, one per queued song.
#[tokio::test]
async fn probe_rejects_a_forbidden_audio_host() {
    let (url, _request) =
        spawn_server("403 Forbidden").await.expect("start mock server");
    let client =
        music::stream_client_with(CONNECT_TIMEOUT, READ_TIMEOUT).expect("client");

    let err = music::probe_stream(&client, &format_at(url, Some(1024)))
        .await
        .expect_err("a 403 from the audio host must not be treated as playable");

    assert!(
        err.to_string().contains("403"),
        "the status the host gave belongs in the error: {err}"
    );
}

#[tokio::test]
async fn probe_accepts_a_ranged_response() {
    let (url, _request) =
        spawn_server("206 Partial Content").await.expect("start mock server");
    let client =
        music::stream_client_with(CONNECT_TIMEOUT, READ_TIMEOUT).expect("client");

    music::probe_stream(&client, &format_at(url, Some(1024)))
        .await
        .expect("a partial-content response is what a healthy range request gets");
}

/// The probe is only worth anything if it is the same request playback makes:
/// `YouTube` answers a bounded range and an open-ended one differently, and
/// songbird sends the bounded form whenever it knows the length.
#[tokio::test]
async fn probe_sends_the_same_request_songbird_will() {
    let (url, request) =
        spawn_server("206 Partial Content").await.expect("start mock server");
    let client =
        music::stream_client_with(CONNECT_TIMEOUT, READ_TIMEOUT).expect("client");

    music::probe_stream(&client, &format_at(url, Some(3_433_755)))
        .await
        .expect("probe");

    let request = request.await.expect("the server saw a request");
    let lower = request.to_lowercase();

    assert!(lower.contains("range: bytes=0-3433754"), "missing range: {request}");
    assert!(lower.contains("user-agent: zayden-test"), "missing headers: {request}");
}

#[test]
fn range_header_matches_songbirds_bounded_form() {
    assert_eq!(
        format_at(String::new(), Some(3_433_755)).range_header().as_deref(),
        Some("bytes=0-3433754")
    );
    assert_eq!(format_at(String::new(), None).range_header(), None);
}

#[test]
fn hls_formats_are_detected_by_protocol() {
    let mut live = format_at(String::new(), None);
    live.protocol = Some(String::from("m3u8_native"));
    assert!(live.is_hls());

    assert!(!format_at(String::new(), None).is_hls());

    let mut unknown = format_at(String::new(), None);
    unknown.protocol = None;
    assert!(!unknown.is_hls());
}

/// The outage this suite exists for: yt-dlp with no JavaScript runtime falls
/// back to `android_vr` alone, so when `YouTube` starts refusing that client's
/// URLs there is nothing left to try. A chain of one is the bug.
#[test]
fn stream_clients_leave_somewhere_to_fall_back_to() {
    assert!(
        music::STREAM_CLIENTS.len() > 1,
        "a single player client is exactly the failure mode being fixed"
    );

    let mut seen = music::STREAM_CLIENTS.to_vec();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.len(),
        music::STREAM_CLIENTS.len(),
        "a repeated client just repeats a failed attempt"
    );
}

/// Audio-only first: it is a quarter of the bytes of the muxed fallback and
/// lets songbird pass Opus frames through untranscoded.
#[test]
fn stream_format_prefers_audio_only() {
    let first = music::STREAM_FORMAT.split('/').next().expect("a selector");

    assert!(
        first.contains("vcodec=none"),
        "the first choice must be audio-only, got {first}"
    );
}

/// Every client in the chain is tried in turn before a track is given up on,
/// so the chain's total budget is what whoever queued the track waits through.
#[test]
fn the_whole_client_chain_fits_inside_a_sane_wait() {
    let clients = u32::try_from(music::STREAM_CLIENTS.len()).unwrap_or(u32::MAX);
    let worst_case = music::YT_DLP_STREAM_TIMEOUT.saturating_mul(clients);

    assert!(
        worst_case <= Duration::from_secs(90),
        "a fully broken chain would stall playback for {worst_case:?}"
    );
}
