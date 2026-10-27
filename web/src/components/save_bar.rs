#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

use super::button::{ButtonSize, ButtonVariant, button_variants};
use super::flash::flash_message;
use crate::flash::Flash;

#[component]
pub async fn save_bar(
    #[default("Save changes")] label: &str,
    #[default] notice: Option<&Flash>,
) -> Result<impl View> {
    Ok(view! {
        <div class="save-bar" data-save-bar="">
            <div class="flex-1 min-w-[12rem] m-0 text-text-2" role="status">
                <span class="text-text font-semibold" data-dirty-status="" hidden="">
                    "Unsaved changes"
                </span>
                if let Some(notice) = notice {
                    flash_message(notice: notice)
                }
            </div>
            <button
                type="button"
                class=(button_variants(ButtonVariant::Ghost, ButtonSize::Md))
                data-discard=""
                hidden=""
            >
                "Discard"
            </button>
            <button
                type="submit"
                class=(button_variants(ButtonVariant::Primary, ButtonSize::Md))
                data-pending-label="Saving\u{2026}"
            >
                (label)
            </button>
        </div>
    })
}
