use leptos::form::ActionForm;
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_params_map;

use crate::dto::{GreetingImageInfo, GreetingsView};
use crate::server::greetings::{
    AddGreetingImage,
    RemoveGreetingImage,
    SaveGreetingMessages,
    get_greetings,
};
use crate::ui::components::icons::Icon;
use crate::ui::components::layout::AppShell;
use crate::ui::components::settings::{SaveButton, SettingField, save_feedback};

const ANY_TEXT: &str = ".*";

#[component]
pub(crate) fn GreetingsPage() -> impl IntoView {
    let params = use_params_map();
    let guild_id = move || params.with(|p| p.get("id").unwrap_or_default());

    let save = ServerAction::<SaveGreetingMessages>::new();
    let add = ServerAction::<AddGreetingImage>::new();
    let remove = ServerAction::<RemoveGreetingImage>::new();

    let data = Resource::new_blocking(
        move || {
            (
                guild_id(),
                save.version().get(),
                add.version().get(),
                remove.version().get(),
            )
        },
        |(gid, ..)| async move { get_greetings(gid).await },
    );

    let save_result = save.value();
    let add_result = add.value();
    let remove_result = remove.value();

    view! {
        <Title text="Greetings - Zayden Dashboard"/>
        <AppShell>
            <div class="page">
                <div class="page-header">
                    <div>
                        <h1>"Greetings"</h1>
                        <p class="page-lead">
                            "What Zayden posts for "<code>"/good morning"</code>" and "
                            <code>"/good night"</code>". Each subcommand replies with one "
                            "image picked at random from its list, plus the message "
                            "below if you set one."
                        </p>
                    </div>
                </div>
                <Suspense fallback=|| view! {
                    <p class="loading">"Loading greetings\u{2026}"</p>
                }>
                    {move || data.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="error">"Failed to load greetings: " {e.to_string()}</p>
                        }.into_any(),
                        Ok(view) => {
                            let GreetingsView {
                                morning_message,
                                night_message,
                                morning,
                                night,
                            } = view;
                            let gid = guild_id();
                            let form_gid = gid.clone();
                            let morning_gid = gid.clone();

                            view! {
                                <fieldset class="settings-section">
                                    <legend><Icon name="message"/>"Messages"</legend>
                                    {move || save_result.get().map(save_feedback)}
                                    <ActionForm action=save>
                                        <input type="hidden" name="guild" value=form_gid/>
                                        <SettingField
                                            label="Good morning message"
                                            name="morning_message"
                                            value=morning_message
                                            pattern=ANY_TEXT
                                        />
                                        <SettingField
                                            label="Good night message"
                                            name="night_message"
                                            value=night_message
                                            pattern=ANY_TEXT
                                        />
                                        <PlaceholderLegend/>
                                        <SaveButton/>
                                    </ActionForm>
                                </fieldset>

                                {move || add_result.get().map(save_feedback)}
                                {move || remove_result.get().map(save_feedback)}

                                <ImageSection
                                    guild_id=morning_gid
                                    kind="morning"
                                    title="Good morning images"
                                    images=morning
                                    add=add
                                    remove=remove
                                />
                                <ImageSection
                                    guild_id=gid
                                    kind="night"
                                    title="Good night images"
                                    images=night
                                    add=add
                                    remove=remove
                                />
                            }.into_any()
                        },
                    })}
                </Suspense>
            </div>
        </AppShell>
    }
}

#[component]
fn PlaceholderLegend() -> impl IntoView {
    view! {
        <ul class="greet-legend">
            <li>
                <code>"{user}"</code>
                " \u{2014} mentions the person being greeted, or the sender when "
                "the command is run without a user."
            </li>
            <li>
                <code>"{author}"</code>
                " \u{2014} mentions whoever ran the command."
            </li>
            <li>"Leave a message blank to post just the image."</li>
        </ul>
    }
}

#[component]
fn ImageSection(
    guild_id: String,
    kind: &'static str,
    title: &'static str,
    images: Vec<GreetingImageInfo>,
    add: ServerAction<AddGreetingImage>,
    remove: ServerAction<RemoveGreetingImage>,
) -> impl IntoView {
    let add_gid = guild_id.clone();

    view! {
        <fieldset class="settings-section">
            <legend><Icon name="sparkles"/>{title}</legend>
            <ImageGrid guild_id=guild_id images=images remove=remove/>
            <ActionForm action=add>
                <input type="hidden" name="guild" value=add_gid/>
                <input type="hidden" name="kind" value=kind/>
                <div class="setting-field">
                    <label>"Image link"</label>
                    <input
                        type="url"
                        name="url"
                        placeholder="https://example.com/sunrise.gif"
                        pattern="https://.*"
                        required
                    />
                </div>
                <div class="form-actions">
                    <button type="submit" class="btn btn-primary">
                        <Icon name="plus"/>
                        "Add image"
                    </button>
                </div>
            </ActionForm>
            <p class="page-lead">
                "Links must start with "<code>"https://"</code>" and point straight "
                "at an image. Zayden embeds the link rather than storing a copy, so "
                "an image that later disappears from its host shows up blank here "
                "and in Discord. Up to 50 per greeting."
            </p>
        </fieldset>
    }
}

#[component]
fn ImageGrid(
    guild_id: String,
    images: Vec<GreetingImageInfo>,
    remove: ServerAction<RemoveGreetingImage>,
) -> impl IntoView {
    if images.is_empty() {
        return view! {
            <div class="empty">
                "No images yet \u{2014} the command will reply with just the "
                "message until you add one."
            </div>
        }
        .into_any();
    }

    let cards = images
        .into_iter()
        .map(|image| {
            let gid = guild_id.clone();
            let src = image.url.clone();
            let href = image.url.clone();
            let title = image.url.clone();

            view! {
                <div class="greet-card">
                    <img class="greet-thumb" src=src alt="" loading="lazy"/>
                    <a
                        class="greet-url"
                        href=href
                        rel="external noreferrer"
                        target="_blank"
                        title=title
                    >
                        {image.url}
                    </a>
                    <ActionForm action=remove attr:class="greet-remove">
                        <input type="hidden" name="guild" value=gid/>
                        <input type="hidden" name="id" value=image.id/>
                        <button type="submit" class="btn btn-ghost">
                            <Icon name="x"/>
                            "Remove"
                        </button>
                    </ActionForm>
                </div>
            }
        })
        .collect_view();

    view! { <div class="greet-grid">{cards}</div> }.into_any()
}
