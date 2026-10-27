//! Markup contracts of the design-system components: the names, roles and
//! attributes that labels, switches, dialogs, menus and the shared script rely
//! on.

use std::error::Error;

use http_body_util::BodyExt;
use topcoat::Result as ViewResult;
use topcoat::router::request::Request;
use topcoat::router::{Body, Router, RouterBuilder, page};
use topcoat::runtime::RouterBuilderRuntimeExt;
use topcoat::view::{View, view};
use web::components::data_table::{data_cell, data_row, data_table};
use web::components::empty_state::empty_state;
use web::components::error_panel::{ErrorAction, error_panel};
use web::components::field_row::{
    describedby,
    field_id,
    field_row,
    select_row,
    text_row,
    toggle_row,
};
use web::components::flash::{flash, flash_message};
use web::components::guild_grid::GuildCard;
use web::components::lamp::{LampState, lamp};
use web::components::module_row::{
    ModuleControl,
    NOT_SYNCED_NOTE,
    module_group,
    module_row,
};
use web::components::nav_links::NavAccess;
use web::components::nav_rail::nav_rail;
use web::components::nav_sheet::{SHEET_ID, nav_sheet};
use web::components::pickers::SelectOption;
use web::components::plan_note::{plan_note, plan_tag};
use web::components::progress_bar::progress_bar;
use web::components::save_bar::save_bar;
use web::components::server_plate::{FILTER_THRESHOLD, PANEL_ID, server_plate};
use web::flash::{Flash, FlashKind, MAX_MESSAGE_CHARS};
use web::guild::dto::GuildInfo;

mod canonical;
mod support;

use canonical::canonical;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const INPUT: &str = "h-9 max-md:h-11 w-full min-w-0 rounded-lg border border-input bg-popover px-3 text-sm transition-colors outline-none placeholder:text-muted-foreground file:mr-3 file:h-full file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive focus-visible:ring-offset-background disabled:pointer-events-none disabled:opacity-50";

const ERROR_ALERT: &str = "grid w-full grid-cols-[0_1fr] items-start gap-y-1 rounded-lg border bg-background px-4 py-3 text-sm has-[>svg]:grid-cols-[1rem_1fr] has-[>svg]:gap-x-3 [&amp;>svg]:size-4 [&amp;>svg]:translate-y-0.5 border-destructive/50 text-destructive";
const BADGE_OUTLINE: &str = "inline-flex w-fit shrink-0 items-center justify-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium whitespace-nowrap border-border text-foreground";
const TABLE: &str = "w-full caption-bottom border-collapse border-border text-sm [:not(:first-child)>&amp;]:border-t [:not(:last-child)>&amp;]:border-b data-table";
const CAPTION: &str = "mt-4 text-sm text-muted-foreground visually-hidden";
const THEAD: &str = "bg-foreground/5 [&amp;_tr]:border-b";
const TBODY: &str = "[&amp;_tr:last-child]:border-0";
const ROW: &str = "border-b border-border transition-colors hover:bg-foreground/5";
const TH: &str = "p-3 text-left align-middle font-medium whitespace-nowrap text-muted-foreground first:pl-6 last:pr-6";
const TD: &str = "p-3 align-middle whitespace-nowrap first:pl-6 last:pr-6";

async fn render(builder: RouterBuilder, path: &str) -> TestResult<String> {
    let router = builder.runtime().build();
    let response = router.handle(Request::get(path).body(Body::empty())?).await;
    let bytes = response.into_body().collect().await?.to_bytes();
    let html = String::from_utf8(bytes.to_vec())?;
    Ok(strip_comments(&html))
}

fn strip_comments(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((before, after)) = rest.split_once("<!--") {
        out.push_str(before);
        rest = after.split_once("-->").map_or("", |(_, tail)| tail);
    }
    out.push_str(rest);
    out
}

fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

#[page("/lamp")]
async fn lamp_page() -> ViewResult<impl View> {
    Ok(view! {
        lamp(state: LampState::On)
        lamp(state: LampState::Off, text: Some("Disabled"))
        lamp(state: LampState::NotSynced, pending: Some("Saving"))
    })
}

