use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use http_body_util::BodyExt;
use topcoat::Result as ViewResult;
use topcoat::router::request::Request;
use topcoat::router::{Body, Router, page};
use topcoat::runtime::RouterBuilderRuntimeExt;
use topcoat::view::{View, view};
use web::components::confirm::confirm_button;
use web::components::guild_grid::{GuildCard, guild_grid};
use web::components::icons::{Icon, icon};
use web::components::key_list::key_list_field;
use web::components::select::{SelectOption, select_field};
use web::components::settings::{
    save_button,
    save_feedback,
    setting_field,
    toggle_field,
};
use web::components::shape_skeleton::{SkeletonShape, shape_skeleton};

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const CRATE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn partial(name: &str) -> TestResult<String> {
    Ok(fs::read_to_string(Path::new(CRATE_ROOT).join("style/partials").join(name))?)
}

fn stylesheet() -> TestResult<String> {
    let mut css = String::new();
    for entry in fs::read_dir(Path::new(CRATE_ROOT).join("style/partials"))? {
        css.push_str(&fs::read_to_string(entry?.path())?);
        css.push('\n');
    }
    Ok(css)
}

/// Every rule block as (selectors, declarations), split on braces.
fn rules(css: &str) -> Vec<(String, String)> {
    css.split('}')
        .filter_map(|chunk| chunk.rsplit_once('{'))
        .map(|(head, declarations)| {
            let selectors = head.rsplit_once('{').map_or(head, |(_, inner)| inner);
            (selectors.trim().to_owned(), declarations.to_owned())
        })
        .collect()
}

const fn is_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
}

fn names_class(selectors: &str, class: &str) -> bool {
    let needle = format!(".{class}");
    selectors
        .split(needle.as_str())
        .skip(1)
        .any(|rest| rest.chars().next().is_none_or(|c| !is_name(c)))
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) -> TestResult {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_rs(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

#[page("/everything")]
async fn everything() -> ViewResult<impl View> {
    let guilds = [
        GuildCard {
            id: "1".to_owned(),
            name: "Alpha".to_owned(),
            icon: Some("abc".to_owned()),
        },
        GuildCard { id: "2".to_owned(), name: "Beta".to_owned(), icon: None },
    ];
    let options =
        vec![SelectOption { value: "1".to_owned(), label: "# chat".to_owned() }];
    let keys = ["solar".to_owned()];

    Ok(view! {
        <form method="post" data-pending="">
            confirm_button(label: "Delete", prompt: "Sure?", confirm: "Delete it")
            save_button()
        </form>
        guild_grid(guilds: &guilds)
        icon(name: Icon::Plus)
        key_list_field(label: "Tags", keys: &keys, list: "tags", max: 1)
        select_field(label: "A", name: "a", selected: "", options: Ok(options))
        select_field(
            label: "B",
            name: "b",
            selected: "9",
            options: Err("down".to_owned())
        )
        setting_field(label: "C", name: "c", value: "", hint: Some("hint"))
        toggle_field(label: "D", name: "d", value: true)
        save_feedback(outcome: Ok(()))
        save_feedback(outcome: Err("nope"))
        shape_skeleton(shape: SkeletonShape::Btn)
        shape_skeleton(shape: SkeletonShape::Badge)
        shape_skeleton(shape: SkeletonShape::Switcher)
        shape_skeleton(shape: SkeletonShape::Row)
        shape_skeleton(shape: SkeletonShape::Card)
        shape_skeleton(shape: SkeletonShape::Panel)
    })
}

async fn everything_html() -> TestResult<String> {
    let router = Router::builder().page(everything).runtime().build();
    let response =
        router.handle(Request::get("/everything").body(Body::empty())?).await;
    Ok(String::from_utf8(response.into_body().collect().await?.to_bytes().to_vec())?)
}

fn rendered_classes(html: &str) -> BTreeSet<String> {
    html.split("class=\"")
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(classes, _)| classes))
        .flat_map(str::split_whitespace)
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
async fn every_rendered_class_has_a_rule() {
    let css = stylesheet().unwrap();
    let selectors: Vec<_> =
        rules(&css).into_iter().map(|(selectors, _)| selectors).collect();
    let classes = rendered_classes(&everything_html().await.unwrap());

    let missing: Vec<_> = classes
        .iter()
        .filter(|class| {
            !selectors.iter().any(|selectors| names_class(selectors, class))
        })
        .collect();

    assert!(classes.len() > 30, "only {} classes rendered", classes.len());
    assert!(missing.is_empty(), "classes with no rule: {missing:?}");
}

#[tokio::test]
async fn every_disableable_button_has_a_disabled_rule() {
    let css = stylesheet().unwrap();
    let html = everything_html().await.unwrap();

    let buttons: Vec<_> = html
        .split("<button")
        .skip(1)
        .filter_map(|rest| rest.split_once('>').map(|(attrs, _)| attrs))
        .collect();
    let unstyled: Vec<_> = buttons
        .iter()
        .filter(|attrs| {
            attrs.contains("type=\"submit\"") || attrs.contains("disabled")
        })
        .filter(|attrs| {
            !rendered_classes(attrs)
                .iter()
                .any(|class| css.contains(&format!(".{class}:disabled")))
        })
        .collect();

    assert!(buttons.len() >= 4, "only {} buttons rendered", buttons.len());
    assert!(
        unstyled.is_empty(),
        "disabled state with no visible rule: {unstyled:?}"
    );
}

#[test]
fn the_confirm_trigger_swaps_to_cancel_when_it_opens() {
    let css = partial("confirm.css")
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" > ", ">");

    assert!(css.contains(".confirm>summary .confirm-cancel"));
    assert!(css.contains(".confirm[open]>summary .confirm-label"));
    assert!(css.contains(".confirm[open]>summary .confirm-cancel"));
}

