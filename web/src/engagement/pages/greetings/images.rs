use topcoat::Result;
use topcoat::view::{View, component, view};

use super::{GreetingAction, PAGE};
use crate::components::icons::{Icon, icon};
use crate::engagement::GreetingImageInfo;
use crate::engagement::pages::action::{Submitted, form_action, typed};

#[component]
pub(super) async fn image_section(
    guild_id: &str,
    kind: &str,
    title: &str,
    images: &[GreetingImageInfo],
    submitted: Option<&Submitted<GreetingAction>>,
) -> Result<impl View> {
    let add = form_action(guild_id, PAGE, GreetingAction::AddImage);
    let link = typed(submitted, GreetingAction::AddImage, "kind")
        .filter(|submitted_kind| *submitted_kind == kind)
        .and_then(|_| typed(submitted, GreetingAction::AddImage, "url"));

    Ok(view! {
        <fieldset class="settings-section">
            <legend>
                icon(name: Icon::Sparkles)
                (title)
            </legend>
            image_grid(guild_id: guild_id, images: images)
            <form method="post" action=(add) data-pending="">
                <input type="hidden" name="guild" value=(guild_id)>
                <input type="hidden" name="kind" value=(kind)>
                <div class="setting-field">
                    <label>"Image link"</label>
                    <input
                        class="input"
                        type="url"
                        name="url"
                        value=(link)
                        placeholder="https://example.com/sunrise.gif"
                        pattern="https://.*"
                        required=""
                    >
                </div>
                <div class="form-actions">
                    <button type="submit" class="btn btn-primary">
                        icon(name: Icon::Plus)
                        "Add image"
                    </button>
                </div>
            </form>
            <p class="page-lead">
                "Links must start with "
                <code>"https://"</code>
                " and point straight at an image. Zayden embeds the link rather than storing a copy, so an image that later disappears from its host shows up blank here and in Discord. Up to 50 per greeting."
            </p>
        </fieldset>
    })
}

#[component]
async fn image_grid(
    guild_id: &str,
    images: &[GreetingImageInfo],
) -> Result<impl View> {
    let remove = form_action(guild_id, PAGE, GreetingAction::RemoveImage);

    Ok(view! {
        if images.is_empty() {
            <div class="empty">
                "No images yet - the command will reply with just the message until you add one."
            </div>
        } else {
            <div class="greet-grid">
                #[key(index)]
                for (index, image) in images.iter().enumerate() {
                    <div class="greet-card">
                        <img
                            class="greet-thumb"
                            src=(image.url.as_str())
                            alt=""
                            loading="lazy"
                        >
                        <a
                            class="greet-url"
                            href=(image.url.as_str())
                            rel="external noreferrer"
                            target="_blank"
                            title=(image.url.as_str())
                        >
                            (image.url.as_str())
                        </a>
                        <form
                            class="greet-remove"
                            method="post"
                            action=(remove.as_str())
                            data-pending=""
                        >
                            <input type="hidden" name="guild" value=(guild_id)>
                            <input type="hidden" name="id" value=(image.id.as_str())>
                            <button type="submit" class="btn btn-ghost">
                                icon(name: Icon::X)
                                "Remove"
                            </button>
                        </form>
                    </div>
                }
            </div>
        }
    })
}
