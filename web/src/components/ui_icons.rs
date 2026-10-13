use topcoat::icon::IconData;
use topcoat::view::svg::ViewBox;

use super::icons::glyph;

macro_rules! stroked {
    ($($body:tt)*) => {
        IconData::unescaped_unchecked(
            ViewBox::new(0.0, 0.0, 24.0, 24.0),
            concat!(
                r#"<g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2">"#,
                $($body)*,
                "</g>"
            ),
        )
    };
}

pub const CHECK: IconData = stroked!(glyph!(check));

pub const CHEVRON_DOWN: IconData = stroked!(glyph!(chevron_down));

pub const CHEVRON_LEFT: IconData = stroked!(r#"<path d="m15 18-6-6 6-6"/>"#);

pub const CHEVRON_RIGHT: IconData = stroked!(glyph!(chevron_right));

pub const ELLIPSIS: IconData = stroked!(
    r#"<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>"#
);

pub const LOADER_CIRCLE: IconData =
    stroked!(r#"<path d="M21 12a9 9 0 1 1-6.219-8.56"/>"#);
