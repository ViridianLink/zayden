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
use web::components::button::ButtonVariant;
use web::components::confirm::confirm_button;
use web::components::confirm_dialog::confirm_dialog;
use web::components::field_row::{Range, select_row, text_row, toggle_row};
use web::components::guild_grid::{GuildCard, guild_grid};
use web::components::icons::{Icon, icon, module_icon, module_tint};
use web::components::pickers::{
    Channel,
    ForumTag,
    Role,
    SelectOption,
    channel_options,
    forum_tag_options,
    role_options,
};
use web::components::shape_skeleton::{SkeletonShape, shape_skeleton};

mod canonical;
mod support;

use canonical::canonical;

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

const CHEVRON_DOWN: &str = r#"<svg aria-hidden="true" viewBox="0 0 24 24" width="1em" style="vertical-align: -0.125em" height="1em" class="pointer-events-none absolute top-1/2 right-3 size-4 -translate-y-1/2 text-muted-foreground transition-transform"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="m6 9 6 6 6-6"/></g></svg>"#;
const INPUT: &str = "h-9 max-md:h-11 w-full min-w-0 rounded-lg border border-input bg-popover px-3 text-sm transition-colors outline-none placeholder:text-muted-foreground file:mr-3 file:h-full file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";
const SELECT: &str = "h-9 max-md:h-11 w-full appearance-none items-center rounded-lg border border-input bg-popover pr-8 pl-3 text-left text-sm transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive focus-visible:ring-offset-background disabled:pointer-events-none [&amp;::picker(select)]:[appearance:base-select] [&amp;::picker(select)]:mt-1 [&amp;::picker(select)]:rounded-lg [&amp;::picker(select)]:border [&amp;::picker(select)]:border-border [&amp;::picker(select)]:bg-popover [&amp;::picker(select)]:p-1 [&amp;::picker(select)]:text-popover-foreground [&amp;::picker(select)]:shadow-sm [&amp;::picker-icon]:hidden [&amp;_optgroup>legend]:px-2 [&amp;_optgroup>legend]:py-1.5 [&amp;_optgroup>legend]:text-xs [&amp;_optgroup>legend]:font-medium [&amp;_optgroup>legend]:text-muted-foreground [&amp;_optgroup>legend]:cursor-default [&amp;_optgroup>legend]:select-none [&amp;_option]:flex [&amp;_option]:items-center [&amp;_option]:gap-2 [&amp;_option]:rounded-md [&amp;_option]:px-2 [&amp;_option]:py-1.5 [&amp;_option]:text-sm [&amp;_option]:outline-none [&amp;_option:hover]:bg-foreground/5 [&amp;_option:focus]:bg-foreground/5 [&amp;_option:checked]:font-medium [&amp;_option::checkmark]:order-1 [&amp;_option::checkmark]:ml-auto [&amp;_option::checkmark]:size-4 [&amp;_option::checkmark]:shrink-0 [&amp;_option::checkmark]:content-[''] [&amp;_option::checkmark]:bg-muted-foreground [&amp;_option::checkmark]:[mask-size:100%_100%] [&amp;_option::checkmark]:[mask-image:var(--select-checkmark)]";
const SELECT_OPEN: &str = r#"<span class="relative block has-[:disabled]:opacity-50 [&amp;>select]:[appearance:base-select] [&amp;:has(select:open)>svg]:rotate-180" style="--select-checkmark: url(&quot;data:image/svg+xml,<svg xmlns=%22http://www.w3.org/2000/svg%22 viewBox=%220 0 24 24%22><g fill=%22none%22 stroke=%22currentColor%22 stroke-linecap=%22round%22 stroke-linejoin=%22round%22 stroke-width=%222%22><path d=%22M20 6 9 17l-5-5%22/></g></svg>&quot;)">"#;
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
    Ok(opaque_ids(&normalize(&render_raw(builder, path).await?)))
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
            label: "Remove",
            prompt: "Remove this mapping?",
            confirm: "Remove mapping",
            variant: ButtonVariant::Ghost
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
        format!(
            concat!(
                r#"<div class="confirm" data-confirm="">"#,
                r#"<button type="submit" class="{destructive}" data-confirm-trigger="">Delete</button>"#,
                r#"<dialog class="dialog" aria-labelledby="tc-ID" aria-describedby="tc-ID" data-confirm-dialog="">"#,
                r#"<div class="dialog-panel"><h2 class="dialog-title" id="tc-ID">Delete article?</h2>"#,
                r#"<p class="dialog-desc" id="tc-ID">Delete this article?</p><div class="dialog-actions">"#,
                r#"<button type="button" class="{secondary} max-md:flex-1" data-dialog-close="" autofocus="">Cancel</button>"#,
                r#"<button type="submit" class="{destructive} max-md:flex-1">Delete article</button>"#,
                r#"</div></div></dialog></div>"#,
            ),
            destructive = support::btn("destructive"),
            secondary = support::btn("secondary"),
        )
    );
}

