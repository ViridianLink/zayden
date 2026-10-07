#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::view::{Child, View, component, view};

use super::icons::{Icon, icon};
use super::lamp::{LampState, lamp};

pub const NOT_SYNCED_NOTE: &str = "Not synced yet: this module's state appears once Zayden is in the server and has synced its commands.";

/// What a module row offers beside its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleControl {
    /// A command-backed module: a real switch posting to `/guild/{id}`.
    Switch { on: bool },
    /// A module whose state comes from its own page: status text and a link.
    Status { on: bool },
    /// No state yet: an amber lamp and the explanation, no switch.
    NotSynced,
}

const fn lamp_state(on: bool) -> LampState {
    if on { LampState::On } else { LampState::Off }
}

/// One module in the overview rack: name (a link when it has a page),
/// description, lamp with text, and the control for its kind.
#[component]
pub async fn module_row(
    guild_id: &str,
    module_id: &str,
    label: &str,
    description: &str,
    control: ModuleControl,
    #[default] href: Option<&str>,
    #[default] error: Option<&str>,
) -> Result<impl View> {
    Ok(view! {
        <li class="rack-row">
            <div class="rack-main">
                <h3 class="rack-name">
                    if let Some(href) = href {
                        <a href=(href)>(label)</a>
                    } else {
                        (label)
                    }
                </h3>
                <p class="rack-desc">(description)</p>
            </div>
            match control {
                ModuleControl::Switch { on } => {
                    <form
                        class="rack-form"
                        method="post"
                        action=(format!("/guild/{guild_id}"))
                        data-pending=""
                    >
                        <input type="hidden" name="guild" value=(guild_id)>
                        <input type="hidden" name="module_id" value=(module_id)>
                        <div class="rack-state">
                            lamp(state: lamp_state(on), pending: Some("Saving\u{2026}"))
                        </div>
                        <div class="rack-control">
                            <button
                                type="submit"
                                class="switch"
                                role="switch"
                                aria-checked=(if on { "true" } else { "false" })
                                aria-label=(format!("{label} module"))
                                name="enabled"
                                value=(if on { "false" } else { "true" })
                            >
                                <span class="switch-thumb" aria-hidden="true"></span>
                            </button>
                        </div>
                    </form>
                }
                ModuleControl::Status { on } => {
                    <div class="rack-state">lamp(state: lamp_state(on))</div>
                    <div class="rack-control">
                        if let Some(href) = href {
                            <a href=(href) class="rack-link">
                                "Manage"
                                icon(name: Icon::ChevronRight)
                            </a>
                        }
                    </div>
                }
                ModuleControl::NotSynced => {
                    <div class="rack-state">lamp(state: LampState::NotSynced)</div>
                    <div class="rack-control"></div>
                    <p class="rack-note">(NOT_SYNCED_NOTE)</p>
                }
            }
            if let Some(error) = error {
                <p class="rack-error" role="alert">(error)</p>
            }
        </li>
    })
}

/// A purpose group of the rack: a heading over its rows.
#[component]
pub async fn module_group(
    title: &str,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let id = format!(
        "rack-{}",
        title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            })
            .collect::<String>()
    );

    Ok(view! {
        <section class="rack-group" aria-labelledby=(id.as_str())>
            <h2 class="rack-heading label" id=(id.as_str())>(title)</h2>
            <ul class="rack">(child)</ul>
        </section>
    })
}
