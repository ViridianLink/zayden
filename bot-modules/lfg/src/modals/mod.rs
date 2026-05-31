pub mod create;
pub use create::{Create, GuildManager};

pub mod edit;
pub use edit::Edit;
use jiff::Zoned;
use jiff::tz::TimeZone;
use serenity::all::{
    CreateInputText,
    CreateLabel,
    CreateModalComponent,
    InputTextStyle,
};

use crate::{LfgError, Result};

#[must_use]
pub fn modal_components<'a>(
    activity: &'a str,
    start_time: &Zoned,
    fireteam_size: i16,
    description: Option<&'a str>,
) -> Vec<CreateModalComponent<'a>> {
    let mut desc_input =
        CreateInputText::new(InputTextStyle::Paragraph, "description")
            .required(false);
    desc_input = match description {
        Some(description) => desc_input.value(description),
        None => desc_input.placeholder(activity),
    };

    let activity =
        CreateInputText::new(InputTextStyle::Short, "activity").value(activity);
    let start_time_input = CreateInputText::new(InputTextStyle::Short, "start_time")
        .value(format!("{}", start_time.strftime("%Y-%m-%d %H:%M")));
    let fireteam_size = CreateInputText::new(InputTextStyle::Short, "fireteam_size")
        .value(fireteam_size.to_string());

    vec![
        CreateModalComponent::Label(CreateLabel::input_text("Activity", activity)),
        CreateModalComponent::Label(CreateLabel::input_text(
            format!("Start Time ({})", start_time.strftime("%Z")),
            start_time_input,
        )),
        CreateModalComponent::Label(CreateLabel::input_text(
            "Fireteam Size",
            fireteam_size,
        )),
        CreateModalComponent::Label(CreateLabel::input_text(
            "Description",
            desc_input,
        )),
    ]
}

fn start_time(timezone: TimeZone, start_time_str: &str) -> Result<Zoned> {
    let naive_dt = jiff::fmt::strtime::parse("%Y-%m-%d %H:%M", start_time_str)
        .and_then(|bdt| bdt.to_datetime())
        .map_err(|_e| LfgError::InvalidDateTime("YYYY-MM-DD HH:MM".to_string()))?;

    let st = naive_dt.to_zoned(timezone).map_err(|_e| {
        LfgError::InvalidDateTime("Date invalid in this timezone".to_string())
    })?;

    Ok(st)
}
