#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{View, component, view};

use super::icons::{Icon, icon};

#[component]
pub async fn key_list_field(
    label: &str,
    keys: &[String],
    list: &str,
    max: usize,
) -> Result<impl View> {
    let full = keys.len() >= max;

    Ok(view! {
        <div class="setting-field">
            <label>(label)</label>
            <div class="chip-list">
                #[key(index)]
                for (index, key) in keys.iter().enumerate() {
                    <span class="chip">
                        <span class="chip-label">(key.as_str())</span>
                        <button type="button" class="chip-remove" title="Remove">
                            icon(name: Icon::X)
                        </button>
                    </span>
                }
            </div>
            <div class="chip-add">
                <input class="input" list=(list)>
                <button type="button" class="btn btn-secondary" disabled=(full)>
                    "Add"
                </button>
            </div>
        </div>
    })
}
