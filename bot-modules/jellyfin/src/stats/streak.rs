use jiff::civil::Date;
use jiff::{Span, Zoned};
use sqlx::PgPool;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Streak {
    pub current: u32,
    pub longest: u32,
    pub total_days: u32,
    pub total_seconds: i64,
    pub days: Vec<Date>,
}

pub async fn load(pool: &PgPool, jellyfin_user_id: &str) -> sqlx::Result<Streak> {
    let rows = sqlx::query!(
        r#"
        SELECT day AS "day: jiff_sqlx::Date", seconds
        FROM jellyfin_playback_daily
        WHERE jellyfin_user_id = $1
        ORDER BY day
        "#,
        jellyfin_user_id
    )
    .fetch_all(pool)
    .await?;

    let days: Vec<Date> = rows.iter().map(|r| r.day.to_jiff()).collect();
    let total_seconds = rows.iter().map(|r| r.seconds).sum();

    Ok(compute(&days, total_seconds, today()))
}

fn today() -> Date {
    Zoned::now().date()
}

#[must_use]
pub fn compute(days: &[Date], total_seconds: i64, today: Date) -> Streak {
    let mut unique: Vec<Date> = days.to_vec();
    unique.sort_unstable();
    unique.dedup();

    if unique.is_empty() {
        return Streak::default();
    }

    let mut longest = 1_u32;
    let mut run = 1_u32;

    for pair in unique.windows(2) {
        let [previous, current] = pair else { continue };

        if is_next_day(*previous, *current) {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 1;
        }
    }

    let current = current_streak(&unique, today);

    Streak {
        current,
        longest: longest.max(current),
        total_days: u32::try_from(unique.len()).unwrap_or(u32::MAX),
        total_seconds,
        days: unique,
    }
}

fn current_streak(unique: &[Date], today: Date) -> u32 {
    let Some(&last) = unique.last() else {
        return 0;
    };

    let yesterday = today - Span::new().days(1);
    if last != today && last != yesterday {
        return 0;
    }

    let mut streak = 1_u32;
    let mut expected = last;

    for &day in unique.iter().rev().skip(1) {
        let previous = expected - Span::new().days(1);
        if day != previous {
            break;
        }
        streak += 1;
        expected = previous;
    }

    streak
}

fn is_next_day(previous: Date, current: Date) -> bool {
    previous + Span::new().days(1) == current
}
