use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use tower_cookies::{Cookie, Cookies};

use crate::FRONTEND_URL;
use crate::web::AUTH_TOKEN;

pub async fn add_cookie(jar: Cookies) -> impl IntoResponse {
    let token = "Test Token";

    let cookie = Cookie::build((AUTH_TOKEN, token)).path("/");

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, format!("{FRONTEND_URL}/dashboard"))
        .header(header::SET_COOKIE, cookie.to_string())
        .body(String::new())
        .unwrap()
}

pub async fn list_cookies(jar: Cookies) -> impl IntoResponse {
    println!("Jar: {:?}", jar.list());

    StatusCode::OK
}
