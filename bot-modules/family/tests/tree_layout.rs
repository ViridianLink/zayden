//! Geometry invariants for the placed tree.
//!
//! Layout takes no font metrics, so these coordinates are identical on a dev
//! machine and inside the container -- which is what makes exact geometric
//! assertions possible at all.

use std::collections::{BTreeSet, HashMap};

use family::TreeQuota;
use family::tree::layout::{Layout, layout};
use family::tree::model::FamilyGraph;
use family::tree::svg::{canvas_for, render};
use family::tree::{NODE_W, RawGraph, RawPerson, prune};

// Macros rather than functions: `clippy.toml` allows `.expect()` only inside a
// `#[test]` item, so a free helper fn using it trips `expect_used`.
macro_rules! build {
    ($count:expr, $partners:expr, $parents:expr, $focus:expr) => {{
        let raw = RawGraph {
            people: (1..=$count)
                .map(|id| RawPerson { id, username: format!("Member {id}") })
                .collect(),
            partners: $partners,
            parents: $parents,
            truncated: false,
        };

        FamilyGraph::from_raw(&raw, $focus, &HashMap::new())
            .expect("focus is present")
    }};
}

/// A couple per generation, each couple parenting the next.
macro_rules! dynasty {
    ($generations:expr) => {{
        let generations: i64 = $generations;
        let count = generations * 2;
        let mut partners = Vec::new();
        let mut parents = Vec::new();

        for generation in 0..generations {
            let a = generation * 2 + 1;
            partners.push((a, a + 1));

            if generation + 1 < generations {
                parents.push((a, a + 2));
                parents.push((a + 1, a + 2));
            }
        }

        let graph: FamilyGraph = build!(count, partners, parents, 1);
        graph
    }};
}

/// Boxes in the same row must never overlap; this is the single most visible
/// layout failure, and `separate` is meant to make it impossible.
fn assert_no_overlap(graph: &FamilyGraph, placed: &Layout) {
    let mut rows: HashMap<i32, Vec<f32>> = HashMap::new();

    for node in 0..graph.len() {
        let (Some(&generation), Some(&left)) =
            (placed.generation.get(node), placed.x.get(node))
        else {
            continue;
        };
        rows.entry(generation).or_default().push(left);
    }

    for (generation, mut lefts) in rows {
        lefts.sort_by(f32::total_cmp);

        for pair in lefts.windows(2) {
            let [a, b] = pair else { continue };
            assert!(
                b - a >= NODE_W,
                "generation {generation}: boxes at {a} and {b} overlap",
            );
        }
    }
}

#[test]
fn boxes_never_overlap_in_a_dynasty() {
    let graph: FamilyGraph = dynasty!(5);
    let placed = layout(&graph);
    assert_no_overlap(&graph, &placed);
}

#[test]
fn boxes_never_overlap_in_a_wide_sibling_group() {
    let graph: FamilyGraph =
        build!(41, vec![], (2..=41).map(|id| (1, id)).collect(), 1);
    let placed = layout(&graph);
    assert_no_overlap(&graph, &placed);
}

#[test]
fn boxes_never_overlap_at_every_tier_budget() {
    for quota in [TreeQuota::FREE, TreeQuota::PRO, TreeQuota::ULTRA] {
        let raw = RawGraph {
            people: (1..=400)
                .map(|id| RawPerson { id, username: format!("M{id}") })
                .collect(),
            partners: Vec::new(),
            parents: (2..=400).map(|id| (id / 2, id)).collect(),
            truncated: false,
        };

        let pruned = prune(raw, 1, quota);
        let graph = FamilyGraph::from_raw(&pruned.raw, 1, &pruned.hidden)
            .expect("focus survives pruning");
        let placed = layout(&graph);

        assert_no_overlap(&graph, &placed);
    }
}

