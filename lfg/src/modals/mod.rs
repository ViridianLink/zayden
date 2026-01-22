pub mod create;
pub use create::{Create, GuildManager};

pub mod edit;
pub use edit::Edit;

use chrono::{DateTime, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use serenity::all::{CreateInputText, CreateLabel, CreateModalComponent, InputTextStyle};

use crate::{Error, Result};

pub fn modal_components<'a>(
    activity: &'a str,
    start_time: DateTime<Tz>,
    fireteam_size: i16,
    description: Option<&'a str>,
) -> Vec<CreateModalComponent<'a>> {
    let mut desc_input =
        CreateInputText::new(InputTextStyle::Paragraph, "description").required(false);
    desc_input = match description {
        Some(description) => desc_input.value(description),
        None => desc_input.placeholder(activity),
    };

    let activity = CreateInputText::new(InputTextStyle::Short, "activity").value(activity);
    let start_time_input = CreateInputText::new(InputTextStyle::Short, "start_time")
        .value(format!("{}", start_time.format("%Y-%m-%d %H:%M")));
    let fireteam_size = CreateInputText::new(InputTextStyle::Short, "fireteam_size")
        .value(fireteam_size.to_string());

    vec![
        CreateModalComponent::Label(CreateLabel::input_text("Activity", activity)),
        CreateModalComponent::Label(CreateLabel::input_text(
            format!("Start Time ({})", start_time.format("%Z")),
            start_time_input,
        )),
        CreateModalComponent::Label(CreateLabel::input_text("Fireteam Size", fireteam_size)),
        CreateModalComponent::Label(CreateLabel::input_text("Description", desc_input)),
    ]
}

fn start_time(timezone: Tz, start_time_str: &str) -> Result<DateTime<Tz>> {
    let naive_dt = NaiveDateTime::parse_from_str(start_time_str, "%Y-%m-%d %H:%M")
        .map_err(|_| Error::InvalidDateTime("YYYY-MM-DD HH:MM".to_string()))?;

    let st = timezone
        .from_local_datetime(&naive_dt)
        .single()
        .expect("Invalid date time");

    Ok(st)
}
