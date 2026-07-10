//! HTML helper tests: `scraper`-backed extraction used by the map/Next.js
//! sources. No network.

use marathon::parse::html;
use serde_json::Value;

const NEXT_PAGE: &str = r#"
<html><head><title>x</title></head><body>
<div id="app">loading…</div>
<script id="__NEXT_DATA__" type="application/json">
{"props":{"pageProps":{"weapon":{"slug":"m77","damage":30}}},"buildId":"abc"}
</script>
</body></html>
"#;

const GLOBAL_PAGE: &str = r#"
<html><body>
<script>
window.mapData = {"map":{"id":42,"slug":"perimeter"},"nested":{"a":"}"}};
console.log("after");
</script>
</body></html>
"#;

#[test]
fn extracts_next_data_json() {
    let doc = html::document(NEXT_PAGE);
    let value = html::script_json_by_selector(&doc, "script#__NEXT_DATA__")
        .expect("should parse __NEXT_DATA__");
    assert_eq!(value.pointer("/buildId").and_then(Value::as_str), Some("abc"));
    assert_eq!(
        value.pointer("/props/pageProps/weapon/slug").and_then(Value::as_str),
        Some("m77")
    );
}

#[test]
fn extracts_global_assignment_object_with_brace_in_string() {
    let doc = html::document(GLOBAL_PAGE);
    let value = html::script_json_after_marker(&doc, "window.mapData = ")
        .expect("should parse window.mapData object");
    assert_eq!(value.pointer("/map/id").and_then(Value::as_i64), Some(42));
    // The `}` inside a string must not terminate the object early.
    assert_eq!(value.pointer("/nested/a").and_then(Value::as_str), Some("}"));
}

#[test]
fn text_and_attr_helpers() {
    let doc = html::document(
        r#"<html><body><h1 class="t">  Hello   World </h1>
        <img id="p" src="/x.png"></body></html>"#,
    );
    assert_eq!(html::text_of(&doc, "h1.t").unwrap().as_deref(), Some("Hello World"));
    assert_eq!(html::attr_of(&doc, "#p", "src").unwrap().as_deref(), Some("/x.png"));
    assert!(html::text_of(&doc, ".missing").unwrap().is_none());
}
