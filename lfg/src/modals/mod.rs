pub mod create;
pub use create::{Create, GuildManager};

pub mod edit;
pub use edit::Edit;

use chrono::{DateTime, NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use serenity::all::{CreateActionRow, CreateInputText, InputTextStyle};

use crate::{Error, Result};

pub fn modal_components<'a>(
    activity: &'a str,
    start_time: DateTime<Tz>,
    fireteam_size: i16,
    description: Option<&'a str>,
) -> Vec<CreateActionRow<'a>> {
    let mut desc_input =
        CreateInputText::new(InputTextStyle::Paragraph, "Description", "description")
            .required(false);
    desc_input = match description {
        Some(description) => desc_input.value(description),
        None => desc_input.placeholder(activity),
    };

    vec![
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Short, "Activity", "activity").value(activity),
        ),
        CreateActionRow::InputText(
            CreateInputText::new(
                InputTextStyle::Short,
                format!("Start Time ({})", start_time.format("%Z")),
                "start time",
            )
            .value(format!("{}", start_time.format("%Y-%m-%d %H:%M"))),
        ),
        CreateActionRow::InputText(
            CreateInputText::new(InputTextStyle::Short, "Fireteam Size", "fireteam size")
                .value(fireteam_size.to_string()),
        ),
        CreateActionRow::InputText(desc_input),
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
