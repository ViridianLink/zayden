use serenity::all::{Colour, CreateEmbed, CreateMessage, GenericChannelId, Http, UserId};
use sqlx::{Database, Pool};
use zayden_core::EmojiCache;

use crate::events::{Event, EventRow};
use crate::{GEM, GamblingGoalsRow, GoalsManager, tomorrow};

use super::GOAL_REGISTRY;

pub struct GoalHandler;

impl GoalHandler {
    pub async fn daily_reset<Db: Database, Manager: GoalsManager<Db>>(
        pool: &Pool<Db>,
        id: impl Into<UserId>,
        row: &dyn EventRow,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        let id = id.into();

        let selected_goal_definitions = GOAL_REGISTRY.select_daily_goal();

        let goals = selected_goal_definitions
            .into_iter()
            .map(|goal| {
                let target_value = (goal.target)(row);
                (goal.id, target_value)
            })
            .map(|(goal_id, target)| GamblingGoalsRow::new(id, goal_id, target))
            .collect::<Vec<_>>();

        let rows = Manager::update(pool, &goals).await?;

        Ok(rows)
    }

    pub async fn get_user_progress<Db: Database, Manager: GoalsManager<Db>>(
        pool: &Pool<Db>,
        user_id: impl Into<UserId>,
        row: &dyn EventRow,
    ) -> sqlx::Result<Vec<GamblingGoalsRow>> {
        let user_id = user_id.into();

        let mut goals = Manager::full_rows(pool, user_id).await?;

        if goals.is_empty() || !goals[0].is_today() {
            goals = Self::daily_reset::<Db, Manager>(pool, user_id, row).await?;
        }

        Ok(goals)
    }

    pub async fn process_goals<Db: Database, Manager: GoalsManager<Db>>(
        http: &Http,
        pool: &Pool<Db>,
        emojis: &EmojiCache,
        channel: GenericChannelId,
        row: &mut dyn EventRow,
        event: Event,
    ) -> sqlx::Result<Event> {
        let user_id = event.user_id();

        let mut all_goals = Self::get_user_progress::<Db, Manager>(pool, user_id, row).await?;

        let changed = all_goals
            .iter_mut()
            .filter(|goal| !goal.is_complete())
            .filter_map(|goal| {
                GOAL_REGISTRY
                    .get_definition(goal.goal_id())
                    .map(|definition| (goal, definition))
            })
            .fold(Vec::new(), |mut acc, (goal, definition)| {
                let changed = (definition.update_fn)(goal, &event);

                if changed {
                    acc.push(&*goal);
                }

                acc
            });

        let coin = emojis.get("heads").unwrap();

        for &goal in changed.iter().filter(|goal| goal.is_complete()) {
            row.add_coins(5_000);

            channel
                .send_message(
                    http,
                    CreateMessage::new().embed(
                        CreateEmbed::new()
                            .description(format!(
                                "**Daily goal completed:** {}\n**Reward:** 5,000 <:coin:{coin}>",
                                goal.title()
                            ))
                            .colour(Colour::DARK_GREEN),
                    ),
                )
                .await
                .unwrap();
        }

        if !changed.is_empty() {
            if all_goals.iter().all(|row| row.is_complete()) {
                row.add_gems(1);

                channel
                    .send_message(
                        http,
                        CreateMessage::new().embed(CreateEmbed::new().description(format!(
                            "You have completed **all** daily goals! 🎉\n**Reward:** 1 {GEM}\n\nGoals reset <t:{}:R>", tomorrow(None)
                        )).colour(Colour::DARK_GREEN)),
                    )
                    .await
                    .unwrap();
            }

            Manager::update(pool, &all_goals).await.unwrap();
        }

        Ok(event)
    }
}
