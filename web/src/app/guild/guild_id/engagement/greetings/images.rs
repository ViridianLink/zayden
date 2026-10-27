use greetings::GreetingImage;
use greetings::images::MAX_URL_LEN;
use topcoat::Result;
use topcoat::view::{View, ViewExt, component, view};
use url::Url;

use super::{GreetingAction, IMAGES, PAGE, REMOVE_IMAGE, add_image_form};
use crate::app::guild::guild_id::engagement::action::form_action;
use crate::app::guild::guild_id::engagement::state::PageState;
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::field_row::{form_summary, text_row};
use crate::components::flash::flash;
use crate::components::icons::{Icon, icon};
use crate::engagement::GreetingImageInfo;

const MAX_IMAGES: i64 = GreetingImage::MAX_PER_KIND;

const LINK_HELP: &str = "Links must start with https:// and point straight at an \
                         image. Zayden embeds the link rather than storing a \
                         copy, so an image that later disappears from its host \
                         shows up blank here and in Discord.";

#[component]
pub(super) async fn image_section(
    guild_id: &str,
    morning: &[GreetingImageInfo],
    night: &[GreetingImageInfo],
    state: &PageState,
) -> Result<impl View> {
    let removed = state.sent(REMOVE_IMAGE);

    Ok(view! {
        <section
            class="settings-section"
            id=(IMAGES)
            aria-labelledby="greeting-images-title"
        >
            <h2 class="label" id="greeting-images-title">"Images"</h2>
            <p class="page-lead">
                (format!(
                    "Each greeting picks one of its images at random. Up to {MAX_IMAGES} per greeting.",
                ))
            </p>
            flash(notice: state.notice_for(IMAGES))
            if let Some(message) = removed.summary() {
                form_summary(
                    form: REMOVE_IMAGE,
                    message: message,
                    outcome: "Not removed"
                )
            }
            image_list(
                guild_id: guild_id,
                kind: "morning",
                title: "Good morning images",
                images: morning,
                state: state
            )
            image_list(
                guild_id: guild_id,
                kind: "night",
                title: "Good night images",
                images: night,
                state: state
            )
        </section>
    }
    .boxed())
}

#[component]
async fn image_list(
    guild_id: &str,
    kind: &str,
    title: &str,
    images: &[GreetingImageInfo],
    state: &PageState,
) -> Result<impl View> {
    let form = add_image_form(kind);
    let sent = state.sent(form);
    let heading_id = format!("{kind}-images-title");
    let count = format!("{} of {MAX_IMAGES}", images.len());
    let noun = if kind == "night" { "good night" } else { "good morning" };

    Ok(view! {
        <h3 class="label" id=(heading_id.as_str())>
            (title)
            " "
            <span class="mono">(count)</span>
        </h3>
        <form
            method="post"
            action=(form_action(guild_id, PAGE, GreetingAction::AddImage))
            data-pending=""
            data-dirty-guard=""
        >
            if let Some(message) = sent.summary() {
                form_summary(form: form, message: message, outcome: "Not added")
            }
            <input type="hidden" name="guild" value=(guild_id)>
            <input type="hidden" name="kind" value=(kind)>
            text_row(
                form: form,
                name: "url",
                label: "Image link",
                value: sent.value("url", ""),
                help: Some(LINK_HELP),
                error: sent.error("url"),
                max_length: Some(MAX_URL_LEN),
                required: true,
                input_type: "url",
                placeholder: Some("https://example.com/sunrise.gif"),
                pattern: Some("https://.*")
            )
            <div class="form-actions">
                <button
                    type="submit"
                    class=(button_variants(ButtonVariant::Secondary, ButtonSize::Md))
                    data-pending-label="Adding\u{2026}"
                >
                    icon(name: Icon::Plus)
                    "Add image"
                </button>
            </div>
        </form>
        if images.is_empty() {
            <p class="page-lead">
                "No images yet - the command replies with just the message until you add one."
            </p>
        } else {
            <ul
                class="grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-4 mb-6"
                aria-labelledby=(heading_id.as_str())
            >
                #[key(image.id.as_str())]
                for image in images {
                    image_card(guild_id: guild_id, noun: noun, image: image)
                }
            </ul>
        }
    }
    .boxed())
}

fn host_of(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .unwrap_or_else(|| url.to_owned())
}

#[component]
async fn image_card(
    guild_id: &str,
    noun: &str,
    image: &GreetingImageInfo,
) -> Result<impl View> {
    let host = host_of(&image.url);
    let confirm_title = format!("Remove this {noun} image from {host}?");
    let trigger_label = format!("Remove {noun} image {}", image.url);

    Ok(view! {
        <li class="greet-card">
            <div
                class="relative grid place-items-center h-[9rem] overflow-hidden rounded-panel bg-surface-2"
                role="img"
                aria-label=(format!("Image from {host}"))
            >
                <span
                    class="py-0 px-3 text-[12px] text-center text-text-2 wrap-anywhere"
                    aria-hidden="true"
                >
                    (host.as_str())
                </span>
                <img
                    class="greet-thumb-img"
                    src=(image.url.as_str())
                    alt=""
                    aria-hidden="true"
                    loading="lazy"
                    data-host=(host.as_str())
                >
            </div>
            <a
                class="block truncate leading-(--target-min) max-md:leading-(--row-h) text-[12px] text-text-2 hover:text-text"
                href=(image.url.as_str())
                rel="external noreferrer"
                target="_blank"
                title=(image.url.as_str())
            >
                (image.url.as_str())
            </a>
            <form
                class="self-start"
                method="post"
                action=(form_action(guild_id, PAGE, GreetingAction::RemoveImage))
                data-pending=""
            >
                <input type="hidden" name="guild" value=(guild_id)>
                <input type="hidden" name="id" value=(image.id.as_str())>
                confirm_dialog(
                    trigger: "Remove",
                    title: &confirm_title,
                    description: Some(image.url.as_str()),
                    confirm: "Remove image",
                    trigger_label: Some(&trigger_label)
                )
            </form>
        </li>
    }
    .boxed())
}
