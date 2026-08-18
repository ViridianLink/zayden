use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::tree::model::{FamilyGraph, NodeIdx};
use crate::tree::{
    GEN_RELAX_PASSES,
    MARGIN,
    NODE_GAP,
    NODE_H,
    NODE_W,
    ORDER_SWEEPS,
    PARTNER_GAP,
    REFINE_PASSES,
    ROW_PITCH,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    pub generation: Vec<i32>,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub back_edges: Vec<(NodeIdx, NodeIdx)>,
    pub width: f32,
    pub height: f32,
    pub generations: usize,
}

impl Layout {
    #[must_use]
    pub fn centre_x(&self, node: NodeIdx) -> Option<f32> {
        self.x.get(node).map(|left| left + NODE_W / 2.0)
    }

    #[must_use]
    pub fn top(&self, node: NodeIdx) -> Option<f32> {
        self.y.get(node).copied()
    }

    #[must_use]
    pub fn bottom(&self, node: NodeIdx) -> Option<f32> {
        self.y.get(node).map(|top| top + NODE_H)
    }
}

fn count_f32(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

fn gen_f32(value: i32) -> f32 {
    f32::from(i16::try_from(value).unwrap_or(i16::MAX))
}

fn block_width(members: usize) -> f32 {
    let count = count_f32(members).max(1.0);
    count.mul_add(NODE_W, (count - 1.0) * PARTNER_GAP)
}

fn structural_back_edges(graph: &FamilyGraph) -> Vec<bool> {
    let len = graph.len();
    let mut children: Vec<Vec<(usize, NodeIdx)>> = vec![Vec::new(); len];

    for (edge, &(parent, child)) in graph.parent_edges.iter().enumerate() {
        if let Some(slot) = children.get_mut(parent) {
            slot.push((edge, child));
        }
    }

    // 0 = unvisited, 1 = on the current DFS stack, 2 = finished.
    let mut colour = vec![0u8; len];
    let mut back = vec![false; graph.parent_edges.len()];

    for start in 0..len {
        if colour.get(start).copied().unwrap_or(2) != 0 {
            continue;
        }

        if let Some(slot) = colour.get_mut(start) {
            *slot = 1;
        }
        let mut stack: Vec<(NodeIdx, usize)> = vec![(start, 0)];

        while let Some(&(node, cursor)) = stack.last() {
            let next = children.get(node).and_then(|kids| kids.get(cursor));

            let Some(&(edge, child)) = next else {
                if let Some(slot) = colour.get_mut(node) {
                    *slot = 2;
                }
                stack.pop();
                continue;
            };

            if let Some(top) = stack.last_mut() {
                top.1 += 1;
            }

            match colour.get(child).copied().unwrap_or(2) {
                // Reaching a node already on the stack closes a cycle.
                1 => {
                    if let Some(slot) = back.get_mut(edge) {
                        *slot = true;
                    }
                },
                0 => {
                    if let Some(slot) = colour.get_mut(child) {
                        *slot = 1;
                    }
                    stack.push((child, 0));
                },
                _ => {},
            }
        }
    }

    back
}

fn assign_generations(graph: &FamilyGraph) -> (Vec<i32>, Vec<(NodeIdx, NodeIdx)>) {
    let len = graph.len();
    let mut back = structural_back_edges(graph);
    let mut generation = vec![0i32; len];
    let mut seen = vec![false; len];

    let mut neighbours: Vec<Vec<(NodeIdx, i32)>> = vec![Vec::new(); len];
    for &(a, b) in &graph.partner_edges {
        if let Some(slot) = neighbours.get_mut(a) {
            slot.push((b, 0));
        }
        if let Some(slot) = neighbours.get_mut(b) {
            slot.push((a, 0));
        }
    }
    for &(parent, child) in &graph.parent_edges {
        if let Some(slot) = neighbours.get_mut(parent) {
            slot.push((child, 1));
        }
        if let Some(slot) = neighbours.get_mut(child) {
            slot.push((parent, -1));
        }
    }

    let mut queue = VecDeque::from([graph.focus]);
    if let Some(slot) = seen.get_mut(graph.focus) {
        *slot = true;
    }

    while let Some(node) = queue.pop_front() {
        let base = generation.get(node).copied().unwrap_or(0);
        let Some(edges) = neighbours.get(node) else {
            continue;
        };

        for &(next, delta) in edges {
            if seen.get(next).copied().unwrap_or(true) {
                continue;
            }

            if let Some(slot) = seen.get_mut(next) {
                *slot = true;
            }
            if let Some(slot) = generation.get_mut(next) {
                *slot = base + delta;
            }
            queue.push_back(next);
        }
    }

    for _ in 0..GEN_RELAX_PASSES {
        let mut changed = false;

        // Partners share a generation.
        for block in &graph.blocks {
            let target = block
                .members
                .iter()
                .filter_map(|&member| generation.get(member).copied())
                .max()
                .unwrap_or(0);

            for &member in &block.members {
                if let Some(slot) = generation.get_mut(member)
                    && *slot != target
                {
                    *slot = target;
                    changed = true;
                }
            }
        }

        // Children sit below every parent that is not a back edge.
        for (edge, &(parent, child)) in graph.parent_edges.iter().enumerate() {
            if back.get(edge).copied().unwrap_or(false) {
                continue;
            }

            let floor = generation.get(parent).copied().unwrap_or(0) + 1;
            if let Some(slot) = generation.get_mut(child)
                && *slot < floor
            {
                *slot = floor;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    // Anything still pointing the wrong way is unsatisfiable
    for (edge, &(parent, child)) in graph.parent_edges.iter().enumerate() {
        let above = generation.get(parent).copied().unwrap_or(0);
        let below = generation.get(child).copied().unwrap_or(0);

        if below <= above
            && let Some(slot) = back.get_mut(edge)
        {
            *slot = true;
        }
    }

    let floor = generation.iter().copied().min().unwrap_or(0);
    for slot in &mut generation {
        *slot -= floor;
    }

    let back_edges = graph
        .parent_edges
        .iter()
        .enumerate()
        .filter(|(edge, _)| back.get(*edge).copied().unwrap_or(false))
        .map(|(_, &pair)| pair)
        .collect();

    (generation, back_edges)
}

struct BlockLinks {
    children: Vec<Vec<usize>>,
    parents: Vec<Vec<usize>>,
}

fn block_links(graph: &FamilyGraph) -> BlockLinks {
    let count = graph.blocks.len();
    let mut children: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); count];
    let mut parents: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); count];

    let block_of = |node: NodeIdx| graph.block_of.get(node).copied();

    for union in &graph.unions {
        let above: BTreeSet<usize> =
            union.parents.iter().filter_map(|&p| block_of(p)).collect();
        let below: BTreeSet<usize> =
            union.children.iter().filter_map(|&c| block_of(c)).collect();

        for &upper in &above {
            for &lower in &below {
                if let Some(slot) = children.get_mut(upper) {
                    slot.insert(lower);
                }
                if let Some(slot) = parents.get_mut(lower) {
                    slot.insert(upper);
                }
            }
        }
    }

    BlockLinks {
        children: children.into_iter().map(Vec::from_iter).collect(),
        parents: parents.into_iter().map(Vec::from_iter).collect(),
    }
}

