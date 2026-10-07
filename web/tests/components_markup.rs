use std::error::Error;

use http_body_util::BodyExt;
use topcoat::Result as ViewResult;
use topcoat::router::request::Request;
use topcoat::router::{Body, Router, RouterBuilder, page};
use topcoat::runtime::RouterBuilderRuntimeExt;
use topcoat::view::{View, view};
use twilight_model::channel::ChannelType;
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use twilight_model::util::ImageHash;
use web::auth::{ChannelInfo, ForumTagInfo, RoleInfo};
use web::components::confirm::confirm_button;
use web::components::confirm_dialog::confirm_dialog;
use web::components::guild_grid::{GuildCard, guild_grid};
use web::components::icons::{Icon, icon, module_icon, module_tint};
use web::components::key_list::key_list_field;
use web::components::pickers::{
    Channel,
    ForumTag,
    Role,
    SelectOption,
    channel_select,
    forum_tag_select,
    role_select,
    select_field,
};
use web::components::settings::{
    alert,
    create_feedback,
    delete_feedback,
    save_button,
    save_feedback,
    setting_field,
    toggle_field,
};
use web::components::shape_skeleton::{SkeletonShape, shape_skeleton};

/// Every icon's path markup, in `Icon::ALL` order.
const ICON_PATHS: [(Icon, &str); 26] = [
    (
        Icon::Server,
        r#"<rect width="20" height="8" x="2" y="2" rx="2"/><rect width="20" height="8" x="2" y="14" rx="2"/><line x1="6" x2="6.01" y1="6" y2="6"/><line x1="6" x2="6.01" y1="18" y2="18"/>"#,
    ),
    (
        Icon::Settings,
        r#"<line x1="21" x2="14" y1="4" y2="4"/><line x1="10" x2="3" y1="4" y2="4"/><line x1="21" x2="12" y1="12" y2="12"/><line x1="8" x2="3" y1="12" y2="12"/><line x1="21" x2="16" y1="20" y2="20"/><line x1="12" x2="3" y1="20" y2="20"/><line x1="14" x2="14" y1="2" y2="6"/><line x1="8" x2="8" y1="10" y2="14"/><line x1="16" x2="16" y1="18" y2="22"/>"#,
    ),
    (
        Icon::Grid,
        r#"<rect width="7" height="7" x="3" y="3" rx="1"/><rect width="7" height="7" x="14" y="3" rx="1"/><rect width="7" height="7" x="14" y="14" rx="1"/><rect width="7" height="7" x="3" y="14" rx="1"/>"#,
    ),
    (Icon::ChevronDown, r#"<path d="m6 9 6 6 6-6"/>"#),
    (
        Icon::Grip,
        r#"<circle cx="9" cy="6" r="1"/><circle cx="15" cy="6" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="9" cy="18" r="1"/><circle cx="15" cy="18" r="1"/>"#,
    ),
    (Icon::ChevronRight, r#"<path d="m9 18 6-6-6-6"/>"#),
    (Icon::ArrowRight, r#"<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>"#),
    (
        Icon::ExternalLink,
        r#"<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>"#,
    ),
    (
        Icon::LogOut,
        r#"<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" x2="9" y1="12" y2="12"/>"#,
    ),
    (Icon::Plus, r#"<path d="M5 12h14"/><path d="M12 5v14"/>"#),
    (Icon::X, r#"<path d="M18 6 6 18"/><path d="m6 6 12 12"/>"#),
    (Icon::Check, r#"<path d="M20 6 9 17l-5-5"/>"#),
    (
        Icon::Zap,
        r#"<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"/>"#,
    ),
    (
        Icon::Shield,
        r#"<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/>"#,
    ),
    (
        Icon::Message,
        r#"<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>"#,
    ),
    (
        Icon::Users,
        r#"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>"#,
    ),
    (
        Icon::Gauge,
        r#"<path d="m12 14 4-4"/><path d="M3.34 19a10 10 0 1 1 17.32 0"/>"#,
    ),
    (
        Icon::Music,
        r#"<path d="M9 18V5l12-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="18" cy="16" r="3"/>"#,
    ),
    (
        Icon::Mic,
        r#"<path d="M12 19v3"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><rect x="9" y="2" width="6" height="13" rx="3"/>"#,
    ),
    (
        Icon::Gamepad,
        r#"<line x1="6" x2="10" y1="11" y2="11"/><line x1="8" x2="8" y1="9" y2="13"/><line x1="15" x2="15.01" y1="12" y2="12"/><line x1="18" x2="18.01" y1="10" y2="10"/><path d="M17.32 5H6.68a4 4 0 0 0-3.978 3.59c-.006.052-.01.101-.017.152C2.604 9.416 2 14.456 2 16a3 3 0 0 0 3 3c1 0 1.5-.5 2-1l1.414-1.414A2 2 0 0 1 9.828 16h4.344a2 2 0 0 1 1.414.586L17 18c.5.5 1 1 2 1a3 3 0 0 0 3-3c0-1.545-.604-6.584-.685-7.258-.007-.05-.011-.1-.017-.151A4 4 0 0 0 17.32 5z"/>"#,
    ),
    (
        Icon::Trophy,
        r#"<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6"/><path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18"/><path d="M4 22h16"/><path d="M10 14.66V17c0 .55-.47.98-.97 1.21C7.85 18.75 7 20.24 7 22"/><path d="M14 14.66V17c0 .55.47.98.97 1.21C16.15 18.75 17 20.24 17 22"/><path d="M18 2H6v7a6 6 0 0 0 12 0V2Z"/>"#,
    ),
    (
        Icon::Dice,
        r#"<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M16 8h.01"/><path d="M8 8h.01"/><path d="M8 16h.01"/><path d="M16 16h.01"/><path d="M12 12h.01"/>"#,
    ),
    (
        Icon::Heart,
        r#"<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>"#,
    ),
    (
        Icon::Ticket,
        r#"<path d="M2 9a3 3 0 0 1 0 6v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2a3 3 0 0 1 0-6V7a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2Z"/><path d="M13 5v2"/><path d="M13 17v2"/><path d="M13 11v2"/>"#,
    ),
    (
        Icon::Sparkles,
        r#"<path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .962 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.962 0z"/><path d="M20 3v4"/><path d="M22 5h-4"/><path d="M4 17v2"/><path d="M5 18H3"/>"#,
    ),
    (
        Icon::Menu,
        r#"<line x1="4" x2="20" y1="12" y2="12"/><line x1="4" x2="20" y1="6" y2="6"/><line x1="4" x2="20" y1="18" y2="18"/>"#,
    ),
];

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const CHEVRON_DOWN: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>"#;
const SVG_OPEN: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">"#;
const X: &str = r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;

async fn render_raw(builder: RouterBuilder, path: &str) -> TestResult<String> {
    let router = builder.runtime().build();
    let response = router.handle(Request::get(path).body(Body::empty())?).await;
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

/// Rendered HTML with comments and runtime binding attributes removed and
/// boolean attributes written bare.
async fn render(builder: RouterBuilder, path: &str) -> TestResult<String> {
    Ok(normalize(&render_raw(builder, path).await?))
}

const BOOLEAN_ATTRIBUTES: [&str; 6] =
    ["selected", "disabled", "hidden", "open", "checked", "required"];

fn normalize(html: &str) -> String {
    let html = strip_between(html, "<!--", |rest| {
        rest.split_once("-->").map_or("", |(_, tail)| tail)
    });
    let html = strip_between(&html, " data-topcoat-", |rest| {
        rest.split_once("=\"")
            .and_then(|(_, value)| value.split_once('"'))
            .map_or("", |(_, tail)| tail)
    });

    BOOLEAN_ATTRIBUTES.iter().fold(html, |html, name| {
        html.replace(&format!(" {name}=\"\""), &format!(" {name}"))
    })
}

/// Drops every `marker` and whatever `skip` consumes after it.
fn strip_between(html: &str, marker: &str, skip: impl Fn(&str) -> &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((before, after)) = rest.split_once(marker) {
        out.push_str(before);
        rest = skip(after);
    }
    out.push_str(rest);
    out
}

fn channel(id: &str, name: &str, kind: ChannelType) -> Channel {
    Channel { id: id.to_owned(), name: name.to_owned(), kind, tags: Vec::new() }
}

fn options() -> Vec<SelectOption> {
    vec![
        SelectOption { value: "10".to_owned(), label: "# general".to_owned() },
        SelectOption { value: "20".to_owned(), label: "# rules".to_owned() },
    ]
}

#[page("/confirm/default")]
async fn confirm_default() -> ViewResult<impl View> {
    Ok(view! {
        confirm_button(
            id: "faq-1-delete",
            label: "Delete",
            prompt: "Delete this article?",
            confirm: "Delete article"
        )
    })
}

#[page("/confirm/ghost")]
async fn confirm_ghost() -> ViewResult<impl View> {
    Ok(view! {
        confirm_button(
            id: "rr-0-remove",
            label: "Remove",
            prompt: "Remove this mapping?",
            confirm: "Remove mapping",
            class: "btn btn-ghost"
        )
    })
}

#[tokio::test]
async fn confirm_button_opens_a_modal_dialog_named_and_described_by_its_text() {
    let html = render(Router::builder().page(confirm_default), "/confirm/default")
        .await
        .unwrap();

    assert_eq!(
        html,
        concat!(
            r#"<div class="confirm" data-confirm="">"#,
            r#"<button type="submit" class="btn btn-danger" data-confirm-trigger="">Delete</button>"#,
            r#"<dialog class="dialog" aria-labelledby="faq-1-delete-title" aria-describedby="faq-1-delete-desc" data-confirm-dialog="">"#,
            r#"<div class="dialog-panel"><h2 class="dialog-title" id="faq-1-delete-title">Delete article?</h2>"#,
            r#"<p class="dialog-desc" id="faq-1-delete-desc">Delete this article?</p><div class="dialog-actions">"#,
            r#"<button type="button" class="btn btn-secondary" data-dialog-close="" autofocus="">Cancel</button>"#,
            r#"<button type="submit" class="btn btn-danger">Delete article</button>"#,
            r#"</div></div></dialog></div>"#,
        )
    );
}

#[tokio::test]
async fn confirm_button_class_styles_only_the_trigger() {
    let html = render(Router::builder().page(confirm_ghost), "/confirm/ghost")
        .await
        .unwrap();

    assert!(html.contains(
        r#"<button type="submit" class="btn btn-ghost" data-confirm-trigger="">Remove</button>"#
    ));
    assert!(html.contains(
        r#"<button type="submit" class="btn btn-danger">Remove mapping</button>"#
    ));
    assert!(!html.contains(" open"), "the dialog renders closed");
}

#[page("/confirm/named")]
async fn confirm_named() -> ViewResult<impl View> {
    Ok(view! {
        confirm_button(
            id: "rr-1-remove",
            label: "Remove",
            prompt: "Reactions already on the message stay.",
            confirm: "Remove role",
            object: Some("@Helpers")
        )
    })
}

#[tokio::test]
async fn confirm_button_names_the_object_in_the_dialog_title() {
    let html = render(Router::builder().page(confirm_named), "/confirm/named")
        .await
        .unwrap();

    assert!(html.contains(
        r#"<dialog class="dialog" aria-labelledby="rr-1-remove-title" aria-describedby="rr-1-remove-desc" data-confirm-dialog="">"#
    ), "{html}");
    assert!(
        html.contains(
            r#"<h2 class="dialog-title" id="rr-1-remove-title">Remove role @Helpers?</h2>"#
        ),
        "{html}"
    );
    assert!(
        html.contains(
            r#"<button type="submit" class="btn btn-danger">Remove role</button>"#
        ),
        "{html}"
    );
}

#[page("/confirm/dialog/form")]
async fn confirm_dialog_form() -> ViewResult<impl View> {
    Ok(view! {
        confirm_dialog(
            id: "remove-7-confirm",
            trigger: "Remove",
            title: "Remove role @Helpers?",
            confirm: "Remove role",
            form: Some("remove-7")
        )
    })
}

#[tokio::test]
async fn confirm_dialog_names_its_form_on_both_submits_and_omits_an_absent_description()
 {
    let html =
        render(Router::builder().page(confirm_dialog_form), "/confirm/dialog/form")
            .await
            .unwrap();

    assert!(html.contains(
        r#"<button type="submit" class="btn btn-ghost" form="remove-7" data-confirm-trigger="">Remove</button>"#
    ));
    assert!(html.contains(
        r#"<button type="submit" class="btn btn-danger" form="remove-7">Remove role</button>"#
    ));
    assert!(html.contains(
        r#"<dialog class="dialog" aria-labelledby="remove-7-confirm-title" data-confirm-dialog="">"#
    ), "{html}");
    assert!(!html.contains("dialog-desc"));
    assert!(!html.contains("aria-describedby"));
}

#[page("/skeleton/one")]
async fn skeleton_one() -> ViewResult<impl View> {
    Ok(view! { shape_skeleton(shape: SkeletonShape::Btn) })
}

#[page("/skeleton/grid")]
async fn skeleton_grid() -> ViewResult<impl View> {
    Ok(view! {
        <div class="skeleton-grid">
            shape_skeleton(shape: SkeletonShape::Card, count: 6)
        </div>
    })
}

#[tokio::test]
async fn skeleton_defaults_to_one_hidden_shape() {
    let html =
        render(Router::builder().page(skeleton_one), "/skeleton/one").await.unwrap();

    assert_eq!(html, r#"<div class="skeleton-btn" aria-hidden="true"></div>"#);
}

#[tokio::test]
async fn skeleton_repeats_its_shape_count_times() {
    let html = render(Router::builder().page(skeleton_grid), "/skeleton/grid")
        .await
        .unwrap();

    let card = r#"<div class="skeleton-card" aria-hidden="true"></div>"#;
    assert_eq!(
        html,
        format!(r#"<div class="skeleton-grid">{}</div>"#, card.repeat(6))
    );
}

#[test]
fn skeleton_shapes_map_to_their_classes() {
    let classes: Vec<_> =
        SkeletonShape::ALL.iter().map(|shape| shape.class()).collect();

    assert_eq!(classes, [
        "skeleton-btn",
        "skeleton-badge",
        "skeleton-switcher",
        "skeleton-row",
        "skeleton-card",
        "skeleton-panel"
    ]);
}

#[page("/icon/x")]
async fn icon_x() -> ViewResult<impl View> {
    Ok(view! { icon(name: Icon::X) })
}

#[tokio::test]
async fn icon_renders_inline_stroke_svg_hidden_from_assistive_tech() {
    let html = render(Router::builder().page(icon_x), "/icon/x").await.unwrap();

    assert_eq!(html, X);
}

#[page("/icon/all")]
async fn icon_all() -> ViewResult<impl View> {
    Ok(view! {
        #[key(index)]
        for (index, name) in Icon::ALL.into_iter().enumerate() {
            icon(name: name)
        }
    })
}

#[tokio::test]
async fn every_icon_draws_its_paths() {
    let order: Vec<_> = ICON_PATHS.iter().map(|(name, _)| *name).collect();
    assert_eq!(order, Icon::ALL);

    let html = render(Router::builder().page(icon_all), "/icon/all").await.unwrap();
    let expected = ICON_PATHS.iter().fold(String::new(), |mut out, (_, paths)| {
        out.push_str(SVG_OPEN);
        out.push_str(paths);
        out.push_str("</svg>");
        out
    });
    assert_eq!(html, expected);
}

#[test]
fn module_icons_and_tints_follow_the_module_id() {
    let cases = [
        ("music", Icon::Music, "#a78bfa"),
        ("palworld", Icon::Gamepad, "#34d399"),
        ("marathon", Icon::Trophy, "#f59e0b"),
        ("gambling", Icon::Dice, "#f472b6"),
        ("family", Icon::Heart, "#fb7185"),
        ("ticket", Icon::Ticket, "#38bdf8"),
        ("honeypot", Icon::Shield, "#fbbf24"),
        ("greetings", Icon::Message, "#818cf8"),
        ("ai", Icon::Sparkles, "#22d3ee"),
        ("jellyfin", Icon::Sparkles, "#94a3b8"),
    ];

    for (id, expected_icon, expected_tint) in cases {
        assert_eq!(module_icon(id), expected_icon, "{id}");
        assert_eq!(module_tint(id), expected_tint, "{id}");
    }
}

#[page("/guild-grid")]
async fn guild_grid_page() -> ViewResult<impl View> {
    let guilds = [
        GuildCard {
            id: "1".to_owned(),
            name: "Alpha".to_owned(),
            icon: Some("abc".to_owned()),
        },
        GuildCard { id: "2".to_owned(), name: "beta <b>".to_owned(), icon: None },
        GuildCard { id: "3".to_owned(), name: String::new(), icon: None },
    ];
    Ok(view! { guild_grid(guilds: &guilds) })
}

#[tokio::test]
async fn guild_grid_links_every_guild_to_its_overview() {
    let html = render(Router::builder().page(guild_grid_page), "/guild-grid")
        .await
        .unwrap();

    assert_eq!(
        html,
        concat!(
            r#"<div class="guild-grid">"#,
            r#"<a href="/guild/1" class="guild-card"><img src="https://cdn.discordapp.com/icons/1/abc.png?size=64" alt="" class="guild-icon">"#,
            r#"<div class="guild-card-body"><div class="guild-name">Alpha</div><div class="guild-card-hint">Manage →</div></div></a>"#,
            r#"<a href="/guild/2" class="guild-card"><span class="guild-icon placeholder">b</span>"#,
            r#"<div class="guild-card-body"><div class="guild-name">beta &lt;b&gt;</div><div class="guild-card-hint">Manage →</div></div></a>"#,
            r##"<a href="/guild/3" class="guild-card"><span class="guild-icon placeholder">#</span>"##,
            r#"<div class="guild-card-body"><div class="guild-name"></div><div class="guild-card-hint">Manage →</div></div></a>"#,
            r#"</div>"#,
        )
    );
}

#[page("/select/known")]
async fn select_known() -> ViewResult<impl View> {
    Ok(view! {
        select_field(
            label: "Log channel",
            name: "log_channel",
            selected: "20",
            options: Ok(options())
        )
    })
}

#[page("/select/unset")]
async fn select_unset() -> ViewResult<impl View> {
    Ok(view! {
        select_field(
            label: "Log channel",
            name: "log_channel",
            selected: "",
            options: Ok(options())
        )
    })
}

#[page("/select/unknown")]
async fn select_unknown() -> ViewResult<impl View> {
    Ok(view! {
        select_field(
            label: "Log channel",
            name: "log_channel",
            selected: "99",
            options: Ok(options())
        )
    })
}

#[page("/select/locked")]
async fn select_locked() -> ViewResult<impl View> {
    Ok(view! {
        select_field(
            label: "Log channel",
            name: "log_channel",
            selected: "99",
            options: Err("Discord is down".to_owned())
        )
    })
}

#[page("/select/locked-unset")]
async fn select_locked_unset() -> ViewResult<impl View> {
    Ok(view! {
        select_field(
            label: "Log channel",
            name: "log_channel",
            selected: "",
            options: Err("Discord is down".to_owned())
        )
    })
}

#[tokio::test]
async fn select_field_marks_the_stored_option() {
    let html =
        render(Router::builder().page(select_known), "/select/known").await.unwrap();

    assert_eq!(
        html,
        format!(
            concat!(
                r#"<div class="setting-field"><label for="field-log_channel">Log channel</label><div class="select">"#,
                r#"<select class="input" id="field-log_channel" name="log_channel"><option value="">(not set)</option>"#,
                r##"<option value="10"># general</option><option value="20" selected># rules</option></select>"##,
                r#"<span class="select-chevron">{}</span></div></div>"#,
            ),
            CHEVRON_DOWN
        )
    );
}

#[tokio::test]
async fn select_field_selects_not_set_when_nothing_is_stored() {
    let html =
        render(Router::builder().page(select_unset), "/select/unset").await.unwrap();

    assert!(html.contains(r#"<option value="" selected>(not set)</option>"#));
    assert_eq!(html.matches(" selected").count(), 1);
}

#[tokio::test]
async fn select_field_keeps_an_unknown_stored_id() {
    let html = render(Router::builder().page(select_unknown), "/select/unknown")
        .await
        .unwrap();

    assert!(html.contains(concat!(
        r#"<option value="">(not set)</option><option value="99" selected>Unknown (99)</option>"#,
        r#"<option value="10">"#,
    )));
    assert_eq!(html.matches(" selected").count(), 1);
}

#[tokio::test]
async fn locked_select_posts_the_stored_value_through_a_hidden_input() {
    let html = render(Router::builder().page(select_locked), "/select/locked")
        .await
        .unwrap();

    assert_eq!(
        html,
        format!(
            concat!(
                r#"<div class="setting-field"><label for="field-log_channel">Log channel</label><div class="select">"#,
                r#"<select class="input" id="field-log_channel" aria-describedby="field-log_channel-help" disabled><option selected>Unchanged (99)</option></select>"#,
                r#"<span class="select-chevron">{}</span></div>"#,
                r#"<input type="hidden" name="log_channel" value="99">"#,
                r#"<p class="field-hint field-warning" id="field-log_channel-help">Discord is down</p></div>"#,
            ),
            CHEVRON_DOWN
        )
    );
}

#[tokio::test]
async fn locked_select_with_nothing_stored_reads_not_set() {
    let html =
        render(Router::builder().page(select_locked_unset), "/select/locked-unset")
            .await
            .unwrap();

    assert!(html.contains("<option selected>(not set)</option>"));
    assert!(html.contains(r#"<input type="hidden" name="log_channel" value="">"#));
}

#[page("/select/channels")]
async fn select_channels() -> ViewResult<impl View> {
    let channels = [
        channel("1", "chat", ChannelType::GuildText),
        channel("2", "lounge", ChannelType::GuildVoice),
        channel("3", "Info", ChannelType::GuildCategory),
        channel("4", "news", ChannelType::GuildAnnouncement),
        channel("5", "stage", ChannelType::GuildStageVoice),
        channel("6", "help", ChannelType::GuildForum),
        channel("7", "thread", ChannelType::PublicThread),
    ];
    Ok(view! {
        channel_select(
            label: "Channel",
            name: "channel",
            selected: "",
            channels: Ok(&channels)
        )
    })
}

#[page("/select/channels-filtered")]
async fn select_channels_filtered() -> ViewResult<impl View> {
    let channels = [
        channel("1", "chat", ChannelType::GuildText),
        channel("3", "Info", ChannelType::GuildCategory),
    ];
    Ok(view! {
        channel_select(
            label: "Category",
            name: "category",
            selected: "",
            channels: Ok(&channels),
            kinds: &[ChannelType::GuildCategory]
        )
    })
}

#[page("/select/channels-down")]
async fn select_channels_down() -> ViewResult<impl View> {
    Ok(view! {
        channel_select(
            label: "Channel",
            name: "channel",
            selected: "5",
            channels: Err("timeout")
        )
    })
}

#[tokio::test]
async fn channel_select_prefixes_each_label_by_channel_type() {
    let html = render(Router::builder().page(select_channels), "/select/channels")
        .await
        .unwrap();

    assert!(html.contains(concat!(
        r##"<option value="1"># chat</option>"##,
        r#"<option value="2">🔊 lounge</option>"#,
        r#"<option value="3">▸ Info</option>"#,
        r#"<option value="4">📢 news</option>"#,
        r#"<option value="5">🎤 stage</option>"#,
        r#"<option value="6">💬 help</option>"#,
        r##"<option value="7"># thread</option>"##,
    )));
}

#[tokio::test]
async fn channel_select_filters_by_kind() {
    let html = render(
        Router::builder().page(select_channels_filtered),
        "/select/channels-filtered",
    )
    .await
    .unwrap();

    assert!(html.contains(r#"<option value="3">▸ Info</option>"#));
    assert!(!html.contains("chat"));
}

#[tokio::test]
async fn channel_select_explains_an_unreachable_channel_list() {
    let html = render(
        Router::builder().page(select_channels_down),
        "/select/channels-down",
    )
    .await
    .unwrap();

    assert!(html.contains(concat!(
        r#"<p class="field-hint field-warning" id="field-channel-help">Couldn't reach Discord; the channel list is "#,
        "unavailable. Saving keeps the current value. (timeout)</p>",
    )));
    assert!(html.contains("<option selected>Unchanged (5)</option>"));
}

#[page("/select/roles")]
async fn select_roles() -> ViewResult<impl View> {
    let roles = [Role { id: "8".to_owned(), name: "Mods".to_owned() }];
    Ok(view! {
        role_select(label: "Role", name: "role", selected: "8", roles: Ok(&roles))
    })
}

#[page("/select/roles-down")]
async fn select_roles_down() -> ViewResult<impl View> {
    Ok(view! {
        role_select(label: "Role", name: "role", selected: "", roles: Err("timeout"))
    })
}

#[tokio::test]
async fn role_select_labels_roles_with_an_at_sign() {
    let html =
        render(Router::builder().page(select_roles), "/select/roles").await.unwrap();

    assert!(html.contains(r#"<option value="8" selected>@Mods</option>"#));
}

#[tokio::test]
async fn role_select_explains_an_unreachable_role_list() {
    let html =
        render(Router::builder().page(select_roles_down), "/select/roles-down")
            .await
            .unwrap();

    assert!(html.contains("Discord; the role list is unavailable. Saving keeps the current value. (timeout)</p>"));
}

#[page("/select/tags")]
async fn select_tags() -> ViewResult<impl View> {
    let mut forum = channel("6", "help", ChannelType::GuildForum);
    forum.tags =
        vec![ForumTag { id: "61".to_owned(), name: "Open".to_owned() }, ForumTag {
            id: "62".to_owned(),
            name: "Solved".to_owned(),
        }];
    let channels = [channel("1", "chat", ChannelType::GuildText), forum];
    Ok(view! {
        forum_tag_select(
            label: "Tag",
            name: "tag",
            selected: "62",
            channels: Ok(&channels)
        )
    })
}

#[tokio::test]
async fn forum_tag_select_flattens_tags_under_their_forum() {
    let html =
        render(Router::builder().page(select_tags), "/select/tags").await.unwrap();

    assert!(html.contains(concat!(
        r#"<option value="">(not set)</option>"#,
        r##"<option value="61">#help / Open</option>"##,
        r##"<option value="62" selected>#help / Solved</option></select>"##,
    )));
}

#[page("/toggle/on")]
async fn toggle_on() -> ViewResult<impl View> {
    Ok(view! { toggle_field(label: "Enabled", name: "enabled", value: true) })
}

#[page("/toggle/off")]
async fn toggle_off() -> ViewResult<impl View> {
    Ok(view! {
        toggle_field(
            label: "Visibility",
            name: "public_only",
            value: false,
            on_label: "Public only",
            off_label: "All posts"
        )
    })
}

#[tokio::test]
async fn toggle_field_is_a_true_false_select() {
    let html =
        render(Router::builder().page(toggle_on), "/toggle/on").await.unwrap();

    assert_eq!(
        html,
        format!(
            concat!(
                r#"<div class="setting-field"><label for="field-enabled">Enabled</label><div class="select">"#,
                r#"<select class="input" id="field-enabled" name="enabled"><option value="true" selected>Enabled</option>"#,
                r#"<option value="false">Disabled</option></select>"#,
                r#"<span class="select-chevron">{}</span></div></div>"#,
            ),
            CHEVRON_DOWN
        )
    );
}

#[tokio::test]
async fn toggle_field_takes_custom_labels() {
    let html =
        render(Router::builder().page(toggle_off), "/toggle/off").await.unwrap();

    assert!(html.contains(concat!(
        r#"<option value="true">Public only</option>"#,
        r#"<option value="false" selected>All posts</option>"#,
    )));
}

#[page("/setting/default")]
async fn setting_default() -> ViewResult<impl View> {
    Ok(
        view! { setting_field(label: "Channel ID", name: "channel_id", value: "123") },
    )
}

#[page("/setting/custom")]
async fn setting_custom() -> ViewResult<impl View> {
    Ok(view! {
        setting_field(
            label: "Image URL",
            name: "image",
            value: "",
            pattern: "https://.*",
            placeholder: "https://",
            hint: Some("Shown in the embed."),
            input_type: "url"
        )
    })
}

#[tokio::test]
async fn setting_field_defaults_to_a_numeric_text_input() {
    let html = render(Router::builder().page(setting_default), "/setting/default")
        .await
        .unwrap();

    assert_eq!(
        html,
        concat!(
            r#"<div class="setting-field"><label for="field-channel_id">Channel ID</label>"#,
            r#"<input class="input" id="field-channel_id" type="text" name="channel_id" value="123" placeholder="(not set)" pattern="[0-9]*">"#,
            r#"</div>"#,
        )
    );
}

#[tokio::test]
async fn setting_field_renders_its_hint() {
    let html = render(Router::builder().page(setting_custom), "/setting/custom")
        .await
        .unwrap();

    assert!(html.contains(
        r#"<input class="input" id="field-image" aria-describedby="field-image-help" type="url" name="image" value="" placeholder="https://" pattern="https://.*">"#
    ));
    assert!(html.contains(r#"<p class="field-hint" id="field-image-help">Shown in the embed.</p></div>"#));
}

#[page("/save-button")]
async fn save_button_page() -> ViewResult<impl View> {
    Ok(view! { save_button() })
}

#[page("/save-form")]
async fn save_form_page() -> ViewResult<impl View> {
    Ok(view! {
        <form method="post" data-pending="">
            setting_field(label: "Channel ID", name: "channel_id", value: "")
            save_button()
        </form>
    })
}

#[tokio::test]
async fn save_button_submits_the_form_and_names_its_pending_label() {
    let html = render(Router::builder().page(save_button_page), "/save-button")
        .await
        .unwrap();

    assert_eq!(
        html,
        concat!(
            r#"<div class="form-actions"><button type="submit" class="btn btn-primary" "#,
            r#"data-pending-label="Saving…">Save</button></div>"#,
        )
    );
}

#[tokio::test]
async fn save_button_sits_in_a_form_that_opts_into_the_pending_state() {
    let html =
        render(Router::builder().page(save_form_page), "/save-form").await.unwrap();

    assert!(html.starts_with(r#"<form method="post" data-pending="">"#));
    assert!(html.ends_with(concat!(
        r#"<button type="submit" class="btn btn-primary" data-pending-label="Saving…">"#,
        "Save</button></div></form>",
    )));
    assert_eq!(html.matches("type=\"submit\"").count(), 1);
}

#[page("/alert")]
async fn alert_page() -> ViewResult<impl View> {
    Ok(view! { alert(class: "alert warning", role: "status", message: "Heads up") })
}

#[tokio::test]
async fn alert_carries_its_role_and_a_labelled_dismiss_control() {
    let html = render(Router::builder().page(alert_page), "/alert").await.unwrap();

    assert_eq!(
        html,
        format!(
            concat!(
                r#"<div class="alert warning" role="status"><span>Heads up</span>"#,
                r#"<button type="button" class="alert-dismiss" aria-label="Dismiss">{}</button></div>"#,
            ),
            X
        )
    );
}

#[tokio::test]
async fn alert_binds_its_visibility_to_the_dismiss_button() {
    let html =
        render_raw(Router::builder().page(alert_page), "/alert").await.unwrap();

    assert!(html.contains(
        r#"<div class="alert warning" role="status" data-topcoat-bind:hidden="#
    ));
    assert!(html.contains(
        r#"<button type="button" class="alert-dismiss" aria-label="Dismiss" data-topcoat-on:click="#
    ));
    assert!(!html.contains(" hidden=\"\""), "the alert renders visible");
}

#[page("/feedback/ok")]
async fn feedback_ok() -> ViewResult<impl View> {
    Ok(view! {
        save_feedback(outcome: Ok(()))
        create_feedback(outcome: Ok(()))
        delete_feedback(outcome: Ok(()))
    })
}

#[page("/feedback/err")]
async fn feedback_err() -> ViewResult<impl View> {
    Ok(view! {
        save_feedback(outcome: Err("nope"))
        create_feedback(outcome: Err("no category"))
        delete_feedback(outcome: Err("gone"))
    })
}

#[tokio::test]
async fn success_feedback_is_a_polite_status() {
    let html =
        render(Router::builder().page(feedback_ok), "/feedback/ok").await.unwrap();

    for message in ["Saved.", "Creator channel created.", "Loadout deleted."] {
        assert!(html.contains(&format!("<span>{message}</span>")), "{message}");
    }
    assert_eq!(
        html.matches(r#"<div class="alert success" role="status""#).count(),
        3
    );
}

#[tokio::test]
async fn failure_feedback_is_an_assertive_alert() {
    let html =
        render(Router::builder().page(feedback_err), "/feedback/err").await.unwrap();

    for message in [
        "Failed to save: error running server function: nope",
        "Failed to create channel: error running server function: no category",
        "Failed to delete: error running server function: gone",
    ] {
        assert!(html.contains(&format!("<span>{message}</span>")), "{message}");
    }
    assert_eq!(html.matches(r#"<div class="alert error" role="alert""#).count(), 3);
}

#[tokio::test]
async fn each_alert_dismisses_independently() {
    let html = render_raw(Router::builder().page(feedback_ok), "/feedback/ok")
        .await
        .unwrap();

    let ids: std::collections::BTreeSet<_> = html
        .split("data-topcoat-bind:hidden=\"")
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(binding, _)| binding))
        .collect();
    assert_eq!(ids.len(), 3, "three alerts need three distinct signals: {ids:?}");
}

#[page("/key-list/open")]
async fn key_list_open() -> ViewResult<impl View> {
    let keys = ["solar".to_owned(), "void".to_owned()];
    Ok(view! {
        key_list_field(label: "Tags", keys: &keys, list: "tag-suggestions", max: 3)
    })
}

#[page("/key-list/full")]
async fn key_list_full() -> ViewResult<impl View> {
    let keys = ["solar".to_owned(), "void".to_owned()];
    Ok(view! {
        key_list_field(label: "Tags", keys: &keys, list: "tag-suggestions", max: 2)
    })
}

#[tokio::test]
async fn key_list_renders_a_chip_per_key_and_an_add_row() {
    let html = render(Router::builder().page(key_list_open), "/key-list/open")
        .await
        .unwrap();

    let chip = |key: &str| {
        format!(
            r#"<span class="chip"><span class="chip-label">{key}</span><button type="button" class="chip-remove" title="Remove">{X}</button></span>"#
        )
    };
    assert_eq!(
        html,
        format!(
            concat!(
                r#"<div class="setting-field"><label>Tags</label><div class="chip-list">{}{}</div>"#,
                r#"<div class="chip-add"><input class="input" list="tag-suggestions">"#,
                r#"<button type="button" class="btn btn-secondary">Add</button></div></div>"#,
            ),
            chip("solar"),
            chip("void")
        )
    );
}

#[tokio::test]
async fn key_list_disables_add_at_the_limit() {
    let html = render(Router::builder().page(key_list_full), "/key-list/full")
        .await
        .unwrap();

    assert!(html.contains(
        r#"<button type="button" class="btn btn-secondary" disabled>Add</button>"#
    ));
}

#[test]
fn auth_channels_convert_into_select_channels() {
    let info = ChannelInfo {
        id: "6".to_owned(),
        name: "help".to_owned(),
        kind: ChannelType::GuildForum,
        tags: vec![ForumTagInfo { id: "61".to_owned(), name: "Open".to_owned() }],
    };
    let expected = Channel {
        id: "6".to_owned(),
        name: "help".to_owned(),
        kind: ChannelType::GuildForum,
        tags: vec![ForumTag { id: "61".to_owned(), name: "Open".to_owned() }],
    };

    assert_eq!(Channel::from(&info), expected);
    assert_eq!(Channel::from(info), expected);
}

#[test]
fn auth_roles_convert_into_select_roles() {
    let info =
        RoleInfo { id: "8".to_owned(), name: "Mods".to_owned(), color: 0xff_00_00 };
    let expected = Role { id: "8".to_owned(), name: "Mods".to_owned() };

    assert_eq!(Role::from(&info), expected);
    assert_eq!(Role::from(info), expected);
}

#[page("/select/from-auth")]
async fn select_from_auth() -> ViewResult<impl View> {
    let roles: Result<Vec<RoleInfo>, String> =
        Ok(vec![RoleInfo { id: "8".to_owned(), name: "Mods".to_owned(), color: 0 }]);
    let roles: Result<Vec<Role>, String> =
        roles.map(|roles| roles.iter().map(Role::from).collect());
    Ok(view! {
        role_select(
            label: "Role",
            name: "role",
            selected: "8",
            roles: roles.as_deref().map_err(String::as_str)
        )
    })
}

#[tokio::test]
async fn converted_auth_roles_render_as_role_options() {
    let html = render(Router::builder().page(select_from_auth), "/select/from-auth")
        .await
        .unwrap();

    assert!(html.contains(r#"<option value="8" selected>@Mods</option>"#));
}

#[test]
fn discord_user_guilds_convert_into_guild_cards() {
    let guild = CurrentUserGuild {
        id: Id::new(42),
        name: "Alpha".to_owned(),
        icon: Some(ImageHash::parse(b"1234567890abcdef1234567890abcdef").unwrap()),
        owner: false,
        permissions: Permissions::ADMINISTRATOR,
        features: Vec::new(),
    };

    let card = GuildCard::from(&guild);
    assert_eq!(card, GuildCard {
        id: "42".to_owned(),
        name: "Alpha".to_owned(),
        icon: Some("1234567890abcdef1234567890abcdef".to_owned()),
    });
    assert_eq!(
        card.icon_url().as_deref(),
        Some(
            "https://cdn.discordapp.com/icons/42/1234567890abcdef1234567890abcdef.png?size=64"
        )
    );
}
