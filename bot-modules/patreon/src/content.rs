use std::fmt::Write as _;

use ego_tree::NodeRef;
use scraper::node::Element;
use scraper::{Html, Node, Selector};

pub const BODY_LIMIT: usize = 3000;
pub const ELLIPSIS: &str = "\n\n-# _(truncated - read the full post on Patreon)_";

#[must_use]
pub fn to_discord(content_html: &str) -> String {
    let markdown = zayden_core::html::strip(&to_markdown(content_html));

    zayden_core::text::truncate(
        &collapse_blank_lines(&markdown),
        BODY_LIMIT,
        ELLIPSIS,
    )
}

#[must_use]
pub fn to_markdown(content_html: &str) -> String {
    let fragment = Html::parse_fragment(content_html);

    let Ok(selector) = Selector::parse("html") else {
        return content_html.to_owned();
    };
    let Some(root) = fragment.select(&selector).next() else {
        return content_html.to_owned();
    };

    let mut out = String::with_capacity(content_html.len());
    render_children(*root, &mut out);
    out
}

fn render_children(node: NodeRef<'_, Node>, out: &mut String) {
    for child in node.children() {
        render(child, out);
    }
}

fn render(node: NodeRef<'_, Node>, out: &mut String) {
    match node.value() {
        Node::Text(text) => out.push_str(text),
        Node::Element(element) => render_element(node, element, out),
        Node::Document
        | Node::Fragment
        | Node::Doctype(_)
        | Node::Comment(_)
        | Node::ProcessingInstruction(_) => {},
    }
}

fn render_element(node: NodeRef<'_, Node>, element: &Element, out: &mut String) {
    match element.name() {
        // Script-like elements carry no prose, and Patreon's lead image is
        // already the og:image the thumbnail lookup finds.
        "script" | "style" | "svg" | "iframe" | "img" => {},
        "br" => out.push('\n'),
        "hr" => out.push_str("\n---\n"),
        "b" | "strong" => wrap(node, out, "**", "**"),
        "i" | "em" => wrap(node, out, "_", "_"),
        "u" => wrap(node, out, "__", "__"),
        "s" | "del" | "strike" => wrap(node, out, "~~", "~~"),
        "code" => wrap(node, out, "`", "`"),
        "pre" => wrap(node, out, "\n```\n", "\n```\n"),
        "blockquote" => wrap(node, out, "\n> ", "\n"),
        "h1" | "h2" => wrap(node, out, "\n## ", "\n"),
        "h3" | "h4" | "h5" | "h6" => wrap(node, out, "\n### ", "\n"),
        "li" => wrap(node, out, "\n- ", ""),
        "p" | "div" | "ul" | "ol" | "tr" => wrap(node, out, "", "\n"),
        "a" => render_link(node, element, out),
        _ => render_children(node, out),
    }
}

fn wrap(node: NodeRef<'_, Node>, out: &mut String, open: &str, close: &str) {
    let mut inner = String::new();
    render_children(node, &mut inner);

    if inner.trim().is_empty() {
        return;
    }

    out.push_str(open);
    out.push_str(&inner);
    out.push_str(close);
}

fn render_link(node: NodeRef<'_, Node>, element: &Element, out: &mut String) {
    let mut label = String::new();
    render_children(node, &mut label);
    let label = label.trim();

    match element.attr("href") {
        // A label that is already the URL would render as `[url](url)`.
        Some(href) if label.is_empty() || label == href => out.push_str(href),
        Some(href) => {
            let _ = write!(out, "[{label}]({href})");
        },
        None => out.push_str(label),
    }
}

fn collapse_blank_lines(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut newlines = 0_usize;

    for c in content.trim().chars() {
        if c == '\n' {
            newlines += 1;
            if newlines <= 2 {
                out.push(c);
            }
            continue;
        }

        newlines = 0;
        out.push(c);
    }

    out
}
