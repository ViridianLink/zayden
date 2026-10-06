use serde::{Deserialize, Serialize};
use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::content::Json;
use topcoat::router::{StatusCode, route};

use crate::admin::dto::{EmojiInfo, EmojiSource, LoadoutCheck, LoadoutForm};
use crate::admin::emoji::create_zayden_emoji;
use crate::admin::loadouts::{check_loadout, save_loadout};
use crate::util::server_error_text;

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

#[route(POST "/admin/destiny2/loadouts/editor/check")]
pub(super) async fn check(
    cx: &Cx,
    Json(form): Json<LoadoutForm>,
) -> Result<Answer<LoadoutCheck>> {
    Ok(answer(check_loadout(cx, &form).await, |e| e.to_string()))
}

#[route(POST "/admin/destiny2/loadouts/editor/save")]
pub(super) async fn save(
    cx: &Cx,
    Json(form): Json<LoadoutForm>,
) -> Result<Answer<i32>> {
    Ok(answer(save_loadout(cx, &form).await, server_error_text))
}

#[route(POST "/admin/destiny2/loadouts/editor/emoji")]
pub(super) async fn emoji(
    cx: &Cx,
    Json(new): Json<NewEmoji>,
) -> Result<Answer<EmojiInfo>> {
    Ok(answer(create_zayden_emoji(cx, new.name, new.source).await, |e| {
        e.to_string()
    }))
}
