use image::imageops::FilterType;
use image::{GenericImageView, ImageFormat};

use crate::error::{JellyfinError, Result};

pub const TMDB_IMAGE_ROOT: &str = "https://image.tmdb.org/t/p/w500";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "hard" => Self::Hard,
            "medium" => Self::Medium,
            _ => Self::Easy,
        }
    }

    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::Easy => "The poster is blurred.",
            Self::Medium => "These are the poster's dominant colours.",
            Self::Hard => "This is a 50x50 crop of the poster's centre.",
        }
    }
}

#[must_use]
pub fn poster_url(poster_path: &str) -> String {
    format!("{TMDB_IMAGE_ROOT}{poster_path}")
}

pub fn derive(bytes: &[u8], difficulty: Difficulty) -> Result<Vec<u8>> {
    let image = image::load_from_memory(bytes).map_err(decode_err)?;

    let derived = match difficulty {
        Difficulty::Easy => image.blur(18.0),
        Difficulty::Medium => palette_strip(&image),
        Difficulty::Hard => {
            let (w, h) = image.dimensions();
            let size = 50.min(w).min(h);
            image.crop_imm(
                w.saturating_sub(size) / 2,
                h.saturating_sub(size) / 2,
                size,
                size,
            )
        },
    };

    let mut out = std::io::Cursor::new(Vec::new());
    derived.write_to(&mut out, ImageFormat::Png).map_err(decode_err)?;

    Ok(out.into_inner())
}

fn palette_strip(image: &image::DynamicImage) -> image::DynamicImage {
    const BANDS: u32 = 5;
    const BAND_PX: u32 = 100;

    let sampled = image.resize_exact(BANDS, 1, FilterType::Gaussian).to_rgba8();
    let mut strip = image::RgbaImage::new(BANDS * BAND_PX, BAND_PX);

    for (index, pixel) in sampled.pixels().enumerate() {
        let x0 = u32::try_from(index).unwrap_or(0) * BAND_PX;
        for x in x0..x0 + BAND_PX {
            for y in 0..BAND_PX {
                strip.put_pixel(x, y, *pixel);
            }
        }
    }

    image::DynamicImage::ImageRgba8(strip)
}

fn decode_err(e: impl std::fmt::Display) -> JellyfinError {
    JellyfinError::Internal(format!("poster processing failed: {e}"))
}
