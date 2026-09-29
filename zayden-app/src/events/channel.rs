use sqlx::PgExecutor;

use super::AppEvent;
use crate::entitlement::EntitlementScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    ConfigChanged,
    ModulesChanged,
    EntitlementChanged,
    PatreonPost,
    YoutubeUpload,
    HostingPaid,
    ServingChanged,
    CustomBotsChanged,
    LoadoutsChanged,
}

impl Channel {
    pub const ALL: [Self; 9] = [
        Self::ConfigChanged,
        Self::ModulesChanged,
        Self::EntitlementChanged,
        Self::PatreonPost,
        Self::YoutubeUpload,
        Self::HostingPaid,
        Self::ServingChanged,
        Self::CustomBotsChanged,
        Self::LoadoutsChanged,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConfigChanged => "config_changed",
            Self::ModulesChanged => "modules_changed",
            Self::EntitlementChanged => "entitlement_changed",
            Self::PatreonPost => "patreon_post",
            Self::YoutubeUpload => "youtube_upload",
            Self::HostingPaid => "hosting_paid",
            Self::ServingChanged => "serving_changed",
            Self::CustomBotsChanged => "custom_bots_changed",
            Self::LoadoutsChanged => "loadouts_changed",
        }
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|channel| channel.as_str() == name)
    }

    #[must_use]
    pub fn decode(self, payload: &str) -> Option<AppEvent> {
        let event = match self {
            Self::ConfigChanged => AppEvent::ConfigChanged(payload.parse().ok()?),
            Self::ModulesChanged => AppEvent::ModulesChanged(payload.parse().ok()?),
            Self::EntitlementChanged => AppEvent::EntitlementChanged(
                EntitlementScope::from_notify_payload(payload).ok()?,
            ),
            Self::PatreonPost => AppEvent::PatreonPost(payload.to_owned()),
            Self::YoutubeUpload => AppEvent::YoutubeUpload(payload.to_owned()),
            Self::HostingPaid => AppEvent::HostingPaid(payload.parse().ok()?),
            Self::ServingChanged => AppEvent::ServingChanged(payload.parse().ok()?),
            Self::CustomBotsChanged => {
                AppEvent::CustomBotsChanged(payload.parse().ok()?)
            },
            Self::LoadoutsChanged => AppEvent::LoadoutsChanged,
        };

        Some(event)
    }

    pub async fn notify(
        self,
        executor: impl PgExecutor<'_>,
        payload: &str,
    ) -> sqlx::Result<()> {
        sqlx::query!("SELECT pg_notify($1, $2)", self.as_str(), payload)
            .execute(executor)
            .await?;

        Ok(())
    }
}
