#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

use super::icons::{Icon, icon};
use crate::flash::{Flash, FlashKind};

#[component]
pub async fn flash_message(notice: &Flash) -> Result<impl View> {
    let (class, mark) = match notice.kind {
        FlashKind::Success => ("flash flash-success", Icon::Check),
        FlashKind::Error => ("flash flash-error", Icon::X),
    };

    Ok(view! {
        <p class=(class) data-flash="">
            icon(name: mark)
            <span class="flash-text">(notice.message.as_str())</span>
        </p>
    })
}

#[component]
pub async fn flash(#[default] notice: Option<&Flash>) -> Result<impl View> {
    Ok(view! {
        <div class="flash-region" role="status">
            if let Some(notice) = notice {
                flash_message(notice: notice)
            }
        </div>
    })
}
