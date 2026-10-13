#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::asset::{asset, asset_config};
use topcoat::context::Cx;
use topcoat::font::{
    Font,
    FontSource,
    FontSourceUrl,
    FontWeightRange,
    font,
    link,
    preload_link,
};
use topcoat::view::{View, component, view};

pub const GEIST: Font = font! {
    "Geist",
    @font-face {
        src: url(asset!("../assets/fonts/geist/geist-latin-400-normal.woff2")) format("woff2");
        font-weight: 400;
        font-style: normal;
        font-display: swap;
        unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC,
            U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212,
            U+2215, U+FEFF, U+FFFD;
    }
    @font-face {
        src: url(asset!("../assets/fonts/geist/geist-latin-500-normal.woff2")) format("woff2");
        font-weight: 500;
        font-style: normal;
        font-display: swap;
        unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC,
            U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212,
            U+2215, U+FEFF, U+FFFD;
    }
    @font-face {
        src: url(asset!("../assets/fonts/geist/geist-latin-600-normal.woff2")) format("woff2");
        font-weight: 600;
        font-style: normal;
        font-display: swap;
        unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC,
            U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212,
            U+2215, U+FEFF, U+FFFD;
    }
};

const REGULAR: FontWeightRange = FontWeightRange::from_u16(400, 400);

fn bundled_regular(cx: &Cx) -> Option<FontSource> {
    let assets = asset_config(cx);

    GEIST
        .faces()
        .as_slice()
        .iter()
        .filter(|face| face.weight() == Some(REGULAR))
        .filter_map(|face| face.src().as_slice().first())
        .find(|source| match source {
            FontSource::Url { url: FontSourceUrl::Asset(asset), .. } => {
                assets.get(*asset).is_some()
            },
            FontSource::Url { .. } | FontSource::Local { .. } => false,
        })
        .cloned()
}

#[component]
pub async fn font_head(cx: &Cx) -> Result<impl View> {
    let regular = bundled_regular(cx);

    Ok(view! {
        if let Some(source) = regular {
            preload_link(source: &source)
        }
        link(font: GEIST, preload: false)
    })
}
