use jiff::Zoned;
use jiff::civil::Weekday;

use crate::model::RotationWindow;

const PT_TZ: &str = "America/Los_Angeles";

const RANKED_START_WEEKDAY: Weekday = Weekday::Sunday;
const RANKED_START_HOUR_PT: u8 = 10;
const RANKED_END_WEEKDAY: Weekday = Weekday::Thursday;
const RANKED_END_HOUR_PT: u8 = 10;

pub const DUO_MAP_POOL: [&str; 4] =
    ["Perimeter", "Dire Marsh", "Night Marsh", "Outpost"];

#[must_use]
pub fn current_windows() -> (RotationWindow, RotationWindow) {
    let now_pt = Zoned::now().in_tz(PT_TZ).unwrap_or_else(|_| Zoned::now());
    let now_minutes =
        minutes_of_week(now_pt.weekday(), now_pt.hour(), now_pt.minute());

    let ranked_start =
        minutes_of_week_at(RANKED_START_WEEKDAY, RANKED_START_HOUR_PT);
    let ranked_end = minutes_of_week_at(RANKED_END_WEEKDAY, RANKED_END_HOUR_PT);
    let ranked_active = in_window(now_minutes, ranked_start, ranked_end);

    let ranked = RotationWindow {
        start_weekday: RANKED_START_WEEKDAY,
        start_hour_pt: RANKED_START_HOUR_PT,
        end_weekday: RANKED_END_WEEKDAY,
        end_hour_pt: RANKED_END_HOUR_PT,
        active: ranked_active,
    };
    let cryo = RotationWindow {
        start_weekday: RANKED_END_WEEKDAY,
        start_hour_pt: RANKED_END_HOUR_PT,
        end_weekday: RANKED_START_WEEKDAY,
        end_hour_pt: RANKED_START_HOUR_PT,
        active: !ranked_active,
    };

    (ranked, cryo)
}

const MINUTES_PER_DAY: i64 = 24 * 60;

fn minutes_of_week(weekday: Weekday, hour: i8, minute: i8) -> i64 {
    i64::from(weekday.to_sunday_zero_offset()) * MINUTES_PER_DAY
        + i64::from(hour) * 60
        + i64::from(minute)
}

fn minutes_of_week_at(weekday: Weekday, hour: u8) -> i64 {
    i64::from(weekday.to_sunday_zero_offset()) * MINUTES_PER_DAY
        + i64::from(hour) * 60
}

const fn in_window(now: i64, start: i64, end: i64) -> bool {
    if start <= end { now >= start && now < end } else { now >= start || now < end }
}
