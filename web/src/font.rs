#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::asset::asset_config;
use topcoat::context::Cx;
use topcoat::font::fontsource::fontsource_font;
use topcoat::font::{
    Font,
    FontSource,
    FontSourceUrl,
    FontWeightRange,
    link,
    preload_link,
};
use topcoat::view::{View, component, view};

pub const GEIST: Font =
    fontsource_font!(GEIST, weight: [400, 500, 600], style: Normal, host: Asset);

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
