Verified against: topcoat 0.10.0 (Cargo.lock), 2026-10-06

# Topcoat core reference

Framework core as the `web` crate uses it and as the UI redesign will need it. Every entry carries a status (see [README.md](README.md)), the cargo features it needs, a compiling example, and the source it was read from. Source paths are relative to the crate directory in `~/.cargo/registry/src/*/` (`topcoat-<crate>-0.10.0/`). Probe paths are local scratch crates and are not tracked.

## Contents

1. Crate features
2. `view!` syntax
3. Components
4. Client expressions `$(...)`
5. Router
6. Request context `Cx`
7. Forms and body parsing
8. Cookies
9. Sessions
10. Origin policy
11. Assets
12. Runtime script and client navigation
13. Head, title and meta
14. Streaming and async
15. Error handling
16. Test harness

## 1. Crate features

Status: VERIFIED. Features: all. Probe: `scratch/topcoat-kb/core/features/<feature>/` (one crate per feature, `cargo tree -e normal` in `tree.txt`).

The workspace pin is `topcoat = { version = "0.10", default-features = false }` (`Cargo.toml:334`); `web` re-enables exactly the runtime set below and adds two build-time features (`web/Cargo.toml` `[dependencies.topcoat]` and `[build-dependencies.topcoat]`).

```toml
[dependencies]
topcoat = { workspace = true, features = ["asset", "compression", "cookie", "router", "runtime", "serve", "view"] }

[build-dependencies]
topcoat = { workspace = true, features = ["tailwind", "ui"] }
```

`default` is `asset, compression, cookie, font, icon, router, runtime, serve, session, view, discover` (`topcoat-0.10.0/Cargo.toml:72`). The runtime set above is that list minus `font`, `icon`, `session`, `discover`. Crate counts are distinct crate names in `cargo tree -e normal` for a crate enabling only that feature: 31 with no feature, 144 for the `web` set, 153 for `default`.

| Feature | Implies | Adds (sub-crates) | Notable external crates it pulls | In `web` |
|---|---|---|---|---|
| `view` | none | `topcoat-view`, `-view-macro`, `-view-grammar` | `futures-util`, `heck`, `memchr`, `slab`, `smallvec` | runtime |
| `router` | `view` | `topcoat-router`, `-router-macro`, `-router-grammar` | `matchit`, `http-body-util`, `serde_json`, `serde_urlencoded`, `serde_path_to_error`, `ipnet` | runtime |
| `serve` | `router` | none (adds the `topcoat::serve`, `serve_until`, `start` functions) | `hyper`, `hyper-util`, `h2`, `tokio-tungstenite`, `tungstenite`, `tokio-util`, `rand`, `sha1`, `tracing` | runtime |
| `runtime` | none | `topcoat-runtime`, `-runtime-macro`, `-runtime-grammar` (the grammar crate depends on `topcoat-router` and `topcoat-asset`) | `sha2`, `toml`, plus everything `router` pulls | runtime |
| `asset` | none | `topcoat-asset` | `sha2`, `toml`, `memchr` | runtime |
| `cookie` | none | `topcoat-cookie` | `cookie`, `aes-gcm`, `hkdf`, `hmac`, `rand`, `time`, `serde_json` | runtime |
| `compression` | `router` | none | `tower-http`, `async-compression`, `brotli`, `flate2`, `tower` | runtime |
| `discover` | none | none | `inventory` (link-time registration; needed by `module_router!`, `.discover()`) | no |
| `session` | none | `topcoat-session` | `sha2`, `web-time` | no |
| `tower` | `router` | none | `tower`, `tower-layer`, `tower-service`, `sync_wrapper` | no |
| `multipart` | `router` | none | `multer`, `mime`, `encoding_rs`, `httparse` | no |
| `fs` | `router` | none | `mime_guess`, `tokio-util`, `httpdate` | no |
| `sse` | `router` | none | none beyond `router` | no |
| `websocket` | `serve` | none | none beyond `serve` | no |
| `sitemap` | `router` | none | `time` | no |
| `anyhow` | none | none | `anyhow` | no |
| `icon` | `view` | `topcoat-icon`, `-icon-macro`, `-icon-grammar` | `serde_json` | no |
| `font` | none | `topcoat-font`, `-font-macro`, `-font-grammar` | none | no |
| `tailwind` | none | `topcoat-tailwind` | `ureq`, `rustls`, `ring`, `webpki-roots`, `flate2` (downloads the Tailwind binary) | build-dependency |
| `ui` | none | `topcoat-ui-registry` | none (the component and theme registry that `topcoat ui add` copies from) | build-dependency |

Also present and not used here: `alpine-ajax`, `htmx`, `datastar` (implies `sse`), `mail`, `mail-smtp`, `font-fontsource`, `icon-iconify`, `full`.

Notes:

- `tailwind` and `ui` belong in `[build-dependencies]` only. Enabling `tailwind` on the runtime dependency would put `ureq`, `rustls` and `ring` in the server binary (they appear in `scratch/topcoat-kb/core/features/tailwind/tree.txt` and not in `web/tree.txt`). Tailwind build wiring is described in [tooling.md](tooling.md), the component registry in [ui.md](ui.md).
- `.discover()` and `module_router!` need `discover`. `web` does not enable it: it registers every route by name (section 5), so no `inventory` link-time collection is involved.
- `topcoat::session` exists under `session` and is not enabled (section 9).
- A feature that adds a `topcoat::<module>` (`asset`, `cookie`, `router`, `runtime`, `session`, `view`, `icon`, `font`) is only importable when enabled. `topcoat::context`, `topcoat::core`, `topcoat::Result` and `topcoat::Error` are always present.

Source: `topcoat-0.10.0/Cargo.toml:52-209` (feature table), `topcoat-0.10.0/src/lib.rs:15-60` (feature-gated modules).

## 2. `view!` syntax

Features: `view` (`router` for status codes and headers in a view). The macros live in `topcoat::view`: `view!`, `attributes!`, `class!`, `live!`, `emit!`, `#[component]`, `#[derive(Props)]`. A `view!` value is lazy and async; it renders when it becomes a response (section 5) or when a test renders it (section 16). Output is minified.

Probe for this section: `scratch/topcoat-kb/core/probe/src/view_syntax.rs`, rendered by `tests/rendering.rs`. Compile-failure cases: `scratch/topcoat-kb/core/probe/exprcases/view.json`.

### 2.1 Elements, text, doctype, void and custom elements

Status: VERIFIED. Purpose: HTML in Rust syntax.

```rust
view! {
    <!DOCTYPE html>
    <html lang="en">
        <head><meta charset="utf-8"><title>"syntax"</title></head>
        <body>
            "bare text " (1 + 2) " fragment"
            <br>
            <img src="/x.png" alt="">
            <my-widget data-id="w"></my-widget>
            <svg viewBox="0 0 24 24"><path d="M5 12h14"/></svg>
        </body>
    </html>
}
```

- Text is always a quoted string literal; an unquoted word or `{expr}` is a compile error (`expected view node`). Interpolation is `(expr)`.
- Void elements (`br`, `img`, `meta`, `link`, `input`, `hr`) take no closing tag; `<br></br>` is a compile error. Every other element needs a matching close tag (`closing tag 'span' does not match opening tag 'div'`).
- Unknown tag names are fine, so custom elements (`<my-widget>`) and SVG work. Self-closing `<path .../>` is accepted.
- A `view!` body may hold several sibling roots. There is no `<> ... </>` fragment syntax and no HTML comment syntax (`<!-- -->` is rejected, only `<!DOCTYPE>` is accepted).
- Attribute names may contain `-`, `:` and `.`, and Rust keywords are valid (`type=`, `for=`). `class:active=(flag)` and `style:color="red"` are not directives: they render an attribute literally named `class:active`.

Source: `topcoat-view-macro-0.10.0/docs/view.md:1-43`, `topcoat-view-macro-0.10.0/src/lib.rs:8`. Probe: `view_syntax.rs:46-101` (rendered output in `tests/rendering.rs:16`). Web: `web/src/document.rs:90-102` (document skeleton), `web/src/components/icons.rs:150` (SVG).

### 2.2 Interpolation, escaping and raw HTML

Status: VERIFIED.

```rust
let trusted = Unescaped::new_unchecked("<b>trusted</b>");
view! {
    <p id="escaped">(user_input)</p>
    <p id="escaped-attr" title=(user_input)>"attr"</p>
    <p id="raw">(trusted)</p>
    <script>(Unescaped::new_unchecked("if (1 < 2 && true) {}"))</script>
    <script>"if (1 < 2 && true) {}"</script>
}
```

Rendered for `user_input = <b>&"x"</b>`:

```html
<p id="escaped">&lt;b&gt;&amp;"x"&lt;/b&gt;</p>
<p id="escaped-attr" title="<b>&amp;&quot;x&quot;</b>">attr</p>
<p id="raw"><b>trusted</b></p>
<script>if (1 < 2 && true) {}</script>
<script>if (1 &lt; 2 &amp;&amp; true) {}</script>
```

- Text nodes escape `& < >`. Attribute values escape `& "` only. Comment payloads escape `& > "`.
- `topcoat::view::Unescaped::new_unchecked(x)` writes `x` verbatim. Use it only for trusted markup: inline `<script>` bodies (a string literal inside `<script>` is escaped and breaks the code), SVG path data, pre-rendered HTML.
- Dynamic attribute and element names (`(name)="x"`, `<(tag)>`) are validated as identifiers, not escaped. An invalid name panics at render and the router answers `500 internal server error` (`invalid attribute key "bad name": forbidden character ' '`).

Source: `topcoat-view-0.10.0/src/html/escape.rs:197-219` (per-context escaping), `:221-250` (identifier validation), `topcoat-view-0.10.0/src/string.rs:65-79` (`Unescaped`). Probe: `view_syntax.rs:54-56,97-98,124` (`tests/rendering.rs:34`). Web: `web/src/components/icons.rs:150`, `web/src/admin/editor/view.rs:116`.

### 2.3 Attributes: boolean, optional, enumerated, spread, conditional

Status: VERIFIED.

```rust
let extra: Attributes = attributes! { data-extra="yes" if is_active { aria-pressed="true" } };
view! {
    <button
        id="bools"
        disabled=(is_disabled)
        required=""
        aria-current=(is_active.then_some("page"))
        title=(maybe_title)
        aria-expanded=(if is_active { "true" } else { "false" })
        type="button"
    >"b"</button>
    <button disabled="false">"still disabled"</button>
    <div (extra)></div>
    <div if is_active { class="active" data-on="" } else { data-off="" }></div>
    <div match status { Status::Draft => class="d", _ => class="p" }></div>
    <div for (n, v) in pairs { (n)=(v) }></div>
    <div let value = 5; data-value=(value)></div>
}
```

Rendered: `<button id="bools" required="" aria-current="page" aria-expanded="true" type="button">`, `<button disabled="false">`, `<div data-extra="yes" aria-pressed="true">`, `<div class="active" data-on="" id="cond-attrs">`.

- `bool` expression: `true` writes the bare attribute, `false` omits it. `Option<T>`: `Some` writes the value, `None` omits the attribute. A literal `disabled="false"` is still present and disabled.
- A literal boolean attribute needs an empty value: `required=""`. A bare `defer` is a compile error (`expected '='`).
- `aria-expanded`, `aria-pressed`, `contenteditable` take the strings `"true"`/`"false"`: pass strings, not `bool`.
- Attribute order is not preserved once a spread or control flow is involved (`class="active" data-on="" id="cond-attrs"` above). Do not depend on order.
- `(attrs)` consumes an `Attributes` value. Clone it to reuse it.

Source: `topcoat-view-macro-0.10.0/docs/view.md:408-470`, `topcoat-view-macro-0.10.0/docs/attributes.md:22-62,89-113`. Probe: `view_syntax.rs:57-71`. Web: `web/src/components/settings.rs:24` (`:hidden`), `web/src/components/sidebar.rs:560-561` (`aria-current=$(...)`).

### 2.4 `class!`, `StaticClass` and inline style

Status: VERIFIED.

```rust
const BTN: StaticClass = class!("btn btn-lg");
view! {
    <div class=(class!("card", variant, "active" if is_active, "on" if is_disabled else "off"))></div>
    <div class=(BTN)></div>
    <div class=(class!(maybe_title, "x" if is_disabled))></div>
    <div style=(format!("width: {width_pct}%"))></div>
}
```

Rendered: `class="card primary active off"`, `class="btn btn-lg"`, no `class` attribute when every entry is absent, `style="width: 40%"`.

- Entries are separated by commas. An entry may be a `&str`, `Option<&str>`, a collection of entries, or another `Class`; `expr if cond` and `expr if cond else alt` are conditional. Absent entries (`None`, empty string, false condition) add nothing, and the attribute is dropped when nothing remains.
- There is no `class:name=` or `style:prop=` directive. Build the style string yourself.
- Tailwind finds class names by scanning source files for string literals (`@source "../src/**/*.rs"`, `web/style/input.css:28`), so keep class names as literals inside `class!(...)` and `class="..."`; a name assembled at runtime is not generated.
- A forwarded `class` from an `Attributes` argument is combined with `attrs.remove("class")` (section 2.8).

Source: `topcoat-view-macro-0.10.0/docs/class.md:20-66,88-106`, `topcoat-view-0.10.0/src/html/class.rs:333,378`. Probe: `view_syntax.rs:8,72-75`. Web: `web/src/components/card.rs:8,38`, `web/src/components/button.rs:32`.

### 2.5 Control flow: `if`, `if let`, `match`, keyed `for`, `let`

Status: VERIFIED.

```rust
view! {
    <ul>
        #[key(row.id)]
        for row in rows {
            let label = row.name.to_uppercase();
            <li data-id=(row.id)>(label)</li>
        }
    </ul>
    match status {
        Status::Draft => <span>"Draft"</span>,
        Status::Published { title } => {
            <span id="match">(title)</span>
            <span>"sibling"</span>
        },
        Status::Archived if show_archived => <span>"Archived"</span>,
        _ => "",
    }
    if let Some(v) = variant { <p>(v)</p> } else { <p>"none"</p> }
}
```

- `if`, `else if`, `else`, `if let`, `match` (with guards), `for pat in expr`, and `let pat = expr;` all take markup bodies. A `match` arm is one node: wrap several siblings in `{ ... }` and end the arm with a comma. A catch-all arm that renders nothing is `_ => ""`.
- `#[key(expr)]` on a `for` loop gives every iteration its own identity. The key may be anything implementing `IdentityKey` (the probe keys on an integer and on a `&str`).
- An unkeyed loop renders fine until a component inside it needs an identity (any `signal(cx, ...)`). Then the render panics and the router answers 500: `ambiguous identity: 'for' loop at src/client.rs:89:8 repeats without a key attribute; add '#[key(...)]'`. Key every loop that contains a component with signals or shards.
- Helpers that are plain functions and need a distinct identity per call take `&cx.keyed(item.id)`.
- Components inside one `view!` render concurrently; the output is always in source order. Do not rely on side-effect order between components or template expressions.
- A `view!` moves the variables it uses, like an `async move` block. Clone before building the view if the value is needed afterwards.