fn order_blocks(
    graph: &FamilyGraph,
    block_gen: &[i32],
    generations: usize,
    links: &BlockLinks,
) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); generations];
    let mut placed = vec![false; graph.blocks.len()];

    let focus_block = graph.block_of.get(graph.focus).copied().unwrap_or(0);
    let mut queue = VecDeque::from([focus_block]);

    let push = |block: usize, rows: &mut Vec<Vec<usize>>, placed: &mut Vec<bool>| {
        if placed.get(block).copied().unwrap_or(true) {
            return false;
        }
        if let Some(slot) = placed.get_mut(block) {
            *slot = true;
        }
        let row =
            usize::try_from(block_gen.get(block).copied().unwrap_or(0)).unwrap_or(0);
        if let Some(slot) = rows.get_mut(row) {
            slot.push(block);
        }
        true
    };

    push(focus_block, &mut rows, &mut placed);

    while let Some(block) = queue.pop_front() {
        let below = links.children.get(block).cloned().unwrap_or_default();
        let above = links.parents.get(block).cloned().unwrap_or_default();

        for next in below.into_iter().chain(above) {
            if push(next, &mut rows, &mut placed) {
                queue.push_back(next);
            }
        }
    }

    // Anything the walk could not reach still needs a home.
    for block in 0..graph.blocks.len() {
        push(block, &mut rows, &mut placed);
    }

    for sweep in 0..ORDER_SWEEPS {
        let downward = sweep % 2 == 0;
        median_sweep(&mut rows, links, downward);
    }

    rows
}

fn median_sweep(rows: &mut [Vec<usize>], links: &BlockLinks, downward: bool) {
    let mut position: BTreeMap<usize, f32> = BTreeMap::new();
    for row in rows.iter() {
        for (index, &block) in row.iter().enumerate() {
            position.insert(block, count_f32(index));
        }
    }

    let indices: Vec<usize> = if downward {
        (0..rows.len()).collect()
    } else {
        (0..rows.len()).rev().collect()
    };

    for row_index in indices {
        let Some(row) = rows.get_mut(row_index) else {
            continue;
        };

        let mut keyed: Vec<(f32, usize, usize)> = row
            .iter()
            .enumerate()
            .map(|(order, &block)| {
                let neighbours = if downward {
                    links.parents.get(block)
                } else {
                    links.children.get(block)
                };

                let median = neighbours
                    .and_then(|list| {
                        let mut values: Vec<f32> = list
                            .iter()
                            .filter_map(|n| position.get(n).copied())
                            .collect();
                        values.sort_by(f32::total_cmp);
                        values.get(values.len() / 2).copied()
                    })
                    .unwrap_or_else(|| count_f32(order));

                (median, order, block)
            })
            .collect();

        // Sorting on (median, original order) keeps the sweep stable
        keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        *row = keyed.into_iter().map(|(_, _, block)| block).collect();

        for (index, &block) in row.iter().enumerate() {
            position.insert(block, count_f32(index));
        }
    }
}

