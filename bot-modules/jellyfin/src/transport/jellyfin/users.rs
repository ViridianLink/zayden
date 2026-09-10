use super::JellyfinClient;
use super::model::{AuthenticationResult, NewUser, User, UserPolicy};
use crate::transport::http::{ApiResult, fetch_json, send_ok};

impl JellyfinClient {
    pub async fn users(&self) -> ApiResult<Vec<User>> {
        fetch_json(super::SERVICE, "user list", || self.get("Users")).await
    }

    pub async fn create_user(
        &self,
        name: &str,
        password: &str,
    ) -> ApiResult<AuthenticationResult> {
        fetch_json(super::SERVICE, "user creation", || {
            self.post("Users/New").json(&NewUser {
                name: name.to_owned(),
                password: password.to_owned(),
            })
        })
        .await
    }

    pub async fn set_policy(
        &self,
        user_id: &str,
        policy: &UserPolicy,
    ) -> ApiResult<()> {
        send_ok(super::SERVICE, "policy update", || {
            self.post(&format!("Users/{user_id}/Policy")).json(policy)
        })
        .await
    }

    pub async fn delete_user(&self, user_id: &str) -> ApiResult<()> {
        match send_ok(super::SERVICE, "user deletion", || {
            self.delete(&format!("Users/{user_id}"))
        })
        .await
        {
            Err(e) if e.is_not_found() => Ok(()),
            other => other,
        }
    }
}