Source: `topcoat-view-macro-0.10.0/docs/view.md:107-292,348-407`, `topcoat-core-0.10.0/src/context.rs:92` (`Cx::keyed`). Probe: `view_syntax.rs:77-95` (output `tests/rendering.rs:16`), `client.rs:81-103` and `tests/client.rs:22` (unkeyed panic, keyed success). Web: `web/src/admin/pages/servers.rs:146-147` (keyed loop), `web/src/admin/pages/servers.rs:161` (`match` with `Some`/`None` arms).

### 2.6 Status codes and response headers from a view

Status: VERIFIED. Features: `view`, `router`.

```rust
view! {
    (StatusCode::IM_A_TEAPOT)
    ((header::CACHE_CONTROL, HeaderValue::from_static("no-store")))
    <p>"teapot"</p>
}
```

Response: `418`, `cache-control: no-store`, body `<p>teapot</p>`.

- A `StatusCode` in node position sets the response status. A `HeaderMap`, or a `(HeaderName, HeaderValue)` pair written with double parentheses, adds headers. None of them renders content. A single pair in single parentheses is a compile error.
- The first status in markup order wins; for each header name the first declaration supplies all its values. In a layout, put declarations before `(slot)` to override the page, after it for defaults.
- Both must be set before the body streams. A status declared inside a `suspense` child that finishes later is ignored: the response stays `200` (probe `streaming.rs:77-89`). Declare it outside the `suspense` (section 14).
- Rendering a view to a string (section 16) discards them. A header-only declaration works: `((header::LOCATION, ...))` in a page without a status leaves 200, with `(StatusCode::FOUND)` gives 302 (section 5.8).

Source: `topcoat-view-macro-0.10.0/docs/view.md:495-532`, `topcoat-view-0.10.0/src/html/node.rs:155,169,180`. Probe: `view_syntax.rs:109-115`, `routes.rs:129-134`, `streaming.rs:51-57`. Web: `web/src/document.rs:76` (`(StatusCode::NOT_FOUND)`), `web/src/admin/pages/loadouts.rs:54` (`UNPROCESSABLE_ENTITY`).

### 2.7 Rendering outside a component, `boxed`, `single`

Status: VERIFIED.

```rust
async fn render_outside(cx: &Cx) -> Result<String> {
    let handle = view! { cx => <p>"outside"</p> }.single().await?;
    Ok(handle.render(cx))
}
```

- Inside `#[component]`, `#[page]`, `#[layout]`, `#[shard]` the request context is implicit. In a plain function start the macro with `cx =>`; without it the macro fails with `cannot find value __cx`.
- `ViewExt` (import it) adds `.single()` (collect to a `ViewHandle`, `.render(&cx)` makes the string) and `.boxed()`. Recursive components and `if/else` branches that build different view types need `.boxed()`; the error without it is `no method named boxed` until `ViewExt` is imported.

Source: `topcoat-view-macro-0.10.0/docs/view.md:533-547`, `topcoat-view-0.10.0/src/view.rs:59,79,95`, `topcoat-view-0.10.0/src/buffer/handle.rs:179`. Probe: `view_syntax.rs:118-121`, `components.rs:63-70`, `tests/harness.rs:9`. Web: `web/src/public/login.rs:16-17` (`.boxed()` branches), `web/src/engagement/pages/greetings/mod.rs:247`.

### 2.8 `attributes!` and `Attributes` forwarding

Status: VERIFIED.

```rust
let mut attrs = attributes! {
    class="button"
    id=(id)
    :data-bound=$(id.to_owned())
    @input="(e) => console.log(e)"
    if id == "submit" { type="submit" } else { type="button" }
};
attrs.insert(cx, "data-state", "loading");
view! { <button (attrs)>"go"</button> }
```

`Attributes` methods: `new`, `with_capacity`, `contains_key`, `get`, `insert(cx, key, value)`, `remove(key)`, `clear`, `extend`, `iter`. Each key appears once; inserting again replaces; order is unspecified. A `bool` value inserts as a boolean attribute.

Forwarding pattern used by every component in `web/src/components/`:

```rust
#[component]
pub async fn card(#[default] mut attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div class=(class!(CARD, attrs.remove("class"))) (attrs)>(child)</div> })
}
// call site
card(attrs: attributes! { class="max-w-sm" }, <p>"body"</p>)
```

`attrs.remove("class")` pulls the caller's class out so it is merged by `class!`; the remaining attributes spread onto the element. Output (probe `panel`): `<section class="panel x" data-x="1">`.

Source: `topcoat-view-0.10.0/src/html/attribute/attributes.rs:20-113`, `topcoat-view-macro-0.10.0/docs/attributes.md:1-152`, `docs/class.md:68-86`. Probe: `components.rs:32-46`, `view_syntax.rs:130-141`. Web: `web/src/components/card.rs:35-38`.

### 2.9 Custom values in markup

Status: VERIFIED.

```rust
pub struct Badge(pub String);
impl NodeViewParts for Badge {
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) { parts.push_string(self.0); }
}
pub struct DataId(pub Option<String>);
impl AttributeValueViewParts for DataId {
    fn attribute_present(&self) -> bool { self.0.is_some() }
    fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
        if let Some(v) = self.0 { parts.push_string(v); }
    }
}
view! { <p>(Badge("<New>".to_owned()))</p> <article data-id=(DataId(Some("post-1".to_owned()))) data-none=(DataId(None))></article> }
```

Rendered: `<p>&lt;New&gt;</p><article data-id="post-1"></article>`. `push_string` escapes for the position; the `push_*_unescaped` methods do not. Other position traits: `AttributeKeyViewParts`, `AttributeViewParts`, `ElementNameViewParts`, `ClassViewParts`.

Source: `topcoat-view-macro-0.10.0/docs/view.md:548-633`, `topcoat-view-0.10.0/src/html/node.rs:24`, `topcoat-view-0.10.0/src/html/attribute/value.rs:31`. Probe: `view_syntax.rs:144-172`. Web: no direct use.

## 3. Components

Features: `view`. A component is an `async fn` marked `#[component]` that returns `Result<impl View>`. Call it inside `view!` with lowercase `name(arg: value, ...)`; calls compile to `topcoat::view` component structs. Probe: `scratch/topcoat-kb/core/probe/src/components.rs`, rendered by `tests/rendering.rs:28`.

### 3.1 Props, `#[default]`, `#[default(expr)]`, `#[into]`, `Result`

Status: VERIFIED.

```rust
#[component]
pub async fn badge(
    #[into] label: String,
    #[default] tone: Tone,
    #[default(80)] max_len: usize,
    #[default] note: Option<String>,
) -> Result<impl View> {
    Ok(view! {
        <span class=(class!("badge", "badge-danger" if tone == Tone::Danger)) data-max=(max_len)>
            (label)
            if let Some(n) = note { <i>(n)</i> }
        </span>
    })
}

view! {
    badge(label: "plain")
    badge(label: String::from("danger"), tone: Tone::Danger, max_len: 3, note: Some("n".to_owned()))
}
```

Rendered: `<span class="badge" data-max="80">plain</span><span class="badge badge-danger" data-max="3">danger<i>n</i></span>`.

- Arguments are named and comma separated. `#[default]` uses `Default::default()` when the argument is omitted; `#[default(expr)]` uses `expr` and does not need `Default`; `#[into]` accepts anything `Into<T>` (a `&str` for a `String` parameter). They combine (`web/src/components/spinner.rs:25-31`).
- Parameters may borrow (`title: &str`, `guilds: &[GuildCard]`), be generic (`<F: Fn(u32) -> String + Send + Sync>`, `items: Vec<T>` with `T: Send + Sync`) or take `impl Into<String> + Send`. Closure props work because the component runs on the server (browser handlers are `$(...)`, section 4).
- `?` inside a component bubbles to the nearest `error_boundary` or the router (section 15).
- `#[derive(Props)]` generates a type-state builder with the same `#[default]`/`#[into]` field attributes: `ButtonProps::builder().label("Save").kind(Tone::Danger).build()` compiles and yields `disabled == false` (probe `components.rs:110-122`). No component in this repo uses it.
- A plain `fn -> impl View` that calls `view!` does not work (no implicit `cx`); use a component, or `view! { cx => ... }` (section 2.7).
- A `#[page]` or `#[layout]` doubles as a component: `crate::routes::root_layout(slot: Slot::new(view! { <p>"inner"</p> }))` and `crate::routes::home()` inside another view render inline (observed: `<!DOCTYPE html><html><body><nav>root</nav><p>inner</p></body></html><h1>home</h1>`, probe `components.rs:123-130`). A page that reads a body takes it as a `body:` prop (`topcoat-router-macro-0.10.0/docs/page.md`; not compiled).

Source: `topcoat-view-macro-0.10.0/docs/component.md:1-174`, `topcoat-view-macro-0.10.0/src/lib.rs:42-52`. Probe: `components.rs:16-31,48-61`. Web: `web/src/components/spinner.rs:22-34`, `web/src/admin/editor/view.rs:249` (`#[default] diamond: bool`).

### 3.2 Children, named slots and attribute forwarding

Status: VERIFIED.

```rust
#[component]
pub async fn panel(
    title: &str,
    #[default] mut attrs: Attributes,
    header: Child<'_>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <section class=(class!("panel", attrs.remove("class"))) (attrs)>
            <header>(header)</header>
            <h2>(title)</h2>
            <div class="body">(child)</div>
        </section>
    })
}

view! {
    panel(
        title: "Panel",
        attrs: attributes! { class="x" data-x="1" },
        header: view! { <em>"slot"</em> }.into(),
        <p>"child"</p>
        badge(label: "nested")
    )
}
```

Rendered: `<section class="panel x" data-x="1"><header><em>slot</em></header><h2>Panel</h2><div class="body"><p>child</p><span class="badge" data-max="80">nested</span></div></section>`.

- Nodes written after the named arguments (no commas) are collected into the parameter named `child`; `#[default]` lets callers omit it. Any other `Child<'_>` parameter is a named slot and takes `view! { ... }.into()`.
- `Slot<'a>` (in layouts) is a type alias for `Child<'a>`.
- `Attributes` parameters forward caller attributes (section 2.8).

Source: `topcoat-view-macro-0.10.0/docs/component.md:40-80`, `topcoat-view-0.10.0/src/child.rs:18`, `topcoat-router-0.10.0/src/page.rs:135`. Probe: `components.rs:32-46`. Web: `web/src/components/card.rs:35-38`, `web/src/document.rs:86-102`.

### 3.3 Async components with `cx`, recursion, memoize

Status: VERIFIED.

```rust
#[component]
pub async fn db_row(cx: &Cx, id: u32) -> Result<impl View> {
    let path = uri(cx).path().to_owned();
    Ok(view! { <p class="db-row">(id) " @ " (path)</p> })
}

#[component]
pub async fn countdown(n: u32) -> Result<impl View> {
    Ok(view! { <li>(n)</li> if n > 0 { countdown(n: n - 1) } }.boxed())
}
```

- A parameter named `cx` of type `&Cx` is injected and omitted at call sites. Awaiting database or HTTP work inside the body is the normal data-loading pattern; siblings render concurrently.
- A recursive component must `.boxed()` its view (`use topcoat::view::ViewExt`).
- Per-request caching of an expensive helper is `#[memoize]` (section 6.3).

Source: `topcoat-view-macro-0.10.0/docs/component.md:100-174`. Probe: `components.rs:63-70,82-86`. Web: `web/src/engagement/pages/greetings/mod.rs:247` (`.boxed()`), `web/src/admin/pages/servers.rs:65-66` (`cx: &Cx` component).

### 3.4 `error_boundary`

Status: VERIFIED.

```rust
error_boundary(
    fallback: |error| Ok(view! { <p class="caught">"caught: " (error.to_string())</p> }),
    failing()
)
```

Rendered: `<p class="caught">caught: bad request: nope</p>`. The closure receives `topcoat::Error`; returning `Err(error)` re-throws to the enclosing boundary or the router. The fallback replaces everything the boundary guarded, including already-streamed content. It is also the 404 mechanism (section 5.7).

Source: `topcoat-0.10.0/src/view/error_boundary.rs:50`. Probe: `components.rs:74-80,103-106`. Web: `web/src/document.rs:69-81`.

## 4. Client expressions `$(...)`

Features: `runtime` (plus `router` for pages, shards and procedures). The runtime is the only client-side code Topcoat generates: `$(...)` is a small Rust subset that runs on the server for the first render and is compiled to JavaScript that the browser runtime (49 KB, `topcoat-runtime-0.10.0/browser/dist/index.js`) evaluates. There is no WASM. Everything here is experimental upstream; expressions accept a closed vocabulary of types and syntax and reject the rest at compile time.

Probe: `scratch/topcoat-kb/core/probe/src/client.rs`, `tests/client.rs`; accept and reject matrices `exprcases/accept*.json`, `exprcases/reject.json`, results in `exprcases/results.txt`.

### 4.1 Setup

Status: VERIFIED. Features: `runtime`, `router`, `asset`.

```rust
Router::builder().assets(AssetBundle::load()?).runtime().build()   // router
// in <head>:
topcoat::runtime::script()
```

`.runtime()` registers `RuntimeLayer` (page reruns and the connection endpoint) and the runtime setup marker; `script()` renders the tag and asserts the marker, so a page rendered by a router built without `.runtime()` panics with `the browser runtime is not set up on this router; call '.runtime()' on the router builder` and the router answers `500 internal server error`. The script file is an asset (`topcoat::runtime::SCRIPT`), so the asset bundle must be registered or the render panics (section 11). Call `.runtime()` after registering your own layers so they handle a rerun as the rewritten `GET`.

Source: `topcoat-0.10.0/src/runtime.rs:6-26`, `topcoat-runtime-0.10.0/src/router.rs:14-80`, `topcoat-runtime-0.10.0/src/lib.rs:45`. Probe: `tests/assets.rs:46-58`. Web: `web/src/router.rs:35-38` (`.origin_policy(..).runtime().cookies().build()`), `web/src/document.rs:95` (`script()`).

### 4.2 Signals

Status: VERIFIED.

```rust
let count = signal(cx, || 0usize);
let name = signal(cx, String::new);
let agreed = signal(cx, || false);
let task = signal(cx, || Task { title: "Write".to_owned(), done: false });   // Task is a #[record]
view! { <p>"count=" $(count.get())</p> }
```

