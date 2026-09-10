use std::collections::HashSet;
use std::fmt::Write as _;

use jiff::civil::Date;
use jiff::{Span, Zoned};
use zayden_graphics::{Canvas, RasterLimits, Renderer};

const WEEKS: i64 = 53;
const CELL: u32 = 11;
const GAP: u32 = 2;
const PAD: u32 = 12;
const TOP: u32 = 22;

const LIMITS: RasterLimits = RasterLimits { max_pixels: 4_000_000, max_dim: 4_096 };
const LEVELS: [&str; 5] = ["#20242c", "#2d4a35", "#3c6e45", "#4a9152", "#57b660"];

#[must_use]
pub fn canvas() -> Canvas {
    Canvas {
        width: PAD * 2 + u32::try_from(WEEKS).unwrap_or(53) * (CELL + GAP),
        height: TOP + PAD + 7 * (CELL + GAP),
    }
}

#[must_use]
pub fn svg(days: &[Date], title: &str) -> String {
    let today = Zoned::now().date();
    let watched: HashSet<Date> = days.iter().copied().collect();

    // Start on the Sunday at or before the first day of the window, so every
    // column is a whole week and rows line up with weekdays.
    let window_start = today - Span::new().days(WEEKS * 7 - 1);
    let start = window_start
        - Span::new()
            .days(i64::from(window_start.weekday().to_sunday_zero_offset()));

    let Canvas { width, height } = canvas();
    let mut out = String::with_capacity(16 * 1024);

    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    );
    let _ =
        write!(out, r##"<rect width="{width}" height="{height}" fill="#15171c"/>"##);
    let _ = write!(
        out,
        r##"<text x="{PAD}" y="15" fill="#c9d1d9" font-size="12" font-family="sans-serif">{}</text>"##,
        escape(title)
    );

    for week in 0..WEEKS {
        for weekday in 0..7_i64 {
            let day = start + Span::new().days(week * 7 + weekday);
            if day > today {
                continue;
            }

            let x = PAD + u32::try_from(week).unwrap_or(0) * (CELL + GAP);
            let y = TOP + u32::try_from(weekday).unwrap_or(0) * (CELL + GAP);
            let fill = if watched.contains(&day) { LEVELS[4] } else { LEVELS[0] };

            let _ = write!(
                out,
                r#"<rect x="{x}" y="{y}" width="{CELL}" height="{CELL}" rx="2" fill="{fill}"/>"#
            );
        }
    }

    out.push_str("</svg>");
    out
}

pub async fn render(days: &[Date], title: &str) -> Option<Vec<u8>> {
    let renderer = Renderer::shared().ok()?;

    renderer.render(svg(days, title), canvas(), Vec::new(), LIMITS).await.ok()
}

fn escape(raw: &str) -> String {
    raw.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
