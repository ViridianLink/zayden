//! Name sanitising, escaping and canvas budgeting for the SVG stage.
//!
//! Display names are attacker-controlled: a member can rename themselves to
//! anything Discord accepts, and that string is interpolated straight into
//! markup. Escaping is the whole defence, so it is asserted here rather than
//! assumed.

use std::collections::BTreeSet;

use family::TreeQuota;
use family::tree::layout::layout;
use family::tree::model::FamilyGraph;
use family::tree::svg::{canvas_for, render, sanitise};
use family::tree::{MAX_NAME_CHARS, RawGraph, RawPerson};

// Macros rather than functions: `clippy.toml` allows `.expect()` only inside a
// `#[test]` item, so a free helper fn using it trips `expect_used` under the
// workspace `-D warnings` gate.

macro_rules! graph_of {
    ($names:expr) => {{
        let people = $names
            .iter()
            .enumerate()
            .map(|(index, name)| RawPerson {
                id: i64::try_from(index).unwrap_or(0) + 1,
                username: (*name).to_string(),
            })
            .collect();

        let raw = RawGraph {
            people,
            partners: Vec::new(),
            parents: Vec::new(),
            truncated: false,
        };

        FamilyGraph::from_raw(&raw, 1, &std::collections::HashMap::new())
            .expect("focus is present")
    }};
}

macro_rules! markup_for {
    ($names:expr) => {{
        let graph: FamilyGraph = graph_of!($names);
        let placed = layout(&graph);
        render(&graph, &placed, TreeQuota::FREE, &BTreeSet::new()).markup
    }};
}