- `signal(cx, init)` creates browser state. Its identity comes from `cx` plus the call site, so a call inside a keyed loop needs `#[key]` (section 2.5). The initial value is serialized into an HTML comment (`<!--::topcoat::signal({...})-->`) before the element that reads it.
- Value types: `f64`, all integer types (`u8`..`u128`, `i8`..`i128`, `usize`, `isize`), `bool`, `String`, `Option<T>`, `Result<T, E>`, `Vec<T>`, `[T; N]`, tuples up to 12 elements, and `#[record]` structs. A plain struct is rejected as a signal value (``error[E0277]: the trait bound `Plain: SignalValue` is not satisfied``) and as a captured value (``error[E0277]: `Plain` cannot be used in runtime expressions``, a diagnostic new in 0.10).
- Server reads: `get()` (clone) and `read()` (borrow) are tracked: a browser change re-renders the enclosing shard or the whole page on the server. `get_untracked()` and `read_untracked()` read without tracking. There is no server-side `set`; the server sees whatever the browser sent.
- **Every signal value read on the server is client input.** Validate it exactly like a form field.
- Passing a signal to a component: `child_reads(flag: &open)` with `flag: &Signal<bool>`.
- Page rerun: a `POST` to a page URL with header `X-Topcoat-Runtime: true` and JSON body `{"signals": {"<id>": <value>}}` is rewritten to a `GET` of that page with the values restored. Observed: `POST /client` with `count`'s id set to `5` renders `count=5` and answers `200`; the same `POST` without the header answers `405` on a `GET` page. A `POST` page keeps working normally because only requests carrying the header are intercepted.

Source: `topcoat-runtime-0.10.0/src/signal.rs:17-300`, `topcoat-runtime-0.10.0/src/layer/rerun.rs:13-59`, `topcoat-runtime-0.10.0/src/surrogate.rs:31-34` (diagnostic), `topcoat-runtime-0.10.0/src/surrogate/tuple.rs` (`impl_tuple_surrogate!` up to 12). Probe: `client.rs:38-80` (page), `tests/client.rs:39-58` (rerun). Web: `web/src/shell/sidebar.rs:156`, `web/src/components/settings.rs:21`, `web/src/admin/pages/servers.rs:74-75`.

### 4.3 What `$(...)` and `expr!` accept

Status: VERIFIED (each row compiled or failed to compile in the probe). `$(...)` inside `view!` and `expr!(...)` outside it use the same grammar.

Accepted:

| Construct | Notes |
|---|---|
| Literals | string, bool, `f64` (`2.5`), integers. Unsuffixed integers are `usize`; suffixes `u8..u128`, `i8..i128`, `usize`, `isize`. `-1i32` works, `-1u32` is an error |
| Operators | `+ - * / %` (numbers only), `== != < <= > >=` (also on `String`/`&str`), unary `!` and `-`. Operands must have the same type |
| Methods | `String`/`&str`: `len`, `is_empty`, `trim`, `trim_start`, `trim_end`, `starts_with`, `ends_with`, `contains`, `to_owned`. `bool`: `then`, `then_some`. `Option`: `is_some`, `is_none`, `unwrap`, `expect`. `Result`: `is_ok`, `is_err`, `ok`, `err`, `unwrap`, `expect`, `unwrap_err`, `expect_err`. `Vec`/array/slice: `len`, `is_empty`, `get`, `index`, `first`, `last`, `to_vec`, `to_owned`, plus `as_slice`, `clone`. `get`, `first` and `last` borrow, so an expression cannot end on them (`error[E0515]: cannot return reference to temporary value`); finish with `.is_some()` or `.unwrap().clone()`. `Signal`: `get`, `set`, `toggle` (bool), `increment`/`decrement` (numbers), `push_str` (String) |
| Constructors | `Some(x)`, `None`, `None::<T>`, `Ok(x)`, `Err(x)`, tuple literals `(a, b)`, `()`, `(a,)`, record struct literals `P { x: 1u32 }` |
| Field access and indexing | tuple `.0`, record `.field`, `Event` and `EventTarget` fields, `v.get()[0usize]` |
| Blocks | `{ let x = ...; expr }` (plain identifier `let` only), `if`/`else if`/`else`, closures (`|e: Event| ...`, `|_e| ...`, `async |_e| ...`), `.await` on a procedure call, `loop`, `while`, `break`, `continue`, `return` |
| Macro | `raw!("js ${ident}", rust_equivalent)` only |

