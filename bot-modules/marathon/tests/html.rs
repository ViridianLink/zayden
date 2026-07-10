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

const FLIGHT_PAGE: &str = r#"
<html><body>
<script>self.__next_f=self.__next_f||[]</script>
<script>self.__next_f.push([1,"22:{\"id\":\"a1\",\"slug\":\"m77\",\"name\":\"M77\",\"stats\":\"$25\"}\n"])</script>
<script>self.__next_f.push([1,"25:{\"damage\":30}\n26:[\"$\",\"div\",null,{\"item\":{\"id\":\"a1\",\"slug\":\"m77\",\"name\":\"M77\",\"stats\":{\"damage\":30},\"note\":\"has a } brace\"}}]\n"])</script>
</body></html>
"#;

#[test]
fn next_flight_concatenates_and_resolves_object() {
    let doc = html::document(FLIGHT_PAGE);
    let flight = html::next_flight(&doc);
    assert!(flight.contains("\"slug\":\"m77\""));

    let obj = html::flight_object_by_slug(&flight, "m77")
        .expect("should find resolved m77 object");
    // The fully-inlined copy (stats as an object) must win over the `$25` stub.
    assert_eq!(obj.pointer("/stats/damage").and_then(Value::as_i64), Some(30));
    assert_eq!(obj.pointer("/note").and_then(Value::as_str), Some("has a } brace"));
}

#[test]
fn flight_object_missing_slug_is_none() {
    let doc = html::document(FLIGHT_PAGE);
    let flight = html::next_flight(&doc);
    assert!(html::flight_object_by_slug(&flight, "nope").is_none());
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
