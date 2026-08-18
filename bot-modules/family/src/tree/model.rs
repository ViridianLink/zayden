use std::collections::{BTreeMap, HashMap};

use crate::tree::fetch::RawGraph;

pub type NodeIdx = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub hidden: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub members: Vec<NodeIdx>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Union {
    pub parents: Vec<NodeIdx>,
    pub children: Vec<NodeIdx>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FamilyGraph {
    pub people: Vec<Person>,
    pub focus: NodeIdx,
    pub blocks: Vec<Block>,
    pub block_of: Vec<usize>,
    pub unions: Vec<Union>,
    pub partner_edges: Vec<(NodeIdx, NodeIdx)>,
    pub parent_edges: Vec<(NodeIdx, NodeIdx)>,
}

struct Dsu {
    parent: Vec<usize>,
}

impl Dsu {
    fn new(len: usize) -> Self {
        Self { parent: (0..len).collect() }
    }

    fn find(&mut self, node: usize) -> usize {
        let mut current = node;

        while let Some(&next) = self.parent.get(current) {
            if next == current {
                return current;
            }

            // Path halving, so repeated lookups stay near-flat.
            let grand = self.parent.get(next).copied().unwrap_or(next);
            if let Some(slot) = self.parent.get_mut(current) {
                *slot = grand;
            }
            current = grand;
        }

        current
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }

        if let Some(slot) = self.parent.get_mut(ra.max(rb)) {
            *slot = ra.min(rb);
        }
    }
}

impl FamilyGraph {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.people.len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.people.is_empty()
    }

    #[must_use]
    pub fn person(&self, node: NodeIdx) -> Option<&Person> {
        self.people.get(node)
    }

    #[must_use]
    pub fn from_raw(
        raw: &RawGraph,
        focus_id: i64,
        hidden: &HashMap<i64, u32>,
    ) -> Option<Self> {
        let index: HashMap<i64, NodeIdx> = raw
            .people
            .iter()
            .enumerate()
            .map(|(node, person)| (person.id, node))
            .collect();

        let focus = index.get(&focus_id).copied()?;

        let people: Vec<Person> = raw
            .people
            .iter()
            .map(|person| Person {
                id: person.id,
                name: person.username.clone(),
                hidden: hidden.get(&person.id).copied().unwrap_or(0),
            })
            .collect();

        let resolve = |pair: &(i64, i64)| -> Option<(NodeIdx, NodeIdx)> {
            Some((index.get(&pair.0).copied()?, index.get(&pair.1).copied()?))
        };

        let partner_edges: Vec<(NodeIdx, NodeIdx)> =
            raw.partners.iter().filter_map(resolve).collect();
        let parent_edges: Vec<(NodeIdx, NodeIdx)> =
            raw.parents.iter().filter_map(resolve).collect();

        let (blocks, block_of) = build_blocks(people.len(), &partner_edges, focus);
        let unions = build_unions(&parent_edges);

        Some(Self {
            people,
            focus,
            blocks,
            block_of,
            unions,
            partner_edges,
            parent_edges,
        })
    }
}

fn build_blocks(
    len: usize,
    partner_edges: &[(NodeIdx, NodeIdx)],
    focus: NodeIdx,
) -> (Vec<Block>, Vec<usize>) {
    let mut dsu = Dsu::new(len);
    for &(a, b) in partner_edges {
        dsu.union(a, b);
    }

    // BTreeMap keyed on the root keeps block order stable and ascending.
    let mut grouped: BTreeMap<usize, Vec<NodeIdx>> = BTreeMap::new();
    for node in 0..len {
        grouped.entry(dsu.find(node)).or_default().push(node);
    }

    let mut blocks = Vec::with_capacity(grouped.len());
    let mut block_of = vec![0usize; len];

    for (block_index, (_, mut members)) in grouped.into_iter().enumerate() {
        // People arrive sorted by id, so index order is id order.
        members.sort_unstable_by_key(|&node| (node != focus, node));

        for &node in &members {
            if let Some(slot) = block_of.get_mut(node) {
                *slot = block_index;
            }
        }

        blocks.push(Block { members });
    }

    (blocks, block_of)
}

fn build_unions(parent_edges: &[(NodeIdx, NodeIdx)]) -> Vec<Union> {
    let mut parents_of: BTreeMap<NodeIdx, Vec<NodeIdx>> = BTreeMap::new();
    for &(parent, child) in parent_edges {
        parents_of.entry(child).or_default().push(parent);
    }

    let mut by_parent_set: BTreeMap<Vec<NodeIdx>, Vec<NodeIdx>> = BTreeMap::new();
    for (child, mut parents) in parents_of {
        parents.sort_unstable();
        parents.dedup();
        by_parent_set.entry(parents).or_default().push(child);
    }

    by_parent_set
        .into_iter()
        .map(|(parents, mut children)| {
            children.sort_unstable();
            Union { parents, children }
        })
        .collect()
}
