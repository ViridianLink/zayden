use deadlock::error::DeadlockError;
use deadlock_api::DeadlockApiError;
use reqwest::StatusCode;

#[test]
fn server_errors_are_transient() {
    let error = DeadlockError::Api(DeadlockApiError::Http {
        status: StatusCode::BAD_GATEWAY,
    });

    assert!(error.is_transient());
}

#[test]
fn client_errors_are_not_transient() {
    let error = DeadlockError::Api(DeadlockApiError::Http {
        status: StatusCode::BAD_REQUEST,
    });

    assert!(!error.is_transient());
}
