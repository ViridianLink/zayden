use super::JellyfinClient;
use super::model::ScheduledTask;
use crate::transport::http::{ApiResult, fetch_json, send_ok};

impl JellyfinClient {
    pub async fn scheduled_tasks(&self) -> ApiResult<Vec<ScheduledTask>> {
        fetch_json(super::SERVICE, "scheduled tasks", || self.get("ScheduledTasks"))
            .await
    }

    pub async fn start_scheduled_task(&self, task_id: &str) -> ApiResult<()> {
        send_ok(super::SERVICE, "scheduled task start", || {
            self.post(&format!("ScheduledTasks/Running/{task_id}"))
        })
        .await
    }
}
