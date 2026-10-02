#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

/// A placeholder shape sized by `style/partials/skeleton.css` to match the
/// element it stands in for. Wrap repeated shapes in a `skeleton-grid`,
/// `skeleton-list` or `skeleton-stack` container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkeletonShape {
    Btn,
    Badge,
    Switcher,
    Row,
    Card,
    Panel,
}

impl SkeletonShape {
    pub const ALL: [Self; 6] =
        [Self::Btn, Self::Badge, Self::Switcher, Self::Row, Self::Card, Self::Panel];

    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Btn => "skeleton-btn",
            Self::Badge => "skeleton-badge",
            Self::Switcher => "skeleton-switcher",
            Self::Row => "skeleton-row",
            Self::Card => "skeleton-card",
            Self::Panel => "skeleton-panel",
        }
    }
}

#[component]
pub async fn skeleton(
    shape: SkeletonShape,
    #[default(1)] count: usize,
) -> Result<impl View> {
    let class = shape.class();

    Ok(view! {
        #[key(index)]
        for index in 0..count {
            <div class=(class) aria-hidden="true"></div>
        }
    })
}
