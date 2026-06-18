use std::fmt;
use std::str::FromStr;

use google_sheets_api::types::sheet::CellData;
use serde::{Deserialize, Serialize};
use serenity::all::Colour;
use zayden_core::CoreError as ZaydenError;

use crate::EndgameAnalysisError;

pub const TIERS: [TierLabel; 7] = [
    TierLabel::S,
    TierLabel::A,
    TierLabel::B,
    TierLabel::C,
    TierLabel::D,
    TierLabel::E,
    TierLabel::F,
];

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Tier {
    pub tier: TierLabel,
    pub colour: Colour,
}

impl Tier {
    #[must_use]
    pub fn tier(&self) -> String {
        self.tier.to_string()
    }
}

impl TryFrom<CellData> for Tier {
    type Error = EndgameAnalysisError;

    fn try_from(value: CellData) -> Result<Self, Self::Error> {
        let tier = value
            .formatted_value
            .map(|s| s.parse().unwrap_or_default())
            .unwrap_or_default();
        let effective_format = value
            .effective_format
            .ok_or_else(|| ZaydenError::missing_data("effective_format"))?;
        let background_color_style = effective_format
            .background_color_style
            .ok_or_else(|| ZaydenError::missing_data("background_color_style"))?;
        let colour = background_color_style
            .rgb_color
            .ok_or_else(|| ZaydenError::missing_data("rgb_color"))?;

        Ok(Self { tier, colour: google_colour_to_serde_colour(&colour) })
    }
}

#[derive(
    Debug, Default, PartialEq, Eq, Hash, Clone, Copy, Deserialize, Serialize,
)]
pub enum TierLabel {
    S,
    A,
    B,
    C,
    D,
    E,
    F,
    #[default]
    None,
}

impl FromStr for TierLabel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S" => Ok(Self::S),
            "A" => Ok(Self::A),
            "B" => Ok(Self::B),
            "C" => Ok(Self::C),
            "D" => Ok(Self::D),
            "E" => Ok(Self::E),
            "F" => Ok(Self::F),
            _ => Err(()),
        }
    }
}

impl fmt::Display for TierLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::S => write!(f, "S"),
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
            Self::E => write!(f, "E"),
            Self::F => write!(f, "F"),
            Self::None => write!(f, "None"),
        }
    }
}

fn google_colour_to_serde_colour(
    colour: &google_sheets_api::types::common::Color,
) -> Colour {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamping and scaling guarantees the value is within [0.0, 255.0], making the cast to u8 safe"
    )]
    fn f64_to_u8(value: f64) -> u8 {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    Colour::from_rgb(
        f64_to_u8(colour.red),
        f64_to_u8(colour.green),
        f64_to_u8(colour.blue),
    )
}
