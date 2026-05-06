use std::{fmt, str::FromStr};

use google_sheets_api::types::sheet::CellData;
use serde::{Deserialize, Serialize};
use serenity::all::Colour;

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
    pub fn tier(&self) -> String {
        self.tier.to_string()
    }
}

impl From<CellData> for Tier {
    fn from(value: CellData) -> Self {
        let tier = value
            .formatted_value
            .map(|s| s.parse().unwrap())
            .unwrap_or_default();
        let colour = value
            .effective_format
            .unwrap()
            .background_color_style
            .unwrap()
            .rgb_color
            .unwrap();

        Self {
            tier,
            colour: google_colour_to_serde_colour(colour),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy, Deserialize, Serialize)]
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
            "S" => Ok(TierLabel::S),
            "A" => Ok(TierLabel::A),
            "B" => Ok(TierLabel::B),
            "C" => Ok(TierLabel::C),
            "D" => Ok(TierLabel::D),
            "E" => Ok(TierLabel::E),
            "F" => Ok(TierLabel::F),
            _ => Err(()),
        }
    }
}

impl fmt::Display for TierLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TierLabel::S => write!(f, "S"),
            TierLabel::A => write!(f, "A"),
            TierLabel::B => write!(f, "B"),
            TierLabel::C => write!(f, "C"),
            TierLabel::D => write!(f, "D"),
            TierLabel::E => write!(f, "E"),
            TierLabel::F => write!(f, "F"),
            TierLabel::None => write!(f, "None"),
        }
    }
}

fn google_colour_to_serde_colour(
    colour: google_sheets_api::types::common::Color,
) -> serenity::all::Colour {
    fn f64_to_u8(value: f64) -> u8 {
        (value.clamp(0.0, 1.0) * 255.0).round() as u8
    }

    Colour::from_rgb(
        f64_to_u8(colour.red),
        f64_to_u8(colour.green),
        f64_to_u8(colour.blue),
    )
}