fn separate(order: &[usize], widths: &[f32], desired: &mut [f32]) {
    let mut previous: Option<(f32, f32)> = None;

    for (slot, &block) in order.iter().enumerate() {
        let width = widths.get(block).copied().unwrap_or(NODE_W);
        let Some(centre) = desired.get_mut(slot) else {
            continue;
        };

        if let Some((prev_centre, prev_width)) = previous {
            let minimum = (prev_width + width).mul_add(0.5, prev_centre + NODE_GAP);
            if *centre < minimum {
                *centre = minimum;
            }
        }

        previous = Some((*centre, width));
    }
}

fn mean(values: &[f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }

    let total: f32 = values.iter().sum();
    Some(total / count_f32(values.len()))
}

#[must_use]
pub fn layout(graph: &FamilyGraph) -> Layout {
    if graph.is_empty() {
        return Layout::default();
    }

    let (generation, back_edges) = assign_generations(graph);
    let generations = generation
        .iter()
        .copied()
        .max()
        .and_then(|top| usize::try_from(top + 1).ok())
        .unwrap_or(1);

    let block_gen: Vec<i32> = graph
        .blocks
        .iter()
        .map(|block| {
            block
                .members
                .first()
                .and_then(|&member| generation.get(member).copied())
                .unwrap_or(0)
        })
        .collect();

    let widths: Vec<f32> =
        graph.blocks.iter().map(|block| block_width(block.members.len())).collect();

    let links = block_links(graph);
    let rows = order_blocks(graph, &block_gen, generations, &links);

    // Initial packing: left to right in row order.
    let mut centre: Vec<f32> = vec![0.0; graph.blocks.len()];
    for row in &rows {
        let mut cursor = 0.0f32;
        for &block in row {
            let width = widths.get(block).copied().unwrap_or(NODE_W);
            if let Some(slot) = centre.get_mut(block) {
                *slot = cursor + width / 2.0;
            }
            cursor += width + NODE_GAP;
        }
    }

    for pass in 0..REFINE_PASSES {
        let downward = pass % 2 == 0;

        let order: Vec<usize> = if downward {
            (0..rows.len()).collect()
        } else {
            (0..rows.len()).rev().collect()
        };

        for row_index in order {
            let Some(row) = rows.get(row_index) else {
                continue;
            };

            let mut desired: Vec<f32> = row
                .iter()
                .map(|&block| {
                    let anchors = if downward {
                        links.parents.get(block)
                    } else {
                        links.children.get(block)
                    };

                    anchors
                        .and_then(|list| {
                            let values: Vec<f32> = list
                                .iter()
                                .filter_map(|&n| centre.get(n).copied())
                                .collect();
                            mean(&values)
                        })
                        .or_else(|| centre.get(block).copied())
                        .unwrap_or(0.0)
                })
                .collect();

            let before = mean(&desired);
            separate(row, &widths, &mut desired);
            let after = mean(&desired);

            // Rigid recentre, so anchoring intent survives separation.
            let shift = match (before, after) {
                (Some(before), Some(after)) => before - after,
                _ => 0.0,
            };

            for (slot, &block) in row.iter().enumerate() {
                if let Some(value) = desired.get(slot).copied()
                    && let Some(target) = centre.get_mut(block)
                {
                    *target = value + shift;
                }
            }
        }
    }

    // Translate so the drawing starts at the margin.
    let leftmost = rows
        .iter()
        .flatten()
        .filter_map(|&block| {
            let width = widths.get(block).copied()?;
            Some(centre.get(block).copied()? - width / 2.0)
        })
        .min_by(f32::total_cmp)
        .unwrap_or(0.0);

    let mut x = vec![0.0f32; graph.len()];
    let mut y = vec![0.0f32; graph.len()];

    for block in &graph.blocks {
        let index = block
            .members
            .first()
            .and_then(|&m| graph.block_of.get(m).copied())
            .unwrap_or(0);

        let width = widths.get(index).copied().unwrap_or(NODE_W);
        let start =
            centre.get(index).copied().unwrap_or(0.0) - width / 2.0 - leftmost
                + MARGIN;

        for (offset, &member) in block.members.iter().enumerate() {
            let step = count_f32(offset) * (NODE_W + PARTNER_GAP);
            if let Some(slot) = x.get_mut(member) {
                *slot = start + step;
            }

            let row = generation.get(member).copied().unwrap_or(0);
            if let Some(slot) = y.get_mut(member) {
                *slot = gen_f32(row).mul_add(ROW_PITCH, MARGIN);
            }
        }
    }

    let width =
        x.iter().map(|left| left + NODE_W).max_by(f32::total_cmp).unwrap_or(NODE_W)
            + MARGIN;

    let height = gen_f32(i32::try_from(generations.saturating_sub(1)).unwrap_or(0))
        .mul_add(ROW_PITCH, MARGIN.mul_add(2.0, NODE_H));

    Layout { generation, x, y, back_edges, width, height, generations }
}
