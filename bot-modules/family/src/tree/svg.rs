use std::collections::BTreeSet;
use std::fmt::Write as _;

use zayden_graphics::Canvas;

use crate::tree::layout::Layout;
use crate::tree::model::{FamilyGraph, NodeIdx};
use crate::tree::{
    AVATAR_BOX,
    FONT_SIZE,
    MAX_NAME_CHARS,
    NODE_H,
    NODE_PAD,
    NODE_W,
    TreeQuota,
};

const COLOUR_BG: &str = "#2b2d31";
const COLOUR_NODE: &str = "#404249";
const COLOUR_FOCUS: &str = "#5865f2";
const COLOUR_TEXT: &str = "#f2f3f5";
const COLOUR_EDGE: &str = "#6d6f78";
const COLOUR_CHIP: &str = "#1e1f22";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvatarSlot {
    pub node: NodeIdx,
    pub id: i64,
    pub x: i32,
    pub y: i32,
    pub size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeSvg {
    pub markup: String,
    pub canvas: Canvas,
    pub avatars: Vec<AvatarSlot>,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to 1.0..=u32::MAX as f32 first, so the cast is in range"
)]
const fn device_px(value: f32) -> u32 {
    value.clamp(1.0, 16_777_216.0).round() as u32
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "clamped to i32-safe bounds first, so the cast is in range"
)]
const fn device_offset(value: f32) -> i32 {
    value.clamp(-16_777_216.0, 16_777_216.0).round() as i32
}

#[must_use]
pub fn sanitise(name: &str, id: i64) -> String {
    escape_xml(&clean(name, id))
}

fn clean(name: &str, id: i64) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !is_control_or_bidi(*c) && !is_pictographic(*c))
        .collect();

    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

    let usable = if collapsed.is_empty() {
        format!("user-{:04}", id.rem_euclid(10_000))
    } else {
        collapsed
    };

    if usable.chars().count() > MAX_NAME_CHARS {
        let kept: String =
            usable.chars().take(MAX_NAME_CHARS.saturating_sub(1)).collect();
        format!("{kept}\u{2026}")
    } else {
        usable
    }
}

fn label(name: &str, id: i64, room: f32) -> String {
    escape_xml(&fit(&clean(name, id), room))
}

fn is_control_or_bidi(c: char) -> bool {
    let code = u32::from(c);

    c.is_control()
        || (0x007F..=0x009F).contains(&code)
        // LRM/RLM, the LRE..RLO overrides, PDF, and the isolate controls.
        || matches!(code, 0x200B..=0x200F | 0x202A..=0x202E | 0x2066..=0x2069)
}

fn is_pictographic(c: char) -> bool {
    let code = u32::from(c);

    matches!(
        code,
        0x2600..=0x27BF        // Misc Symbols, Dingbats
            | 0x2B00..=0x2BFF  // Misc Symbols and Arrows
            | 0x20E3           // Combining enclosing keycap
            | 0xFE00..=0xFE0F  // Variation selectors
            | 0x1F000..=0x1FAFF // Emoji planes, incl. regional indicators
    )
}

fn escape_xml(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }

    out
}

fn advance_ems(text: &str) -> f32 {
    text.chars()
        .map(|c| match c {
            'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '.' | ',' | ':' | ';'
            | '\'' | '|' | '!' | '[' | ']' | '(' | ')' => 0.30,
            'm' | 'w' | 'M' | 'W' | '@' => 0.88,
            ' ' => 0.28,
            c if c.is_ascii_uppercase() || c.is_ascii_digit() => 0.60,
            c if c.is_ascii() => 0.52,
            // Non-Latin scripts are typically much wider per glyph.
            _ => 0.95,
        })
        .sum()
}

fn fit(text: &str, max_width: f32) -> String {
    if advance_ems(text) * FONT_SIZE <= max_width {
        return text.to_string();
    }

    let mut kept = String::new();
    let budget = advance_ems("\u{2026}").mul_add(-FONT_SIZE, max_width);

    for c in text.chars() {
        let mut candidate = kept.clone();
        candidate.push(c);

        if advance_ems(&candidate) * FONT_SIZE > budget {
            break;
        }
        kept = candidate;
    }

    format!("{}\u{2026}", kept.trim_end())
}

