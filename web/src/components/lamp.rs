#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LampState {
    On,
    Off,
    NotSynced,
}

impl LampState {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::On => {
                "lamp-on inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full)"
            },
            Self::Off => {
                "inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full) border-2 border-solid border-line bg-transparent"
            },
            Self::NotSynced => {
                "lamp-sync inline-block w-[10px] h-[10px] shrink-0 box-border rounded-(--radius-full)"
            },
        }
    }

    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::On => "On",
            Self::Off => "Off",
            Self::NotSynced => "Not synced",
        }
    }
}

#[component]
pub async fn lamp(
    state: LampState,
    #[default] text: Option<&str>,
    #[default] pending: Option<&str>,
) -> Result<impl View> {
    let text = text.unwrap_or_else(|| state.text());

    Ok(view! {
        <span class="inline-flex items-center gap-2 whitespace-nowrap">
            <span class=(state.class()) aria-hidden="true"></span>
            <span class="lamp-text" data-pending-text=(pending)>(text)</span>
        </span>
    })
}
