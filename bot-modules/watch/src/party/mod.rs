pub mod cron;
pub mod deeplink;
pub mod lifecycle;
pub mod reaper;
pub mod row;

use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
pub use reaper::JellyfinPartyReaperCron;
pub use row::{PartyGuestRow, PartyRow, PartyState};
use serenity::all::Context;
use zayden_core::CronJobData;

#[derive(Debug, Default)]
pub enum Scheduled {
    #[default]
    Nothing,
    Party(Box<PartyRow>),
    Clear(i64),
}

impl Scheduled {
    pub async fn apply<Data: CronJobData>(
        self,
        ctx: &Context,
        runtime: &Arc<JellyfinRuntime>,
    ) {
        match self {
            Self::Nothing => {},
            Self::Party(party) => {
                cron::create_jobs::<Data>(ctx, runtime, &party).await;
            },
            Self::Clear(party_id) => cron::clear_jobs::<Data>(ctx, party_id).await,
        }
    }
}
