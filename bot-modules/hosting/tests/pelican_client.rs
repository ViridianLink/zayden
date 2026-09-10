//! The panel client against a scripted server.
//!
//! Hand-rolled over `tokio::net` rather than a mock-server crate: the three
//! behaviours worth pinning here are a status-code branch, a JSON shape and a
//! poll deadline, none of which justify a new third-party dependency.

use std::time::Duration;

use hosting::HostingError;
use hosting::pelican::PelicanApp;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;

/// Serves one canned response per connection, in order, then stops.
async fn serve(responses: Vec<(u16, String)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.ok();
    let Some(listener) = listener else {
        return String::new();
    };

    let base = listener
        .local_addr()
        .map_or_else(|_e| String::new(), |addr| format!("http://{addr}"));

    tokio::spawn(async move {
        for (status, body) in responses {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };

            let mut buf = [0_u8; 2048];
            let _ = socket.read(&mut buf).await;

            let response = format!(
                "HTTP/1.1 {status} X\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        }
    });

    base
}

fn server_json(status: &str, installed: bool) -> String {
    format!(
        r#"{{"object":"server","attributes":{{
            "id":7,"uuid":"u-7","identifier":"abc","name":"n",
            "suspended":false,"status":{status},"allocation":11,
            "container":{{"installed":{installed}}},
            "relationships":{{"allocations":{{"object":"list","data":[
                {{"object":"allocation","attributes":{{
                    "id":11,"ip":"10.0.3.208","port":25565,"alias":"play.example.com"
                }}}}
            ]}}}}
        }}}}"#
    )
}

#[tokio::test]
async fn a_missing_server_deletes_successfully() {
    // A panel outage or a hand-deleted server must not wedge the delete sweep.
    let base = serve(vec![(404, "{}".to_owned())]).await;
    let client = PelicanApp::new(reqwest::Client::new(), &base, "key");

    let result = client.delete_server(7).await;

    assert!(result.is_ok(), "404 on delete has to read as already gone");
}

#[tokio::test]
async fn a_rejected_delete_is_still_an_error() {
    let base = serve(vec![(403, r#"{"errors":[]}"#.to_owned())]).await;
    let client = PelicanApp::new(reqwest::Client::new(), &base, "key");

    let result = client.delete_server(7).await;

    assert!(matches!(result, Err(HostingError::Panel(_))));
}

#[tokio::test]
async fn a_missing_server_unsuspends_successfully() {
    let base = serve(vec![(404, "{}".to_owned())]).await;
    let client = PelicanApp::new(reqwest::Client::new(), &base, "key");

    assert!(client.unsuspend(7).await.is_ok());
}

#[tokio::test]
async fn the_primary_allocation_becomes_the_advertised_address() {
    let base = serve(vec![(200, server_json("null", true))]).await;
    let client = PelicanApp::new(reqwest::Client::new(), &base, "key");

    let address = client.server(7).await.ok().and_then(|s| s.primary_address());

    assert_eq!(
        address.as_deref(),
        Some("play.example.com:25565"),
        "the alias is what players type, not the private IP"
    );
}

#[tokio::test]
async fn an_install_that_never_finishes_times_out() {
    let responses =
        (0..4).map(|_| (200, server_json(r#""installing""#, false))).collect();
    let base = serve(responses).await;
    let client = PelicanApp::new(reqwest::Client::new(), &base, "key");

    let result = client
        .await_install(7, Duration::from_millis(20), Duration::from_millis(60))
        .await;

    assert!(matches!(result, Err(HostingError::InstallTimeout)));
}

#[tokio::test]
async fn a_finished_install_returns_the_server() {
    let base = serve(vec![(200, server_json("null", true))]).await;
    let client = PelicanApp::new(reqwest::Client::new(), &base, "key");

    let result = client
        .await_install(7, Duration::from_millis(20), Duration::from_secs(2))
        .await;

    assert_eq!(result.ok().map(|s| s.id), Some(7));
}

#[tokio::test]
async fn a_failed_install_script_does_not_wait_out_the_deadline() {
    let base = serve(vec![(200, server_json(r#""install_failed""#, false))]).await;
    let client = PelicanApp::new(reqwest::Client::new(), &base, "key");

    let result = client
        .await_install(7, Duration::from_millis(20), Duration::from_secs(30))
        .await;

    assert!(matches!(result, Err(HostingError::Panel(_))));
}

#[tokio::test]
async fn a_panel_five_hundred_reads_as_unavailable_not_as_a_bad_request() {
    let base = serve(vec![(503, "upstream down".to_owned())]).await;
    let client = PelicanApp::new(reqwest::Client::new(), &base, "key");

    let result = client.server(7).await;

    assert!(matches!(result, Err(HostingError::PanelUnavailable)));
}
