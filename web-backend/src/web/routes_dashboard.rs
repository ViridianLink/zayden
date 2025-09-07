// const userData = {
//     name: "OscarSix",
//     handle: "@oscarsix",
//     avatar: "/images/avatar.png",
// };

// const serverList = [
//     { id: 1, name: "REDACTED", icon: "/images/servers/redacted.png" },
//     { id: 2, name: "Zayden's Server", icon: "/images/servers/zayden.png" },
//     { id: 3, name: "Bradster", icon: "/images/servers/bradster.png" },
//     { id: 4, name: "Quiet Space", icon: "/images/servers/quiet-space.png" },
//     {
//         id: 5,
//         name: "Underground Railroad",
//         icon: "/images/servers/underground-railroad.png",
//     },
//     {
//         id: 6,
//         name: "Gay Balls Club",
//         icon: "/images/servers/gay-balls-club.png",
//     },
//     {
//         id: 7,
//         name: "Autists Anonymous",
//         icon: "/images/servers/autists-anonymous.png",
//     },
//     { id: 8, name: "chiara's server", icon: "/images/servers/chiara.png" },
//     {
//         id: 9,
//         name: "Deep Stone Therapy",
//         icon: "/images/servers/deep-stone-therapy.png",
//     },
//     {
//         id: 10,
//         name: "Shroomie's server",
//         icon: "/images/servers/shroomie.png",
//     },
//     { id: 11, name: "The Circus", icon: "/images/servers/the-circus.png" },
//     {
//         id: 12,
//         name: "Moon Patrollers",
//         icon: "/images/servers/moon-patrollers.png",
//     },
// ];

use axum::response::IntoResponse;
use serenity::all::{GatewayIntents, Token};
use tower_cookies::Cookies;

use crate::{Error, Result};

use super::AUTH_TOKEN;

pub async fn dashboard(cookies: Cookies) -> impl IntoResponse {
    println!("Cookies: {:?}", cookies.list());

    let auth_token = cookies
        .get(AUTH_TOKEN)
        .expect("auth-token should exist in cookies");

    let client = serenity::Client::builder(
        format!("Bearer {}", auth_token.value()).parse().unwrap(),
        GatewayIntents::empty(),
    )
    .await
    .unwrap();
}
