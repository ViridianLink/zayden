use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use tracing::warn;

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Genre {
    Pop,
    Rock,
    HipHop,
    Electronic,
    Dance,
    Jazz,
    Classical,
    Metal,
    Indie,
    RnB,
    Country,
    Reggae,
    Latin,
    FunkSoul,
    LoFi,
    Oldies,
    Chill,
    Relax,
    Energy,
    Focus,
    Party,
    Sleep,
    Romance,
    FeelGood,
}

impl Genre {
    pub const ALL: [Self; 24] = [
        Self::Pop,
        Self::Rock,
        Self::HipHop,
        Self::Electronic,
        Self::Dance,
        Self::Jazz,
        Self::Classical,
        Self::Metal,
        Self::Indie,
        Self::RnB,
        Self::Country,
        Self::Reggae,
        Self::Latin,
        Self::FunkSoul,
        Self::LoFi,
        Self::Oldies,
        Self::Chill,
        Self::Relax,
        Self::Energy,
        Self::Focus,
        Self::Party,
        Self::Sleep,
        Self::Romance,
        Self::FeelGood,
    ];

    const fn spec(self) -> (&'static str, &'static str) {
        match self {
            Self::Pop => ("Pop", "pop"),
            Self::Rock => ("Rock", "rock"),
            Self::HipHop => ("Hip-Hop", "hip-hop"),
            Self::Electronic => ("Electronic", "electronic"),
            Self::Dance => ("Dance", "dance"),
            Self::Jazz => ("Jazz", "jazz"),
            Self::Classical => ("Classical", "classical"),
            Self::Metal => ("Metal", "metal"),
            Self::Indie => ("Indie", "indie"),
            Self::RnB => ("R&B", "rnb"),
            Self::Country => ("Country", "country"),
            Self::Reggae => ("Reggae", "reggae"),
            Self::Latin => ("Latin", "latin"),
            Self::FunkSoul => ("Funk & Soul", "funk-soul"),
            Self::LoFi => ("Lo-Fi", "lofi"),
            Self::Oldies => ("Oldies", "oldies"),
            Self::Chill => ("Chill", "chill"),
            Self::Relax => ("Relax", "relax"),
            Self::Energy => ("Energy", "energy"),
            Self::Focus => ("Focus", "focus"),
            Self::Party => ("Party", "party"),
            Self::Sleep => ("Sleep", "sleep"),
            Self::Romance => ("Romance", "romance"),
            Self::FeelGood => ("Feel-Good", "feel-good"),
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        self.spec().0
    }

    #[must_use]
    pub const fn value(self) -> &'static str {
        self.spec().1
    }

    #[must_use]
    pub fn from_value(raw: &str) -> Option<Self> {
        let raw = raw.trim();

        Self::ALL.into_iter().find(|genre| {
            genre.value().eq_ignore_ascii_case(raw)
                || genre.label().eq_ignore_ascii_case(raw)
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RadioStation {
    pub id: String,
    pub name: String,
    pub stream_url: String,
    pub genre: String,
    pub homepage: Option<String>,
    pub logo_url: Option<String>,
}

impl RadioStation {
    #[must_use]
    pub fn display_url(&self) -> &str {
        self.homepage.as_deref().unwrap_or(&self.stream_url)
    }

    #[must_use]
    pub fn genre(&self) -> Option<Genre> {
        Genre::from_value(&self.genre)
    }
}

#[derive(Debug, Default, Deserialize)]
struct RadioToml {
    #[serde(default)]
    stations: Vec<RadioStation>,
}

pub fn load() -> Result<Vec<RadioStation>> {
    let path = if Path::new("radio.toml").exists() {
        Path::new("radio.toml")
    } else if Path::new("bot/radio.toml").exists() {
        Path::new("bot/radio.toml")
    } else {
        return Ok(Vec::new());
    };

    let content = std::fs::read_to_string(path)?;
    let cfg: RadioToml = toml::from_str(&content)?;

    Ok(cfg.stations)
}

fn is_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[must_use]
pub fn validate_all(stations: Vec<RadioStation>) -> Arc<[RadioStation]> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut valid: Vec<RadioStation> = Vec::with_capacity(stations.len());

    for station in stations {
        if station.id.trim().is_empty() || station.name.trim().is_empty() {
            warn!(
                id = %station.id,
                "radio station dropped: id and name must both be non-empty"
            );
            continue;
        }

        if !is_http_url(&station.stream_url) {
            warn!(
                id = %station.id,
                "radio station dropped: stream_url must be an http(s) URL"
            );
            continue;
        }

        if station.genre().is_none() {
            warn!(
                id = %station.id,
                genre = %station.genre,
                "radio station dropped: unrecognised genre tag"
            );
            continue;
        }

        if !seen.insert(station.id.clone()) {
            warn!(id = %station.id, "radio station dropped: duplicate id");
            continue;
        }

        valid.push(station);
    }

    valid.into()
}

#[must_use]
pub fn find<'a>(stations: &'a [RadioStation], id: &str) -> Option<&'a RadioStation> {
    stations.iter().find(|station| station.id == id)
}

#[must_use]
pub fn pool(stations: &[RadioStation], genre: Genre) -> Vec<&RadioStation> {
    stations.iter().filter(|station| station.genre() == Some(genre)).collect()
}

#[must_use]
pub fn unbacked(stations: &[RadioStation]) -> Vec<Genre> {
    Genre::ALL
        .into_iter()
        .filter(|genre| pool(stations, *genre).is_empty())
        .collect()
}
