//! Code blocks arrive with the wiki's own indentation and language tags Discord
//! does not highlight.

mod common;
use common::render;

#[test]
fn a_known_alias_is_mapped_to_a_tag_discord_highlights() {
    assert!(render("```sh\necho hi\n```").starts_with("```bash"));
    assert!(render("```yml\na: 1\n```").starts_with("```yaml"));
}

#[test]
fn a_supported_tag_is_kept() {
    assert!(render("```rust\nfn main() {}\n```").starts_with("```rust"));
}

/// Discord renders an unrecognised tag as the block's first line, so dropping
/// it is better than passing it through.
#[test]
fn an_unknown_tag_is_dropped() {
    let out = render("```wikitext\nfoo\n```");

    assert!(out.starts_with("```\n"), "{out}");
    assert!(!out.contains("wikitext"), "{out}");
}

/// A block nested under a list item arrives indented, and that indent is
/// rendered as part of the code.
#[test]
fn a_nested_block_is_dedented() {
    let content = "  ```bash\n    echo one\n    echo two\n  ```";

    let out = render(content);

    assert!(out.contains("\necho one\n"), "{out}");
    assert!(!out.contains("    echo one"), "{out}");
}

#[test]
fn relative_indentation_inside_a_block_is_preserved() {
    let content = "```python\n    def f():\n        return 1\n```";

    let out = render(content);

    assert!(out.contains("def f():"), "{out}");
    assert!(out.contains("    return 1"), "{out}");
}

/// Discord renders only fenced blocks, so an indented one has to become fenced.
#[test]
fn an_indented_block_becomes_fenced() {
    let content = "Run this:\n\n    docker compose up -d\n\nThen check it.";

    let out = render(content);

    assert!(out.contains("```\ndocker compose up -d\n```"), "{out}");
}

/// An indented list item is a list item, not a code block.
#[test]
fn an_indented_list_is_not_fenced() {
    let content = "- one\n    - nested\n    - also nested\n";

    let out = render(content);

    assert!(!out.contains("```"), "{out}");
    assert!(out.contains("- nested"), "{out}");
}

/// The block is spliced back into the document, so the newline that separated
/// it from the next paragraph has to survive the rewrite.
#[test]
fn text_after_a_block_stays_on_its_own_line() {
    let out = render("```bash\necho hi\n```\n\nThen restart it.");

    assert!(out.contains("```\n\nThen restart it."), "{out:?}");
}