#[tokio::test]
async fn the_lamp_pairs_a_hidden_light_with_its_state_in_words() {
    let html = render(Router::builder().page(lamp_page), "/lamp").await.unwrap();

    assert_eq!(
        html,
        concat!(
            r#"<span class="lamp-status"><span class="lamp lamp-on" aria-hidden="true"></span><span class="lamp-text">On</span></span>"#,
            r#"<span class="lamp-status"><span class="lamp lamp-off" aria-hidden="true"></span><span class="lamp-text">Disabled</span></span>"#,
            r#"<span class="lamp-status"><span class="lamp lamp-sync" aria-hidden="true"></span><span class="lamp-text" data-pending-text="Saving">Not synced</span></span>"#,
        )
    );
}

#[page("/module/switch")]
async fn module_switch() -> ViewResult<impl View> {
    Ok(view! {
        module_group(
            title: "Voice & games",
            module_row(
                guild_id: "7",
                module_id: "music",
                label: "Music",
                description: "Play audio in voice channels.",
                control: ModuleControl::Switch { on: true },
                href: Some("/guild/7/settings/music")
            )
        )
    })
}

#[tokio::test]
async fn a_module_switch_is_a_named_role_switch_posting_the_opposite_state() {
    let html = render(Router::builder().page(module_switch), "/module/switch")
        .await
        .unwrap();

    assert!(html.starts_with(
        r#"<section class="rack-group" aria-labelledby="rack-voice---games"><h2 class="rack-heading label" id="rack-voice---games">Voice &amp; games</h2><ul class="rack"><li class="rack-row">"#
    ), "{html}");
    assert!(canonical(&html).contains(&canonical(
        r#"<h3 class="rack-name"><a href="/guild/7/settings/music" data-topcoat-link="intent">Music</a></h3>"#
    )), "{html}");
    assert!(html.contains(
        r#"<form class="rack-form" method="post" action="/guild/7" data-pending=""><input type="hidden" name="guild" value="7"><input type="hidden" name="module_id" value="music">"#
    ), "{html}");
    assert!(html.contains(
        r#"<button type="submit" class="switch" role="switch" aria-checked="true" aria-label="Music module" name="enabled" value="false"><span class="switch-thumb" aria-hidden="true"></span></button>"#
    ), "{html}");
    assert!(html.contains(r#"data-pending-text="Saving…">On</span>"#), "{html}");
}

#[page("/module/status")]
async fn module_status() -> ViewResult<impl View> {
    Ok(view! {
        module_row(
            guild_id: "7",
            module_id: "patreon",
            label: "Patreon",
            description: "Announce new posts.",
            control: ModuleControl::Status { on: false },
            href: Some("/guild/7/settings/patreon"),
            error: Some("Couldn't save.")
        )
        module_row(
            guild_id: "7",
            module_id: "lfg",
            label: "LFG",
            description: "Looking for group.",
            control: ModuleControl::NotSynced
        )
    })
}

#[tokio::test]
async fn status_and_unsynced_rows_have_no_switch() {
    let html = render(Router::builder().page(module_status), "/module/status")
        .await
        .unwrap();

    assert_eq!(count(&html, "role=\"switch\""), 0, "{html}");
    assert_eq!(count(&html, "<form"), 0, "{html}");
    assert!(
        canonical(&html).contains(&canonical(
            r#"<a href="/guild/7/settings/patreon" class="rack-link" data-topcoat-link="intent">Manage"#
        )),
        "{html}"
    );
    assert!(
        canonical(&html).contains(&canonical(&format!(
            r#"<div class="{ERROR_ALERT} rack-error" role="alert"><p class="col-start-2 font-medium tracking-tight">Couldn't save.</p></div>"#
        ))),
        "{html}"
    );
    assert!(html.contains(r#"<h3 class="rack-name">LFG</h3>"#), "{html}");
    assert!(
        html.contains(r#"<span class="lamp lamp-sync" aria-hidden="true"></span>"#),
        "{html}"
    );
    assert!(
        html.contains(&format!(r#"<p class="rack-note">{NOT_SYNCED_NOTE}</p>"#)),
        "{html}"
    );
}

#[page("/save-bar")]
async fn save_bar_page() -> ViewResult<impl View> {
    let notice = Flash::new(FlashKind::Success, "Saved.");
    Ok(view! { save_bar(notice: Some(&notice)) })
}

#[tokio::test]
async fn the_save_bar_announces_its_state_and_hides_discard_until_dirty() {
    let html =
        render(Router::builder().page(save_bar_page), "/save-bar").await.unwrap();

    assert!(html.starts_with(
        r#"<div class="save-bar" data-save-bar=""><div class="save-bar-status" role="status"><span class="save-bar-dirty" data-dirty-status="" hidden="">Unsaved changes</span><p class="flash flash-success" data-flash="">"#
    ), "{html}");
    assert!(html.contains(&format!(
        r#"<button type="button" class="{}" data-discard="" hidden="">Discard</button><button type="submit" class="{}" data-pending-label="Saving…">Save changes</button></div>"#,
        support::btn("ghost"),
        support::btn("primary")
    )), "{html}");
}

#[page("/flash")]
async fn flash_page() -> ViewResult<impl View> {
    let notice = Flash::new(FlashKind::Error, "Couldn't save.");
    Ok(view! {
        flash()
        flash_message(notice: &notice)
    })
}

#[tokio::test]
async fn the_flash_region_is_a_live_status_even_when_empty() {
    let html = render(Router::builder().page(flash_page), "/flash").await.unwrap();

    assert!(html.starts_with(r#"<div class="flash-region" role="status"></div><p class="flash flash-error" data-flash=""><svg"#), "{html}");
    assert!(
        html.ends_with(r#"<span class="flash-text">Couldn't save.</span></p>"#),
        "{html}"
    );
}

#[test]
fn a_flash_survives_its_cookie_encoding_and_rejects_tampering() {
    let notice = Flash::new(FlashKind::Success, "Role removed.");
    assert_eq!(Flash::decode(&notice.encode()), Some(notice));

    let long = "x".repeat(MAX_MESSAGE_CHARS + 10);
    assert_eq!(
        Flash::new(FlashKind::Error, &long).message.chars().count(),
        MAX_MESSAGE_CHARS
    );
    assert_eq!(Flash::decode("not base64 !"), None);
    assert_eq!(Flash::decode(&Flash::new(FlashKind::Error, "").encode()), None);
}

#[page("/field-row")]
async fn field_row_page() -> ViewResult<impl View> {
    let described = describedby("prefix", true, true);
    Ok(view! {
        field_row(
            id: "prefix",
            label: "Prefix",
            help: Some("One or two characters."),
            error: Some("Too long."),
            <input
                class="input"
                id="prefix"
                name="prefix"
                aria-describedby=(described.as_deref())
                aria-invalid="true"
            >
        )
    })
}

#[tokio::test]
async fn a_field_row_labels_its_control_and_names_its_help_and_error() {
    let html =
        render(Router::builder().page(field_row_page), "/field-row").await.unwrap();

    assert_eq!(
        html,
        concat!(
            r#"<div class="field-row" data-invalid=""><label class="field-label" for="prefix">Prefix</label>"#,
            r#"<input class="input" id="prefix" name="prefix" aria-describedby="prefix-help prefix-error" aria-invalid="true">"#,
            r#"<p class="field-help" id="prefix-help">One or two characters.</p>"#,
            r#"<p class="field-error" id="prefix-error">Too long.</p></div>"#,
        )
    );
}

#[test]
fn describedby_names_only_the_lines_that_exist() {
    assert_eq!(describedby("a", false, false), None);
    assert_eq!(describedby("a", true, false).as_deref(), Some("a-help"));
    assert_eq!(describedby("a", false, true).as_deref(), Some("a-error"));
}

#[page("/settings-ids")]
async fn settings_ids() -> ViewResult<impl View> {
    Ok(view! {
        text_row(form: "faq-3", name: "title", label: "Title", value: "")
        toggle_row(form: "ai", name: "enabled", label: "Enabled", value: true)
        select_row(
            form: "rr",
            name: "channel_id",
            label: "Channel",
            selected: "",
            options: Ok(
                vec![SelectOption { value: "1".to_owned(), label: "# a".to_owned() }],
            )
        )
    })
}

#[tokio::test]
async fn field_rows_take_their_id_from_the_form_for_repeated_forms() {
    let html =
        render(Router::builder().page(settings_ids), "/settings-ids").await.unwrap();

    for id in ["faq-3-title", "ai-enabled", "rr-channel-id"] {
        assert!(
            html.contains(&format!(r#"<label class="field-label" for="{id}">"#)),
            "{id}: {html}"
        );
        assert_eq!(count(&html, &format!(r#"id="{id}""#)), 1, "{id}: {html}");
    }
    assert_eq!(field_id("faq-new", "channel_id"), "faq-new-channel-id");
}

#[page("/table")]
async fn table_page() -> ViewResult<impl View> {
    Ok(view! {
        data_table(
            caption: "Reaction roles",
            columns: &["Role", "Members"],
            data_row(
                data_cell(label: "Role", header: true, "@Mod")
                data_cell(label: "Members", numeric: true, "12")
            )
        )
    })
}

#[tokio::test]
async fn the_data_table_keeps_table_roles_and_cell_labels_for_reflow() {
    let html = render(Router::builder().page(table_page), "/table").await.unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            r#"<div class="w-full overflow-x-auto" data-slot="table"><table class="{TABLE}" role="table"><caption class="{CAPTION}">Reaction roles</caption><thead class="{THEAD}" role="rowgroup"><tr class="{ROW}" role="row"><th class="{TH}" scope="col" role="columnheader">Role</th><th class="{TH}" scope="col" role="columnheader">Members</th></tr></thead><tbody class="{TBODY}" role="rowgroup"><tr class="{ROW}" role="row"><th class="{TH}" scope="row" role="rowheader" data-label="Role">@Mod</th><td class="{TD} num" role="cell" data-label="Members">12</td></tr></tbody></table></div>"#
        ))
    );
}

#[page("/empty-and-error")]
async fn empty_and_error() -> ViewResult<impl View> {
    let actions = [
        ErrorAction::new("Add Zayden to Guild 7", "/invite?guild=7").external(),
        ErrorAction::new("Back to servers", "/guilds"),
    ];
    Ok(view! {
        empty_state(
            title: "No servers yet",
            text: "Add Zayden to a server you manage.",
            action: Some("Add Zayden"),
            href: Some("/invite"),
            external: true
        )
        error_panel(
            title: "Zayden isn't in this server yet",
            message: "Add it, then come back.",
            actions: &actions,
            alert: true
        )
    })
}

#[tokio::test]
async fn empty_and_error_states_lead_with_one_primary_action() {
    let html = render(Router::builder().page(empty_and_error), "/empty-and-error")
        .await
        .unwrap();

    let canonical_html = canonical(&html);
    assert!(canonical_html.starts_with(&canonical(&format!(
        r#"<div class="empty-state"><h2 class="empty-title">No servers yet</h2><p class="empty-text">Add Zayden to a server you manage.</p><a href="/invite" rel="external" class="{}">Add Zayden</a></div>"#,
        support::btn("primary")
    ))), "{html}");
    assert!(html.contains(
        r#"<section class="error-panel" role="alert"><h1 class="error-title">Zayden isn't in this server yet</h1><p class="error-text">Add it, then come back.</p>"#
    ), "{html}");
    assert!(canonical_html.contains(&canonical(&format!(
        r#"<a href="/invite?guild=7" rel="external" class="{}">Add Zayden to Guild 7</a><a href="/guilds" data-topcoat-link="intent" class="{}">Back to servers</a>"#,
        support::btn("primary"),
        support::btn("secondary")
    ))), "{html}");
    assert_eq!(count(&html, &support::btn("primary")), 2, "{html}");
}

#[page("/plan")]
async fn plan_page() -> ViewResult<impl View> {
    Ok(view! {
        plan_note(tier: "Pro", text: "Cooldowns below 60 seconds need Pro.")
        plan_tag(label: "Pro")
    })
}

#[tokio::test]
async fn a_plan_note_names_the_tier_in_text_and_links_the_plans_once() {
    let html = render(Router::builder().page(plan_page), "/plan").await.unwrap();

    assert_eq!(
        canonical(&html),
        canonical(&format!(
            r#"<p class="plan-note"><span class="{BADGE_OUTLINE}">Pro</span><span>Cooldowns below 60 seconds need Pro.</span><a href="/upgrade" data-topcoat-link="intent">See plans</a></p><span class="{BADGE_OUTLINE}">Pro</span>"#
        ))
    );
}

#[page("/progress")]
async fn progress_page() -> ViewResult<impl View> {
    Ok(view! { progress_bar() })
}

#[tokio::test]
async fn the_progress_bar_is_hidden_from_assistive_tech() {
    let html =
        render(Router::builder().page(progress_page), "/progress").await.unwrap();

    assert_eq!(
        html,
        r#"<div class="nav-progress" data-nav-progress="" aria-hidden="true"></div>"#
    );
}

#[page("/guild/{guild_id}/music")]
async fn nav_on_music() -> ViewResult<impl View> {
    let staff = NavAccess { operator: true, admin: true, on_operator_page: true };
    Ok(view! {
        nav_rail(access: staff, guild_id: Some("7"))
        nav_sheet(access: NavAccess::default(), guild_id: Some("7"))
    })
}

#[tokio::test]
async fn the_rail_and_sheet_mark_the_current_page_and_label_their_groups() {
    let html = render(Router::builder().page(nav_on_music), "/guild/7/music")
        .await
        .unwrap();

    assert_eq!(
        count(&html, r#"<nav class="nav" aria-label="Dashboard">"#),
        2,
        "{html}"
    );
    assert_eq!(
        count(
            &canonical(&html),
            &canonical(
                r#"<a href="/guild/7/music" class="nav-link" aria-current="page" data-topcoat-link="intent"><span>Music</span></a>"#
            )
        ),
        2,
        "{html}"
    );
    assert_eq!(count(&html, r#"aria-current="page""#), 2, "{html}");
    for prefix in ["rail", "sheet"] {
        assert!(html.contains(&format!(
            r#"<p class="nav-heading" id="{prefix}-voice---games">Voice &amp; games</p><ul class="nav-list" aria-labelledby="{prefix}-voice---games">"#
        )), "{prefix}: {html}");
    }
    assert_eq!(
        count(&html, &format!(r#"<div class="{BADGE_OUTLINE} operator-badge">"#)),
        1,
        "{html}"
    );
    assert_eq!(count(&html, r#"id="rail-admin""#), 1, "{html}");
    assert_eq!(count(&html, r#"id="sheet-admin""#), 0, "{html}");
    assert!(html.contains(&format!(
        r#"<div id="{SHEET_ID}" popover="" class="sheet-panel" role="dialog" aria-label="Menu">"#
    )), "{html}");
    assert!(html.contains(&format!(
        r#"popovertarget="{SHEET_ID}" popovertargetaction="hide" aria-label="Close menu""#
    )), "{html}");
}

fn guilds(n: usize) -> Vec<GuildInfo> {
    (1..=n)
        .map(|i| GuildInfo {
            id: i.to_string(),
            name: format!("Guild {i}"),
            icon: None,
        })
        .collect()
}

#[page("/guild/{guild_id}/support")]
async fn plate_page() -> ViewResult<impl View> {
    let list = guilds(FILTER_THRESHOLD + 1);
    let current =
        GuildCard { id: "2".to_owned(), name: "Guild 2".to_owned(), icon: None };
    Ok(view! { server_plate(current: Some(&current), guilds: &list) })
}

#[page("/guild/{guild_id}/levels")]
async fn plate_small() -> ViewResult<impl View> {
    let list = guilds(2);
    Ok(view! { server_plate(current: None, guilds: &list) })
}

#[tokio::test]
async fn the_server_plate_opens_a_filterable_menu_keeping_the_page() {
    let html = render(Router::builder().page(plate_page), "/guild/2/support")
        .await
        .unwrap();

    assert!(html.starts_with(&format!(
        r#"<button type="button" class="topbar-button plate" popovertarget="{PANEL_ID}" aria-controls="{PANEL_ID}" aria-expanded="false" aria-label="Switch server, current: Guild 2">"#
    )), "{html}");
    assert!(html.contains(r#"data-filter="" data-filter-noun="servers""#), "{html}");
    assert!(canonical(&html).contains(&canonical(&format!(
        r#"<label class="visually-hidden" for="server-filter">Filter servers</label><input class="{INPUT}" id="server-filter" type="search" autofocus="" autocomplete="off" data-filter-input="" placeholder="Filter servers">"#
    ))), "{html}");
    assert!(
        html.contains(
            r#"<p class="menu-count" role="status" data-filter-count=""></p>"#
        ),
        "{html}"
    );
    assert!(canonical(&html).contains(&canonical(
        r#"<a href="/guild/2/support" class="menu-item" aria-current="page" data-filter-item="" data-topcoat-link="intent">"#
    )), "{html}");
    assert!(
        canonical(&html).contains(&canonical(
            r#"<a href="/guild/9/support" class="menu-item" data-filter-item="" data-topcoat-link="intent">"#
        )),
        "{html}"
    );
    assert_eq!(count(&html, "data-filter-item"), FILTER_THRESHOLD + 1, "{html}");

    let html = render(Router::builder().page(plate_small), "/guild/1/levels")
        .await
        .unwrap();
    assert_eq!(html, "", "an unresolved guild renders no plate");
}
