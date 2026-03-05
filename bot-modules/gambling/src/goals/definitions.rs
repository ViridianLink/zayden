use std::collections::HashMap;
use std::sync::LazyLock;

use rand::rng;
use rand::seq::IteratorRandom;
use zayden_core::FormatNum;

use crate::GamblingGoalsRow;
use crate::events::{Event, EventRow};
use crate::shop::LOTTO_TICKET;

#[derive(Clone, Copy)]
pub struct GoalDefinition {
    pub id: &'static str,
    pub target: fn(&dyn EventRow) -> i64,
    pub description: fn(i64) -> String,
    pub update_fn: fn(&mut GamblingGoalsRow, &Event) -> bool,
}

impl GoalDefinition {
    const fn new(id: &'static str) -> Self {
        Self {
            id,
            target: |_| 1,
            description: |_| String::new(),
            update_fn: |_, _| false,
        }
    }

    const fn set_target(mut self, f: fn(&dyn EventRow) -> i64) -> Self {
        self.target = f;
        self
    }

    const fn set_description(mut self, f: fn(i64) -> String) -> Self {
        self.description = f;
        self
    }

    const fn set_update_fn(mut self, f: fn(&mut GamblingGoalsRow, &Event) -> bool) -> Self {
        self.update_fn = f;
        self
    }
}

const LOTTO: GoalDefinition = GoalDefinition::new("lotto")
    .set_target(|_| rand::random_range(1..=3))
    .set_description(|t| format!("Buy {t} lottery ticket"))
    .set_update_fn(|goal: &mut GamblingGoalsRow, event: &Event| {
        let Event::ShopPurchase(purchase_id) = event else {
            return false;
        };

        if purchase_id.item_id != LOTTO_TICKET.id {
            return false;
        }

        goal.set_completed();
        true
    });

const GIFT: GoalDefinition = GoalDefinition::new("gift")
    .set_description(|_| String::from("Send a gift"))
    .set_update_fn(|goal: &mut GamblingGoalsRow, event: &Event| {
        let Event::Send(event) = event else {
            return false;
        };

        if event.amount < 2500 {
            return false;
        }

        goal.set_completed();
        true
    });

const WIN_10: GoalDefinition = GoalDefinition::new("gift")
    .set_target(|_| rand::random_range(7..=10))
    .set_description(|t| format!("Win {t} times"))
    .set_update_fn(|goal: &mut GamblingGoalsRow, event: &Event| {
        let Event::Game(event) = event else {
            return false;
        };

        if !event.win {
            return false;
        }

        goal.update_progress(1);
        true
    });

const HIGHERLOWER: GoalDefinition = GoalDefinition::new("higherlower")
    .set_target(|_| rand::random_range(6..=10))
    .set_description(|t| format!("Hit a streak of {t}x on Higher or Lower"))
    .set_update_fn(|goal: &mut GamblingGoalsRow, event: &Event| {
        let Event::Game(event) = event else {
            return false;
        };

        if event.game_id != "higherorlower" {
            return false;
        }

        goal.progress = event.payout / 1000;
        goal.progress = goal.progress.min(goal.target);
        true
    });

const WIN_MAX_BET: GoalDefinition = GoalDefinition::new("winmaxbet")
    .set_target(|row| row.max_bet().min(row.coins()))
    .set_description(|t| format!("Win {} coins", t.format()))
    .set_update_fn(|goal, event| {
        let Event::Game(event) = event else {
            return false;
        };

        if !event.win {
            return false;
        }

        goal.update_progress(event.bet);
        true
    });

const WIN_3_ROW: GoalDefinition = GoalDefinition::new("win3row")
    .set_target(|_| 3)
    .set_description(|_| String::from("Win 3 times in a row"))
    .set_update_fn(|goal, event| {
        let Event::Game(event) = event else {
            return false;
        };

        if !event.win {
            goal.reset_progress();
            return false;
        }

        goal.update_progress(1);
        true
    });

const ALL_IN: GoalDefinition = GoalDefinition::new("allin")
    .set_target(|row| row.coins().max(1000).min(row.max_bet()))
    .set_description(|t| format!("Go all in ({})", t.format()))
    .set_update_fn(|goal, event| {
        let Event::Game(event) = event else {
            return false;
        };

        goal.update_progress(event.bet);
        goal.is_complete()
    });

const SEND_COINS: GoalDefinition = GoalDefinition::new("sendcoins")
    .set_target(|row| (row.coins() / 10).min(row.max_bet() / 10).max(2500))
    .set_description(|t| format!("Send coins ({})", t.format()))
    .set_update_fn(|goal, event| {
        let Event::Send(event) = event else {
            return false;
        };

        goal.update_progress(event.amount);
        true
    });

const WORK: GoalDefinition = GoalDefinition::new("work")
    .set_target(|_| rand::random_range(3..=7))
    .set_description(|t| format!("Work or Dig {t}x times"))
    .set_update_fn(|goal, event| {
        let Event::Work(_) = event else {
            return false;
        };

        goal.update_progress(1);
        true
    });

pub struct GoalRegistry(HashMap<&'static str, GoalDefinition>);

impl GoalRegistry {
    pub fn new(goals: [GoalDefinition; 9]) -> Self {
        Self(goals.into_iter().map(|goal| (goal.id, goal)).collect())
    }

    pub fn get_definition(&self, id: &str) -> Option<GoalDefinition> {
        self.0.get(id).copied()
    }

    pub fn select_daily_goal(&self) -> Vec<GoalDefinition> {
        self.0.values().copied().choose_multiple(&mut rng(), 3)
    }
}

pub static GOAL_REGISTRY: LazyLock<GoalRegistry> = LazyLock::new(|| {
    GoalRegistry::new([
        LOTTO,
        GIFT,
        WIN_10,
        HIGHERLOWER,
        WIN_MAX_BET,
        WIN_3_ROW,
        ALL_IN,
        SEND_COINS,
        WORK,
    ])
});