#[must_use]
pub fn canvas_for(layout: &Layout, quota: TreeQuota) -> (Canvas, f32) {
    let width = layout.width.max(1.0);
    let height = layout.height.max(1.0);

    let by_area = (f32::from(
        u16::try_from(quota.max_canvas_pixels / 1_000).unwrap_or(u16::MAX),
    ) * 1_000.0
        / (width * height))
        .sqrt();

    let edge = f32::from(u16::try_from(quota.max_canvas_dim).unwrap_or(u16::MAX));
    let scale = 1.0f32.min(by_area).min(edge / width).min(edge / height);

    let canvas = Canvas {
        width: device_px(width * scale),
        height: device_px(height * scale),
    };

    (canvas, scale)
}

#[must_use]
pub fn render(
    graph: &FamilyGraph,
    layout: &Layout,
    quota: TreeQuota,
    avatar_for: &BTreeSet<NodeIdx>,
) -> TreeSvg {
    let (canvas, scale) = canvas_for(layout, quota);

    let mut markup = String::with_capacity(graph.len() * 512);
    let _ = write!(
        markup,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {:.2} {:.2}">"#,
        canvas.width, canvas.height, layout.width, layout.height,
    );
    let _ = write!(
        markup,
        r#"<rect width="{:.2}" height="{:.2}" fill="{COLOUR_BG}"/>"#,
        layout.width, layout.height,
    );

    let back: BTreeSet<(NodeIdx, NodeIdx)> =
        layout.back_edges.iter().copied().collect();

    draw_partner_bars(&mut markup, graph, layout);
    draw_unions(&mut markup, graph, layout, &back);
    draw_back_edges(&mut markup, layout, &back);

    let avatars = draw_nodes(&mut markup, graph, layout, avatar_for, scale);

    markup.push_str("</svg>");

    TreeSvg { markup, canvas, avatars }
}

fn draw_partner_bars(out: &mut String, graph: &FamilyGraph, layout: &Layout) {
    for block in &graph.blocks {
        if block.members.len() < 2 {
            continue;
        }

        let centres: Vec<f32> = block
            .members
            .iter()
            .filter_map(|&member| layout.centre_x(member))
            .collect();

        let (Some(left), Some(right)) = (
            centres.iter().copied().min_by(f32::total_cmp),
            centres.iter().copied().max_by(f32::total_cmp),
        ) else {
            continue;
        };

        let Some(mid) = block
            .members
            .first()
            .and_then(|&m| layout.top(m))
            .map(|top| top + NODE_H / 2.0)
        else {
            continue;
        };

        let _ = write!(
            out,
            r#"<line x1="{left:.2}" y1="{mid:.2}" x2="{right:.2}" y2="{mid:.2}" stroke="{COLOUR_FOCUS}" stroke-width="4"/>"#,
        );
    }
}

fn draw_unions(
    out: &mut String,
    graph: &FamilyGraph,
    layout: &Layout,
    back: &BTreeSet<(NodeIdx, NodeIdx)>,
) {
    for union in &graph.unions {
        let drawn: Vec<NodeIdx> = union
            .children
            .iter()
            .copied()
            .filter(|&child| {
                union.parents.iter().any(|&p| !back.contains(&(p, child)))
            })
            .collect();

        if drawn.is_empty() {
            continue;
        }

        let anchors: Vec<f32> =
            union.parents.iter().filter_map(|&p| layout.centre_x(p)).collect();
        let Some(anchor) = mean(&anchors) else {
            continue;
        };

        let Some(parent_bottom) = union
            .parents
            .iter()
            .filter_map(|&p| layout.bottom(p))
            .max_by(f32::total_cmp)
        else {
            continue;
        };

        let child_tops: Vec<f32> =
            drawn.iter().filter_map(|&c| layout.top(c)).collect();
        let Some(child_top) = child_tops.iter().copied().min_by(f32::total_cmp)
        else {
            continue;
        };

        let bus = parent_bottom.midpoint(child_top);

        let _ = write!(
            out,
            r#"<path d="M{anchor:.2} {parent_bottom:.2}L{anchor:.2} {bus:.2}" stroke="{COLOUR_EDGE}" stroke-width="2" fill="none"/>"#,
        );

        let centres: Vec<f32> =
            drawn.iter().filter_map(|&c| layout.centre_x(c)).collect();

        if let (Some(left), Some(right)) = (
            centres.iter().copied().min_by(f32::total_cmp),
            centres.iter().copied().max_by(f32::total_cmp),
        ) {
            let from = left.min(anchor);
            let to = right.max(anchor);
            let _ = write!(
                out,
                r#"<path d="M{from:.2} {bus:.2}L{to:.2} {bus:.2}" stroke="{COLOUR_EDGE}" stroke-width="2" fill="none"/>"#,
            );
        }

        for &child in &drawn {
            let (Some(cx), Some(top)) = (layout.centre_x(child), layout.top(child))
            else {
                continue;
            };

            let _ = write!(
                out,
                r#"<path d="M{cx:.2} {bus:.2}L{cx:.2} {top:.2}" stroke="{COLOUR_EDGE}" stroke-width="2" fill="none"/>"#,
            );
        }
    }
}