/// Partners are drawn as one household, so nobody else may sit between them.
#[test]
fn nothing_is_drawn_between_two_partners() {
    let graph: FamilyGraph = dynasty!(4);
    let placed = layout(&graph);

    for &(a, b) in &graph.partner_edges {
        let (Some(&ga), Some(&gb)) =
            (placed.generation.get(a), placed.generation.get(b))
        else {
            continue;
        };
        assert_eq!(ga, gb, "partners must share a row");

        let (Some(&xa), Some(&xb)) = (placed.x.get(a), placed.x.get(b)) else {
            continue;
        };
        let (low, high) = (xa.min(xb), xa.max(xb));

        for other in 0..graph.len() {
            if other == a || other == b {
                continue;
            }
            if placed.generation.get(other) != Some(&ga) {
                continue;
            }
            let Some(&x) = placed.x.get(other) else { continue };

            assert!(
                x <= low || x >= high,
                "member {other} at {x} sits between partners at {low}/{high}",
            );
        }
    }
}

#[test]
fn a_polygamous_household_stays_contiguous() {
    let graph: FamilyGraph = build!(6, vec![(1, 2), (1, 3), (2, 3)], vec![], 1);
    let placed = layout(&graph);

    assert_no_overlap(&graph, &placed);
    assert_eq!(graph.blocks.len(), 4, "one group of three plus three singles");
}

#[test]
fn generations_are_evenly_pitched_and_ordered() {
    let graph: FamilyGraph = dynasty!(4);
    let placed = layout(&graph);

    assert_eq!(placed.generations, 4);

    for node in 0..graph.len() {
        let (Some(&generation), Some(&y)) =
            (placed.generation.get(node), placed.y.get(node))
        else {
            continue;
        };

        let expected = f32::from(i16::try_from(generation).unwrap_or(0))
            .mul_add(family::tree::ROW_PITCH, family::tree::MARGIN);
        assert!(
            (y - expected).abs() < f32::EPSILON,
            "generation {generation} sits at {y}, expected {expected}",
        );
    }
}

#[test]
fn the_canvas_stays_inside_the_tier_budget() {
    for quota in [TreeQuota::FREE, TreeQuota::PRO, TreeQuota::ULTRA] {
        let count = i64::try_from(quota.node_budget).expect("fits i64");
        let graph: FamilyGraph =
            build!(count, vec![], (2..=count).map(|id| (1, id)).collect(), 1);
        let placed = layout(&graph);
        let (canvas, _) = canvas_for(&placed, quota);

        assert!(
            canvas.width * canvas.height <= quota.max_canvas_pixels,
            "a full {}-node tree needs {}x{}, over the {} budget",
            quota.node_budget,
            canvas.width,
            canvas.height,
            quota.max_canvas_pixels,
        );
    }
}

/// The same input must place identically every time; a picture that reshuffles
/// between identical invocations reads as a bug.
#[test]
fn layout_is_reproducible() {
    let a: FamilyGraph = dynasty!(5);
    let b: FamilyGraph = dynasty!(5);
    let first = layout(&a);
    let second = layout(&b);

    assert_eq!(first.x, second.x);
    assert_eq!(first.y, second.y);
    assert_eq!(first.generation, second.generation);
}

#[test]
fn an_empty_graph_lays_out_to_nothing() {
    let placed = layout(&FamilyGraph::default());

    assert_eq!(placed.generations, 0);
    assert_eq!(placed.x, Vec::<f32>::new());
}

/// A star graph is the pathological case: one parent, hundreds of children,
/// all in one row. It fits any node budget by count while being far too wide
/// to draw legibly, so the budget must give way.
#[test]
fn a_graph_too_wide_to_read_loses_people_rather_than_legibility() {
    let raw = RawGraph {
        people: (1..=400)
            .map(|id| RawPerson { id, username: format!("Member {id}") })
            .collect(),
        partners: Vec::new(),
        parents: (2..=400).map(|id| (1, id)).collect(),
        truncated: false,
    };

    let composed =
        family::tree::compose(&raw, 1, TreeQuota::ULTRA).expect("focus survives");

    assert!(
        composed.scale >= family::tree::MIN_LEGIBLE_SCALE,
        "scale {} is below the legibility floor",
        composed.scale,
    );
    assert!(
        composed.shown < TreeQuota::ULTRA.node_budget,
        "the budget should have given way, showed {}",
        composed.shown,
    );
    assert!(composed.is_collapsed());
}

