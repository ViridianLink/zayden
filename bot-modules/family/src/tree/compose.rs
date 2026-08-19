use crate::tree::layout::{Layout, layout};
use crate::tree::model::FamilyGraph;
use crate::tree::svg::canvas_for;
use crate::tree::{MIN_LEGIBLE_SCALE, MIN_NODE_BUDGET, RawGraph, TreeQuota, prune};

#[derive(Debug, Clone)]
pub struct Composed {
    pub graph: FamilyGraph,
    pub layout: Layout,
    pub shown: usize,
    pub total: usize,
    pub scale: f32,
    pub truncated: bool,
}

impl Composed {
    #[must_use]
    pub const fn is_collapsed(&self) -> bool {
        self.shown < self.total
    }
}

#[must_use]
pub fn compose(raw: &RawGraph, focus: i64, quota: TreeQuota) -> Option<Composed> {
    let total = raw.people.len();
    let truncated = raw.truncated;
    let mut budget = quota.node_budget;

    loop {
        let attempt = TreeQuota { node_budget: budget, ..quota };
        let pruned = prune(raw.clone(), focus, attempt);
        let shown = pruned.shown();

        let graph = FamilyGraph::from_raw(&pruned.raw, focus, &pruned.hidden)?;
        let placed = layout(&graph);
        let (_, scale) = canvas_for(&placed, quota);

        if scale >= MIN_LEGIBLE_SCALE || budget <= MIN_NODE_BUDGET {
            return Some(Composed {
                graph,
                layout: placed,
                shown,
                total,
                scale,
                truncated,
            });
        }

        budget = (budget * 3 / 4).max(MIN_NODE_BUDGET);
    }
}
