use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use crate::tree::TreeQuota;
use crate::tree::fetch::RawGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EdgeKind {
    Partner,
    Parent,
    Child,
}

impl EdgeKind {
    const fn delta(self) -> i32 {
        match self {
            Self::Partner => 0,
            Self::Parent => -1,
            Self::Child => 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Pruned {
    pub raw: RawGraph,
    pub hidden: HashMap<i64, u32>,
    pub total: usize,
}

impl Pruned {
    #[must_use]
    pub const fn shown(&self) -> usize {
        self.raw.people.len()
    }

    #[must_use]
    pub const fn is_collapsed(&self) -> bool {
        self.shown() < self.total
    }
}

type Adjacency = BTreeMap<i64, BTreeSet<(i64, EdgeKind)>>;

fn adjacency(raw: &RawGraph) -> Adjacency {
    let mut adjacency: Adjacency = BTreeMap::new();

    for person in &raw.people {
        adjacency.entry(person.id).or_default();
    }

    for &(a, b) in &raw.partners {
        adjacency.entry(a).or_default().insert((b, EdgeKind::Partner));
        adjacency.entry(b).or_default().insert((a, EdgeKind::Partner));
    }

    for &(parent, child) in &raw.parents {
        adjacency.entry(parent).or_default().insert((child, EdgeKind::Child));
        adjacency.entry(child).or_default().insert((parent, EdgeKind::Parent));
    }

    adjacency
}

fn survey(
    adjacency: &Adjacency,
    focus: i64,
) -> BTreeMap<i64, (usize, i32, EdgeKind)> {
    let mut seen: BTreeMap<i64, (usize, i32, EdgeKind)> = BTreeMap::new();
    seen.insert(focus, (0, 0, EdgeKind::Partner));

    let mut queue = VecDeque::from([focus]);

    while let Some(current) = queue.pop_front() {
        let Some(&(hops, offset, _)) = seen.get(&current) else {
            continue;
        };

        let Some(neighbours) = adjacency.get(&current) else {
            continue;
        };

        for &(neighbour, kind) in neighbours {
            if seen.contains_key(&neighbour) {
                continue;
            }

            seen.insert(neighbour, (hops + 1, offset + kind.delta(), kind));
            queue.push_back(neighbour);
        }
    }

    seen
}

#[must_use]
pub fn prune(raw: RawGraph, focus: i64, quota: TreeQuota) -> Pruned {
    let total = raw.people.len();

    if total <= quota.node_budget {
        return Pruned { raw, hidden: HashMap::new(), total };
    }

    let adjacency = adjacency(&raw);
    let reachable = survey(&adjacency, focus);

    // (hops, edge kind, id)
    let mut ranked: Vec<(usize, EdgeKind, i64)> = reachable
        .iter()
        .filter(|(_, (_, offset, _))| offset.abs() <= quota.generation_span)
        .map(|(&id, &(hops, _, kind))| (hops, kind, id))
        .collect();
    ranked.sort_unstable();

    let mut kept: BTreeSet<i64> =
        ranked.into_iter().take(quota.node_budget).map(|(_, _, id)| id).collect();

    // The focus is the whole point of the picture
    kept.insert(focus);

    let mut hidden: HashMap<i64, u32> = HashMap::new();
    for &id in &kept {
        let Some(neighbours) = adjacency.get(&id) else {
            continue;
        };

        let dropped = neighbours
            .iter()
            .filter(|(neighbour, _)| !kept.contains(neighbour))
            .map(|(neighbour, _)| *neighbour)
            .collect::<BTreeSet<i64>>()
            .len();

        if dropped > 0 {
            hidden.insert(id, u32::try_from(dropped).unwrap_or(u32::MAX));
        }
    }

    let people =
        raw.people.into_iter().filter(|person| kept.contains(&person.id)).collect();

    let partners = raw
        .partners
        .into_iter()
        .filter(|(a, b)| kept.contains(a) && kept.contains(b))
        .collect();

    let parents = raw
        .parents
        .into_iter()
        .filter(|(parent, child)| kept.contains(parent) && kept.contains(child))
        .collect();

    Pruned {
        raw: RawGraph { people, partners, parents, truncated: raw.truncated },
        hidden,
        total,
    }
}
