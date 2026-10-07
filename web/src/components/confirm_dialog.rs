#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

#[component]
pub async fn confirm_dialog(
    id: &str,
    trigger: &str,
    title: &str,
    confirm: &str,
    #[default] description: Option<&str>,
    #[default("btn btn-ghost")] trigger_class: &str,
    #[default] form: Option<&str>,
    #[default("Cancel")] cancel: &str,
) -> Result<impl View> {
    let title_id = format!("{id}-title");
    let desc_id = description.map(|_| format!("{id}-desc"));

    Ok(view! {
        <div class="confirm" data-confirm="">
            <button
                type="submit"
                class=(trigger_class)
                form=(form)
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
                            class="btn btn-secondary"
                            data-dialog-close=""
                            autofocus=""
                        >
                            (cancel)
                        </button>
                        <button type="submit" class="btn btn-danger" form=(form)>
                            (confirm)
                        </button>
                    </div>
                </div>
            </dialog>
        </div>
    })
}
