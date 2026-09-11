use super::SeerClient;
use super::model::{MediaType, NewRequest, RequestResult, SeerUser, SeerUserPage};
use crate::transport::http::{ApiResult, fetch_json};

const USER_PAGE_SIZE: i64 = 200;

impl SeerClient {
    pub async fn users(&self) -> ApiResult<Vec<SeerUser>> {
        let take = USER_PAGE_SIZE.to_string();

        let page: SeerUserPage = fetch_json(super::SERVICE, "user list", || {
            self.get_query("user", &[("take", take.as_str())])
        })
        .await?;

        Ok(page.results)
    }

    pub async fn user_for_jellyfin(
        &self,
        jellyfin_user_id: &str,
    ) -> ApiResult<Option<SeerUser>> {
        Ok(self
            .users()
            .await?
            .into_iter()
            .find(|u| u.jellyfin_user_id.as_deref() == Some(jellyfin_user_id)))
    }

    pub async fn create_request(
        &self,
        kind: MediaType,
        tmdb_id: i32,
        seasons: Option<serde_json::Value>,
        user_id: Option<i32>,
    ) -> ApiResult<RequestResult> {
        let body = NewRequest {
            media_type: kind.as_str().to_owned(),
            media_id: tmdb_id,
            seasons,
            user_id,
        };

        fetch_json(super::SERVICE, "request creation", || {
            self.post("request").json(&body)
        })
        .await
    }
}
