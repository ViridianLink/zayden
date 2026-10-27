#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::view::{View, class, component, element_id, view};

use super::button::{ButtonSize, ButtonVariant, button_variants};

#[component]
pub async fn confirm_dialog(
    cx: &Cx,
    trigger: &str,
    title: &str,
    confirm: &str,
    #[default] description: Option<&str>,
    #[default(ButtonVariant::Ghost)] trigger_variant: ButtonVariant,
    #[default] form: Option<&str>,
    #[default("Cancel")] cancel: &str,
    #[default] trigger_label: Option<&str>,
) -> Result<impl View> {
    let title_id = element_id(cx);
    let desc_id = description.map(|_| element_id(cx));

    Ok(view! {
        <div class="confirm" data-confirm="">
            <button
                type="submit"
                class=(button_variants(trigger_variant, ButtonSize::Md))
                form=(form)
                aria-label=(trigger_label)
                data-confirm-trigger=""
            >
                (trigger)
            </button>
            <dialog
                class="dialog"
                aria-labelledby=(title_id.as_str())
                aria-describedby=(desc_id.as_deref())
                data-confirm-dialog=""
            >
                <div class="dialog-panel">
                    <h2 class="dialog-title" id=(title_id.as_str())>(title)</h2>
                    if let Some(description) = description {
                        <p class="dialog-desc" id=(desc_id.as_deref())>(description)</p>
                    }
                    <div class="dialog-actions">
                        <button
                            type="button"
                            class=(class!(
                                button_variants(ButtonVariant::Secondary, ButtonSize::Md),
                                "max-md:flex-1",
                            ))
                            data-dialog-close=""
                            autofocus=""
                        >
                            (cancel)
                        </button>
                        <button
                            type="submit"
                            class=(class!(
                                button_variants(ButtonVariant::Destructive, ButtonSize::Md),
                                "max-md:flex-1",
                            ))
                            form=(form)
                        >
                            (confirm)
                        </button>
                    </div>
                </div>
            </dialog>
        </div>
    })
}
