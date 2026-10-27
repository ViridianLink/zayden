#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{Child, View, attributes, component, view};

use super::alert::{AlertVariant, alert, alert_title};
use super::icons::{Icon, icon};
use super::lamp::{LampState, lamp};
use crate::nav::path_segment;

pub const NOT_SYNCED_NOTE: &str = "Not synced yet: this module's state appears once Zayden is in the server and has synced its commands.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModuleControl {
    Switch { on: bool },
    Status { on: bool },
    NotSynced,
}

const fn lamp_state(on: bool) -> LampState {
    if on { LampState::On } else { LampState::Off }
}

#[component]
pub async fn module_row(
    cx: &Cx,
    guild_id: &str,
    module_id: &str,
    label: &str,
    description: &str,
    control: ModuleControl,
    #[default] href: Option<&str>,
    #[default] error: Option<&str>,
) -> Result<impl View> {
    Ok(view! {
        <li
            class="grid grid-cols-[minmax(0,1fr)_auto_auto] items-center gap-x-4 min-h-(--row-h) py-3 px-4 border-t border-solid border-t-rule first:[border-top:0] max-md:grid-cols-[minmax(0,1fr)_auto] max-md:gap-y-2 max-md:min-h-[48px]"
        >
            <div class="min-w-0">
                <h3 class="m-0 text-[15px] font-semibold">
                    if let Some(href) = href {
                        <a
                            class="inline-flex items-center min-h-(--target-min) max-md:min-h-(--row-h) text-text no-underline decoration-current hover:underline"
                            (link_attrs(cx, href, prefetch_mode(cx)))
                        >
                            (label)
                        </a>
                    } else {
                        (label)
                    }
                </h3>
                <p class="m-0 text-[13px] text-text-2">(description)</p>
            </div>
            match control {
                ModuleControl::Switch { on } => {
                    <form
                        class="contents"
                        method="post"
                        action=(format!("/guild/{}", path_segment(guild_id)))
                        data-pending=""
                    >
                        <input type="hidden" name="guild" value=(guild_id)>
                        <input type="hidden" name="module_id" value=(module_id)>
                        <div class="min-w-[7rem] max-md:col-[1] max-md:row-[2]">
                            lamp(state: lamp_state(on), pending: Some("Saving\u{2026}"))
                        </div>
                        <div
                            class="flex items-center justify-end min-w-[48px] max-md:col-[2] max-md:row-[1/span_2]"
                        >
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
                    <div class="min-w-[7rem] max-md:col-[1] max-md:row-[2]">
                        lamp(state: lamp_state(on))
                    </div>
                    <div
                        class="flex items-center justify-end min-w-[48px] max-md:col-[2] max-md:row-[1/span_2]"
                    >
                        if let Some(href) = href {
                            <a
                                class="inline-flex items-center gap-1 min-h-(--target-min) text-text font-medium"
                                (link_attrs(cx, href, prefetch_mode(cx)))
                            >
                                "Manage"
                                icon(name: Icon::ChevronRight)
                            </a>
                        }
                    </div>
                }
                ModuleControl::NotSynced => {
                    <div class="min-w-[7rem] max-md:col-[1] max-md:row-[2]">
                        lamp(state: LampState::NotSynced)
                    </div>
                    <div
                        class="flex items-center justify-end min-w-[48px] max-md:col-[2] max-md:row-[1/span_2]"
                    ></div>
                    <p class="col-span-full m-0 mt-2 text-[13px] text-caution">
                        (NOT_SYNCED_NOTE)
                    </p>
                }
            }
            if let Some(error) = error {
                alert(
                    variant: AlertVariant::Destructive,
                    attrs: attributes! { class="col-span-full m-0 mt-2" role="alert" },
                    alert_title((error))
                )
            }
        </li>
    })
}

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
        <section class="mb-8" aria-labelledby=(id.as_str())>
            <h2 class="m-0 mb-2 label" id=(id.as_str())>(title)</h2>
            <ul class="rack">(child)</ul>
        </section>
    })
}
