use deadlock_api::rank::Badge;

pub const STREAK_MILESTONES: [i32; 3] = [3, 5, 10];
pub const LEADERBOARD_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RankChange {
    Seeded,
    Unchanged,
    Promoted { from: Badge, to: Badge },
    Demoted { from: Badge, to: Badge },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderboardEntry {
    pub discord_id: i64,
    pub badge: Option<Badge>,
    pub streak: i32,
}

#[must_use]
pub fn rank_change(prev: Option<i32>, current: Badge) -> RankChange {
    if !current.is_ranked() {
        return RankChange::Unchanged;
    }

    let stored =
        prev.and_then(|raw| u16::try_from(raw).ok()).and_then(Badge::from_raw);

    match stored {
        None => RankChange::Seeded,
        Some(from) if from == current => RankChange::Unchanged,
        Some(from) if current > from => RankChange::Promoted { from, to: current },
        Some(from) => RankChange::Demoted { from, to: current },
    }
}

#[must_use]
pub const fn streak_milestone(streak: i32) -> i32 {
    let magnitude = streak.saturating_abs();
    let mut reached = 0;
    let mut remaining: &[i32] = &STREAK_MILESTONES;

    while let [milestone, rest @ ..] = remaining {
        if magnitude >= *milestone {
            reached = *milestone;
        }
        remaining = rest;
    }

    if streak < 0 { -reached } else { reached }
}

#[must_use]
pub const fn next_announcement(announced: i32, streak: i32) -> (i32, Option<i32>) {
    let milestone = streak_milestone(streak);

    if milestone == 0 {
        (0, None)
    } else if milestone == announced {
        (announced, None)
    } else {
        (milestone, Some(milestone))
    }
}

fn ranked(entry: &LeaderboardEntry) -> Option<Badge> {
    entry.badge.filter(|badge| badge.is_ranked())
}

pub fn order_leaderboard(entries: &mut Vec<LeaderboardEntry>) {
    entries.sort_by(|a, b| {
        ranked(b)
            .cmp(&ranked(a))
            .then_with(|| b.streak.cmp(&a.streak))
            .then_with(|| a.discord_id.cmp(&b.discord_id))
    });
    entries.truncate(LEADERBOARD_LIMIT);
}
