use serde::{Deserialize, Serialize};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Json;
use topcoat::router::{StatusCode, route, segment};

use crate::admin::dto::{EmojiInfo, EmojiSource, LoadoutCheck, LoadoutForm};
use crate::admin::{check_loadout, create_zayden_emoji, save_loadout};
use crate::util::server_error_text;

segment!(kind = Group);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reply<T> {
    Ok(T),
    Error(String),
}

type Answer<T> = (StatusCode, Json<Reply<T>>);

fn answer<T, E>(
    result: std::result::Result<T, E>,
    text: impl FnOnce(E) -> String,
) -> Answer<T> {
    match result {
        Ok(value) => (StatusCode::OK, Json(Reply::Ok(value))),
        Err(e) => (StatusCode::UNPROCESSABLE_ENTITY, Json(Reply::Error(text(e)))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewEmoji {
    pub name: String,
    pub source: EmojiSource,
}

#[route(POST "./editor/check")]
pub(super) async fn check(
    cx: &Cx,
    Json(form): Json<LoadoutForm>,
) -> Result<Answer<LoadoutCheck>> {
    Ok(answer(check_loadout(cx, &form).await, |e| e.to_string()))
}

#[route(POST "./editor/save")]
pub(super) async fn save(
    cx: &Cx,
    Json(form): Json<LoadoutForm>,
) -> Result<Answer<i32>> {
    Ok(answer(save_loadout(cx, &form).await, server_error_text))
}

#[route(POST "./editor/emoji")]
pub(super) async fn emoji(
    cx: &Cx,
    Json(new): Json<NewEmoji>,
) -> Result<Answer<EmojiInfo>> {
    Ok(answer(create_zayden_emoji(cx, new.name, new.source).await, |e| {
        e.to_string()
    }))
}
