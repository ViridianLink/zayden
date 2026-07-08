use std::sync::Arc;

use serenity::all::{ChannelId, CreateMessage, MessageFlags};
use sqlx::Postgres;
use tracing::error;
use zayden_core::{CronJob, as_u64};

use crate::announce::MarathonAnnounceRow;
use crate::client::MarathonClient;
use crate::embeds;

pub struct MarathonAnnounceCron;

impl MarathonAnnounceCron {
    pub fn cron_job(
        client: Arc<MarathonClient>,
    ) -> Result<CronJob<Postgres>, jiff_cron::error::Error> {
        CronJob::new("marathon_schedule_announce", "0 0 17,18 * * Sun,Thu *").map(|job| {
            job.set_action(move |ctx, pool| {
                let client = Arc::clone(&client);
                async move {
                    let schedule = match client.schedule() {
                        Ok(schedule) => schedule,
                        Err(e) => {
                            error!(error = ?e, "marathon: failed to compute schedule");
                            return;
                        }
                    };
                    let rotation_key = format!("{schedule:?}");

                    let rows = match MarathonAnnounceRow::all(&pool).await {
                        Ok(rows) => rows,
                        Err(e) => {
                            error!(error = ?e, "marathon: failed to load announce rows");
                            return;
                        }
                    };

                    let component = embeds::schedule_component(&schedule);

                    for row in rows {
                        if row.last_rotation.as_deref() == Some(rotation_key.as_str()) {
                            continue;
                        }

                        let channel_id = ChannelId::new(as_u64(row.channel_id));
                        if let Err(e) = channel_id
                            .widen()
                            .send_message(
                                &ctx.http,
                                CreateMessage::new()
                                    .flags(MessageFlags::IS_COMPONENTS_V2)
                                    .components(vec![component.clone()]),
                            )
                            .await
                        {
                            error!(
                                error = ?e,
                                guild_id = row.guild_id,
                                "marathon: failed to post schedule announcement"
                            );
                            continue;
                        }

                        if let Err(e) =
                            MarathonAnnounceRow::set_last_rotation(&pool, row.guild_id, &rotation_key)
                                .await
                        {
                            error!(
                                error = ?e,
                                guild_id = row.guild_id,
                                "marathon: failed to persist last_rotation"
                            );
                        }
                    }
                }
            })
        })
    }
}