#[test]
fn the_danger_style_is_only_reachable_through_the_confirm_component() {
    let mut paths = Vec::new();
    collect_rs(&Path::new(CRATE_ROOT).join("src"), &mut paths).unwrap();

    let mut unguarded = Vec::new();
    for path in &paths {
        let text = fs::read_to_string(path).unwrap();
        if !path.ends_with("components/confirm.rs") && text.contains("btn-danger") {
            unguarded.push(path.display().to_string());
        }
    }

    assert!(paths.len() > 10, "only {} sources scanned", paths.len());
    assert!(
        unguarded.is_empty(),
        "danger buttons outside the confirm component: {unguarded:?}"
    );
}

#[test]
fn every_skeleton_shape_reserves_a_height() {
    let css = partial("skeleton.css").unwrap();
    let rules = rules(&css);

    let unsized_shapes: Vec<_> = SkeletonShape::ALL
        .iter()
        .map(|shape| shape.class())
        .filter(|class| {
            !rules.iter().any(|(selectors, declarations)| {
                names_class(selectors, class) && declarations.contains("height")
            })
        })
        .collect();

    assert!(
        unsized_shapes.is_empty(),
        "skeleton shapes with no height rule: {unsized_shapes:?}"
    );
}

#[test]
fn every_skeleton_shape_pulses_unless_motion_is_reduced() {
    let css = partial("skeleton.css").unwrap();
    assert!(css.contains("@keyframes skeleton-pulse"));

    let (_, reduced) = css
        .split_once("@media (prefers-reduced-motion: reduce)")
        .ok_or("skeleton.css has no reduced-motion query")
        .unwrap();
    let (selectors, declarations) = reduced
        .split_once('}')
        .and_then(|(rule, _)| rule.rsplit_once('{'))
        .ok_or("the reduced-motion query holds no rule")
        .unwrap();

    for shape in SkeletonShape::ALL {
        assert!(
            names_class(selectors, shape.class()),
            "{} keeps pulsing under reduced motion",
            shape.class()
        );
    }
    assert!(declarations.contains("animation: none"));
}