fn draw_back_edges(
    out: &mut String,
    layout: &Layout,
    back: &BTreeSet<(NodeIdx, NodeIdx)>,
) {
    for &(parent, child) in back {
        let (Some(px), Some(py), Some(cx), Some(cy)) = (
            layout.centre_x(parent),
            layout.top(parent).map(|t| t + NODE_H / 2.0),
            layout.centre_x(child),
            layout.top(child).map(|t| t + NODE_H / 2.0),
        ) else {
            continue;
        };

        let bend = px.midpoint(cx);
        let _ = write!(
            out,
            r#"<path d="M{px:.2} {py:.2}Q{bend:.2} {:.2} {cx:.2} {cy:.2}" stroke="{COLOUR_EDGE}" stroke-width="2" stroke-dasharray="6 4" fill="none"/>"#,
            py.midpoint(cy) - 40.0,
        );
    }
}

fn draw_nodes(
    out: &mut String,
    graph: &FamilyGraph,
    layout: &Layout,
    avatar_for: &BTreeSet<NodeIdx>,
    scale: f32,
) -> Vec<AvatarSlot> {
    let mut slots = Vec::new();

    for (node, person) in graph.people.iter().enumerate() {
        let (Some(&left), Some(&top)) = (layout.x.get(node), layout.y.get(node))
        else {
            continue;
        };

        let focused = node == graph.focus;
        let fill = if focused { COLOUR_FOCUS } else { COLOUR_NODE };

        let _ = write!(
            out,
            r#"<rect x="{left:.2}" y="{top:.2}" width="{NODE_W:.2}" height="{NODE_H:.2}" rx="8" fill="{fill}" stroke="{COLOUR_EDGE}" stroke-width="1"/>"#,
        );

        // Clip every label to its own box
        let _ = write!(
            out,
            r#"<clipPath id="c{node}"><rect x="{left:.2}" y="{top:.2}" width="{NODE_W:.2}" height="{NODE_H:.2}" rx="8"/></clipPath>"#,
        );

        let has_avatar = avatar_for.contains(&node);

        let (text_x, anchor, room) = if has_avatar {
            let cx = left + NODE_PAD + AVATAR_BOX / 2.0;
            let cy = top + NODE_H / 2.0;

            let _ = write!(
                out,
                r#"<circle cx="{cx:.2}" cy="{cy:.2}" r="{:.2}" fill="{COLOUR_CHIP}"/>"#,
                AVATAR_BOX / 2.0,
            );

            slots.push(AvatarSlot {
                node,
                id: person.id,
                x: device_offset((left + NODE_PAD) * scale),
                y: device_offset((cy - AVATAR_BOX / 2.0) * scale),
                size: device_px(AVATAR_BOX * scale),
            });

            let start = NODE_PAD.mul_add(2.0, left) + AVATAR_BOX;
            (start, "start", NODE_W - NODE_PAD.mul_add(3.0, AVATAR_BOX))
        } else {
            (left + NODE_W / 2.0, "middle", NODE_PAD.mul_add(-2.0, NODE_W))
        };

        let label = label(&person.name, person.id, room);
        let baseline = FONT_SIZE.mul_add(0.35, top + NODE_H / 2.0);

        let _ = write!(
            out,
            r#"<text x="{text_x:.2}" y="{baseline:.2}" font-family="sans-serif" font-size="{FONT_SIZE:.2}" fill="{COLOUR_TEXT}" text-anchor="{anchor}" clip-path="url(#c{node})">{label}</text>"#,
        );

        if person.hidden > 0 {
            let chip_x = left + NODE_W - 6.0;
            let chip_y = top + NODE_H - 6.0;

            let _ = write!(
                out,
                r#"<circle cx="{chip_x:.2}" cy="{chip_y:.2}" r="11" fill="{COLOUR_CHIP}" stroke="{COLOUR_EDGE}" stroke-width="1"/>"#,
            );
            let _ = write!(
                out,
                r#"<text x="{chip_x:.2}" y="{:.2}" font-family="sans-serif" font-size="11" fill="{COLOUR_TEXT}" text-anchor="middle">+{}</text>"#,
                chip_y + 4.0,
                person.hidden,
            );
        }
    }

    slots
}

fn mean(values: &[f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }

    let total: f32 = values.iter().sum();
    Some(total / f32::from(u16::try_from(values.len()).unwrap_or(u16::MAX)))
}