/// The legibility floor must not punish ordinary trees.
#[test]
fn a_small_graph_is_composed_at_full_scale_and_intact() {
    let raw = RawGraph {
        people: (1..=8)
            .map(|id| RawPerson { id, username: format!("Member {id}") })
            .collect(),
        partners: vec![(1, 2)],
        parents: (3..=8).map(|id| (1, id)).collect(),
        truncated: false,
    };

    let composed =
        family::tree::compose(&raw, 1, TreeQuota::FREE).expect("focus survives");

    assert_eq!(composed.shown, 8);
    assert!(!composed.is_collapsed(), "a small family is never collapsed");
    assert!((composed.scale - 1.0).abs() < f32::EPSILON);
}

/// The premium argument, restated after the legibility floor: a bigger canvas
/// buys *more people at a readable size*, not a larger unreadable picture.
#[test]
fn a_higher_tier_composes_more_people_while_staying_legible() {
    let raw = RawGraph {
        people: (1..=400)
            .map(|id| RawPerson { id, username: format!("Member {id}") })
            .collect(),
        partners: Vec::new(),
        parents: (2..=400).map(|id| (id / 3, id)).collect(),
        truncated: false,
    };

    let free =
        family::tree::compose(&raw, 1, TreeQuota::FREE).expect("focus survives");
    let ultra =
        family::tree::compose(&raw, 1, TreeQuota::ULTRA).expect("focus survives");

    assert!(
        ultra.shown > free.shown,
        "ultra showed {} but free showed {}",
        ultra.shown,
        free.shown,
    );
    for composed in [&free, &ultra] {
        assert!(
            composed.scale >= family::tree::MIN_LEGIBLE_SCALE,
            "scale {} is below the legibility floor",
            composed.scale,
        );
    }
}

/// Not an assertion -- a way to actually look at the thing. Geometry tests
/// cannot tell you whether the picture reads well.
///
/// `cargo test -p family --test tree_layout -- --ignored --nocapture`
#[ignore = "writes sample PNGs for visual inspection"]
#[test]
fn dump_sample_pngs() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime should build");

    let renderer = match zayden_graphics::Renderer::shared() {
        Ok(renderer) => renderer,
        Err(e) => {
            println!("no usable font on this host, skipping: {e}");
            return;
        },
    };

    for (people, quota, tier) in [
        (2i64, TreeQuota::FREE, "free"),
        (12, TreeQuota::FREE, "free"),
        (200, TreeQuota::FREE, "free"),
        (200, TreeQuota::ULTRA, "ultra"),
    ] {
        let raw = RawGraph {
            people: (1..=people)
                .map(|id| RawPerson { id, username: format!("Member {id}") })
                .collect(),
            partners: (1..people).step_by(6).map(|id| (id, id + 1)).collect(),
            parents: (3..=people).map(|id| (id / 3, id)).collect(),
            truncated: false,
        };

        let composed =
            family::tree::compose(&raw, 1, quota).expect("focus survives");
        let svg = render(&composed.graph, &composed.layout, quota, &BTreeSet::new());

        let png = runtime
            .block_on(renderer.render(
                svg.markup,
                svg.canvas,
                Vec::new(),
                quota.raster_limits(),
            ))
            .expect("render should succeed");

        let path = format!("../../target/tree-sample-{people}-{tier}.png");
        std::fs::write(&path, png).expect("sample should write");
        println!(
            "wrote {path} ({}x{} @ {:.2}, {} of {} people)",
            svg.canvas.width,
            svg.canvas.height,
            composed.scale,
            composed.shown,
            composed.total,
        );
    }
}