#[test]
fn xml_metacharacters_are_escaped() {
    assert_eq!(sanitise("a&b", 1), "a&amp;b");
    assert_eq!(sanitise("a<b>c", 1), "a&lt;b&gt;c");
    assert_eq!(sanitise(r#"say "hi""#, 1), "say &quot;hi&quot;");
    assert_eq!(sanitise("it's", 1), "it&apos;s");
}

/// The attack this is really guarding: a name that closes the enclosing
/// element and opens one of its own.
#[test]
fn a_name_cannot_break_out_of_the_markup() {
    let markup = markup_for!(&[r#""/><script>x</script><text a=""#]);

    assert!(
        !markup.contains("<script>"),
        "a display name must never introduce an element",
    );
    assert!(markup.contains("&lt;script&gt;"), "it should appear escaped");
}

#[test]
fn control_and_bidi_characters_are_stripped() {
    // U+202E flips the rendering direction of everything after it.
    assert_eq!(sanitise("ab\u{202E}cd", 1), "abcd");
    assert_eq!(sanitise("a\u{0007}b", 1), "ab");
    assert_eq!(sanitise("a\u{2066}b\u{2069}c", 1), "abc");
    assert_eq!(sanitise("a\u{200B}b", 1), "ab");
}

/// Noto Core and Noto CJK do not cover emoji, so leaving them in would draw
/// tofu boxes rather than the name.
#[test]
fn pictographic_characters_are_stripped() {
    assert_eq!(sanitise("Alice \u{1F600}", 1), "Alice");
    assert_eq!(sanitise("\u{2764}\u{FE0F} Bob", 1), "Bob");
    assert_eq!(sanitise("A\u{1F1EC}\u{1F1E7}B", 1), "AB");
}

#[test]
fn a_name_that_strips_to_nothing_falls_back_to_the_id() {
    assert_eq!(sanitise("\u{1F600}\u{1F601}", 123_456_789), "user-6789");
    assert_eq!(sanitise("   ", 42), "user-0042");
    assert_eq!(sanitise("", 7), "user-0007");
}

#[test]
fn scripts_that_are_not_emoji_survive() {
    assert_eq!(sanitise("Ünïcödé", 1), "Ünïcödé");
    assert_eq!(sanitise("Пётр", 1), "Пётр");
    assert_eq!(sanitise("日本語", 1), "日本語");
}

/// Truncation counts `chars`, never bytes: byte slicing a multi-byte name
/// would panic, which is why `string_slice` is a workspace lint.
#[test]
fn long_names_truncate_on_a_character_boundary() {
    let long = "é".repeat(60);
    let short = sanitise(&long, 1);

    assert!(short.chars().count() <= MAX_NAME_CHARS);
    assert!(short.ends_with('\u{2026}'));
}

#[test]
fn a_name_at_the_limit_is_left_alone() {
    let name = "a".repeat(MAX_NAME_CHARS);
    assert_eq!(sanitise(&name, 1), name);
}

#[test]
fn the_markup_is_a_well_formed_single_svg_root() {
    let markup = markup_for!(&["Alice", "Bob"]);

    assert!(markup.starts_with("<svg "));
    assert!(markup.ends_with("</svg>"));
    assert_eq!(markup.matches("<svg ").count(), 1);
    assert_eq!(markup.matches("</svg>").count(), 1);
}

/// The renderer refuses a canvas that disagrees with the SVG's declared size,
/// so these two must be produced together and stay consistent.
#[test]
fn the_declared_size_matches_the_reported_canvas() {
    let graph: FamilyGraph = graph_of!(&["Alice", "Bob", "Carol"]);
    let placed = layout(&graph);
    let svg = render(&graph, &placed, TreeQuota::FREE, &BTreeSet::new());

    assert!(
        svg.markup.contains(&format!(r#"width="{}""#, svg.canvas.width)),
        "declared width should match the canvas",
    );
    assert!(
        svg.markup.contains(&format!(r#"height="{}""#, svg.canvas.height)),
        "declared height should match the canvas",
    );
}

/// A wide graph must shrink to fit rather than being cropped or rejected --
/// and it must stay inside both the area and the edge ceiling.
#[test]
fn an_oversized_layout_is_scaled_into_the_tier_budget() {
    let names: Vec<&str> = vec!["Someone"; 200];
    let graph: FamilyGraph = graph_of!(&names);
    let placed = layout(&graph);

    for quota in [TreeQuota::FREE, TreeQuota::PRO, TreeQuota::ULTRA] {
        let (canvas, scale) = canvas_for(&placed, quota);

        assert!(canvas.width <= quota.max_canvas_dim);
        assert!(canvas.height <= quota.max_canvas_dim);
        assert!(
            canvas.width * canvas.height <= quota.max_canvas_pixels,
            "canvas {}x{} exceeds the {} pixel budget",
            canvas.width,
            canvas.height,
            quota.max_canvas_pixels,
        );
        assert!(scale > 0.0 && scale <= 1.0);
    }
}

/// The same picture at a higher tier is rendered larger, which is the whole
/// legibility argument for the bigger budget.
#[test]
fn a_higher_tier_renders_the_same_graph_larger() {
    let names: Vec<&str> = vec!["Someone"; 200];
    let graph: FamilyGraph = graph_of!(&names);
    let placed = layout(&graph);

    let (free, _) = canvas_for(&placed, TreeQuota::FREE);
    let (ultra, _) = canvas_for(&placed, TreeQuota::ULTRA);

    assert!(
        ultra.width * ultra.height > free.width * free.height,
        "ultra {}x{} should beat free {}x{}",
        ultra.width,
        ultra.height,
        free.width,
        free.height,
    );
}

/// A small tree must not be blown up to fill the budget; scale caps at 1.
#[test]
fn a_small_tree_is_never_upscaled() {
    let graph: FamilyGraph = graph_of!(&["Alice", "Bob"]);
    let placed = layout(&graph);
    let (_, scale) = canvas_for(&placed, TreeQuota::ULTRA);

    assert!((scale - 1.0).abs() < f32::EPSILON, "scale was {scale}");
}

#[test]
fn hidden_neighbour_counts_are_drawn_as_chips() {
    let raw = RawGraph {
        people: vec![RawPerson { id: 1, username: "Alice".to_string() }],
        partners: Vec::new(),
        parents: Vec::new(),
        truncated: false,
    };
    let hidden = std::collections::HashMap::from([(1i64, 7u32)]);
    let graph = FamilyGraph::from_raw(&raw, 1, &hidden).expect("focus is present");
    let placed = layout(&graph);
    let markup = render(&graph, &placed, TreeQuota::FREE, &BTreeSet::new()).markup;

    assert!(markup.contains(">+7<"), "the chip should report 7 hidden");
}