Rejected (error text is the compiler's):

| Construct | Error | Do instead |
|---|---|---|
| `a && b`, `a \|\| b`, `& \| ^ << >>` | `unsupported operator` | nested `if a { b } else { false }` |
| `x += 1`, `x = 2` | `unsupported operator` / `unsupported expression` | `signal.increment()`, `signal.set(..)`, new `let` |
| `match`, `if let`, `for`, `let (a, b) = ..`, `let ... else`, `let ref` | `unsupported expression` / `unsupported pattern` / `let-else is not supported` | `if`/`else` chain; compute server-side |
| `format!`, `vec!`, `view!`, any macro except `raw!` | `unsupported expression macro` | string building on the server, `raw!` |
| `[1, 2]`, `&x`, `x?`, `0..3`, `x as f64` | `unsupported expression` | |
| `*x` | `type 'UsizeSurrogate' cannot be dereferenced` | values are already owned |
| `String::new()`, any multi-segment path | `only single-identifier paths are supported` | `String::new()` outside, capture it |
| `s.parse::<u32>()` (turbofish) | `turbofish is not supported` | parse server-side |
| `s.get() + "x"` (string concatenation) | `cannot add &StrSurrogate to StringSurrogate` | build the string on the server |
| `s.parse()`, `n.to_string()`, `s.to_uppercase()`, `o.unwrap_or(..)` | `no method named ...`/`String: Surrogate not satisfied` | not in the vocabulary |
| `'c'`, `b"x"` | `unsupported literal type` | |
| `1u7` | `unsupported integer suffix` | |
| `'a: loop { break 'a }` | `labels are not supported` | |
| `fn g() {}` or other items | `unsupported statement` | |
| `P { ..base }` | `struct update syntax is not supported` | |
| calling a local closure, `g(1)` | `use of unstable library feature fn_traits` | inline the logic |
| comparing tuples or records | `no method ... eq` | compare fields |
| mixing integer types, `m.get() + 1` with `m: u32` | `expected U32Surrogate, found UsizeSurrogate` | `1u32` |
| `f.get() + 1` | `expected F64Surrogate, found UsizeSurrogate` | `1.0` |
| `s.get().len() > 100.0` | `expected &UsizeSurrogate, found &F64Surrogate` | `100usize` (0.10: `len` returns `usize`) |
| a captured non-vocabulary struct | ``'Plain' cannot be used in runtime expressions`` | `#[record]` |
| `|_e| ...` in a bare `expr!` | `type annotations needed` | `|_e: Event|`. Inside `@event=$(|_e| ...)` the parameter type is inferred |
| `q(1usize)` (procedure) without `.await` | trait bound not satisfied | `q(1usize).await` inside an `async` closure |

Behavior notes:

- Server evaluation is synchronous. An expression that reads no signal renders as plain static text (`$(1 + 2)` renders `3` with no comment markers).
- Integer overflow, division by zero and `MIN / -1` panic on both sides. Number inputs arrive as `String`; parse server-side or in `raw!`.
- A captured value is a snapshot taken at render; later server changes do not update it. Capturing clones. A captured `Expr<T>` (the result of `expr!`) behaves as `T` and re-evaluates on each use in the browser, so `let doubled = expr!(count.get() * 2); let label = expr!(if doubled > 10 { "big" } else { "small" });` stays reactive.
- `raw!` is the escape hatch. The Rust argument must read the same signals the JavaScript depends on; omit it only for browser-only code (`raw!("document.getElementById('x').focus()")`). `${ident}` must name a binding inside the expression; prefix with `_` to avoid `unused_variables`.

Source: `topcoat-runtime-macro-0.10.0/docs/expr.md:54-85`, grammar entry points (all under `topcoat-runtime-grammar-0.10.0/`) `src/expr.rs:129-153` (dispatch and `unsupported expression`), `src/expr/expr_binary.rs:12-32`, `src/expr/expr_unary.rs:23-33`, `src/expr/builtin_macro.rs:44-56`, `src/expr/expr_path.rs:28`, `src/expr/pat.rs:26`, `src/expr/stmt.rs:27,63`, `src/expr/expr_method_call.rs:19`, `src/expr/expr_lit.rs:33,53`, `src/expr/expr_struct.rs:38`, `src/expr/expr_break.rs:16`. Probe: `exprcases/accept.json`, `accept2.json`, `a3.json` (every listed method and signal value type), `reject.json`, `r3.json`, `r4.json`; results `exprcases/results.txt`. Web: `web/src/admin/pages/servers.rs:77-100` (`expr!` with `raw!` and a Rust fallback), `web/src/admin/pages/loadouts.rs:167`.

### 4.4 `@event` handlers and `:attr` bindings

Status: VERIFIED (server HTML observed; no browser run).

```rust
view! {
    <button @click=$(|_e| count.increment())>"+1"</button>
    <input :value=$(name.get()) @input=$(|e: Event| name.set(e.target.value))>
    <input type="checkbox" :checked=$(agreed.get()) @change=$(|e: Event| agreed.set(e.target.checked))>
    <button :disabled=$(!agreed.get())>"go"</button>
    <p :class=$(if choice.get() == "a" { "pick-a" } else { "pick-b" })>$(choice.get())</p>
    <button @click=$(|_e| open.toggle()) :aria-expanded=$(if open.get() { "true" } else { "false" })>"t"</button>
    <div :hidden=$(!open.get()) @keydown=$(|e: Event| { if e.key == "Escape" { open.set(false); } })>"panel"</div>
    <form @submit=$(|e: Event| { e.prevent_default(); count.increment(); })></form>
    <button @click=$(async |_e| { let d = double(count.get()).await; count.set(d); })>"double"</button>
    <button @click=$(|_e| { let _o = open; raw!("setTimeout(() => ${_o}.toggle(), 1000)") })>"later"</button>
}
```

Emitted HTML for the first line: `<button id="inc" data-topcoat-on:click="(__local0) => (cx.hydrate({...})).increment()">`; for a binding: `<input value="" data-topcoat-bind:value="(cx.hydrate({...})).get()" data-topcoat-on:input="...">`; a bound boolean renders the bare attribute when true (`hidden=""`), and a bound expression with no signal reads renders only the static attribute.

- `@name=$(closure)` attaches a DOM event handler; `@name="js source"` is raw JavaScript that must evaluate to a function. `:name=$(expr)` binds an attribute or property; the server renders the initial value and the browser updates it when a signal the expression reads changes. `:class` replaces the whole `class` attribute. `:name=(expr)` (plain parentheses) is evaluated once on the server and renders as an ordinary attribute with no `data-topcoat-bind` marker.
- `Event` fields (all surrogate values): `alt_key`, `bubbles`, `button`, `buttons`, `cancelable`, `client_x`, `client_y`, `code`, `ctrl_key`, `current_target`, `data`, `default_prevented`, `delta_x/y/z`, `event_type`, `input_type`, `is_composing`, `key`, `meta_key`, `movement_x/y`, `offset_x/y`, `page_x/y`, `pointer_id`, `pointer_type`, `repeat`, `screen_x/y`, `shift_key`, `target`, `time_stamp`. `target` and `current_target` (`EventTarget`) expose only `checked`, `id`, `name`, `text_content`, `value`. Methods: `prevent_default()`, `stop_propagation()`, `stop_immediate_propagation()`. There is no `files`, no `FormData`, no `dataset`. Numeric fields are `f64`.
- **Defect, unchanged in 0.10:** a string-literal handler written after another attribute loses the separating space: `<input id="lit"data-topcoat-on:click="alert(1)">`. Browsers recover (`HTML parse error`, attribute still applies), but prefer `$(...)`. The same handler built through `attributes!` and spread renders correctly (`probe view_syntax.rs:130-141`).
- The `data-topcoat-*` attributes and `<!--::topcoat::...-->` comments are part of the delivered HTML. Tests that compare markup strip them first (`web/tests/shell_pages.rs:271-293`).
- The runtime evaluates handlers with `new Function(...)`, so a strict Content-Security-Policy needs `script-src 'unsafe-eval'`, and streamed `suspense` regions need inline scripts (section 14). Topcoat has no CSP nonce support. `web` sends no CSP header.

Source: `topcoat-runtime-0.10.0/src/event_handler.rs:11-31`, `topcoat-runtime-0.10.0/src/bind_attribute.rs:12-40`, `topcoat-view-grammar-0.10.0/src/attributes/event_handler.rs:15-95`, `topcoat-view-grammar-0.10.0/src/attributes/bind_attribute.rs:17-62`, `topcoat-runtime-0.10.0/src/surrogate/event.rs:10-67`. Probe: `client.rs:38-80`, output `tests/client.rs:16`. Web: `web/src/admin/pages/servers.rs:107-109`, `web/src/shell/sidebar.rs:174-187`, `web/src/components/settings.rs:24-30`.

### 4.5 Procedures, shards, records

Status: VERIFIED (compile and server HTML; no browser).

```rust
#[procedure("/probe/double")]
pub async fn double(value: usize) -> Result<usize> { Ok(value * 2) }

#[shard("/probe/rows")]
pub async fn rows_shard(cx: &Cx, label: String) -> Result<impl View> {
    let page = signal(cx, || 1usize);
    let current = page.get();
    Ok(view! { <p id="shard">(label) " page " (current)</p><button @click=$(|_e| page.increment())>"next"</button> })
}

#[record]
#[derive(Clone)]
pub struct Task { pub title: String, pub done: bool }

Router::builder().route(double).route(rows_shard)   // or .discover()
view! { rows_shard(label: "rows".to_owned()) }
```

- A **procedure** is an async server function the browser calls from an `async` closure with `.await`. Argument types and the `Ok` type must be in the expression vocabulary. A parameter `cx: &Cx` is injected. `Err` is opaque to the client (the expression throws); return `Ok(Result<T, String>)` to surface a failure. The wire format is internal.
- A **shard** is a component with its own HTTP endpoint that re-renders on the server when its arguments or tracked signals change; the browser morphs the returned HTML into place (elements match by position and tag, or by `id`). Arguments accept a plain value or a `$(...)` expression. A server read of a signal inside the shard re-renders only the shard. Observed: the shard renders inline in the page between `<!--::topcoat::shard::start("/probe/rows", "<id>", [...])-->` and `<!--::topcoat::shard::end(...)-->`, followed by its own signal comment and `<!--::topcoat::dep("<signal id>")-->`.
- **Shard and procedure endpoints run without the page's layouts and guards. Authorize inside each one.** `web` has no shards or procedures; every guard is a page, route or layer.
- Without an explicit path, the endpoint path is `/_topcoat/runtime/procedures/<16 hex>` (shards: `/_topcoat/runtime/shards/...`), derived from a hash of the function name and its source location. **0.10 changed this**: 0.9 generated a random path per build. A path now changes only when the function is renamed or moved, but give anything cached or bookmarked an explicit absolute path.
- `#[record]` (new in 0.10) allows named-field structs without generics in signals, procedure arguments, and expression struct literals. Capturing a record exposes every field, including private ones, to the browser; validate records sent back.
- A WebSocket carries connected renders: `connected(cx)` returns `false` during the HTTP render and requests a connection; the connected render sees `true`. One socket per document, at most 64 concurrent renders per socket unless `.max_runs_per_connection(n)` is set; extra requests get `429`.

Source: `topcoat-runtime-macro-0.10.0/docs/procedure.md:1-90`, `topcoat-runtime-macro-0.10.0/docs/shard.md:1-160`, `topcoat-runtime-macro-0.10.0/docs/record.md:1-62`, `topcoat-runtime-grammar-0.10.0/src/common/endpoint_path.rs:19-40` (hashed path), `topcoat-runtime-grammar-0.10.0/src/procedure.rs:20,170`, `topcoat-runtime-0.10.0/src/connection.rs:36`, `topcoat-runtime-0.10.0/src/router.rs:48-80`. Probe: `client.rs:10-35,106-117`, `tests/client.rs:30-37` (default procedure path). Web: none (the loadout editor uses a hand-written module, `web/src/admin/editor/mod.rs:13`).

### 4.6 Hand-written JavaScript

Status: VERIFIED. Features: `asset`.

```rust
pub const PENDING_SUBMIT: Asset = asset!("../assets/pending-submit.js");
view! { <script type="module" src=(PENDING_SUBMIT)></script> }
```

Use an `asset!` module (section 11) for behaviour the expression grammar cannot express (form submission state, file readers, dialogs, focus management, `fetch`). Inline script text must go through `Unescaped::new_unchecked` (section 2.2). `web` does both: `web/src/document.rs:24` (`PENDING_SUBMIT`) and `web/src/admin/editor/mod.rs:13` (`LOADOUT_EDITOR_JS`).

Source: `topcoat-0.10.0/docs/asset.md`. Probe: `assets.rs:6-27`. Web: `web/src/document.rs:24,96`.

## 5. Router

Features: `router` (`serve` for the HTTP server, `discover` for link-time registration, `tower` for tower/axum interop). Topcoat has its own router (matchit paths, hyper server); it is not axum. There is no client-side router: every page is a full server render, and `topcoat::runtime::link` (section 12) adds navigation without a full reload.

Probe: `scratch/topcoat-kb/core/probe/src/routes.rs`, `routes2.rs`, `modroute.rs`, `interop.rs`; observed responses in `tests/routes.rs`, `tests/routes2.rs`, `tests/modroute.rs`, `tests/interop.rs`.

### 5.1 `Router::builder`, explicit registration, route groups

Status: VERIFIED.

```rust
Router::builder()
    .layout(root_layout)          // #[layout("/")]
    .layout(guild_layout)         // #[layout("/guild/{guild_id}")]
    .layer(api_layer)             // #[layer("/api")]
    .page(home)                   // #[page("/")]
    .page(form_get)               // #[page("/form")]
    .page(form_post)              // #[page(POST "/form")]
    .route(api_json)              // #[route(GET "/api/health/{guild_id}")]
    .page(not_found)              // not_found!("/")
    .app_context(state)
    .origin_policy(OriginPolicy::new().exempt_paths(["/webhooks/{*rest}"]))
    .trailing_slash(TrailingSlash::Redirect)
    .runtime()
    .build()
```

- `#[page("/path")]`, `#[layout("/path")]`, `#[layer("/path")]` and `#[route(METHOD "/path")]` generate a unit struct with the function's name; pass that name to `.page`, `.layout`, `.layer`, `.route`. Because the struct lives in the value namespace, **a local variable or parameter with the same name as a route function is read as a unit-struct pattern**: a test helper `fn go(r: &Router, req: Builder)` fails to compile (`req is interpreted as a unit struct`) when a route called `req` is imported. Likewise `not_found!("/")` defines an item named `not_found` and clashes with an imported `topcoat::router::error::not_found()`.
- Registration order does not affect matching. `build()` panics on a duplicate `METHOD path` (`duplicate route registered for 'GET /'`) and on a layer whose path matches no route (`layer with path '/api' did not match any route, this is likely a mistake`); register layers only for prefixes that have routes.
- Other builder methods: `.app_context(value)` (section 6), `.trusted_proxies(..)`, `.compression(Compression::..)`, `.base_url(..)`, `.assets(..)` (section 11), `.cookies()` (section 8), `.runtime()` (section 4).
- `web` registers by name through route groups, so the whole route table is one fold: `ROUTE_GROUPS: [fn(RouterBuilder) -> RouterBuilder; 8]` and each module exports `pub fn routes(base: RouterBuilder) -> RouterBuilder`. Tests build the same router with their own base builder (`web::router(Router::builder().assets(..).app_context(..))`, section 16).

Source: `topcoat-router-0.10.0/src/builder.rs:92-401` (builder methods, panics at `:469` and `:485`), `topcoat-router-macro-0.10.0/src/lib.rs:8-74`, `topcoat-0.10.0/docs/router.md`. Probe: `routes.rs:20-183`, `tests/routes.rs:8-34,122-132`. Web: `web/src/router.rs:11-39`, `web/src/public/mod.rs:15-23`, `web/src/engagement/pages/greetings/mod.rs:89,96`.

### 5.2 Module routing and `.discover()`

Status: VERIFIED. Features: `router`, `discover`. **Not used by `web`.**

`module_router!()` derives a handler's path from its enclosing Rust module; `#[page]`, `#[layout]`, `#[layer]`, `#[route(GET)]` carry no path string. Upstream now recommends this over explicit paths.

```rust
pub mod app {
    pub fn router() -> Router { module_router!().build() }

    #[layout] async fn root_layout(slot: Slot<'_>) -> Result<impl View> { /* ... */ }
    #[page]   async fn home() -> Result<impl View> { /* GET / */ }
    #[page(POST "./export")] async fn export() -> Result<impl View> { /* POST /export */ }
    not_found!();                                   // /{*rest} under this module

    pub mod blog_posts {                            // /blog-posts
        #[page] async fn list() -> Result<impl View> { /* ... */ }
        path_param!(pub tag);
        #[page("./tagged/{tag}")] async fn tagged(cx: &Cx) -> Result<impl View> { /* /blog-posts/tagged/{tag} */ }
        pub mod post_id {                           // /blog-posts/{post_id}
            module_param!(pub post_id: u32, error = bad_request);
            #[page] pub async fn post(cx: &Cx) -> Result<impl View> { /* ... */ }
        }
    }
    pub mod docs { pub mod doc_path { module_param!(pub *doc_path); /* /docs/{*doc_path} */ } }
    pub mod _marketing { /* group: no URL segment */ pub mod pricing { /* GET /pricing */ } }
}
```

Observed (`tests/modroute.rs`): `/` 200, `/export` GET 405 `allow: POST`, `/blog-posts` 200, `/blog-posts/tagged/rust` 200, `/blog-posts/12` 200 with links `/blog-posts`, `/blog-posts/12`, `/blog-posts/tagged/x`, `/blog-posts/x` 400 `invalid value for path parameter "post_id"`, `/docs/a/b/c` 200 (`a/b/c`), `/pricing` 200, `/marketing/pricing` 404, `/blog-posts/` 308 to `/blog-posts`.

- Module name to segment: static names are kebab-cased (`blog_posts` to `blog-posts`); a module starting with `_` is a group (no URL segment, still part of layout/layer matching); `segment!(rename = "..", kind = ..)` overrides. The function name does not matter; two module-derived handlers in one module share a path (different methods only).
- A path starting `./` is joined below the module. An absolute path string opts out of the module tree and must be registered by name (`.page(legacy)`) or by `.discover()`.
- `.discover()` (trait `RouterBuilderDiscoverExt`) also registers explicit-path handlers, procedures and shards linked into the binary. Discovered layers must have unique paths. Do not mix `.discover()` with `.page(x)` of the same item (double registration panics).
- Discovery is `inventory`-based link-time collection, so the final binary must link the crate that defines the handlers. The probe's tests link the library by referencing it (`use core_probe::modroute::app`).
- **Breaking in 0.10:** `path_param!` no longer sets the module's segment; `module_param!` does (see [changes-0.9-to-0.10.md](changes-0.9-to-0.10.md)).

Source: `topcoat-router-0.10.0/docs/module.md`, `topcoat-router-0.10.0/src/module.rs:13-16`, `topcoat-router-0.10.0/src/module/segment.rs:16-40`, `topcoat-router-macro-0.10.0/docs/module_param.md`. Probe: `modroute.rs` (all), `tests/modroute.rs`. Web: none (it registers explicitly).

### 5.3 Path syntax and path parameters

Status: VERIFIED.

```rust
path_param!(pub guild_id: i64, error = not_found);   // typed: FromStr; failure maps to a router error
path_param!(pub section);                            // untyped: &str, never fails
path_param!(pub *rest_path);                         // catch-all: CatchAllSegments

#[page("/guild/{guild_id}/settings/{section}")]
pub async fn guild_settings(cx: &Cx) -> Result<impl View> {
    let guild_id = path_param::<GuildId>(cx)?;       // Result<&i64, _>, 404 on failure
    let section: &str = path_param::<Section>(cx);
    Ok(view! { <h1>"settings " (guild_id) " / " (section)</h1> })
}
```

- Paths: `/users` static, `/{id}` one non-empty segment, `/{*path}` one or more trailing segments, `/(group)` group (stripped from the URL, counts for layout/layer matching). Parameter names start with an ASCII letter or `_`. The old `:id` syntax does not parse.
- `path_param!(name)` generates a PascalCase type (`GuildId`); read with `path_param::<GuildId>(cx)`. Untyped returns `&str` (percent-decoded); typed returns `Result<&T, &T::Err>`, or `&T` after `?` when `error = ...` is set. `error =` accepts `not_found`, `unauthorized`, `forbidden`, `bad_request` (message `invalid value for path parameter "post_id"`), `bad_request("msg")`, `redirect("/x")`, `redirect_permanent("/x")`; an untyped parameter cannot use it. Parsing is lazy and memoized per request. Reading a parameter the matched route did not capture panics.
- `href!(guild_settings, GuildId(7), Section("general"))` builds `/guild/7/settings/general` (section 5.12).
- **A layout or layer matches a handler only when its leading segments spell out the same names.** A layout at `/team/{team_id}` wraps `/team/{team_id}/y` and does not wrap `/team/{team}/x` (observed: the second page renders without `<div class="team-shell">`). Use one `path_param!(guild_id)` and the same `{guild_id}` name in every `/guild/...` page and the layout.
- A static segment beats a parameter: `/guild/new` is served by its own page, and the guild layout (`/guild/{guild_id}`) does not wrap it.
- Declare the same `path_param!` once and import it where both layout and pages need it. `web` shares `path_param!(pub guild_id)` between `shell/guild_layout.rs:11` and the guild pages; `settings/mod.rs:30` declares `section`; `admin/editor/mod.rs:15` declares `loadout_id`. All three are untyped and parsed by the handler.
- 0.10 change: `path_param!` only declares the type; it no longer emits a `segment!` override (that is `module_param!`). With explicit paths nothing changes.

Source: `topcoat-router-macro-0.10.0/docs/path_param.md:1-204`, `topcoat-router-0.10.0/src/path_param.rs:40-83`, `topcoat-router-grammar-0.10.0/src/path_param.rs:283-322`, `topcoat-router-macro-0.10.0/docs/layout.md`, `docs/layer.md`. Probe: `routes.rs:16-18,39-76`, `routes2.rs:10-31`, `tests/routes.rs:61-94`, `tests/routes2.rs:6-20`. Web: `web/src/shell/guild_layout.rs:11,27-30`, `web/src/settings/mod.rs:30`.

### 5.4 Query parameters

Status: VERIFIED.

```rust
#[query_params(error = bad_request)]
pub struct SearchQuery { pub q: Option<String>, pub page: Option<u32> }

let query = topcoat::router::query_params::<SearchQuery>(cx)?;
```

Observed: `/search` is `q=- page=1`; `?q=a&page=2` parses; `?page=x` is `400 text/plain` ``bad request: invalid query value: invalid digit found in string (at `page`)``; `?page=` and `?q=` read as `None`. A missing key on a non-`Option` field is an error (`#[serde(default)]` is not applied). The macro derives `Deserialize`; the struct must be `Send + Sync + 'static` (it is memoized). `error = redirect("?")` clears a bad query and loops if an empty query is invalid.

Source: `topcoat-router-macro-0.10.0/docs/query_params.md:1-60`, `topcoat-router-0.10.0/src/query_param.rs:32`. Probe: `routes.rs:79-91`. Web: `web/src/auth/login.rs:23-26,55`.

### 5.5 Methods, HEAD, 405, handler return values

Status: VERIFIED.

- `#[page]` serves `GET` unless methods are named (`#[page(POST "/x")]`); `#[route]` always names them: one (`GET`), a list (`[GET, POST]`) or `*` for every method. A specific method beats `*` at the same path. A GET page and a POST page can share a path (web: every settings form).
- `HEAD` is served by the `GET` handler. A wrong method answers `405` with `allow: GET, POST, HEAD` (observed on `PUT /both`, and `DELETE /` gives `allow: GET, HEAD`).
- Handlers take optional `cx: &Cx` and at most one body parameter (section 7). A route returns `Result<T>` where `T` is a response: `String` or `&'static str` (text/plain), `()` (empty), `Json<T>` (application/json), `Html<T>` (text/html), `Js`, `Css`, `Wasm`, a view, `Response`, `SeeOther`, or a tuple `(StatusCode, [(HeaderName, HeaderValue); N], body)`. Observed `Ok((StatusCode::ACCEPTED, Json(Health { .. })))` is `202 application/json {"ok":true,"guild":7}`; `Html("<b>raw</b>")` is `200 text/html`.
- Panics inside a handler become `500 internal server error`.

Source: `topcoat-router-macro-0.10.0/docs/route.md`, `topcoat-router-0.10.0/src/content.rs`, `topcoat-router-0.10.0/src/content/html.rs:19`, `topcoat-router-0.10.0/src/response.rs`. Probe: `routes.rs:157-175`, `routes2.rs:34-37`. Web: `web/src/admin/editor/endpoints.rs:19-27` (`(StatusCode, Json<Reply<T>>)`), `web/src/providers/kofi.rs:28`.

### 5.6 Layouts and layers

Status: VERIFIED.

```rust
#[layout("/")]
pub async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! { <!DOCTYPE html><html><body><nav>"root"</nav>(slot)</body></html> })
}

#[layer("/api")]
pub async fn api_layer(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let mut response = next.run(cx, body).await?;
    response.headers_mut().insert("x-api", HeaderValue::from_static("1"));
    Ok(response)
}

// Function layer without a macro, with an optional path:
pub fn require_auth(path: &'static str) -> LayerFn { LayerFn::new(Some(path), require_auth_handler) }
```

- A layout takes `slot: Slot<'_>` (and optionally `cx`) and nests from least specific (outermost) to most specific. A layout re-renders on every request; there is no persistent shell on the client.
- **Routes (`#[route]`) are not wrapped by layouts**: `/api/health/x` (a route) answers a plain-text `404 not found`, not the branded page.
- A layer receives `cx`, `body`, `next` and may answer without calling `next` (`web` returns `401` that way). A layer with a path wraps only matched handlers under that path prefix (compared segment by segment at build time); a `404` or `405` never runs it. `LayerFn::new(None, handler)` wraps every request, misses included, and receives a miss as the `Err` from `next.run` (observed: `/nope` reaches the layer and the layer turns it into `418 caught by layer`).
- Layers registered explicitly on one path nest by registration order (the last is outermost); collected-by-`discover` layers need unique paths.
- A layer that registers request-scoped values derives a child context with `cx.with(value)` (section 6) and passes that to `next.run` (documented upstream, not exercised in the probe).

Source: `topcoat-router-0.10.0/src/layer.rs:66-130,170`, `topcoat-router-0.10.0/src/page.rs:135`, `topcoat-router-macro-0.10.0/docs/layout.md`, `topcoat-router-macro-0.10.0/docs/layer.md`. Probe: `routes.rs:20-51,176-181`, `routes2.rs:39-57`. Web: `web/src/document.rs:64-82` (`#[layout("/")]`), `web/src/shell/guild_layout.rs:26-37`, `web/src/auth/middleware.rs:10-44` (`LayerFn` guard), `web/src/providers/mod.rs:32-33` (`.layer(require_auth("/patreon"))`).

### 5.7 Not-found pages, fallbacks, status pages

Status: VERIFIED.

```rust
not_found!("/");                                    // registers a page named `not_found` at /{*rest}

// root layout: turn NotFoundError into a branded 404, re-throw everything else
error_boundary(
    fallback: |error| {
        if error.downcast_ref::<NotFoundError>().is_none() { return Err(error); }
        Ok(view! { (StatusCode::NOT_FOUND) <h1>"404 branded"</h1> })
    },
    (slot)
)
```

- `not_found!("/")` (or `not_found!()` under module routing) is a catch-all page that returns `NotFoundError`; a layout's `error_boundary` renders the branded view. The catch-all requires at least one segment, so `/` itself must have its own page.
- Without `not_found!`, an unmatched URL answers a bare `404 text/plain "not found"` and runs no layout (observed). With it but without `(StatusCode::NOT_FOUND)` in the fallback, the branded page answers `200` (probe `routes2.rs:58-80`). Re-throwing other errors keeps their own status: under the same layout `/search?page=x` still answers `400`.
- Typed `path_param!(.., error = not_found)` failures and `ok_or_not_found()` land in the same boundary (observed: `/guild/abc` and `/guild/42/settings/missing` render `404 branded`).
- Other router errors answer plain text and are not caught by the 404 branch: `401 unauthorized`, `403 forbidden`, `400 bad request: <msg>`, `500 internal server error` for anything else.

Source: `topcoat-router-macro-0.10.0/docs/not_found.md`, `topcoat-router-0.10.0/docs/error.md:45-100`, `topcoat-router-0.10.0/src/error/not_found.rs:23`. Probe: `routes.rs:20-37,183`, `tests/routes.rs:116-120`. Web: `web/src/document.rs:64-82`, `web/src/pages/not_found.rs:7`.

### 5.8 Redirects, rewrites and their status codes

Status: VERIFIED. All rows observed in `tests/routes.rs`.

| Construct | Status | Notes |
|---|---|---|
| `Err(redirect(uri).into())` | `307` | `topcoat::router::error::redirect` |
| `Err(redirect_permanent(uri).into())` | `308` | |
| `Ok(see_other(uri))` / `Err(see_other(uri).into())` | `303` | `Result<SeeOther>` from a route; `Err` from a page. `web` redirects with `see_other` after POSTs and in the OAuth routes |
| `opt.ok_or_redirect(uri)?` | `307` | `RouterErrorExt` |
| `(StatusCode::FOUND, [(header::LOCATION, v)], ())` from a route | `302` | no built-in 302 constructor; a literal tuple works |
| `view! { (StatusCode::FOUND) ((header::LOCATION, v)) }` in a page | `302` | rendered inside the layouts, so the body is the empty document |
| `Err(rewrite("/form", Body::empty()).method(Method::GET).with(Saved).into())` | `200` | handled again at another path without a client round trip; the browser URL does not change; the receiving page reads `try_request_context::<Saved>(cx)` |

A redirect error returned from a page layout or component bubbles up as any other error. A custom error type with no mapping layer becomes `500`.

Source: `topcoat-router-0.10.0/src/error/redirect.rs:33,59,161`, `topcoat-router-0.10.0/src/error/rewrite.rs:58-75`, `topcoat-router-0.10.0/src/error.rs:126-136`. Probe: `routes.rs:98-156`, `tests/routes.rs:61-94`. Web: `web/src/auth/login.rs:50,68`, `web/src/engagement/pages/greetings/mod.rs:107-110` (post then 303).

### 5.9 Trailing slash

Status: VERIFIED. `.trailing_slash(TrailingSlash::Redirect | Serve | Strict)`.

- `Redirect` (default): the other form of a declared route answers `308` with the slashless/slash form and keeps the query (`/search/?q=a` to `/search?q=a`). It only applies to declared routes: `/home/` with no `/home` route is `404`.
- `Serve`: both forms answer `200`; the handler sees the requested URL in `uri(cx)`.
- `Strict`: the other form is `404`.

Source: `topcoat-router-0.10.0/src/trailing_slash.rs:31-55`, `topcoat-router-0.10.0/src/builder.rs:283`. Probe: `tests/routes.rs:105-114`. Web: default (`web/src/router.rs` does not set it).

### 5.10 Mounting axum or tower services and layers

Status: VERIFIED. Features: `tower` (`discover` for `.discover()`). Needs `axum = "0.8"` (workspace pin 0.8.9) in the application, not in Topcoat.

```rust
pub fn axum_webhooks() -> TowerRoute<axum::Router> {
    let app = axum::Router::new()
        .route("/webhooks/kofi", post(|State(s): State<AxumState>, Form(body): Form<KoFi>| async move { format!("kofi ok {} {}", s.secret, body.data.len()) }))
        .route("/webhooks/youtube", get(|Query(v): Query<Verify>| async move { v.challenge }))
        .with_state(AxumState { secret: "s3" });
    TowerRoute::any("/webhooks/{*rest}", app)
}
pub fn axum_redirect() -> TowerRoute<axum::Router> {
    TowerRoute::new(Method::GET, "/patreon/callback", axum::Router::new().route("/patreon/callback", get(|| async { Redirect::to("/guild/1/settings/patreon") })))
}
Router::builder().route(axum_webhooks()).route(axum_redirect()).layer(TowerLayer::new(tower::layer::util::Identity::new())).build()
// reverse direction: axum outside, Topcoat as the fallback
axum::Router::new().route("/outer", get(|| async { "outer" })).fallback_service(TowerService::new(topcoat_router))
```

Observed: the mounted axum app serves `POST /webhooks/kofi` (`kofi ok s3 3`), `GET /webhooks/youtube?challenge=zz` (`zz`), `GET /patreon/callback` (`303 location /guild/1/settings/patreon`); `/webhooks/unknown` is axum's empty `404` (not the branded page); `/webhooks` alone does not match `{*rest}` and falls to the branded `404`. A cross-origin `POST` is rejected with `403` before reaching axum until the path is exempted (section 10). The axum service receives the full URI; add `StripPrefixLayer::new("/legacy")` if it expects relative paths. Services must be `Clone + Send + Sync`; `TowerLayer` allows one inner call per request (no retry middleware); `TowerLayer::new(l).at("/api")` limits it to matched handlers. In the reverse direction insert `RemoteAddr` yourself for `remote_addr`/`client_ip`.

`web` does not use this: the Ko-fi, Patreon and YouTube webhooks, OAuth callbacks and `/logout` are native `#[route]`s taking `Bytes` or `Form<Vec<(String, String)>>` (`web/src/providers/*.rs`).

Source: `topcoat-router-0.10.0/src/tower.rs:65-152` (`TowerRoute`), `:183-290` (`TowerLayer`), `:528-560` (`TowerService`), `topcoat-router-0.10.0/docs/tower.md`. Probe: `interop.rs:1-52`, `tests/interop.rs`. Web: `web/src/providers/kofi.rs:27-28` (native route instead).

### 5.11 Serving and shutdown

Status: VERIFIED (signatures); the server was not started in the probe.

```rust
let listener = tokio::net::TcpListener::bind(addr).await?;
topcoat::serve(listener, router).await?;                 // graceful shutdown on Ctrl+C / SIGTERM
topcoat::serve_until(listener, router, shutdown_future).await?;
topcoat::start(router).await?;                           // binds HOST:PORT, default 127.0.0.1:3000
```

`serve` and `serve_until` take any `Listener` (`TcpListener`, and `UnixListener` on Unix); active requests have `RouterService::shutdown_timeout` (default 30 s) to finish. `web` binds the address from `BotConfig` or an env override and calls `topcoat::serve` (`web/src/main.rs:57-58`). Under `topcoat dev`, `serve` notifies the dev server when ready.

Source: `topcoat-0.10.0/src/serve.rs:29,44,66`. Web: `web/src/main.rs:42-58`.

### 5.12 `href!`, `href()` and `Href::is_current`

Status: VERIFIED.

```rust
let a = href!(post, PostId(5)).query(Pagination { page: 2 }).fragment("comments");   // /posts/5?page=2#comments
let b = href!(posts).query(Pagination { page: 3 });                                  // /posts?page=3
let c = href("/posts/{post_id}", (PostId(9),));                                      // /posts/9
let resolved = href!(post, PostId(1)).resolve(cx);                                   // String, "/posts/1"
let current = href!(guild_overview, GuildId(*guild_id)).is_current(cx);              // active-link styling
view! { <a href=(a) aria-current=(current.then_some("page"))>"x"</a> }
```

`href!(handler, ParamValue, ..)` takes the handler's Rust name so URLs follow its registered path; values are the `path_param!` types, filled in path order, `Display`ed and percent-encoded. The `Href` renders directly in a view; `.resolve(cx)` gives a `String` for redirects. `.query(..)` takes a `Serialize` struct. Observed on `/guild/42` inside the guild layout: `aria-current="page"` and `class="nav on"` on the overview link only.

Source: `topcoat-router-0.10.0/src/href.rs:443,499,528,590,650`, `topcoat-router-0.10.0/docs/href.md`. Probe: `extras.rs:15-30`, `routes.rs:39-51`, `tests/extras.rs`. Web: none (it writes paths as strings and compares `uri(cx).path()`: `web/src/shell/sidebar.rs:22`, `web/src/admin/pages/loadouts.rs:91`).

## 6. Request context `Cx`

Features: none (`topcoat::context` is always present; the `request` accessors need `router`). `Cx` is a cheap handle to the state of one request: the request parts, app and request context values, the memoize cache and identity. Add `cx: &Cx` to any `#[page]`, `#[route]`, `#[layout]`, `#[layer]`, `#[component]`, `#[shard]` or `#[procedure]` and Topcoat supplies it. Everything else is a plain function that takes `&Cx` (the "functions, not middlewares" style: `web` guards pages with plain async helpers such as `auth::guild_admin_context(cx, guild).await?` (`web/src/admin/access.rs:35`), not with extractors).

Probe: `scratch/topcoat-kb/core/probe/src/routes.rs`, `extras.rs`, `streaming.rs`, `tests/harness.rs`.

### 6.1 Request accessors

Status: VERIFIED. Features: `router`.

```rust
use topcoat::router::request::{client_ip, content_type, headers, method, original_uri, parts, remote_addr, uri};

let ua = headers(cx).get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("-");
format!("{} {} orig={} ua={ua}", method(cx), uri(cx), original_uri(cx))   // "GET /req orig=/req ua=t"
```

| Function | Returns |
|---|---|
| `parts(cx)` | `&http::request::Parts` (method, uri, version, headers, extensions) |
| `method`, `uri`, `version`, `headers`, `extensions` | one field of the (possibly rewritten) request |
| `content_type(cx)` | `Option<&str>` |
| `original_parts`, `original_method`, `original_uri`, `original_version`, `original_headers`, `original_content_type`, `original_extensions` | the request as the client sent it (differs only after a rewrite or `StripPrefixLayer`) |
| `remote_addr(cx)` | `Option<SocketAddr>` of the direct peer |
| `client_ip(cx)` | `Option<IpAddr>`; honours `.trusted_proxies(TrustedProxies::new().networks([..]).nearest(n))` |
| `endpoint(cx)` / `try_endpoint(cx)` | the matched `Endpoint`; `.path().as_str()` is the registered pattern (e.g. `/guild/{guild_id}/settings`), `try_` returns `None` when nothing matched |
| `path_param::<T>(cx)`, `query_params::<T>(cx)` | typed, memoized (section 5) |
| `router(cx)` / `try_router(cx)`, `route(cx)` / `try_route(cx)` | the router and matched route |

Observed in the probe with no listener: `remote_addr` and `client_ip` are `None` under `Router::handle`. `web` reads `uri(cx).path()` for active-link logic and `try_endpoint(cx).path()` for page titles.

Source: `topcoat-router-0.10.0/src/request.rs:173-512`, `topcoat-router-0.10.0/src/router.rs:248-341`, `topcoat-router-0.10.0/src/proxy.rs:59`. Probe: `routes.rs:165-175`, `extras.rs:15-30` (`tests/extras.rs`). Web: `web/src/document.rs:47-52` (`try_endpoint`), `web/src/shell/sidebar.rs:22` (`uri`), `web/src/providers/patreon.rs:259` (`headers`), `web/src/providers/fields.rs:49` (`content_type`).

### 6.2 App context (shared state) and request context

Status: VERIFIED.

```rust
Router::builder().app_context(state)                  // by type; any Send + Sync + 'static, not Clone
fn db(cx: &Cx) -> &Db { app_context::<Db>(cx) }       // panics if unregistered
try_app_context::<WebState>(cx)                       // Option<&T>
request_context::<T>(cx) / try_request_context::<T>(cx)
let child = cx.with(9u8);                             // child Cx with an added request value
let child = cx.with_many((A, B));                      // a tuple of values or a RequestContext
```

- App context is registered once on the builder, keyed by type. A second value of the same type panics at the `.app_context(..)` call (`duplicate context entry for type 'core_probe::streaming::Db'`). `app_context::<T>` on an unregistered type panics when the page renders, so the router answers `500 internal server error`; `try_app_context` returns `Option`. `web` uses `try_app_context::<WebState>` everywhere and maps `None` to `AuthError::MissingContext`.
- Request context values attach to one request: `cookies` (`CookieJarCell`), `SignalValues` (page reruns), values carried by `rewrite(..).with(v)`, and anything a layer adds with `cx.with(v)`. A child context inherits the parent's values, shares its request state, and replaces a value of the same type only in the child scope (observed: `Some(9) Some(7)`).
- `PrefetchMode` and `SuspenseMode` are read from request context, then app context, then default (sections 12 and 14).
- A `Cx` is a handle to shared state: clone it into a streaming body or a spawned task (`let cx = cx.clone();`) instead of borrowing the one passed to the handler.

Source: `topcoat-core-0.10.0/src/context.rs:31-140`, `topcoat-core-0.10.0/src/context/app_context.rs:25,59`, `topcoat-core-0.10.0/src/context/request_context.rs:31,66`, `topcoat-router-0.10.0/src/builder.rs:355`, `topcoat-0.10.0/docs/context.md`. Probe: `streaming.rs:59-75`, `tests/streaming.rs:31-42`, `tests/harness.rs:8-17`. Web: `web/src/auth/context.rs:14-41`, `web/src/main.rs:57` (`Router::builder().app_context(state).assets(assets)`), `web/src/state.rs:67` (`WebState`).

### 6.3 `#[memoize]` (per-request cache) and `Cx::keyed`

Status: VERIFIED. Features: none.

```rust
#[memoize]
pub async fn lookup(cx: &Cx, key: &str) -> usize { /* runs once per distinct key per request */ 42 }

#[memoize(as_ref)]
pub async fn opt_user(cx: &Cx, token: &str) -> Option<String> { (!token.is_empty()).then(|| token.to_owned()) }   // returns Option<&String>

let a = lookup(cx, "k").await;   // &usize
let b = lookup(cx, "k").await;   // cached: same request, one execution
```

- The function must take `cx: &Cx` by name; every other argument must be `Hash`. The return type becomes `&T`; `as_ref` turns `Option<T>` into `Option<&T>` and `Result<T, E>` into `Result<&T, &E>`. Two calls in one request run the body once (observed: `LOOKUPS=1`).
- **A by-value `String` argument fails to compile** (`error[E0505]: cannot move out of 'key' because it is borrowed`) although the upstream docs say by-value works; `u64` and `&str` arguments compile. Take `&str`.
- Cache lives for one request. There is no cross-request cache; keep a `moka` cache inside the app context (`web` does: `WebState.sessions`, `WebState.discord.user_guilds`).
- `web` does not use `#[memoize]`; `web/src/auth/session.rs` looks the session up through its own cache each call. It is the natural fit for `current_session_identity` (called by every guard on a page).
- `cx.keyed(id)` derives a context with a distinct identity for helper calls inside loops (`signal`, shards); see section 2.5.

Source: `topcoat-core-macro-0.10.0/docs/memoize.md`, `topcoat-core-macro-0.10.0/src/lib.rs:8`, `topcoat-core-0.10.0/src/context.rs:92`. Probe: `streaming.rs:61-107`, `tests/streaming.rs:6-15,24-30`. Web: none (`web/src/auth/session.rs:79-86` is the call site that would use it).

### 6.4 `Cx` without a request

Status: VERIFIED. `CxTestBuilder::new().app_context(v).request_context(w).build()` returns a `Cx` for unit tests that render components or call helpers without routing (section 16).

Source: `topcoat-core-0.10.0/src/context.rs:198-240`. Probe: `tests/harness.rs`. Web: `web/tests/auth_routes.rs:550,555` (`CxTestBuilder` to call `auth::current_session_identity`).

## 7. Forms and body parsing

Features: `router` (`multipart` for uploads). A page or route takes `cx: &Cx` and **at most one body parameter** (any `FromRequest`), in either order. Wrap an extractor in `Option` to accept an absent body. Browsers submit `<form method="post">` as `application/x-www-form-urlencoded`; there is no client-side form state in Topcoat.

Probe: `scratch/topcoat-kb/core/probe/src/forms.rs`, `routes.rs`, `optional.rs`; `tests/forms.rs`, `tests/routes.rs`, `tests/optional.rs`.

### 7.1 Extractors

Status: VERIFIED. Observed in `tests/forms.rs` and `tests/routes.rs`.

| Extractor | Reads | Failure |
|---|---|---|
| `Form<T>` | `GET`/`HEAD`: the query string. Other methods: the body, requires `Content-Type: application/x-www-form-urlencoded` | `400 text/plain`, e.g. ``bad request: invalid form value: invalid digit found in string (at `limit`)``; missing or wrong content type: ``bad request: expected request with `Content-Type: application/x-www-form-urlencoded` `` |
| `Form<Vec<(String, String)>>` | every `name=value` pair in order, percent-decoded, `+` as space (`a=b%20c+d&e=` gives `[("a","b c d"),("e","")]`) | none beyond the content type |
| `RawForm(bytes)` | the buffered body; decode yourself with `Form::<T>::from_bytes(&bytes)` | the handler still runs when `from_bytes` fails, so it can re-render |
| `Json<T>` | body, requires `application/json` | `400` (`invalid JSON syntax: EOF while parsing an object at line 1 column 1`, or the content-type message) |
| `Bytes` (`topcoat::router::request::Bytes`) | the buffered body, no content-type check | `413` over the limit |
| `Body` (`topcoat::router::Body`) | the unbuffered stream; no limit | none |
| `Multipart` (feature `multipart`) | streamed fields; `next_field().await?`, `field.name()`, `.file_name()`, `.content_type()`, `.bytes().await?` | `?` propagates a multipart error |
| `Option<Form<T>>` | `None` when the request has no content type (non-GET) or no query (GET) | |

- Decoding rules for `Form<struct>`: an empty value (`limit=`) is `None` for an `Option<T>` field; `limit=5` is `Some(5)`; a checkbox that is present is `Some("on")` and absent is `None`; a missing non-`Option` field is a `400`.
- **Repeated keys do not decode into `Vec<T>`.** `struct { roles: Vec<u64> }` with `roles=1&roles=2`, or even one `roles=1`, answers `400` (``invalid type: string "1", expected a sequence (at `roles`)``). Take `Form<Vec<(String, String)>>` and fold the pairs yourself. `web` does this for every form: `web/src/form.rs:15-40` (`fold(pairs, ["name", ...])` rejects unknown, duplicate and missing names) behind the `form_args!` macros in `web/src/engagement/form.rs` and `web/src/guild/form.rs`.
- Multipart bodies are streamed, but the raw-body extractors buffer: **default limit 2 MiB** (`413 content too large`; observed at 2 MiB + 1). Raise or lower it per path with `.layer(BodyLimit::max(n).at("/upload"))` (verified: with `max(16)` a 22-byte body answers `413`). Taking `Body` directly is not limited.
- Implement `FromRequest` yourself for bodies needing a signature check; delegate buffering to `Bytes` to keep the limit.
- `web` reads webhook bodies as `Bytes` and parses them by hand so it can check the content type exactly and return its own rejection (`web/src/providers/fields.rs:44-57`).

Source: `topcoat-router-0.10.0/src/content/form.rs:56-160`, `topcoat-router-0.10.0/src/content/json.rs:60`, `topcoat-router-0.10.0/src/urlencoded.rs:1-50`, `topcoat-router-0.10.0/src/content/multipart.rs:60`, `topcoat-router-0.10.0/src/body_limit.rs:9,40,71`, `topcoat-router-0.10.0/src/request.rs:85`, `topcoat-router-0.10.0/docs/content.md`. Probe: `forms.rs:14-40,73-80`, `tests/forms.rs:18-41,70-81`, `optional.rs:2-19`, `tests/optional.rs:7-15`. Web: `web/src/shell/overview.rs:37` (`Form<Vec<(String, String)>>`), `web/src/admin/editor/endpoints.rs:40` (`Json`), `web/src/providers/kofi.rs:28` (`Bytes`).

### 7.2 A POST page: validation re-render or post/redirect/get

Status: VERIFIED.

```rust
#[page("/form")]
pub async fn form_get(cx: &Cx) -> Result<impl View> {
    let saved = try_request_context::<Saved>(cx).is_some();
    Ok(view! { <p>"form " (saved)</p> })
}

#[page(POST "/form")]
pub async fn form_post(Form(input): Form<Settings>) -> Result<impl View> {
    if input.prefix.trim().is_empty() {
        return Ok(view! { (StatusCode::UNPROCESSABLE_ENTITY) <p class="error">"prefix required"</p> });
    }
    Err(see_other("/form?saved=1").into())
}
```

Observed: invalid input answers `422` with the page markup; valid input answers `303 location: /form?saved=1`. A shared view for the GET and the error re-render must be a `#[component]`. `web` follows this pattern in its form pages (greetings, reaction roles, settings): `Err(see_other(location).into())` on success, otherwise render the same page component with the submitted values and an error status (`web/src/engagement/pages/greetings/mod.rs:96-119`).

To show a result without a redirect, re-handle the request as a `GET` carrying a value: `Err(rewrite("/form", Body::empty()).method(Method::GET).with(Saved).into())`; the receiving page sees `try_request_context::<Saved>(cx)` (observed: `POST /form/rewrite` answers `200` with `form true`).

Source: `topcoat-router-macro-0.10.0/docs/page.md`, `topcoat-router-0.10.0/src/error/rewrite.rs:58-110`. Probe: `routes.rs:98-118`, `tests/routes.rs:80-86`. Web: `web/src/engagement/pages/greetings/mod.rs:96-119`, `web/src/admin/pages/loadouts.rs:47-56` (`422`).

### 7.3 Multipart uploads

Status: VERIFIED. Features: `multipart` (not enabled in `web`).

```rust
#[route(POST "/upload")]
pub async fn upload(mut multipart: Multipart) -> Result<String> {
    let mut out = Vec::new();
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().map(str::to_owned);
        let file = field.file_name().map(str::to_owned);
        let ctype = field.content_type().map(ToString::to_string);
        let data = field.bytes().await?;
        out.push(format!("{name:?}:{file:?}:{ctype:?}:{}", data.len()));
    }
    Ok(out.join(","))
}
```

Observed: a body with a text field and a `x.png` file part answers `Some("a"):None:None:5,Some("f"):Some("x.png"):Some("image/png"):7`. Finish or drop each field before requesting the next. `web` receives its only upload (emoji images) as a URL or data URI inside a `Json` body instead (`web/src/admin/editor/endpoints.rs:32-35,53-60`).

Source: `topcoat-router-0.10.0/docs/content/multipart.md`, `topcoat-router-0.10.0/src/content/multipart.rs:37-60`. Probe: `optional.rs:2-19`, `tests/optional.rs:7-15`. Web: `web/src/admin/editor/endpoints.rs:53`.

## 8. Cookies

Features: `cookie`. Topcoat builds on the `cookie` 0.18 crate: a cookie is `topcoat::cookie::Cookie`, builder helpers and `SameSite` are re-exported, and `topcoat::cookie::time` is the `time` crate. Register the layer once with `.cookies()`, then call `cookies(cx)` from any handler; changes queue on a jar shared by the request and are written as `Set-Cookie` headers when the handler returns, including when it returns an error or a redirect.

Probe: `scratch/topcoat-kb/core/probe/src/forms.rs:44-112`, `tests/forms.rs:43-57,82-89`.

### 8.1 Read, add, remove

Status: VERIFIED.

```rust
Router::builder().route(set_cookie).cookies().build()      // required

pub fn build_cookie(name: &'static str, value: String, max_age: i64) -> Cookie<'static> {
    Cookie::build((name, value)).path("/").http_only(true)
        .secure(!cfg!(debug_assertions)).same_site(SameSite::Lax)
        .max_age(Duration::seconds(max_age)).build()
}

let jar = cookies(cx);                                       // &CookieJar; import the `Cookies` trait for its methods
jar.remove(Cookie::build(("oauth_state", "")).path("/").build());
jar.add(build_cookie("session", "tok".to_owned(), 3600));
let value = cookies(cx).get("session").map(|c| c.value().to_owned());
cookies(cx).add(build_cookie("session", String::new(), 0)); // logout: expire immediately
```

Observed `Set-Cookie`:

- add: `session=tok; HttpOnly; SameSite=Lax; Path=/; Max-Age=3600` (debug build: no `Secure`).
- `remove` with the cookie present in the request: `oauth_state=; Path=/; Max-Age=0; Expires=<one year earlier>`. **With the cookie absent from the request, `remove` emits nothing.**
- `cookies(cx).add(.. max_age 0)` always emits `Max-Age=0`: use this form to force removal (`web` logout).
- Without `.cookies()`, `cookies(cx)` panics (`attempted to access request context of type "topcoat_cookie::CookieJarCell"`) and the router answers `500`. `web` guards with `try_request_context::<CookieJarCell>(cx)` and returns `AuthError::MissingContext` instead.
- Writes must happen before the body streams. Upstream documents that `add`/`remove` after the jar is sealed (for example inside a streamed `suspense` child) panic; set cookies in the page body before any `suspense`. Reading still works. (Documented, not exercised in the probe.)
- `.cookies()` installs a root layer; the most recently registered root layer runs first, so register it after other root layers that call `cookies(cx)`. `web` registers it last (`web/src/router.rs:37`).

Source: `topcoat-cookie-0.10.0/src/lib.rs:37,261,307`, `topcoat-cookie-0.10.0/src/router.rs:30-69`, `topcoat-0.10.0/docs/cookie.md`. Probe: `forms.rs:44-72`, `tests/forms.rs:43-57`. Web: `web/src/auth/cookie.rs:9-21` (`build`), `web/src/auth/login.rs:44,122,149`, `web/src/auth/context.rs:35-41` (`cookie_jar`).

### 8.2 Attribute defaults, prefixes, signed, private, typed stores

Status: VERIFIED (compiled and observed; not used by `web`). Features: `cookie`.

```rust
let jar = cookies(cx).default_secure(true).default_http_only(true).default_same_site(SameSite::Lax).default_path("/");
jar.add(cookie!("plain" = "v"));                                            // plain=v; HttpOnly; SameSite=Lax; Secure; Path=/
cookies(cx).override_prefix_host().add(cookie!("hostcookie" = "v"));       // __Host-hostcookie=v; Secure; Path=/
signed_cookies(cx).add(cookie!("signed" = "v"; Path = "/"));                // value prefixed with an HMAC
private_cookies(cx).add(cookie!("private" = "secret"; Path = "/"));        // AES-GCM encrypted
let cart = cookie_store::<Cart, _>(private_cookies(cx), "cart")
    .parse_or_default().update(|c| c.items.push("widget".to_owned())).commit()?;   // nothing is written before commit()
Router::builder().app_context(Key::generate()).cookies()                    // key for signed_cookies/private_cookies
```

`default_*` fill an attribute only when the cookie lacks it; `override_*` force it. `signed_cookies(cx)` and `private_cookies(cx)` read a `Key` from app context and panic when none is registered; `.signed(&key)` and `.private(&key)` take one explicitly. Generate the key once and persist it, or every restart invalidates the cookies.

Source: `topcoat-0.10.0/docs/cookie.md`, `topcoat-cookie-0.10.0/src/lib.rs:319,331`, `topcoat-cookie-0.10.0/src/macros.rs:45`, `topcoat-cookie-0.10.0/src/store.rs:131,253,282`. Probe: `forms.rs:83-112`, `tests/forms.rs:83-89`. Web: none.

## 9. Sessions

Status: VERIFIED for `start`, `token_hash`, `refresh`, `rotate`, `stop` (compiled and observed); a custom `TokenStore` is not compiled. Features: `session` (plus `cookie` and `router`). **`web` does not use `topcoat::session`.**

```rust
Router::builder().cookies().sessions(SessionConfig::default()).build()

#[route(POST "/s/login")]
pub async fn login(cx: &Cx) -> Result<String> {
    let session = session::start(cx).await?;          // Session { token_hash, expires_at }; you persist both
    let _ = session.token_hash;
    Ok("started".to_owned())
}
```

Observed `Set-Cookie` from `session::start`: `__Host-session=<44 chars URL-safe base64>; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=2592000` (30 days, `DEFAULT_LIFETIME`).

What the module is: a token transport and a lifecycle API. It mints a 32-byte random token, sends it in a hardened cookie, and hands the application the token's SHA-256 hash (`TokenHash`) to store; `session::token_hash(cx)` returns the hash of the token the request presented; `stop`, `refresh` (sliding expiry) and `rotate` complete the lifecycle. Observed: with no cookie `token_hash`, `refresh`, `rotate` and `stop` all return `None`; with the cookie sent back all four return `Some`, and `stop` queues `__Host-session=; ...; Max-Age=0`. The application owns the table.

Why the app avoids it, from the current schema and code:

- `web` keeps the existing browser cookie `session` and the existing table `web_sessions(token TEXT PRIMARY KEY, discord_user_id, discord_access_token, expires_at)` (`migrations/0002_v2_schema.up.sql:113-119`), which stores the 64-hex token itself, generated in `web/src/auth/login.rs:122-126`. `topcoat::session` uses `__Host-session`, `Secure` and a base64 32-byte token, and expects the stored key to be the SHA-256 of that token.
- Adopting it means a migration (hash the key column or add one), a new cookie name, and signing every user out once.
- The default store forces `Secure` and `Path=/` (`override_secure(true)`), which `web` currently makes depend on `cfg!(debug_assertions)` (`web/src/auth/cookie.rs:17`), so a debug build over plain http still receives the cookie.
- A custom `TokenStore` can change the cookie name and encoding, but the stored value would still need to be the token hash.

Source: `topcoat-0.10.0/docs/session.md`, `topcoat-session-0.10.0/src/session.rs:34,55,72,95,123`, `topcoat-session-0.10.0/src/token/store.rs:21-110` (cookie store: `override_prefix_host`, `override_secure(true)`), `topcoat-session-0.10.0/src/config.rs:23-95`, `topcoat-session-0.10.0/src/router.rs:31-42`, `topcoat-session-0.10.0/src/token.rs:13-60`. Probe: `optional.rs:20-46`, `tests/optional.rs:17-23,34-44`. Web: `web/src/auth/session.rs:11-77` (lookup against `web_sessions`), `web/src/auth/login.rs:116-156` (`start_session`).

## 10. Origin policy (CSRF)

Status: VERIFIED. Features: `router`.

```rust
.origin_policy(OriginPolicy::new().exempt_paths(["/webhooks/kofi", "/webhooks/patreon", "/webhooks/youtube"]))
OriginPolicy::new().trust_origins(["https://accounts.example.com"])      // full origin, no trailing slash
OriginPolicy::dangerous_disable()
```

The policy runs before every layer and handler and is on by default. A request is rejected with `403 forbidden` (text/plain) when it can change state and does not look same-origin:

| Request | Result |
|---|---|
| `GET`, `HEAD`, `OPTIONS` (not a WebSocket upgrade) | allowed |
| path matches an `exempt_paths` pattern | allowed |
| `Origin` equals a `trust_origins` entry (ASCII case-insensitive) | allowed |
| `Sec-Fetch-Site: same-origin` or `none` | allowed |
| `Sec-Fetch-Site: cross-site` or `same-site` | `403` |
| no `Sec-Fetch-Site`, `Origin` host equals `Host` | allowed |
| no `Sec-Fetch-Site`, `Origin` host differs from `Host` | `403` |
| neither header (curl, server-to-server) | allowed |

Observed: `POST /both` with `sec-fetch-site: cross-site` is `403`; `same-origin` is `200`; `Origin: https://evil.example` with `Host: app.example` is `403`; matching origin is `200`; a cross-site `GET` is `200`. With `exempt_paths(["/both"])` the same cross-site `POST` is `200` while `POST /form` stays `403`; `dangerous_disable()` allows all. A webhook such as `POST /webhooks/kofi` carrying `Origin: https://ko-fi.com` is `403` until exempted (section 5.10). Exempt paths use route-path syntax and accept `{*rest}`.

`web` exempts exactly its three webhook endpoints (`web/src/providers/mod.rs:15-19`); every other POST (settings forms, the editor's JSON endpoints) depends on the browser sending `Sec-Fetch-Site` or a matching `Origin`. There is no CSRF token: do not add state-changing `GET` routes.

Source: `topcoat-router-0.10.0/src/origin.rs:31-216`, `topcoat-router-0.10.0/src/builder.rs:216`. Probe: `tests/routes.rs:88-103`, `tests/interop.rs:23-46`. Web: `web/src/providers/mod.rs:15-19`, `web/src/router.rs:35`.

## 11. Assets

Features: `asset` (plus `router` to serve the bundle and `view` to render `Asset` values). Static files are declared in Rust with `asset!`, bundled by the CLI after the build (`topcoat asset bundle`, see [tooling.md](tooling.md)), and served by the router under a content-hashed URL.

Probe: `scratch/topcoat-kb/core/probe/src/assets.rs`, `tests/assets.rs`, `assets/probe.js`.

### 11.1 `asset!`, URLs, serving and caching

Status: VERIFIED.

```rust
pub const PROBE_JS: Asset = asset!("../assets/probe.js");              // relative to this source file
pub const PROBE_CT: Asset = asset!("../assets/probe.js", rename: "renamed", content_type: "text/javascript");
pub const STYLESHEET: Asset = asset!(concat!(env!("OUT_DIR"), "/tailwind.css"));

view! {
    <script type="module" src=(PROBE_JS)></script>
    <script src=(PROBE_CT) defer=""></script>
    <link rel="stylesheet" href=(STYLESHEET)>
}
```

Rendered with a bundle that maps `probe` and `renamed`: `<script type="module" src="/_topcoat/assets/probe-0123456789abcdef.js"></script>` and `<script src="/_topcoat/assets/renamed-0123456789abcdef.js" defer="">`.

- Paths: `./x` and `../x` are relative to the source file that calls `asset!`; a bare relative path is relative to the crate's `CARGO_MANIFEST_DIR`; absolute paths are used as-is; `http(s)://...` is downloaded and cached by the bundler (UNVERIFIED here). A computed path (`concat!(env!("OUT_DIR"), ...)`) works, which is how the Tailwind output is referenced.
- Options: `rename: "stem"`, `extension: "ext"`, `checksum: "sha256:<hex>"` (required match for the raw source), `content_type: "text/css"` (otherwise guessed from the extension).
- An `Asset` renders as the served URL, so it can be used wherever an attribute value goes. `asset.id().as_u64()` is its manifest key (tests use it to forge manifests, section 16).
- Served by `.assets(bundle)` at `GET /_topcoat/assets/<stem>-<16 hex>.<ext>` (prefix is fixed). Observed headers: `content-type: text/javascript`, `cache-control: public, max-age=31536000, immutable`; no `ETag`, no `Content-Length`; the file is read from disk on each request; an unknown file is `404 text/plain`. Compression (below) applies when the body is larger than 32 bytes and the client accepts it.
- `AssetConfig::hosted_at("https://cdn.example.com/assets", bundle)` renders URLs on an external host and registers no routes (UNVERIFIED).
- Assets whose handle is never referenced may be dropped by the compiler and then do not appear in the bundle.

Source: `topcoat-asset-0.10.0/src/asset.rs:15,32,314` (`Asset`, `id`, `asset!`), `topcoat-asset-0.10.0/src/options.rs:14-40`, `topcoat-asset-0.10.0/src/serve.rs:15-90`, `topcoat-asset-0.10.0/src/config.rs:38,78`, `topcoat-0.10.0/docs/asset.md`. Probe: `assets.rs:6-27`, `tests/assets.rs:26-44`. Web: `web/src/document.rs:22-24` (stylesheet and `pending-submit.js`), `web/src/admin/editor/mod.rs:13` (`loadout-editor.js`).

### 11.2 The bundle: `AssetBundle`, manifest, failure modes

Status: VERIFIED.

```rust
let assets = AssetBundle::load()?;                     // <dir of current_exe>/assets
let assets = AssetBundle::load_dir("dist/assets")?;    // explicit directory
Router::builder().assets(assets)                       // RouterBuilderAssetExt
```

A bundle directory holds the hashed files and `manifest.toml`:

```toml
version = 1

[[assets]]
id = 123456789          # Asset::id().as_u64()
file = "probe-0123456789abcdef.js"
hash = "0"
content_type = "text/javascript"
```

- `load`/`load_dir` return `io::Result`: `NotFound` when the directory or manifest is missing, `InvalidData` for a manifest whose `version` is not `1`. `web` turns a failure into `StartupError::AssetBundle` with the hint `run 'topcoat asset bundle -p web'` (`web/src/error.rs:19-22`).
- **Rendering an `Asset` the router has no bundle for panics** (`no asset config registered in this router context; load the asset bundle with '.assets(AssetBundle::load().unwrap())'`; the router answers `500`). Rendering an asset missing from a loaded bundle panics as well. Page tests that render `runtime::script()`, `STYLESHEET` or any `asset!` must register a bundle (section 16).
- The binary and its bundle must come from the same build; asset ids depend on the build.
- Fixed-URL directory serving is separate (feature `fs`): `Router::builder().public_dir("./assets")` serves `./assets/probe.js` at `/probe.js` with `last-modified`, `content-type` and `content-length` and no immutable caching (observed). `web` has no public directory.

Source: `topcoat-asset-0.10.0/src/bundle.rs:29,60`, `topcoat-asset-0.10.0/src/manifest.rs:8-40`, `topcoat-asset-0.10.0/src/router.rs:62`, `topcoat-asset-0.10.0/src/view.rs:10`, `topcoat-router-0.10.0/src/route/directory.rs`. Probe: `tests/assets.rs:8-24,46-60`, `optional.rs:49-55`, `tests/optional.rs:25-30`. Web: `web/src/main.rs:48` (`AssetBundle::load()`), `web/src/main.rs:57` (`.assets(assets)`).

### 11.3 Response compression

Status: VERIFIED. Features: `compression` (in `web`).

Every response body is negotiated against `Accept-Encoding` after all layers run: `gzip` and `br` are offered, at the `Balanced` level, for bodies of 32 bytes or more and compressible content types; `vary: accept-encoding` is added. Observed on a page: `gzip` gives `content-encoding: gzip`, `br` gives `br`, `br, gzip` picks `br`, `identity` is untouched. Tune or disable with `.compression(Compression::new().brotli(false).level(CompressionLevel::Fastest))` or `Compression::off()`.

Source: `topcoat-router-0.10.0/src/compression.rs:11-60`, `topcoat-router-0.10.0/src/builder.rs:261`. Probe: `tests/forms.rs:59-68`. Web: default configuration.

## 12. Runtime script and client navigation

Features: `runtime` (+ `router`, `asset`). Status: VERIFIED for server output; behavior in the browser is as documented upstream.

### 12.1 `runtime::script()` and `data-topcoat-usize-bits`

Status: VERIFIED.

```rust
view! { <head> topcoat::runtime::script() </head> }
```

Observed with a bundle and `.runtime()`:

```html
<script type="module" src="/_topcoat/assets/topcoat-0123456789abcdef.js" data-topcoat-usize-bits="64"></script>
```

**New in 0.10:** the tag carries `data-topcoat-usize-bits` (`usize::BITS` of the server) so browser-created lengths match the server's integer width. Do not write your own `<script src=(topcoat::runtime::SCRIPT)>`; use `script()`. The upstream-visible effect for this repo is the exact head markup asserted in `web/tests/router.rs:127-147`.

Source: `topcoat-0.10.0/src/runtime.rs:6-26`. Probe: `tests/assets.rs:26-31`. Web: `web/src/document.rs:95`, `web/tests/router.rs:127-147`.

### 12.2 `link`, `link_attrs`, `PrefetchMode` (new in 0.10)

Status: VERIFIED (server markup); navigation behavior UNVERIFIED in a browser. **Not used by `web`.**

```rust
link(href: "/client", "self")                                          // <a data-topcoat-link="intent" href="/client">self</a>
link(href: "/client", prefetch: PrefetchMode::Viewport, "view")        // data-topcoat-link="viewport"
link(href: href!(about::page), attrs: attributes! { class="nav-link" }, "About")
<a id="custom" (link_attrs(cx, "/client", prefetch_mode(cx)))>"custom"</a>
Router::builder().runtime().prefetch(PrefetchMode::Viewport)           // app-wide default
let cx = cx.with(PrefetchMode::Never);                                 // scoped default
```

- `link` renders an ordinary `<a>`, so it works without JavaScript. With the runtime it fetches the destination, updates the document (including `<title>`) and history, keeps signals declared on both pages, and restores scroll on back/forward. Modifier clicks, downloads, external links and `target=` links keep browser behavior. If a destination needs scripts that are not loaded, the browser loads the page normally.
- `PrefetchMode::Intent` (default: hover, focus, touch), `Viewport` (when visible), `Never`. Precedence: the link's `prefetch:` argument, then request context, then app context, then `Intent`. **Prefetching renders the destination page on the server even if it is never opened, so page rendering must not mutate state.**
- `href` and `prefetch` override the same keys in `attrs`. `.runtime()` must be on the router and `script()` in the document head.
- A WebSocket carries connected renders; `.max_runs_per_connection(n)` (default 64) caps them.

Source: `topcoat-runtime-0.10.0/src/link.rs:24-130`, `topcoat-runtime-0.10.0/docs/link.md`, `topcoat-runtime-0.10.0/src/router.rs:48-80`. Probe: `client.rs:74-77`, `tests/client.rs:16`. Web: none (`web` navigates with plain `<a href>` and form posts).

### 12.3 Dev script

Status: VERIFIED (renders nothing without `TOPCOAT_DEV_URL`; the tag itself was read from the source).

`topcoat::dev::script()` (feature `router`) renders the dev-server live-reload tag when `TOPCOAT_DEV_URL` is set by `topcoat dev`, and nothing otherwise (`status_indicator: false` hides the badge). `web` does not include it. See [tooling.md](tooling.md).

Source: `topcoat-0.10.0/src/dev.rs:63-77`.

## 13. Head, title and meta

Status: NO EQUIVALENT for per-page head management. Features: `view`.

0.10 has no `Title`, `Meta`, `Link` or head-hoisting component: a `<title>` or `<meta>` is emitted exactly where the markup that contains it is written. A page body renders inside the layouts' `<body>`, so a `<title>` written in a page lands in `<body>` and is not a document title. The pieces that exist:

- A layout owns the document skeleton (`<!DOCTYPE html><html lang=..><head>..</head><body>(slot)</body></html>`), so `<meta charset>`, viewport, stylesheet, `runtime::script()` and `<title>` go there.
- The title can be derived from the route the layout is wrapping. `web` does this: `try_endpoint(cx)` gives the matched pattern (`/guild/{guild_id}/levels`), a static table maps patterns to titles (`PAGE_TITLES`), and `settings::route_title` computes the dynamic ones. The result is passed to a `document(title: &str, child)` component that writes `<title>(title)</title>`.
- Per-page extras (a description, `og:` tags) need another prop on the `document` component and a way for the page to supply it before the layout renders; the layout renders around the page and cannot read a value the page computes. Options: derive it from the route like the title, or have each page render the `document` component itself instead of relying on one root layout.
- With `runtime::link` navigation the runtime replaces the title from the new document's `<title>`.
- Response `status` and headers (including `Cache-Control`) are set from the view body, not from `<head>` (section 2.6).

```rust
#[layout("/")]
pub(crate) async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let title = page_title(cx);                        // Cow<'static, str> from try_endpoint(cx)
    Ok(view! { document(title: &title, (slot)) })
}

#[component]
async fn document(title: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-bot="zayden">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link rel="stylesheet" href=(STYLESHEET)>
                topcoat::runtime::script()
                <script type="module" src=(PENDING_SUBMIT)></script>
                <title>(title)</title>
            </head>
            <body>(child)</body>
        </html>
    })
}
```

Source: absence confirmed by `grep -rniE "fn title|struct Title|head!" topcoat-{view,router,runtime}-0.10.0/src` (no match); `topcoat-router-0.10.0/src/router.rs:295-307` (`endpoint`, `try_endpoint`); `topcoat-view-0.10.0/src/hoist.rs:78` (hoisting places parts before the enclosing view's content, not in `<head>`). Probe: `routes.rs:20-37` (layout skeleton), `tests/assets.rs:26-31` (head output). Web: `web/src/document.rs:26-102`, `web/tests/router.rs:127-141` (exact head markup).

## 14. Streaming and async

Features: `view` (`router` to stream a response). The response starts when the first view content is ready; everything not yet ready streams later.

### 14.1 `suspense`

Status: VERIFIED.

```rust
suspense(fallback: view! { skeleton() }, slow_stats(ms: 30))
suspense(fallback: view! { skeleton() }, mode: SuspenseMode::Wait, slow_stats(ms: 10))   // no fallback
Router::builder().suspense(SuspenseMode::Wait)     // RouterSuspenseExt: app-wide default
```

The shell and each fallback arrive first, wrapped in `<!--::topcoat::region::start(ID)-->` ... `<!--::topcoat::region::end(ID)-->`. When a child finishes, the server appends `<template data-topcoat-swap="ID">content</template><script>topcoat.swap("ID")</script>`; results arrive out of order, as they complete (observed: the failing child, then the `live!` update, then the 30 ms child). The first streamed chunk also carries a one-time inline script that defines `window.topcoat.swap`; **0.10 added `document.currentScript.remove();` to it**. No runtime script is needed.

- A child that is ready immediately never shows its fallback. `mode: SuspenseMode::Wait` (or a `Wait` default) holds the response until the child's first content, so no fallback is rendered (observed: `/stream-wait` is the plain `<p id="stats">stats 10</p>`).
- `SuspenseMode` resolution: the `mode:` argument, then a `SuspenseMode` in request context, then in app context, then `Stream`. 0.10 replaced `SuspenseMode::resolve(cx, explicit)` with `suspense_mode(cx)`.
- `suspense` does not catch errors; wrap it in `error_boundary` (observed: a failing child inside `error_boundary` replaces its region with the fallback markup).
- **After the first byte, status, headers and cookies cannot change.** A `StatusCode` set before the `suspense` applies (observed `202`); one set inside a later-finishing child is ignored (`200`). Upstream documents that `cookies(cx).add` after the jar is sealed panics.
- A page without `suspense` sends nothing until every component is ready. `web` wraps the data-heavy parts in `suspense` with skeleton fallbacks: the tier badge in the nav, the overview module grid, the levels leaderboard, the FAQ articles, the loadout table and the upgrade page content.

Source: `topcoat-0.10.0/src/view/suspense.rs:52-140`, `topcoat-0.10.0/src/view/error_boundary.rs:50`, `topcoat-router-0.10.0/src/content/view.rs:273-300` (`SWAP_SCRIPT`). Probe: `streaming.rs:7-57`, `tests/streaming.rs:6-15,17-22,44-50`. Web: `web/src/shell/overview.rs:94-100`, `web/src/shell/chrome.rs:57-60`, `web/src/engagement/pages/levels.rs:34`, `web/src/settings/support/faq.rs:51`, `web/src/admin/pages/loadouts.rs:114`.

### 14.2 `live!` and `emit!`

Status: VERIFIED (HTTP stream only).

```rust
(live! {
    emit! { <p>"first"</p> }?;
    tokio::time::sleep(Duration::from_millis(10)).await;
    emit! { <p>"second"</p> }
})
```

A `live!` region runs ordinary async Rust; each `emit!` replaces the region's content (first emission renders with the page, later ones stream as swaps; observed `first` inline, `second` as a trailing `<template data-topcoat-swap>`). The body must emit at least once and returns `Result<EmitToken>`. `connected(cx)` keeps a region updating over a WebSocket after the response finishes; one socket per document, 64 concurrent connected renders by default (`.max_runs_per_connection(n)`). Dev reload applies streamed `live!` updates without waiting for the whole response (0.10).

Source: `topcoat-view-macro-0.10.0/docs/live.md`, `topcoat-view-macro-0.10.0/src/lib.rs:14-26`, `topcoat-runtime-0.10.0/src/connection.rs:36-51`. Probe: `streaming.rs:36-41`. Web: none.

### 14.3 Concurrent rendering

Status: VERIFIED.

Components, shards and template expressions in one `view!` are polled concurrently and written in source order. Observed: two sibling components that each sleep 200 ms render in 201 ms (`/concurrent`). Treat component bodies as free of side effects and do not depend on another component having run first.

Source: `topcoat-view-macro-0.10.0/docs/view.md:400-407`. Probe: `streaming.rs:108-112`, `tests/streaming.rs:52-58`.

## 15. Error handling

Features: none for the types; `router` for status mapping.

### 15.1 `topcoat::Result` and `topcoat::Error`

Status: VERIFIED.

```rust
pub type Result<T = (), E = topcoat::Error> = core::result::Result<T, E>;
#[derive(Debug, thiserror::Error)]
enum AppError { #[error(transparent)] Topcoat(#[from] topcoat::Error), #[error("wrapped: {0}")] Wrapped(#[source] topcoat::Error) }
let e: topcoat::Error = not_found().into();
e.downcast_ref::<NotFoundError>().is_some()      // true
let dyn_err: &(dyn std::error::Error + Send + Sync) = &*e;   // Deref to the stored error (new in 0.10)
```

- `topcoat::Error` is a cheap-to-clone handle (`Arc` inside; two words wide). Any `std::error::Error + Send + Sync + 'static` converts into it with `?`. `downcast_ref`, `downcast_cloned`, `backtrace()` are available; the backtrace is captured only when `RUST_BACKTRACE`/`RUST_LIB_BACKTRACE` enable it.
- **0.10 restored `thiserror` compatibility:** `#[error(transparent)]` forwards the message and `source()`, and `#[from]` with `#[error("...")]` keeps the topcoat error as the source (observed: `not found / None` and `wrapped: not found / Some("not found")`). `Deref<Target = dyn Error + Send + Sync>` is new.
- Feature `anyhow` adds `Error::from_anyhow`.
- `web` keeps its own domain errors (`AuthError`, `GuildError`, `EngagementError`, `AdminError`, all `thiserror` enums) and converts them at the edge: a redirect with `see_other`, a status in the view, or `Err(not_found().into())`.

Source: `topcoat-core-0.10.0/src/error.rs:9-122,272`, `topcoat-core-0.10.0/CHANGELOG.md` (0.10.0 fixes). Probe: `tests/errors.rs`. Web: `web/src/auth/error.rs:8-30`, `web/src/engagement/error.rs:97` (`redirect_unauthenticated`).

### 15.2 Router errors and their responses

Status: VERIFIED. Observed in `tests/routes.rs`.

| Constructor | Status | Body |
|---|---|---|
| `not_found()` | 404 | `not found` |
| `unauthorized()` | 401 | `unauthorized` |
| `forbidden()` | 403 | `forbidden` |
| `bad_request("msg")` | 400 | `bad request: msg` (message shown to the client) |
| `content_too_large()` | 413 | `content too large` |
| `method_not_allowed(methods)` | 405 | `Allow` header |
| `service_unavailable(secs)`, `too_many_requests(secs)` | 503, 429 | with `Retry-After` |
| `redirect`, `redirect_permanent`, `see_other` | 307, 308, 303 | `Location` (section 5.8) |
| `rewrite(path, body)` | handled again internally | section 5.8 |
| any other error, or a panic | 500 | `internal server error` (details are not sent) |

`RouterErrorExt` adds `ok_or_not_found()`, `ok_or_unauthorized()`, `ok_or_forbidden()`, `ok_or_bad_request(msg)`, `ok_or_redirect(uri)` and `ok_or_redirect_permanent(uri)` to `Option` and `Result`. Raise any of them with `?` from a page, route, layout or component. Errors bubble through `error_boundary` (section 3.4) and layouts before the router maps them; a layout can render a branded page for one type and re-throw the rest (section 5.7). `web` logs the cause (`tracing::warn!`) and answers a bare status from layers; pages map domain errors to redirects or statuses.

Source: `topcoat-router-0.10.0/src/error.rs:30-136`, `topcoat-router-0.10.0/src/error/*.rs`, `topcoat-router-0.10.0/docs/error.md`. Probe: `routes.rs:129-155`, `tests/routes.rs:61-94`. Web: `web/src/auth/middleware.rs:19-44`.

## 16. Test harness

Features: `router` (the harness is `Router::handle`; no extra feature). There is no `topcoat::test` module in 0.10. Tests build the real `Router` and call it with an in-memory request. No socket, no server. A panic inside a handler is turned into a `500`, so a failing page shows up as a status, not as a test panic.

Probe: `scratch/topcoat-kb/core/probe/tests/` (every file). Web: `web/tests/`.

### 16.1 Render a page through the router

Status: VERIFIED.

```rust
use http_body_util::BodyExt;
use topcoat::router::request::Request;
use topcoat::router::{Body, Router};
use topcoat::runtime::RouterBuilderRuntimeExt;

async fn render_raw(path: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let router = Router::builder().page(listed).runtime().build();
    let response = router.handle(Request::get(path).body(Body::empty())?).await;
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}
```

- `Request::get(path)`, `Request::post(path)` return `http::request::Builder` (`http` is not re-exported under that name; `topcoat::router::{Method, StatusCode, header}` are). Add `.header(header::COOKIE, "session=...")` and `.header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")`, then `.body(Body::empty())` or `.body(Body::from(text))`.
- `Response` has `.status()`, `.headers()`; `into_body().collect().await?.to_bytes()` (from `http-body-util`, a dev-dependency) reads the body, including streamed `suspense` output.
- Components and pages render without assets unless they render an `Asset` or `runtime::script()`. A page that needs `.runtime()` (any `$(...)`, `signal`, `link`) must call `.runtime()` on the test router.
- Pages in tests are small `#[page("/t/...")]` functions registered next to the component under test, which is how `web` renders a component with fixed data: `web/tests/components_markup.rs:132-146`, `web/tests/admin_pages_markup.rs:18-79`.
- Runtime bookkeeping markup (`<!--::topcoat::...-->` comments and `data-topcoat-*` attributes) is stripped before asserting on markup: `normalize` in `web/tests/shell_pages.rs:271-293`, `web/tests/admin_pages_markup.rs:83-95`.

Source: `topcoat-router-0.10.0/src/router.rs:73` (`Router::handle`), `topcoat-router-0.10.0/src/request.rs:19` (`Request<T = Body>`). Probe: `tests/common/mod.rs`, `tests/rendering.rs`. Web: `web/tests/admin_pages_markup.rs:62-95`, `web/tests/shell_pages.rs:181-250`.

### 16.2 Pages that render assets: a forged bundle

Status: VERIFIED.

```rust
fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir().join(format!("web-shell-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let assets = [
        ("tailwind-0123456789abcdef.css", "text/css", STYLESHEET.id().as_u64()),
        ("topcoat-0123456789abcdef.js", "text/javascript", topcoat::runtime::SCRIPT.id().as_u64()),
        ("pending-submit-0123456789abcdef.js", "text/javascript", PENDING_SUBMIT.id().as_u64()),
    ];
    let mut manifest = String::from("version = 1\n");
    for (file, content_type, id) in assets {
        std::fs::write(dir.join(file), "")?;
        write!(manifest, "\n[[assets]]\nid = {id}\nfile = \"{file}\"\nhash = \"0\"\ncontent_type = \"{content_type}\"\n")?;
    }
    std::fs::write(dir.join("manifest.toml"), manifest)?;
    Ok(dir)
}
let base = Router::builder().assets(AssetBundle::load_dir(bundle_dir()?)?).app_context(state).page(guild_probe);
let router = web::router(base);
```

Each asset the page can render needs an entry whose `id` is `<ASSET>.id().as_u64()`; the files can be empty. This avoids running `topcoat asset bundle` and Tailwind in tests. Without it the render panics and the test sees `500`. `web::router(base)` takes the builder, so a test supplies its own `app_context` and extra probe pages, then gets the production route table.

Source: `topcoat-asset-0.10.0/src/manifest.rs:8-40`, `topcoat-asset-0.10.0/src/bundle.rs:60`. Probe: `tests/assets.rs:8-24`. Web: `web/tests/shell_pages.rs:71-101,220-226`, `web/tests/public_pages.rs:24-48`, `web/tests/document_script.rs:17-43`, `web/tests/router.rs:34-66`.

### 16.3 Unit-level: `CxTestBuilder` and `single().render`

Status: VERIFIED. Render a view or call a `&Cx` helper without routing:

```rust
let cx = CxTestBuilder::new().app_context(Db("d")).request_context(7u8).build();
let html = view! { cx => components::badge(label: "x") }.single().await?.render(&cx);
// <span class="badge" data-max="80">x</span>
```

`status`, headers and cookies declared in the view are discarded by `render`; use a router when those matter.

Source: `topcoat-core-0.10.0/src/context.rs:198-240`, `topcoat-view-0.10.0/src/buffer/handle.rs:179`. Probe: `tests/harness.rs`. Web: `web/tests/auth_routes.rs:550-556`.

### 16.4 Panics at build time

Status: VERIFIED.

`Router::build()` panics on a duplicate `METHOD path` and on a layer path that matches no route (section 5.1); a test that asserts it uses `#[test] #[should_panic(expected = "duplicate route")]` (`tests/routes.rs:122-132`). A page that returns `Err(not_found())` checks the branded 404 (`web/tests/router.rs:23-30,143-155`).

Source: `topcoat-router-0.10.0/src/builder.rs:469,485`.
