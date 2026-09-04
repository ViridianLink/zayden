//! Discord has no table syntax. A narrow table becomes an aligned monospace
//! block; a wide one becomes a list, because a wrapped grid is unreadable.

mod common;
use common::render;

const NARROW: &str = "\
| Port | Use |
| --- | --- |
| 8096 | HTTP |
| 8920 | HTTPS |";

#[test]
fn a_narrow_table_becomes_an_aligned_block() {
    let out = render(NARROW);

    assert!(out.contains("```"), "{out}");
    assert!(out.contains("Port | Use"), "{out}");
    assert!(out.contains("8096 | HTTP"), "{out}");
    assert!(!out.contains("| ---"), "{out}");
}

/// Padding is what makes the columns line up in a proportional-font client.
#[test]
fn cells_are_padded_to_a_common_width() {
    let out = render(NARROW);

    let widths = out
        .lines()
        .filter(|line| line.contains(" | "))
        .map(|line| line.split(" | ").next().unwrap_or_default().len())
        .collect::<Vec<_>>();

    assert!(widths.len() >= 3, "{out}");
    assert!(widths.windows(2).all(|w| w[0] == w[1]), "{widths:?} in {out}");
}

#[test]
fn a_wide_table_becomes_a_list() {
    let content = "\
| Service | Description | Default port | Config path |
| --- | --- | --- | --- |
| Jellyfin | Media server for films and shows | 8096 | /etc/jellyfin/config.xml |";

    let out = render(content);

    assert!(!out.contains("```"), "{out}");
    assert!(out.contains("**Jellyfin**"), "{out}");
    assert!(out.contains("- Default port: 8096"), "{out}");
}

/// Pipes in a code sample delimit nothing.
#[test]
fn a_table_inside_a_fence_is_left_alone() {
    let content = format!("```\n{NARROW}\n```");

    assert!(render(&content).contains("| --- | --- |"), "{content}");
}

#[test]
fn an_escaped_pipe_stays_inside_its_cell() {
    let content = "\
| Command | Use |
| --- | --- |
| a \\| b | pipe |";

    let out = render(content);

    assert!(out.contains("a | b"), "{out}");
}

/// Pipe-delimited prose that is not a table must not be reflowed as one.
#[test]
fn text_without_a_delimiter_row_is_not_a_table() {
    let content = "| just | some | pipes |\n\nand a paragraph";

    let out = render(content);

    assert!(!out.contains("```"), "{out}");
}