#[tokio::test]
async fn confirm_button_class_styles_only_the_trigger() {
    let html = render(Router::builder().page(confirm_ghost), "/confirm/ghost")
        .await
        .unwrap();

    assert!(html.contains(&format!(
        r#"<button type="submit" class="{}" data-confirm-trigger="">Remove</button>"#,
        support::btn("ghost")
    )));
    assert!(html.contains(&format!(
        r#"<button type="submit" class="{} max-md:flex-1">Remove mapping</button>"#,
        support::btn("destructive")
    )));
    assert!(!html.contains(" open"), "the dialog renders closed");
}

#[page("/confirm/named")]
async fn confirm_named() -> ViewResult<impl View> {
    Ok(view! {
        confirm_button(
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
        r#"<dialog class="dialog" aria-labelledby="tc-ID" aria-describedby="tc-ID" data-confirm-dialog="">"#
    ), "{html}");
    assert!(
        html.contains(
            r#"<h2 class="dialog-title" id="tc-ID">Remove role @Helpers?</h2>"#
        ),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"<button type="submit" class="{} max-md:flex-1">Remove role</button>"#,
            support::btn("destructive")
        )),
        "{html}"
    );
}

#[page("/confirm/dialog/form")]
async fn confirm_dialog_form() -> ViewResult<impl View> {
    Ok(view! {
        confirm_dialog(
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

    assert!(html.contains(&format!(
        r#"<button type="submit" class="{}" form="remove-7" data-confirm-trigger="">Remove</button>"#,
        support::btn("ghost")
    )));
    assert!(html.contains(&format!(
        r#"<button type="submit" class="{} max-md:flex-1" form="remove-7">Remove role</button>"#,
        support::btn("destructive")
    )));
    assert!(html.contains(
        r#"<dialog class="dialog" aria-labelledby="tc-ID" data-confirm-dialog="">"#
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
        canonical(&html),
        canonical(concat!(
            r#"<div class="guild-grid">"#,
            r#"<a href="/guild/1" class="guild-card"><img src="https://cdn.discordapp.com/icons/1/abc.png?size=64" alt="" class="guild-icon">"#,
            r#"<div class="guild-card-body"><div class="guild-name">Alpha</div><div class="guild-card-hint">Manage →</div></div></a>"#,
            r#"<a href="/guild/2" class="guild-card"><span class="guild-icon placeholder">b</span>"#,
            r#"<div class="guild-card-body"><div class="guild-name">beta &lt;b&gt;</div><div class="guild-card-hint">Manage →</div></div></a>"#,
            r##"<a href="/guild/3" class="guild-card"><span class="guild-icon placeholder">#</span>"##,
            r#"<div class="guild-card-body"><div class="guild-name"></div><div class="guild-card-hint">Manage →</div></div></a>"#,
            r#"</div>"#,
        ))
    );
}

#[page("/select/known")]
async fn select_known() -> ViewResult<impl View> {
    Ok(view! {
        select_row(
            form: "logs",
            name: "log_channel",
            label: "Log channel",
            selected: "20",
            options: Ok(options())
        )
    })
}

#[page("/select/unset")]
async fn select_unset() -> ViewResult<impl View> {
    Ok(view! {
        select_row(
            form: "logs",
            name: "log_channel",
            label: "Log channel",
            selected: "",
            options: Ok(options())
        )
    })
}

#[page("/select/unknown")]
async fn select_unknown() -> ViewResult<impl View> {
    Ok(view! {
        select_row(
            form: "logs",
            name: "log_channel",
            label: "Log channel",
            selected: "99",
            options: Ok(options()),
            help: Some("Where reports go."),
            error: Some("Pick a text channel.")
        )
    })
}

#[page("/select/locked")]
async fn select_locked() -> ViewResult<impl View> {
    Ok(view! {
        select_row(
            form: "logs",
            name: "log_channel",
            label: "Log channel",
            selected: "99",
            options: Err("Discord is down")
        )
    })
}

#[page("/select/locked-unset")]
async fn select_locked_unset() -> ViewResult<impl View> {
    Ok(view! {
        select_row(
            form: "logs",
            name: "log_channel",
            label: "Log channel",
            selected: "",
            options: Err("Discord is down")
        )
    })
}

#[tokio::test]
async fn select_row_marks_the_stored_option() {
    let html =
        render(Router::builder().page(select_known), "/select/known").await.unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            concat!(
                r#"<div class="field-row"><label class="field-label" for="logs-log-channel">Log channel</label>{open}"#,
                r#"<select class="{select}" id="logs-log-channel" name="log_channel"><option value="">(not set)</option>"#,
                r##"<option value="10"># general</option><option value="20" selected># rules</option></select>"##,
                r#"{chevron}</span></div>"#,
            ),
            open = SELECT_OPEN,
            select = SELECT,
            chevron = CHEVRON_DOWN
        ))
    );
}

#[tokio::test]
async fn select_row_selects_not_set_when_nothing_is_stored() {
    let html =
        render(Router::builder().page(select_unset), "/select/unset").await.unwrap();

    assert!(html.contains(r#"<option value="" selected>(not set)</option>"#));
    assert_eq!(html.matches(" selected").count(), 1);
}

#[tokio::test]
async fn select_row_keeps_an_unknown_stored_id_and_carries_help_and_error() {
    let html = render(Router::builder().page(select_unknown), "/select/unknown")
        .await
        .unwrap();

    assert!(html.contains(concat!(
        r#"<option value="">(not set)</option><option value="99" selected>Unknown (99)</option>"#,
        r#"<option value="10">"#,
    )));
    assert_eq!(html.matches(" selected").count(), 1);
    assert!(html.starts_with(r#"<div class="field-row" data-invalid="">"#));
    assert!(canonical(&html).contains(
        r#"aria-describedby="logs-log-channel-help logs-log-channel-error" aria-invalid="true""#
    ));
    assert!(html.ends_with(concat!(
        r#"<p class="field-help" id="logs-log-channel-help">Where reports go.</p>"#,
        r#"<p class="field-error" id="logs-log-channel-error">Pick a text channel.</p></div>"#,
    )));
}

#[tokio::test]
async fn locked_select_posts_the_stored_value_through_a_hidden_input() {
    let html = render(Router::builder().page(select_locked), "/select/locked")
        .await
        .unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            concat!(
                r#"<div class="field-row"><label class="field-label" for="logs-log-channel">Log channel</label>{open}"#,
                r#"<select class="{select}" id="logs-log-channel" aria-describedby="logs-log-channel-help" disabled><option selected>Unchanged (99)</option></select>"#,
                r#"{chevron}</span>"#,
                r#"<input type="hidden" name="log_channel" value="99">"#,
                r#"<p class="field-help" id="logs-log-channel-help">Discord is down</p></div>"#,
            ),
            open = SELECT_OPEN,
            select = SELECT,
            chevron = CHEVRON_DOWN
        ))
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

fn labels(options: &[SelectOption]) -> Vec<(&str, &str)> {
    options
        .iter()
        .map(|option| (option.value.as_str(), option.label.as_str()))
        .collect()
}

#[test]
fn channel_options_prefix_each_label_by_channel_type() {
    let channels = [
        channel("1", "chat", ChannelType::GuildText),
        channel("2", "lounge", ChannelType::GuildVoice),
        channel("3", "Info", ChannelType::GuildCategory),
        channel("4", "news", ChannelType::GuildAnnouncement),
        channel("5", "stage", ChannelType::GuildStageVoice),
        channel("6", "help", ChannelType::GuildForum),
        channel("7", "thread", ChannelType::PublicThread),
    ];
    let options = channel_options(Ok(&channels), &[]).unwrap();

    assert_eq!(labels(&options), [
        ("1", "# chat"),
        ("2", "🔊 lounge"),
        ("3", "▸ Info"),
        ("4", "📢 news"),
        ("5", "🎤 stage"),
        ("6", "💬 help"),
        ("7", "# thread"),
    ]);
}

#[test]
fn channel_options_filter_by_kind() {
    let channels = [
        channel("1", "chat", ChannelType::GuildText),
        channel("3", "Info", ChannelType::GuildCategory),
    ];
    let options =
        channel_options(Ok(&channels), &[ChannelType::GuildCategory]).unwrap();

    assert_eq!(labels(&options), [("3", "▸ Info")]);
}

#[test]
fn unreachable_lists_explain_themselves() {
    assert_eq!(
        channel_options(Err("timeout"), &[]).unwrap_err(),
        "Couldn't reach Discord; the channel list is unavailable. Saving keeps the current value. (timeout)"
    );
    assert_eq!(
        role_options(Err("timeout")).unwrap_err(),
        "Couldn't reach Discord; the role list is unavailable. Saving keeps the current value. (timeout)"
    );
}

#[test]
fn role_options_label_roles_with_an_at_sign() {
    let roles = [Role { id: "8".to_owned(), name: "Mods".to_owned() }];

    assert_eq!(labels(&role_options(Ok(&roles)).unwrap()), [("8", "@Mods")]);
}

#[test]
fn forum_tag_options_flatten_tags_under_their_forum() {
    let mut forum = channel("6", "help", ChannelType::GuildForum);
    forum.tags =
        vec![ForumTag { id: "61".to_owned(), name: "Open".to_owned() }, ForumTag {
            id: "62".to_owned(),
            name: "Solved".to_owned(),
        }];
    let channels = [channel("1", "chat", ChannelType::GuildText), forum];

    assert_eq!(labels(&forum_tag_options(Ok(&channels)).unwrap()), [
        ("61", "#help / Open"),
        ("62", "#help / Solved"),
    ]);
}

#[page("/toggle/on")]
async fn toggle_on() -> ViewResult<impl View> {
    Ok(
        view! { toggle_row(form: "ai", name: "enabled", label: "Enabled", value: true) },
    )
}

#[page("/toggle/off")]
async fn toggle_off() -> ViewResult<impl View> {
    Ok(view! {
        toggle_row(
            form: "patreon",
            name: "public_only",
            label: "Visibility",
            value: false,
            on_label: "Public only",
            off_label: "All posts"
        )
    })
}

#[tokio::test]
async fn toggle_row_is_a_true_false_select() {
    let html =
        render(Router::builder().page(toggle_on), "/toggle/on").await.unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            concat!(
                r#"<div class="field-row"><label class="field-label" for="ai-enabled">Enabled</label>{open}"#,
                r#"<select class="{select}" id="ai-enabled" name="enabled"><option value="true" selected>On</option>"#,
                r#"<option value="false">Off</option></select>"#,
                r#"{chevron}</span></div>"#,
            ),
            open = SELECT_OPEN,
            select = SELECT,
            chevron = CHEVRON_DOWN
        ))
    );
}

#[tokio::test]
async fn toggle_row_takes_custom_labels() {
    let html =
        render(Router::builder().page(toggle_off), "/toggle/off").await.unwrap();

    assert!(html.contains(concat!(
        r#"<option value="true">Public only</option>"#,
        r#"<option value="false" selected>All posts</option>"#,
    )));
}

#[page("/text/numeric")]
async fn text_numeric() -> ViewResult<impl View> {
    Ok(view! {
        text_row(
            form: "rr",
            name: "message_id",
            label: "Message ID",
            value: "123",
            numeric: true
        )
    })
}

#[page("/text/ranged")]
async fn text_ranged() -> ViewResult<impl View> {
    Ok(view! {
        text_row(
            form: "music",
            name: "auto_disconnect",
            label: "Auto-disconnect",
            value: "60",
            help: Some("Seconds."),
            range: Some(Range { min: 0, max: Some(600) })
        )
    })
}

#[tokio::test]
async fn text_row_marks_numeric_input() {
    let html =
        render(Router::builder().page(text_numeric), "/text/numeric").await.unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            concat!(
                r#"<div class="field-row"><label class="field-label" for="rr-message-id">Message ID</label>"#,
                r#"<input class="{input}" id="rr-message-id" type="text" name="message_id" value="123" inputmode="numeric" pattern="[0-9]*" autocomplete="off">"#,
                r#"</div>"#,
            ),
            input = INPUT
        ))
    );
}

#[tokio::test]
async fn text_row_bounds_a_ranged_number_and_describes_it() {
    let html =
        render(Router::builder().page(text_ranged), "/text/ranged").await.unwrap();

    assert!(canonical(&html).contains(&canonical(&format!(
        r#"<input class="{INPUT}" id="music-auto-disconnect" type="number" name="auto_disconnect" value="60" min="0" max="600" step="1" autocomplete="off" aria-describedby="music-auto-disconnect-help">"#
    ))));
    assert!(html.contains(
        r#"<p class="field-help" id="music-auto-disconnect-help">Seconds.</p></div>"#
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

#[test]
fn converted_auth_roles_become_role_options() {
    let info = RoleInfo { id: "8".to_owned(), name: "Mods".to_owned(), color: 0 };
    let roles = [Role::from(&info)];

    assert_eq!(labels(&role_options(Ok(&roles)).unwrap()), [("8", "@Mods")]);
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

/// Replaces `element_id` values, which hash the render site, with `tc-ID`.
fn opaque_ids(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((head, tail)) = rest.split_once("\"tc-") {
        out.push_str(head);
        out.push_str("\"tc-ID\"");
        rest = tail.split_once('"').map_or("", |(_, after)| after);
    }
    out.push_str(rest);
    out
}
