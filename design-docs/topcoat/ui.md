Verified against: topcoat 0.10.0 (Cargo.lock), 2026-10-06

# Topcoat UI: registry components, theme, font and icons

Reference for the built-in `topcoat` registry (crate `topcoat-ui-registry` 0.10.0, enabled through topcoat's `ui` feature) and for the files `topcoat ui init` and `topcoat ui add --all` wrote into `web/`. CLI behavior is in [tooling.md](tooling.md); the core framework API is in [core.md](core.md).

Status labels are defined in [README.md](README.md). The probes under `scratch/topcoat-kb/ui/` compile every example below against the real registry sources and assert the server-rendered HTML. They do not run a browser: statements about focus, hover, popovers and CSS support follow the component sources and their comments, and are marked as such.

## Contents

- [What is installed in `web/`](#what-is-installed-in-web)
- [Conventions shared by every component](#conventions-shared-by-every-component)
- [Component index](#component-index)
- [Form controls](#form-controls), [overlays](#overlays), [navigation](#navigation), [data display](#data-display)
- [Theme: `styles.css`](#theme-stylescss)
- [Geist font](#geist-font)
- [Icons](#icons)
- [Using it in this app](#using-it-in-this-app)
- [Probe index](#probe-index)

## What is installed in `web/`

Status: VERIFIED (checked against the files in `web/`).

| File | Role |
|---|---|
| `web/components.toml` | Install state: format `version = 1`, `components_dir = "src/components"`, the theme (`neutral`, file `styles.css`) and a `[registries.topcoat.components.<name>]` table with the registry source hash and file for every component. 31 components are listed. |
| `web/styles.css` | The `neutral` theme, byte-identical to `themes/neutral.css` in the registry crate. It is a complete Tailwind input: `@import "tailwindcss"`, an `@source` for `./src/**/*.rs`, the dark variant, the `@theme inline` token map, the `:root` and `.dark` values and one `@layer base` rule. |
| `web/src/components/<name>.rs` | Verbatim copies of the registry sources (30 files) plus the app's own `select.rs`. `skeleton.rs` is the registry component; the app's skeleton is `shape_skeleton.rs`. |
| `web/src/components/mod.rs` | Declares only the app's own modules (`confirm`, `guild_grid`, `icons`, `key_list`, `legal`, `select`, `settings`, `shape_skeleton`). The registry modules are not declared, so none of the 30 registry files is compiled. |

`topcoat ui add` writes `pub mod <name>;` lines into `mod.rs` when it installs a file; the lines are absent here because they were removed after the install. Registry sources import each other with `super::`, so a component's dependencies must be declared in the same module as siblings under their registry names ([index](#component-index)).

Two names collided with app files:

| Name | State | Consequence |
|---|---|---|
| `select` | `web/src/components/select.rs` is the app's own file (channel, role and forum-tag selects, `SelectOption`, `select_field`). It differs from the registry file, but `components.toml` still records `select` as installed from the registry, with the registry hash and `file = "src/components/select.rs"`. | `topcoat ui remove select` deletes the app's file and its `pub mod select;` line. `topcoat ui add select --overwrite` and `topcoat ui add --all --overwrite` replace it with the registry source. Both were reproduced in a sandbox package. See [coexisting `select`](#registry-select-next-to-the-apps-select). |
| `skeleton` | The registry file is installed. The app's own skeleton is `shape_skeleton.rs`. | No collision. `sidebar` imports `super::skeleton::skeleton`, so the registry file must keep this module name. |

`topcoat ui list` reports every component as `(installed)`, including `select`, because it compares the recorded hash with the registry's current hash and never reads the file on disk. Local edits are invisible to it.

## Conventions shared by every component

Status: VERIFIED.

Verified in all 31 registry modules and in `topcoat-ui-0.10.0/src/manage/add.rs` (install behavior).

| Convention | Behavior |
|---|---|
| Declaration | `#[component] pub async fn name(props) -> Result<impl View>`. Props are passed as `prop: value`, children follow positionally: `button(variant: ButtonVariant::Ghost, "Cancel")`. |
| `#[default]` | The prop is optional and takes `Default::default()`. `#[default(expr)]` supplies another default (shown as `= expr` in the signatures below). A prop without `#[default]` is required. |
| `#[into]` | The prop accepts anything that is `Into<T>` (a `&str` for `String`, a `bool` for `Expr<bool>`, a `f32` for `Option<f32>`). |
| `attrs: Attributes` | Extra attributes, built with `attributes! { ... }`, forwarded to the main element. Each entry below says which element receives them. |
| `class` in `attrs` | The component takes it out with `attrs.remove("class")` and appends it after its own classes. Conflicting utilities are not resolved: the one that appears later in the generated stylesheet wins, not the later class in the attribute. Use a more specific selector (`[&]:max-w-xl`) or edit the component. |
| `child: Child` | Children are text, markup or other components. |
| `Expr<bool>` props | `open` (dialog, alert_dialog, sheet, sidebar, sidebar_trigger, sidebar_rail) and `mobile_open` (sidebar) and `active` (tabs_trigger, sidebar_menu_button, sidebar_menu_sub_button). A plain `bool` renders a static attribute. A runtime expression (`expr!(open.get())`) renders a bound attribute that the browser runtime updates. See [runtime requirements](#runtime-requirements). |
| Enum props | 16 `Copy` enums, listed per component. Every enum has a `#[default]` variant. |
| Class helpers | `button_variants(variant, size)`, `badge_variants(variant)` and `sidebar_menu_button_variants(variant, size)` return a class list for a different element, such as a link. |
| Hooks | `field` and `field_label` emit `data-slot="field"` and `data-slot="field-label"`. Every `sidebar_*` component emits `data-sidebar="..."`. Other components emit no `data-*` hooks; state is read from native attributes (`[open]`, `:checked`, `:disabled`, `[aria-invalid]`, `[aria-current]`). |
| No `type` on buttons | `button` and `dropdown_menu_item` render `<button>` without `type`, which submits a surrounding `<form>`. Pass `type="button"` through `attrs`. The sidebar buttons set `type="button"` themselves. |
| Styling | Tailwind utility classes written literally in the source, and theme tokens only (`bg-primary`, `text-muted-foreground`, `border-border`, `ring-ring`, `bg-sidebar`). No raw colors, no `dark:` variants. Tailwind must scan the component files: `styles.css` does it with `@source "./src/**/*.rs"`. |

### Runtime requirements

- `dialog`, `alert_dialog`, `sheet`, `tabs`, `sidebar` import `topcoat::runtime::Expr`, so they need topcoat's `runtime` feature. In 0.10.0, `runtime` also needs `asset`: building topcoat with `features = ["runtime", "view"]` fails with `Asset: AttributeValueViewParts is not satisfied`. `web` already enables `asset`, `router`, `runtime` and `view`.
- With a literal `bool`, no JavaScript is involved (`open: true` renders `open=""`, `open: false` renders nothing).
- With `expr!(signal.get())` the element carries `data-topcoat-bind:<attr>="..."`; event handlers written as `@click=$(|_e: Event| ...)` render `data-topcoat-on:click="..."`. These need `topcoat::runtime::script()` in `<head>` and `.runtime()` on the router builder, which `web/src/document.rs` and `web/src/router.rs` already do.
- Inside `attributes! { }`, closure parameters need an explicit type (`|_e: Event|`); in a plain `<button @click=$(|_e| ...)>` they are inferred.

### Install behavior (`topcoat ui`)

| Action | Result |
|---|---|
| `add <name>` | Writes `<components_dir>/<file>.rs`, appends `pub mod <file>;` to `mod.rs` (or `<dir>.rs`), installs dependencies, records source hash and file in `components.toml`. An existing file is an error without `--overwrite`. |
| `add --all` | Installs everything missing. Files that exist are skipped and reported, but the component is still recorded in `components.toml`. |
| `add ... --overwrite` | Replaces named (or all swept) files, including local edits. Dependencies pulled in implicitly are never overwritten. |
| `remove <name>` | Deletes the recorded file, removes its `pub mod` line, drops the record. Dependencies stay. |
| `list` | `Available`, `(installed)`, `(update available)` (registry hash differs from the recorded one) or `(orphaned)`. Never reads local files. |

## Component index

Status: VERIFIED (dependencies, icons, features and component counts checked against `registry.toml` and the sources).

| Module | Kind | Components | Registry dependencies | Iconify icons | Topcoat features |
|---|---|---|---|---|---|
| [`button`](#button) | form | `button` | - | - | `view` |
| [`input`](#input) | form | `input` | - | - | `view` |
| [`textarea`](#textarea) | form | `textarea` | - | - | `view` |
| [`label`](#label) | form | `label` | - | - | `view` |
| [`field`](#field) | form | 10 components, see entry | label | - | `view` |
| [`checkbox`](#checkbox) | form | `checkbox` | - | `lucide:check` | `view`, `icon-iconify` |
| [`switch`](#switch) | form | `switch` | - | - | `view` |
| [`radio_group`](#radio_group) | form | `radio_group`, `radio_group_item` | - | - | `view` |
| [`toggle`](#toggle) | form | `toggle`, `toggle_group` | - | - | `view` |
| [`select`](#select) | form | `select` | - | `lucide:check`, `lucide:chevron-down` | `view`, `icon-iconify` |
| [`dialog`](#dialog) | overlay | `dialog`, `dialog_content`, `dialog_header`, `dialog_title`, `dialog_description`, `dialog_footer` | - | - | `view`, `runtime`, `asset` |
| [`alert_dialog`](#alert_dialog) | overlay | `alert_dialog` | dialog | - | `view`, `runtime`, `asset` |
| [`sheet`](#sheet) | overlay | `sheet`, `sheet_content` | dialog | - | `view`, `runtime`, `asset` |
| [`dropdown_menu`](#dropdown_menu) | overlay | 9 components, see entry | - | `lucide:chevron-right` | `view`, `icon-iconify` |
| [`tooltip`](#tooltip) | overlay | `tooltip`, `tooltip_content` | - | - | `view` |
| [`hover_card`](#hover_card) | overlay | `hover_card`, `hover_card_content` | - | - | `view` |
| [`breadcrumb`](#breadcrumb) | navigation | 7 components, see entry | - | `lucide:chevron-right`, `lucide:ellipsis` | `view`, `icon-iconify` |
| [`pagination`](#pagination) | navigation | 7 components, see entry | button | `lucide:chevron-left`, `lucide:chevron-right`, `lucide:ellipsis` | `view`, `icon-iconify` |
| [`tabs`](#tabs) | navigation | `tabs`, `tabs_list`, `tabs_trigger`, `tabs_content` | - | - | `view`, `runtime`, `asset` |
| [`accordion`](#accordion) | navigation | `accordion`, `accordion_item`, `accordion_trigger`, `accordion_content` | - | `lucide:chevron-down` | `view`, `icon-iconify` |
| [`sidebar`](#sidebar) | navigation | 23 components, see entry | button, input, separator, sheet, skeleton | `lucide:panel-left` | `view`, `runtime`, `asset`, `icon-iconify` |
| [`table`](#table) | data | 8 components, see entry | - | - | `view` |
| [`card`](#card) | data | `card`, `card_header`, `card_title`, `card_description`, `card_content`, `card_footer` | - | - | `view` |
| [`badge`](#badge) | data | `badge` | - | - | `view` |
| [`avatar`](#avatar) | data | `avatar`, `avatar_image`, `avatar_fallback` | - | - | `view` |
| [`alert`](#alert) | data | `alert`, `alert_title`, `alert_description` | - | - | `view` |
| [`progress`](#progress) | data | `progress` | - | - | `view` |
| [`skeleton`](#skeleton) | data | `skeleton` | - | - | `view` |
| [`spinner`](#spinner) | data | `spinner` | - | `lucide:loader-circle` | `view`, `icon-iconify` |
| [`kbd`](#kbd) | data | `kbd`, `kbd_group` | - | - | `view` |
| [`separator`](#separator) | data | `separator` | - | - | `view` |

Features are topcoat features on the runtime dependency. `icon-iconify` is needed only while the registry source keeps its `iconify_icon!` calls; see [Icons](#icons). All 31 modules compile together in `scratch/topcoat-kb/ui` (`cargo test`); `select.rs` is the registry file, not the app's.

Every example below is the body of `scratch/topcoat-kb/ui/tests/examples.rs`, compiled and rendered by the probe `examples_render`. `crate::components` is the module the registry files are declared in (the probe crate calls it `kb_ui::components`).

## Form controls

### button

A styled `<button>`, plus a class helper for links.

Status: VERIFIED
Features: `view`

```rust
use crate::components::button::{ButtonSize, ButtonVariant, button, button_variants};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/button")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        button(
            variant: ButtonVariant::Destructive,
            size: ButtonSize::Sm,
            attrs: attributes! { type="submit" },
            "Delete"
        )
        <a href="/login" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
            "Sign in"
        </a>
    })
}
```

```text
button(variant: ButtonVariant = ButtonVariant::Primary, size: ButtonSize = ButtonSize::Md, attrs, child)  // :129
```

- Variants: `ButtonVariant`: Primary (default), Secondary, Outline, Ghost, Destructive. Sizes: `ButtonSize`: Sm, Md (default), Lg, Icon (`Sm` h-8, `Md` h-9, `Lg` h-10, `Icon` size-9).
- Markup: `<button class="BASE VARIANT SIZE [class]" ...attrs>child</button>`. `attrs` go on the `<button>`. `button_variants(variant, size)` returns the same three class groups. Disabled state: `disabled:pointer-events-none disabled:opacity-50`. Focus: `focus-visible:ring-2 ring-ring ring-offset-2 ring-offset-background`.
- Client behavior: none.
- A11y: native button semantics. An icon-only button needs `label` on the icon or `aria-label`.
- Dependencies: none. Used by `pagination`, `sidebar`.
- Icons: none.

Source: `topcoat-ui-registry-0.10.0/src/components/button.rs:129`
Probe: `scratch/topcoat-kb/ui/tests/form_controls.rs` (`button_page`), `tests/examples.rs` (`ex_button`)

### input

A styled `<input>` that fills its container.

Status: VERIFIED
Features: `view`

```rust
use crate::components::input::input;
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/input")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        input(attrs: attributes! { type="email" placeholder="you@example.com" aria-invalid="true" })
    })
}
```

```text
input(attrs)  // :29
```

- Markup: `<input class="INPUT [class]" ...attrs>`. `type` is whatever `attrs` set (browser default is text). `aria-invalid="true"` switches the border and ring to `destructive`. Also styles the placeholder text, the file-input button and the disabled state.
- `w-full` is part of `INPUT`; a `w-64` in `attrs` is appended and does not replace it.
- Client behavior: none. A11y: native; connect labels with `for`/`id`.
- Dependencies: none. Used by `sidebar` (`sidebar_input`).

Source: `topcoat-ui-registry-0.10.0/src/components/input.rs:29`
Probe: `tests/form_controls.rs` (`input_page`), `tests/examples.rs` (`ex_input`)

### textarea

A styled `<textarea>` that grows with its content where `field-sizing: content` is supported.

Status: VERIFIED
Features: `view`

```rust
use crate::components::textarea::textarea;
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/textarea")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        textarea(attrs: attributes! { name="bio" placeholder="Tell us more" }, "Initial text")
    })
}
```

```text
textarea(attrs, child)  // :30
```

- Markup: `<textarea class="TEXTAREA [class]" ...attrs>child</textarea>`; children are the initial value. `min-h-16`. Same `aria-invalid:` and `disabled:` styling as `input`.
- Client behavior: none. A11y: native.

Source: `topcoat-ui-registry-0.10.0/src/components/textarea.rs:30`
Probe: `tests/form_controls.rs` (`input_page`), `tests/examples.rs` (`ex_textarea`)

### label

A `<label>` that dims itself when the control it labels is disabled.

Status: VERIFIED
Features: `view`

```rust
use crate::components::{input::input, label::label};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/label")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col gap-2">
            label(attrs: attributes! { for="email" }, "Email")
            input(attrs: attributes! { id="email" type="email" })
        </div>
    })
}
```

```text
label(attrs, child)  // :31
```

- Markup: `<label class="flex items-center gap-2 text-sm leading-none font-medium select-none peer-disabled:... has-[:disabled]:... [class]" ...attrs>child</label>`. Wrap the control or set `for`.
- Disabled styling reacts to `peer-disabled`, `peer-has-[:disabled]`, `has-[+:disabled]` and `has-[:disabled]`.
- Client behavior: none. A11y: native label association.
- Dependencies: none. Used by `field` (`field_label`).

Source: `topcoat-ui-registry-0.10.0/src/components/label.rs:31`
Probe: `tests/form_controls.rs` (`check_page`), `tests/examples.rs` (`ex_label`)

### field

Layout and state for a control with its label, description and error.

Status: VERIFIED
Features: `view`

```rust
use crate::components::{
    field::{
        FieldOrientation, field, field_content, field_description, field_error, field_group,
        field_label, field_legend, field_set,
    },
    input::input,
    switch::switch,
};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/field")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        field_set(
            field_legend("Account")
            field_group(
                field(
                    field_label(attrs: attributes! { for="name" }, "Name")
                    input(attrs: attributes! { id="name" aria-invalid="true" aria-describedby="name-err" })
                    field_description(attrs: attributes! { id="name-desc" }, "Shown on your profile.")
                    field_error(attrs: attributes! { id="name-err" }, "Required.")
                )
                field(
                    orientation: FieldOrientation::Horizontal,
                    field_content(field_label(attrs: attributes! { for="mail" }, "Emails"))
                    switch(attrs: attributes! { id="mail" })
                )
            )
        )
    })
}
```

```text
field_set(attrs, child)  // :63
field_legend(variant: FieldLegendVariant = FieldLegendVariant::Legend, attrs, child)  // :79
field_group(attrs, child)  // :96
field(orientation: FieldOrientation = FieldOrientation::Vertical, attrs, child)  // :123
field_content(attrs, child)  // :146
field_label(attrs, child)  // :164
field_title(attrs, child)  // :187
field_description(attrs, child)  // :203
field_separator(attrs, child)  // :222
field_error(attrs, child)  // :245
```

- Orientation: `FieldOrientation`: Vertical (default), Horizontal, Responsive. `Responsive` switches to a row at the `@md/field-group` container width, so it needs a `field_group` ancestor. Legend: `FieldLegendVariant`: Legend (default), Label.
- Markup: `field_set` `<fieldset class="flex min-w-0 flex-col gap-5">`, `field_legend` `<legend>`, `field_group` `<div class="@container/field-group flex flex-col gap-5">`, `field` `<div role="group" data-slot="field" class="group/field flex min-w-0 ...">`, `field_content` `<div>`, `field_label` a `label` with `data-slot="field-label"`, `field_title` `<div>`, `field_description` `<p>`, `field_separator` `<div>` with an optional `<span>`, `field_error` `<div role="alert">`. `attrs` go on the element named.
- State hooks: the label turns `text-destructive` when the field contains `[aria-invalid=true]` or carries `data-invalid="true"`, and dims when the field contains a `:disabled` control. Validation and showing the error are the caller's job.
- Client behavior: none. A11y: `role="group"`, `role="alert"` on the error (announced when it is inserted), `aria-describedby` and `aria-invalid` are set by the caller.
- Dependencies: `label`.

Source: `topcoat-ui-registry-0.10.0/src/components/field.rs:63`
Probe: `tests/form_controls.rs` (`field_page`), `tests/examples.rs` (`ex_field`)

### checkbox

A native checkbox drawn with CSS and an overlaid check icon.

Status: VERIFIED
Features: `view`, `icon-iconify` (check)

```rust
use crate::components::{checkbox::checkbox, label::label};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/checkbox")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        <div class="flex items-center gap-2">
            checkbox(attrs: attributes! { id="terms" name="terms" checked="" })
            label(attrs: attributes! { for="terms" }, "Accept terms")
        </div>
    })
}
```

```text
checkbox(attrs)  // :32
```

- Markup: `<span class="peer relative inline-flex shrink-0 has-[:disabled]:opacity-50 [class]"><input type="checkbox" class="peer size-4 appearance-none ..." ...attrs><svg class="... opacity-0 peer-checked:opacity-100"></svg></span>`. `class` goes on the wrapper `<span>`; every other attribute goes on the `<input>`. `checked=""` sets the initial state. The indeterminate state has no styling.
- Client behavior: none. A11y: native checkbox; the icon is `aria-hidden`.
- Icons: `lucide:check`.

Source: `topcoat-ui-registry-0.10.0/src/components/checkbox.rs:32`
Probe: `tests/form_controls.rs` (`check_page`), `tests/examples.rs` (`ex_checkbox`)

### switch

An on/off control built on a native checkbox.

Status: VERIFIED
Features: `view`

```rust
use crate::components::{label::label, switch::switch};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/switch")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        <div class="flex items-center gap-2">
            switch(attrs: attributes! { id="airplane" checked="" })
            label(attrs: attributes! { for="airplane" }, "Airplane mode")
        </div>
    })
}
```

```text
switch(attrs)  // :36
```

- Markup: `<span class="peer relative inline-flex ... [class]"><input type="checkbox" role="switch" class="peer h-4.5 w-8 appearance-none ..." ...attrs><span class="... peer-checked:translate-x-3.5"></span></span>`. `class` goes on the wrapper, other attributes on the `<input>`.
- Client behavior: none. A11y: `role="switch"` on the native checkbox.
- Dependencies: none.

Source: `topcoat-ui-registry-0.10.0/src/components/switch.rs:36`
Probe: `tests/form_controls.rs` (`check_page`), `tests/examples.rs` (`ex_switch`)

### radio_group

A group of native radio inputs.

Status: VERIFIED
Features: `view`

```rust
use crate::components::radio_group::{radio_group, radio_group_item};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/radio_group")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        radio_group(
            attrs: attributes! { aria-label="Billing" },
            radio_group_item(attrs: attributes! { name="billing" value="weekly" checked="" })
            radio_group_item(attrs: attributes! { name="billing" value="monthly" })
        )
    })
}
```

```text
radio_group(attrs, child)  // :28
radio_group_item(attrs)  // :62
```

- Markup: `radio_group` `<div role="radiogroup" class="grid gap-3 [class]" ...attrs>`; `radio_group_item` `<span class="peer relative ..."><input type="radio" class="peer size-4 appearance-none ..." ...attrs><span class="... peer-checked:opacity-100"></span></span>`. Give every item the same `name`. For `radio_group_item`, `class` goes on the wrapper, other attributes on the `<input>`.
- Client behavior: none; native radio grouping gives arrow-key movement. A11y: name the group with `aria-label` or `aria-labelledby` in `attrs`.

Source: `topcoat-ui-registry-0.10.0/src/components/radio_group.rs:28`
Probe: `tests/form_controls.rs` (`check_page`), `tests/examples.rs` (`ex_radio_group`)

### toggle

A pressable control backed by a hidden checkbox or radio input.

Status: VERIFIED
Features: `view`

```rust
use crate::components::toggle::{ToggleKind, ToggleSize, toggle, toggle_group};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/toggle")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        toggle_group(
            toggle(
                kind: ToggleKind::Exclusive,
                size: ToggleSize::Sm,
                attrs: attributes! { name="range" value="day" checked="" },
                "Day"
            )
            toggle(
                kind: ToggleKind::Exclusive,
                size: ToggleSize::Sm,
                attrs: attributes! { name="range" value="week" },
                "Week"
            )
        )
    })
}
```

```text
toggle(kind: ToggleKind = ToggleKind::Independent, size: ToggleSize = ToggleSize::Md, attrs, child)  // :84
toggle_group(attrs, child)  // :130
```

- Kind: `ToggleKind`: Independent (default), Exclusive. Size: `ToggleSize`: Sm, Md (default), Lg.
- Markup: `toggle` `<label class="... has-[:checked]:bg-foreground/10 ... [class]"><input type="checkbox|radio" class="sr-only" ...attrs>child</label>`; `toggle_group` `<div class="inline-flex w-fit items-center gap-1 rounded-lg border border-border p-1" ...attrs>`. `class` goes on the `<label>`, other attributes on the `<input>`. Exclusive toggles share a `name`.
- Client behavior: none. A11y: the `sr-only` input keeps keyboard focus and is announced as a checkbox or radio named by the label; `toggle_group` has no role, add `role="group"` through `attrs` when the toggles are related.

Source: `topcoat-ui-registry-0.10.0/src/components/toggle.rs:84`
Probe: `tests/form_controls.rs` (`toggle_page`), `tests/examples.rs` (`ex_toggle`)

### select

A styled native `<select>` with an optional customizable picker.

Status: VERIFIED
Features: `view`, `icon-iconify` (check, chevron-down)

```rust
use crate::components::select::select;
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/select")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        select(
            attrs: attributes! { name="region" class="w-48" },
            <option>"eu-central-1"</option>
            <option>"us-east-1"</option>
        )
    })
}
```

```text
select(cx: &Cx, attrs, child)  // :88
```

- Signature note: `cx: &Cx` is injected by `#[component]`; callers do not pass it.
- Markup: `<span class="relative block ... [&>select]:[appearance:base-select] [class]" style="--select-checkmark: url(&quot;data:image/svg+xml,...&quot;)"><select class="SELECT PICKER" ...attrs>children</select><svg chevron-down ...></svg></span>`. `class` goes on the wrapper, other attributes on the `<select>`. The check icon is rendered to a data URI in an inline `style` custom property so CSS can use it as a mask.
- Browsers with customizable select support (`appearance: base-select`) style the picker, option rows and `<optgroup><legend>`; other browsers use the native picker. This is from the source comments, not from a browser run.
- Client behavior: none. A11y: native select; `aria-invalid="true"` styles the error state.
- Registry `select` is not the app's `select`: see [coexisting `select`](#registry-select-next-to-the-apps-select).

Source: `topcoat-ui-registry-0.10.0/src/components/select.rs:88`
Probe: `tests/form_controls.rs` (`select_page`), `tests/examples.rs` (`ex_select`)

## Overlays

All overlays are CSS or native elements. None of them sets focus, traps focus, closes on Escape or closes on an outside click; each entry lists what the application must add.

### dialog

A panel over the page, driven by an `open` expression.

Status: VERIFIED
Features: `view`, `runtime`, `asset`

```rust
use crate::components::{
    button::{ButtonVariant, button},
    dialog::{
        dialog, dialog_content, dialog_description, dialog_footer, dialog_header,
        dialog_title,
    },
};
use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{Event, expr, signal},
    view::{View, attributes, view},
};

#[page("/ex/dialog")]
async fn demo(cx: &Cx) -> Result<impl View> {
    let open = signal(cx, || false);

    Ok(view! {
        button(attrs: attributes! { @click=$(|_e: Event| open.set(true)) }, "Delete workspace")
        dialog(
            open: expr!(open.get()),
            attrs: attributes! {
                aria-labelledby="dlg-title"
                @keydown=$(|e: Event| if e.key == "Escape" { open.set(false) })
            },
            dialog_content(
                dialog_header(
                    dialog_title(attrs: attributes! { id="dlg-title" }, "Delete workspace")
                    dialog_description("This cannot be undone.")
                )
                dialog_footer(
                    button(
                        variant: ButtonVariant::Ghost,
                        attrs: attributes! { @click=$(|_e: Event| open.set(false)) },
                        "Cancel"
                    )
                    button(variant: ButtonVariant::Destructive, "Delete")
                )
            )
        )
    })
}
```

```text
dialog(open: Expr<bool>, attrs, child)  // :61
dialog_content(attrs, child)  // :102
dialog_header(attrs, child)  // :116
dialog_title(attrs, child)  // :129
dialog_description(attrs, child)  // :145
dialog_footer(attrs, child)  // :162
```

- Markup: `<dialog class="fixed inset-0 z-50 size-full ... bg-background/80 backdrop-blur-sm open:flex opacity-0 open:opacity-100 ... [class]" ...attrs>` with `open=""` when open. `dialog_content` `<div class="relative my-auto flex w-full max-w-lg flex-col gap-4 rounded-xl border border-border bg-card p-6 ... scale-95 opacity-0 in-[[open]]:scale-100 ...">`, `dialog_header` `<div>`, `dialog_title` `<h2>`, `dialog_description` `<p>`, `dialog_footer` `<div>`. `attrs` go on the `<dialog>` (or the `<div>` for the content parts). With a signal, `<dialog>` carries `data-topcoat-bind:open`.
- It is not a modal dialog: the component sets the `open` attribute and never calls `showModal()`. There is no top layer, no `::backdrop`, no inert page and no `aria-modal`. The fixed full-viewport overlay blocks pointer clicks on the page behind it.
- Application scripting required: Escape (the example's `@keydown` on the `<dialog>` works while focus is inside it), moving focus into the dialog on open and back on close, trapping Tab, scroll lock, clicking the overlay to close, and `aria-modal="true"` if wanted. Closing can also be a navigation to a page that renders it closed.
- A11y: name it with `aria-label` or `aria-labelledby` (the title's `id`).
- Dependencies: none. Imported by `alert_dialog`; listed as a dependency of `sheet` in `registry.toml` (the sheet source only links to it in a doc comment).

Source: `topcoat-ui-registry-0.10.0/src/components/dialog.rs:61`
Probe: `tests/overlays.rs` (`dialog_closed`, `dialog_open`, `dialog_signal`), `tests/examples.rs` (`ex_dialog`)

### alert_dialog

A `dialog` with `role="alertdialog"`.

Status: VERIFIED
Features: `view`, `runtime`, `asset`

```rust
use crate::components::{
    alert_dialog::alert_dialog,
    dialog::{dialog_content, dialog_description, dialog_header, dialog_title},
};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/alert_dialog")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        alert_dialog(
            open: true,
            attrs: attributes! { aria-labelledby="t" aria-describedby="d" },
            dialog_content(
                dialog_header(
                    dialog_title(attrs: attributes! { id="t" }, "Delete?")
                    dialog_description(attrs: attributes! { id="d" }, "Gone for good.")
                )
            )
        )
    })
}
```

```text
alert_dialog(open: Expr<bool>, attrs, child)  // :39
```

- Markup: identical to `dialog`, with `role="alertdialog"` merged into the `<dialog>` attributes. Use `dialog_content`, `dialog_header`, `dialog_title`, `dialog_description` and `dialog_footer` for the body.
- Scripting and a11y: as `dialog`; set `aria-labelledby` and `aria-describedby` to the title and description ids.
- Dependencies: `dialog`.

Source: `topcoat-ui-registry-0.10.0/src/components/alert_dialog.rs:39`
Probe: `tests/overlays.rs` (`alert_dialog_page`), `tests/examples.rs` (`ex_alert_dialog`)

### sheet

A panel that slides in from an edge.

Status: VERIFIED
Features: `view`, `runtime`, `asset`

```rust
use crate::components::{
    dialog::{dialog_header, dialog_title},
    sheet::{SheetSide, sheet, sheet_content},
};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/sheet")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        sheet(
            open: true,
            attrs: attributes! { aria-label="Filters" },
            sheet_content(side: SheetSide::Left, dialog_header(dialog_title("Filters")))
        )
    })
}
```

```text
sheet(open: Expr<bool>, attrs, child)  // :46
sheet_content(side: SheetSide = SheetSide::Right, attrs, child)  // :131
```

- Side: `SheetSide`: Left, Right (default), Top, Bottom. Panel classes: `Left` `mr-auto h-full w-full max-w-sm border-r`, `Right` `ml-auto h-full w-full max-w-sm border-l`, `Top` `mb-auto max-h-full w-full border-b`, `Bottom` `mt-auto max-h-full w-full border-t`, each with a matching translate transition.
- Markup: `sheet` renders the same `<dialog>` overlay as `dialog` without padding; `sheet_content` `<div class="flex flex-col gap-4 overflow-y-auto border-border bg-card p-6 ...">`. Use `dialog_header`, `dialog_title` and the other dialog parts inside.
- Scripting and a11y: as `dialog`.
- Dependencies: `dialog` is listed in `registry.toml` (so `topcoat ui add sheet` installs it); the source does not import it. Imported by `sidebar`.

Source: `topcoat-ui-registry-0.10.0/src/components/sheet.rs:46`
Probe: `tests/overlays.rs` (`sheet_page`), `tests/examples.rs` (`ex_sheet`)

### dropdown_menu

A floating action menu on a native `<details>`.

Status: VERIFIED
Features: `view`, `icon-iconify` (chevron-right)

```rust
use crate::components::{
    button::{ButtonSize, ButtonVariant, button_variants},
    dropdown_menu::{
        dropdown_menu, dropdown_menu_content, dropdown_menu_item, dropdown_menu_label,
        dropdown_menu_separator, dropdown_menu_sub, dropdown_menu_sub_content,
        dropdown_menu_sub_trigger, dropdown_menu_trigger,
    },
};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/dropdown_menu")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        dropdown_menu(
            dropdown_menu_trigger(
                attrs: attributes! { class=(button_variants(ButtonVariant::Outline, ButtonSize::Md)) },
                "Options"
            )
            dropdown_menu_content(
                dropdown_menu_label("Actions")
                dropdown_menu_item(attrs: attributes! { type="button" }, "Rename")
                dropdown_menu_separator()
                dropdown_menu_sub(
                    dropdown_menu_sub_trigger("Move to")
                    dropdown_menu_sub_content(dropdown_menu_item("Inbox"))
                )
            )
        )
    })
}
```

```text
dropdown_menu(attrs, child)  // :32
dropdown_menu_trigger(attrs, child)  // :71
dropdown_menu_content(attrs, child)  // :92
dropdown_menu_item(attrs, child)  // :115
dropdown_menu_sub(attrs, child)  // :148
dropdown_menu_sub_trigger(attrs, child)  // :165
dropdown_menu_sub_content(attrs, child)  // :190
dropdown_menu_label(attrs, child)  // :206
dropdown_menu_separator(attrs)  // :225
```

- Markup: `dropdown_menu` `<details class="group relative inline-block">`, `dropdown_menu_trigger` `<summary>`, `dropdown_menu_content` `<div class="absolute z-50 min-w-40 ... top-full left-0 mt-1">`, `dropdown_menu_item` `<button>`, `dropdown_menu_sub` `<details class="group/sub relative">`, `dropdown_menu_sub_trigger` `<summary>` plus a chevron, `dropdown_menu_sub_content` `<div class="... top-0 left-full ml-1">`, `dropdown_menu_label` `<p>`, `dropdown_menu_separator` `<hr>`. `attrs` go on the element named. Style the trigger as a button with `class=(button_variants(...))` in its `attrs`; `group-open:` classes react to the open state.
- Client behavior: the trigger opens and closes it without JavaScript. Application scripting required for closing on an outside click or Escape and for closing after an item is chosen; a submenu keeps its open state when its parent closes. Items are `<button>` elements without `type`.
- A11y: tab navigation only. No `role="menu"`, `aria-haspopup` or arrow-key handling.
- Icons: `lucide:chevron-right` (sub trigger).

Source: `topcoat-ui-registry-0.10.0/src/components/dropdown_menu.rs:32`
Probe: `tests/overlays.rs` (`dropdown_page`), `tests/examples.rs` (`ex_dropdown_menu`)

### tooltip

A short hint shown on hover or keyboard focus.

Status: VERIFIED
Features: `view`

```rust
use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    tooltip::{tooltip, tooltip_content},
};
use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    router::page,
    view::{View, attributes, view},
};

#[page("/ex/tooltip")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        tooltip(
            button(
                size: ButtonSize::Icon,
                variant: ButtonVariant::Outline,
                attrs: attributes! { aria-describedby="tip" },
                icon(data: iconify_icon!("lucide:copy"), label: "Copy link")
            )
            tooltip_content(attrs: attributes! { id="tip" }, "Copy link")
        )
    })
}
```

```text
tooltip(attrs, child)  // :28
tooltip_content(attrs, child)  // :55
```

- Markup: `tooltip` `<span class="group relative inline-flex">`; `tooltip_content` `<span role="tooltip" class="pointer-events-none invisible absolute bottom-full left-1/2 ... group-hover:visible group-hover:opacity-100 group-focus-within:visible group-focus-within:opacity-100">`. The bubble is always in the markup; CSS shows it.
- Client behavior: none (CSS only). Positioning does not adapt to viewport edges.
- A11y: `role="tooltip"`. Give the content an `id` and reference it with the trigger's `aria-describedby`; the trigger needs its own text or label.

Source: `topcoat-ui-registry-0.10.0/src/components/tooltip.rs:28`
Probe: `tests/overlays.rs` (`tooltip_page`), `tests/examples.rs` (`ex_tooltip`)

### hover_card

Extra content shown on hover or focus, with a short open delay.

Status: VERIFIED
Features: `view`

```rust
use crate::components::hover_card::{hover_card, hover_card_content};
use topcoat::{Result, router::page, view::{View, view}};

#[page("/ex/hover_card")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        hover_card(
            <a href="/people/ada" class="font-medium underline">"@ada"</a>
            hover_card_content(<span class="text-sm">"Ada Lovelace"</span>)
        )
    })
}
```

```text
hover_card(attrs, child)  // :24
hover_card_content(attrs, child)  // :50
```

- Markup: `hover_card` `<span class="group relative inline-flex">`; `hover_card_content` `<span class="flex flex-col gap-2 invisible absolute top-full left-0 z-50 mt-2 w-64 ... opacity-0 [transition:...300ms,...300ms] group-hover:visible group-focus-within:visible">`. Both are `<span>`, so only phrasing content is valid inside the card.
- Client behavior: none (CSS only). The panel is `visibility: hidden` until shown, so it is out of the accessibility tree and unfocusable until the trigger is hovered or holds focus. Keep essential information elsewhere; touch users may not see it.

Source: `topcoat-ui-registry-0.10.0/src/components/hover_card.rs:24`
Probe: `tests/overlays.rs` (`tooltip_page`), `tests/examples.rs` (`ex_hover_card`)

## Navigation

### breadcrumb

The path to the current page.

Status: VERIFIED
Features: `view`, `icon-iconify` (chevron-right, ellipsis)

```rust
use crate::components::breadcrumb::{
    breadcrumb, breadcrumb_ellipsis, breadcrumb_item, breadcrumb_link, breadcrumb_list,
    breadcrumb_page, breadcrumb_separator,
};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/breadcrumb")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        breadcrumb(
            breadcrumb_list(
                breadcrumb_item(breadcrumb_link(attrs: attributes! { href="/" }, "Home"))
                breadcrumb_separator()
                breadcrumb_item(breadcrumb_ellipsis())
                breadcrumb_separator()
                breadcrumb_item(breadcrumb_page("Settings"))
            )
        )
    })
}
```

```text
breadcrumb(attrs, child)  // :26
breadcrumb_list(attrs, child)  // :40
breadcrumb_item(attrs, child)  // :60
breadcrumb_link(attrs, child)  // :76
breadcrumb_page(attrs, child)  // :97
breadcrumb_separator(attrs)  // :114
breadcrumb_ellipsis(attrs)  // :127
```

- Markup: `breadcrumb` `<nav aria-label="breadcrumb">`, `breadcrumb_list` `<ol class="flex flex-wrap items-center gap-2 text-sm text-muted-foreground">`, `breadcrumb_item` `<li>`, `breadcrumb_link` `<a>` (destination in `attrs`), `breadcrumb_page` `<span aria-current="page">`, `breadcrumb_separator` `<li aria-hidden="true">` with a chevron, `breadcrumb_ellipsis` `<span>` with an ellipsis icon and `<span class="sr-only">More</span>`.
- Client behavior: none. A11y as above.
- Icons: `lucide:chevron-right`, `lucide:ellipsis`.

Source: `topcoat-ui-registry-0.10.0/src/components/breadcrumb.rs:26`
Probe: `tests/navigation.rs` (`breadcrumb_page_route`), `tests/examples.rs` (`ex_breadcrumb`)

### pagination

Links for a list split across pages.

Status: VERIFIED
Features: `view`, `icon-iconify` (chevron-left, chevron-right, ellipsis)

```rust
use crate::components::pagination::{
    pagination, pagination_content, pagination_ellipsis, pagination_item,
    pagination_link, pagination_next, pagination_previous,
};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/pagination")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        pagination(
            pagination_content(
                pagination_item(pagination_previous(attrs: attributes! { href="?page=1" }))
                pagination_item(pagination_link(attrs: attributes! { href="?page=1" }, "1"))
                pagination_item(
                    pagination_link(active: true, attrs: attributes! { href="?page=2" }, "2")
                )
                pagination_item(pagination_ellipsis())
                pagination_item(pagination_next(label: "Older", attrs: attributes! { href="?page=3" }))
            )
        )
    })
}
```

```text
pagination(attrs, child)  // :33
pagination_content(attrs, child)  // :56
pagination_item(attrs, child)  // :75
pagination_link(active: bool = false, attrs, child)  // :87
pagination_previous(label: String = String::from("Previous") /* into */, attrs)  // :124
pagination_next(label: String = String::from("Next") /* into */, attrs)  // :149
pagination_ellipsis(attrs)  // :174
```

- Markup: `pagination` `<nav aria-label="pagination" class="@container mx-auto flex w-full justify-center">`, `pagination_content` `<ul>`, `pagination_item` `<li>`, `pagination_link` `<a>` styled with `button_variants` (Outline when `active`, otherwise Ghost, size Icon) and `aria-current="page"` when active, `pagination_previous` and `pagination_next` `<a>` with an icon and a `<span class="sr-only @xs:not-sr-only">` label, `pagination_ellipsis` `<span>` with an icon and `<span class="sr-only">More pages</span>`. Destinations come from `href` in `attrs`.
- The label text is hidden below the `@xs` container width but stays available to assistive technology.
- Client behavior: none. Dependencies: `button`.
- Icons: `lucide:chevron-left`, `lucide:chevron-right`, `lucide:ellipsis`.

Source: `topcoat-ui-registry-0.10.0/src/components/pagination.rs:33`
Probe: `tests/navigation.rs` (`pagination_route`), `tests/examples.rs` (`ex_pagination`)

### tabs

Links that select a panel. This is not the ARIA tab pattern.

Status: VERIFIED
Features: `view`, `runtime`, `asset`

```rust
use crate::components::tabs::{tabs, tabs_content, tabs_list, tabs_trigger};
use topcoat::{
    Result,
    context::Cx,
    router::page,
    runtime::{Event, expr, signal},
    view::{View, attributes, view},
};

#[page("/ex/tabs")]
async fn demo(cx: &Cx) -> Result<impl View> {
    let tab = signal(cx, || 0usize);

    Ok(view! {
        tabs(
            tabs_list(
                tabs_trigger(
                    active: expr!(tab.get() == 0usize),
                    attrs: attributes! {
                        href="#a"
                        @click=$(|e: Event| { e.prevent_default(); tab.set(0usize) })
                    },
                    "A"
                )
                tabs_trigger(
                    active: expr!(tab.get() == 1usize),
                    attrs: attributes! {
                        href="#b"
                        @click=$(|e: Event| { e.prevent_default(); tab.set(1usize) })
                    },
                    "B"
                )
            )
            tabs_content(attrs: attributes! { :hidden=$(tab.get() != 0usize) }, "Panel A")
            tabs_content(attrs: attributes! { :hidden=$(tab.get() != 1usize) }, "Panel B")
        )
    })
}
```

```text
tabs(attrs, child)  // :33
tabs_list(attrs, child)  // :46
tabs_trigger(active: Expr<bool> = false /* into */, attrs, child)  // :81
tabs_content(attrs, child)  // :109
```

- Markup: `tabs` `<div class="flex flex-col gap-4">`, `tabs_list` `<div class="inline-flex w-fit items-center gap-1 rounded-lg border border-border p-1">`, `tabs_trigger` `<a>` with `aria-current="page"` when active (a bound `:aria-current` for an expression), `tabs_content` `<div>`. Destination in `href` through `attrs`.
- Two modes: render only the selected panel on the server and let the triggers be ordinary links (`active: value == tab`), or render all panels, bind `active` and each panel's `hidden` to a signal, and prevent the link navigation in the click handler (the example).
- A11y: links with `aria-current`; no `role="tablist"`, `role="tab"` or arrow-key movement.

Source: `topcoat-ui-registry-0.10.0/src/components/tabs.rs:33`
Probe: `tests/navigation.rs` (`tabs_route`, `tabs_signal`), `tests/examples.rs` (`ex_tabs`)

### accordion

Collapsible sections on native `<details>`.

Status: VERIFIED
Features: `view`, `icon-iconify` (chevron-down)

```rust
use crate::components::accordion::{accordion, accordion_content, accordion_item, accordion_trigger};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/accordion")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        accordion(
            accordion_item(
                attrs: attributes! { name="faq" open="" },
                accordion_trigger("What is it?")
                accordion_content("A thing.")
            )
            accordion_item(
                attrs: attributes! { name="faq" },
                accordion_trigger("Why?")
                accordion_content("Because.")
            )
        )
    })
}
```

```text
accordion(attrs, child)  // :31
accordion_item(attrs, child)  // :58
accordion_trigger(attrs, child)  // :80
accordion_content(attrs, child)  // :110
```

- Markup: `accordion` `<div class="w-full">`, `accordion_item` `<details class="group border-b border-border last:border-b-0 ...">`, `accordion_trigger` `<summary>` plus a chevron that rotates with `group-open:rotate-180`, `accordion_content` `<div class="pb-4 text-sm text-muted-foreground">`. Give items the same `name` (through `attrs`) to allow one open item at a time; `open=""` opens one initially.
- The height animation uses `interpolate-size` and `::details-content`; browsers without them use the native disclosure (source comment).
- Client behavior: none. A11y: native disclosure semantics.
- Icons: `lucide:chevron-down`.

Source: `topcoat-ui-registry-0.10.0/src/components/accordion.rs:31`
Probe: `tests/navigation.rs` (`accordion_route`), `tests/examples.rs` (`ex_accordion`)

### sidebar

An application sidebar that becomes a sheet below `md` (48rem).

Status: VERIFIED
Features: `view`, `runtime`, `asset`, `icon-iconify` (panel-left)

```rust
use crate::components::sidebar::{
    SidebarCollapsible, sidebar, sidebar_content, sidebar_group, sidebar_group_content,
    sidebar_group_label, sidebar_header, sidebar_inset, sidebar_menu, sidebar_menu_button,
    sidebar_menu_item, sidebar_provider, sidebar_trigger,
};
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    router::page,
    runtime::{Event, expr, signal},
    view::{View, attributes, view},
};

#[page("/ex/sidebar")]
async fn demo(cx: &Cx) -> Result<impl View> {
    let open = signal(cx, || true);
    let mobile = signal(cx, || false);

    Ok(view! {
        sidebar_provider(
            sidebar(
                open: expr!(open.get()),
                mobile_open: expr!(mobile.get()),
                collapsible: SidebarCollapsible::Icon,
                attrs: attributes! { id="app-sidebar" },
                sheet_attrs: attributes! {
                    aria-label="Main navigation"
                    @keydown=$(|e: Event| if e.key == "Escape" { mobile.set(false) })
                },
                sidebar_header("Zayden")
                sidebar_content(
                    sidebar_group(
                        sidebar_group_label("Server")
                        sidebar_group_content(
                            sidebar_menu(
                                sidebar_menu_item(
                                    sidebar_menu_button(
                                        href: Some("/guild/1"),
                                        active: true,
                                        tooltip: Some("Overview"),
                                        icon(data: iconify_icon!("lucide:panel-left"))
                                        <span>"Overview"</span>
                                    )
                                )
                            )
                        )
                    )
                )
            )
            sidebar_inset(
                sidebar_header(
                    sidebar_trigger(
                        open: expr!(open.get()),
                        attrs: attributes! {
                            aria-controls="app-sidebar"
                            @click=$(|_e: Event| open.set(!open.get()))
                        }
                    )
                )
                <p>"Page content"</p>
            )
        )
    })
}
```

```text
sidebar_provider(attrs, child)  // :96
sidebar(open: Expr<bool> = true /* into */, mobile_open: Expr<bool> = false /* into */, side: SidebarSide = SidebarSide::Left, variant: SidebarVariant = SidebarVariant::Sidebar, collapsible: SidebarCollapsible = SidebarCollapsible::Offcanvas, attrs, sheet_attrs: Attributes = empty, child)  // :141
sidebar_trigger(open: Expr<bool> = true /* into */, attrs)  // :219
sidebar_rail(open: Expr<bool> = true /* into */, attrs)  // :245
sidebar_inset(attrs, child)  // :272
sidebar_header(attrs, child)  // :295
sidebar_footer(attrs, child)  // :315
sidebar_content(attrs, child)  // :332
sidebar_group(attrs, child)  // :352
sidebar_group_label(attrs, child)  // :372
sidebar_group_action(attrs, child)  // :397
sidebar_group_content(attrs, child)  // :415
sidebar_menu(attrs, child)  // :432
sidebar_menu_item(attrs, child)  // :449
sidebar_input(attrs)  // :466
sidebar_separator(attrs)  // :483
sidebar_menu_button(variant: SidebarMenuButtonVariant = SidebarMenuButtonVariant::Default, size: SidebarMenuButtonSize = SidebarMenuButtonSize::Md, active: Expr<bool> = false /* into */, href: Option<&str> = None, tooltip: Option<&str> = None, attrs, child)  // :547
sidebar_menu_action(show_on_hover: bool = false, attrs, child)  // :581
sidebar_menu_badge(attrs, child)  // :605
sidebar_menu_skeleton(show_icon: bool = false, attrs)  // :625
sidebar_menu_sub(attrs, child)  // :653
sidebar_menu_sub_item(attrs, child)  // :673
sidebar_menu_sub_button(active: Expr<bool> = false /* into */, size: SidebarMenuButtonSize = SidebarMenuButtonSize::Md, attrs, child)  // :690
```

- Enums: `SidebarSide`: Left (default), Right; `SidebarVariant`: Sidebar (default), Floating, Inset; `SidebarCollapsible`: Offcanvas (default), Icon, None; `SidebarMenuButtonVariant`: Default (default), Outline; `SidebarMenuButtonSize`: Sm, Md (default), Lg (`Sm` h-7 text-xs, `Md` h-8, `Lg` h-12).
- Structure and hooks (`data-sidebar` value, element): `provider` `<div>`, `sidebar` `<aside data-side data-variant data-state data-collapsible>` containing a `<dialog role="navigation" aria-label="Sidebar">` (the mobile sheet and the desktop panel are the same element) whose content `<div data-sidebar="panel">` holds the children once, `trigger` `<button aria-label="Toggle sidebar" aria-expanded>`, `rail` `<button>`, `inset` `<main>`, `header`, `footer`, `content`, `group`, `group-label`, `group-action` `<button>`, `group-content`, `menu` `<ul>`, `menu-item` `<li>`, `input`, `separator` `<hr>`, `menu-button` `<a>` (with `href`) or `<button type="button">` with `data-active` and `aria-current`, `menu-action` `<button>`, `menu-badge` `<span>`, `menu-skeleton` `<div aria-hidden>`, `menu-sub` `<ul>`, `menu-sub-item` `<li>`, `menu-sub-button` `<a>`.
- State: `open` (default `true`) drives desktop expansion through `data-state="expanded|collapsed"` and `data-collapsible` (`""` when expanded, otherwise the collapsible mode); `mobile_open` (default `false`) drives the sheet. `collapsible: None` ignores `open`. With `Icon`, group labels, badges, actions, sub menus and the second skeleton bar are hidden, menu buttons become `size-8` icon buttons, and the last `<span>` of a menu button becomes `sr-only`. A menu button needs its label in a `<span>` after the icon.
- Sizing variables on the provider: `--sidebar-width` 16rem, `--sidebar-width-mobile` 18rem, `--sidebar-width-icon` 3rem (override with a class such as `[--sidebar-width:20rem]` in the provider's `attrs`). The provider is `min-h-svh` and, from `md`, `h-svh overflow-hidden`: the page scrolls inside `sidebar_inset`.
- Palette: the panel re-aliases `--background`, `--foreground`, `--card`, `--card-foreground`, `--primary`, `--primary-foreground`, `--border`, `--ring` and `--muted-foreground` to the `--sidebar-*` tokens, so nested buttons and inputs follow the sidebar palette.
- `attrs` go on the `<aside>`; `sheet_attrs` go on the `<dialog>` (label, id, Escape or backdrop handlers).
- Client behavior: state belongs to the caller. Pass `expr!(signal.get())` to `open`, `mobile_open`, `sidebar_trigger` and `sidebar_rail`, and update the signal in `@click` handlers on the trigger, rail and menu buttons. Focus trapping on mobile needs application scripting, as for `sheet`.
- A11y: trigger and rail expose `aria-expanded`; set `aria-controls` on the trigger to the panel id; `title` from `tooltip` is the only hint in icon mode.
- Dependencies: `button`, `input`, `separator`, `sheet`, `skeleton` (`super::` imports). Icons: `lucide:panel-left`.
- Conflict: the app's own shell (`web/style/partials/layout.css`, `.app-sidebar*`, `--sidebar-width: 248px`) is a separate implementation; inside a `sidebar_provider` the registry's `--sidebar-width` is set on the provider element and shadows the app's `:root` value.

Source: `topcoat-ui-registry-0.10.0/src/components/sidebar.rs:96`
Probe: `tests/navigation.rs` (`sidebar_route`), `tests/examples.rs` (`ex_sidebar`)

## Data display

### table

A table in a horizontally scrollable wrapper.

Status: VERIFIED
Features: `view`

```rust
use crate::components::table::{
    table, table_body, table_caption, table_cell, table_head, table_header, table_row,
};
use topcoat::{Result, router::page, view::{View, view}};

#[page("/ex/table")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        table(
            table_caption("Deploys")
            table_header(table_row(table_head("Environment") table_head("Status")))
            table_body(
                for (env, status) in [("production", "Live"), ("staging", "Idle")] {
                    table_row(table_cell((env)) table_cell((status)))
                }
            )
        )
    })
}
```

```text
table(attrs, child)  // :30
table_header(attrs, child)  // :51
table_body(attrs, child)  // :64
table_footer(attrs, child)  // :80
table_row(attrs, child)  // :99
table_head(attrs, child)  // :118
table_cell(attrs, child)  // :138
table_caption(attrs, child)  // :154
```

- Markup: `table` `<div class="w-full overflow-x-auto"><table class="w-full caption-bottom border-collapse text-sm [class]" ...attrs>` (attributes go on the `<table>`), `table_header` `<thead>`, `table_body` `<tbody>`, `table_footer` `<tfoot>`, `table_row` `<tr class="border-b border-border hover:bg-foreground/5">`, `table_head` `<th class="h-10 px-3 text-left ... text-muted-foreground">`, `table_cell` `<td class="p-3 align-middle whitespace-nowrap">`, `table_caption` `<caption>` (placed below the table).
- Client behavior: none. A11y: native table elements; `colspan`, `scope` and similar go through `attrs`.

Source: `topcoat-ui-registry-0.10.0/src/components/table.rs:30`
Probe: `tests/data_display.rs` (`table_route`), `tests/examples.rs` (`ex_table`)

### card

A bordered panel with header, content and footer.

Status: VERIFIED
Features: `view`

```rust
use crate::components::{
    button::{ButtonSize, ButtonVariant, button},
    card::{card, card_content, card_description, card_footer, card_header, card_title},
};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/card")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        card(
            attrs: attributes! { class="max-w-sm" },
            card_header(card_title("Delete workspace") card_description("This cannot be undone."))
            card_content("Body")
            card_footer(
                attrs: attributes! { class="justify-end" },
                button(variant: ButtonVariant::Destructive, size: ButtonSize::Sm, "Delete")
            )
        )
    })
}
```

```text
card(attrs, child)  // :34
card_header(attrs, child)  // :44
card_title(attrs, child)  // :60
card_description(attrs, child)  // :76
card_content(attrs, child)  // :92
card_footer(attrs, child)  // :101
```

- Markup: `card` `<div class="flex flex-col gap-5 rounded-xl border border-border bg-card py-6 text-card-foreground shadow-sm">`, `card_header` `<div class="flex flex-col gap-1.5 px-6">`, `card_title` `<h3>`, `card_description` `<p>`, `card_content` `<div class="px-6">`, `card_footer` `<div class="flex items-center gap-2 px-6">`. Sections carry their own horizontal padding.
- Client behavior: none. Dependencies: none.

Source: `topcoat-ui-registry-0.10.0/src/components/card.rs:34`
Probe: `tests/data_display.rs` (`card_route`), `tests/examples.rs` (`ex_card`)

### badge

A small status or count label.

Status: VERIFIED
Features: `view`

```rust
use crate::components::badge::{BadgeVariant, badge, badge_variants};
use topcoat::{Result, router::page, view::{View, view}};

#[page("/ex/badge")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        badge(variant: BadgeVariant::Outline, "Draft")
        <a href="/releases" class=(badge_variants(BadgeVariant::Secondary))>"v2.0"</a>
    })
}
```

```text
badge(variant: BadgeVariant = BadgeVariant::Primary, attrs, child)  // :70
```

- Variants: `BadgeVariant`: Primary (default), Secondary, Outline, Destructive.
- Markup: `<span class="inline-flex w-fit shrink-0 items-center ... rounded-md border px-2 py-0.5 text-xs font-medium VARIANT [class]" ...attrs>`. `badge_variants(variant)` returns the class list for another element.
- Client behavior: none. A11y: none beyond the text.

Source: `topcoat-ui-registry-0.10.0/src/components/badge.rs:70`
Probe: `tests/data_display.rs` (`badge_alert_route`), `tests/examples.rs` (`ex_badge`)

### avatar

A circular image with a fallback.

Status: VERIFIED
Features: `view`

```rust
use crate::components::avatar::{AvatarSize, avatar, avatar_fallback, avatar_image};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/avatar")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        avatar(
            size: AvatarSize::Lg,
            avatar_image(alt: "Ada", attrs: attributes! { src="/avatars/ada.jpg" })
            avatar_fallback("AL")
        )
    })
}
```

```text
avatar(size: AvatarSize = AvatarSize::Md, attrs, child)  // :51
avatar_image(alt: String = "" /* into */, attrs)  // :74
avatar_fallback(attrs, child)  // :104
```

- Size: `AvatarSize`: Sm, Md (default), Lg (`Sm` size-8, `Md` size-10, `Lg` size-12).
- Markup: `avatar` `<span class="relative flex shrink-0 overflow-hidden rounded-full SIZE">`, `avatar_image` `<img alt class="absolute inset-0 size-full object-cover">` (`src` through `attrs`), `avatar_fallback` `<span class="flex size-full items-center justify-center bg-foreground/10 ...">`. The fallback sits behind the image.
- A11y: `alt` defaults to empty text; pass a real `alt` when no adjacent text names the person.

Source: `topcoat-ui-registry-0.10.0/src/components/avatar.rs:51`
Probe: `tests/data_display.rs` (`avatar_route`), `tests/examples.rs` (`ex_avatar`)

### alert

A notice displayed within the page.

Status: VERIFIED
Features: `view`

```rust
use crate::components::alert::{AlertVariant, alert, alert_description, alert_title};
use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    router::page,
    view::{View, attributes, view},
};

#[page("/ex/alert")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        alert(
            variant: AlertVariant::Destructive,
            attrs: attributes! { role="alert" },
            icon(data: iconify_icon!("lucide:triangle-alert"))
            alert_title("Build failed")
            alert_description("The last deploy did not finish.")
        )
    })
}
```

```text
alert(variant: AlertVariant = AlertVariant::Neutral, attrs, child)  // :53
alert_title(attrs, child)  // :77
alert_description(attrs, child)  // :96
```

- Variants: `AlertVariant`: Neutral (default), Destructive.
- Markup: `alert` `<div class="grid w-full grid-cols-[0_1fr] ... has-[>svg]:grid-cols-[1rem_1fr] ...">` (an icon child is laid out in the first column), `alert_title` `<p class="col-start-2 font-medium tracking-tight">`, `alert_description` `<div class="col-start-2 text-sm text-muted-foreground">`.
- A11y: `role="alert"` is deliberately not set (it interrupts a screen reader the moment the element appears); pass it in `attrs` for a message that arrives after the page is shown.
- Icons: none inside the component; the example passes `lucide:triangle-alert` as a child.

Source: `topcoat-ui-registry-0.10.0/src/components/alert.rs:53`
Probe: `tests/data_display.rs` (`badge_alert_route`), `tests/examples.rs` (`ex_alert`)

### progress

A native `<progress>` bar.

Status: VERIFIED
Features: `view`

```rust
use crate::components::progress::progress;
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/progress")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        progress(value: 62.0, attrs: attributes! { aria-label="Upload" })
        progress(attrs: attributes! { aria-label="Working" })
    })
}
```

```text
progress(value: Option<f32> = None /* into */, max: f32 = 100.0, attrs)  // :31
```

- Markup: `<progress class="h-2 w-full appearance-none ... [&::-webkit-progress-value]:bg-primary [&::-moz-progress-bar]:bg-primary" value max ...attrs></progress>`. `value` is omitted when `None` (indeterminate; the appearance depends on the browser). `max` defaults to 100.
- A11y: give it `aria-label` or an associated label.

Source: `topcoat-ui-registry-0.10.0/src/components/progress.rs:31`
Probe: `tests/data_display.rs` (`misc_route`), `tests/examples.rs` (`ex_progress`)

### skeleton

A pulsing loading shape.

Status: VERIFIED
Features: `view`

```rust
use crate::components::skeleton::skeleton;
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/skeleton")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        <div class="flex flex-col gap-2" aria-hidden="true">
            skeleton(attrs: attributes! { class="h-4 w-32" })
            skeleton(attrs: attributes! { class="h-4 w-full" })
        </div>
    })
}
```

```text
skeleton(attrs)  // :24
```

- Markup: `<div class="animate-pulse rounded-md bg-foreground/10 [class]" ...attrs></div>`; set the size with classes in `attrs`.
- A11y: the shape has no `aria-hidden`; mark the group hidden as in the example. Used by `sidebar_menu_skeleton`. Unrelated to the app's `shape_skeleton`.

Source: `topcoat-ui-registry-0.10.0/src/components/skeleton.rs:24`
Probe: `tests/data_display.rs` (`misc_route`), `tests/examples.rs` (`ex_skeleton`)

### spinner

An animated icon for work in progress.

Status: VERIFIED
Features: `view`, `icon-iconify` (loader-circle)

```rust
use crate::components::{button::button, spinner::spinner};
use topcoat::{Result, router::page, view::{Length, View, attributes, view}};

#[page("/ex/spinner")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        button(attrs: attributes! { disabled="" }, spinner() "Saving...")
        spinner(size: Length::px(24.0), label: "Loading servers")
    })
}
```

```text
spinner(size: Length = Length::em(1.0) /* into */, label: String = String::from("Loading") /* into */, attrs)  // :23
```

- Markup: the `topcoat::icon::icon` component with `class="animate-spin"`, `size` (default `1em`) and `label` (default `"Loading"`): `<svg role="img" aria-label="Loading" width="1em" height="1em" class="animate-spin">`. `attrs` go on the `<svg>`.
- Icons: `lucide:loader-circle`.

Source: `topcoat-ui-registry-0.10.0/src/components/spinner.rs:23`
Probe: `tests/data_display.rs` (`misc_route`), `tests/examples.rs` (`ex_spinner`)

### kbd

A keyboard key label.

Status: VERIFIED
Features: `view`

```rust
use crate::components::kbd::{kbd, kbd_group};
use topcoat::{Result, router::page, view::{View, view}};

#[page("/ex/kbd")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        kbd_group(kbd("Ctrl") kbd("K"))
    })
}
```

```text
kbd(attrs, child)  // :24
kbd_group(attrs, child)  // :33
```

- Markup: `kbd` `<kbd class="inline-flex h-5 w-fit min-w-5 ... border border-border bg-foreground/5 ...">`, `kbd_group` `<span class="inline-flex items-center gap-1 whitespace-nowrap">`.

Source: `topcoat-ui-registry-0.10.0/src/components/kbd.rs:24`
Probe: `tests/data_display.rs` (`misc_route`), `tests/examples.rs` (`ex_kbd`)

### separator

A thin rule.

Status: VERIFIED
Features: `view`

```rust
use crate::components::separator::{SeparatorOrientation, separator};
use topcoat::{Result, router::page, view::{View, attributes, view}};

#[page("/ex/separator")]
async fn demo() -> Result<impl View> {
    Ok(view! {
        <div class="flex h-5 items-center gap-3">
            <a href="/docs">"Docs"</a>
            separator(
                orientation: SeparatorOrientation::Vertical,
                attrs: attributes! { aria-hidden="true" }
            )
            <a href="/blog">"Blog"</a>
        </div>
    })
}
```

```text
separator(orientation: SeparatorOrientation = SeparatorOrientation::Horizontal, attrs)  // :62
```

- Orientation: `SeparatorOrientation`: Horizontal (default), Vertical.
- Markup: `<hr class="shrink-0 border-0 bg-border h-px w-full|h-full w-px" aria-orientation="vertical"? ...attrs>`. `aria-orientation="vertical"` is emitted only for `Vertical`. A vertical rule needs a container with a height. Use `aria-hidden="true"` for decoration.
- Used by `sidebar` (`sidebar_separator`).

Source: `topcoat-ui-registry-0.10.0/src/components/separator.rs:62`
Probe: `tests/data_display.rs` (`misc_route`), `tests/examples.rs` (`ex_separator`)

## Theme: `styles.css`

Status: VERIFIED.

The `neutral` theme (`web/styles.css`, registry `themes/neutral.css`, hash recorded in `components.toml`). Components use tokens only. Verified by compiling it with the pinned Tailwind CLI 4.3.2 (`scratch/topcoat-kb/ui/styles.css`, output inspected).

### Custom properties

Each token is declared in `:root` (light) and `.dark`. `@theme inline` maps it to a Tailwind color (`--color-<token>: var(--<token>)`), which creates `bg-<token>`, `text-<token>`, `border-<token>` and so on.

| Token | Role | Light | Dark |
|---|---|---|---|
| `--background` | page background (`body`) | `oklch(0.9875 0.003 260)` | `oklch(0.17 0.012 260)` |
| `--foreground` | text and icons on the background | `oklch(0.24 0.012 260)` | `oklch(0.93 0.006 260)` |
| `--card`, `--card-foreground` | cards, dialog and sheet panels | `oklch(0.995 0.003 260)`, `oklch(0.24 0.012 260)` | `oklch(0.22 0.012 260)`, `oklch(0.93 0.006 260)` |
| `--popover`, `--popover-foreground` | menus, tooltips, select pickers | same as card | same as card |
| `--muted-foreground` | secondary text, input hints | `oklch(0.51 0.014 260)` | `oklch(0.68 0.012 260)` |
| `--primary`, `--primary-foreground` | emphasis fill and text on it | `oklch(0.21 0.02 260)`, `oklch(0.985 0.003 260)` | `oklch(0.92 0.008 260)`, `oklch(0.21 0.015 260)` |
| `--destructive`, `--destructive-foreground` | destructive actions, errors | `oklch(0.55 0.2 27)`, `oklch(0.99 0.012 27)` | `oklch(0.66 0.18 25)`, `oklch(0.14 0.02 25)` |
| `--border` | hairlines and dividers (`border-border`, `bg-border`) | `oklch(0.9 0.008 260)` | `oklch(0.3 0.012 260)` |
| `--ring` | focus ring | `oklch(0.32 0.02 260)` | `oklch(0.6 0.02 260)` |
| `--sidebar` | sidebar background | `oklch(0.995 0.003 260)` | `oklch(0.22 0.012 260)` |
| `--sidebar-foreground` | sidebar text | `oklch(0.24 0.012 260)` | `oklch(0.93 0.006 260)` |
| `--sidebar-primary`, `--sidebar-primary-foreground` | primary action inside the sidebar | as `--primary` pair | as `--primary` pair |
| `--sidebar-accent` | hover and selected rows | `color-mix(in oklab, var(--sidebar-foreground) 10%, var(--sidebar))` | same expression |
| `--sidebar-accent-foreground` | text on accents | `oklch(0.24 0.012 260)` | `oklch(0.93 0.006 260)` |
| `--sidebar-border`, `--sidebar-ring` | sidebar border and ring | as `--border`, `--ring` | as `--border`, `--ring` |
| `--shadow-xs` | raised controls (`shadow-xs`) | `0 1px 2px oklch(0.24 0.02 260 / 12%)` | `0 1px 2px oklch(0 0 0 / 35%)` |
| `--shadow-sm` | raised surfaces (`shadow-sm`) | two-layer oklch shadow | two-layer black shadow |

Hover and press states are derived in the components from fills at reduced opacity (`bg-primary/90`, `bg-foreground/5`), not from extra tokens. `--font-sans: "Geist", sans-serif` is set in `@theme inline`; Tailwind derives `--default-font-family` from it, so the compiled `html` rule also uses Geist. Radii are not tokens: components use Tailwind's `rounded-*` scale, which reads `--radius-sm` .. `--radius-xl` from Tailwind's own theme layer.

### Dark mode

- Mechanism: the `dark` class on an ancestor, normally `<html>`. `@custom-variant dark (&:is(.dark *))` defines the `dark:` utility for descendants of a `.dark` element. The registry components never use `dark:`; they switch purely through the variable values. There is no `prefers-color-scheme` rule and no toggle script.
- `:root` and `.dark` have equal specificity; in the compiled output `.dark` follows `:root`, so on `<html class="dark">` the dark values win. A `.dark` element inside the page re-themes its subtree.
- The default is light. The app is dark-only today, so without `class="dark"` the components render light inside a dark page.

### Layering with Tailwind 4

Observed in the compiled CSS (`@layer properties, theme, base, components, utilities`):

| Part of the theme | Where it lands |
|---|---|
| `@theme inline` token map | `@layer theme` `:root, :host`. `--shadow-xs` and `--shadow-sm` appear there as `var(--shadow-xs)` self references; they resolve through the unlayered `:root` values below. |
| `:root { ... }` and `.dark { ... }` token values | Unlayered, emitted after the Tailwind layers' variable declarations. They beat anything in a layer. |
| `@layer base { body { @apply bg-background font-sans text-foreground } }` | `@layer base`: `body{background-color:var(--background);color:var(--foreground);font-family:Geist,sans-serif}`. |
| Utilities used by the components | `@layer utilities`. |

Consequence: unlayered CSS beats utilities regardless of specificity. Any unlayered rule on `a`, `body`, `button` or `input` in the app stylesheet overrides the registry's classes on those elements.

Scanning: `@source "./src/**/*.rs"` is relative to `styles.css`, so for `web/styles.css` it is `web/src`. `@import "tailwindcss"` without `source(none)` also auto-detects sources from the working directory (`web/`, including `tests/` and `assets/`).

Size: the theme plus the utilities of all 31 components compiles to 51,603 bytes minified (observed with Tailwind 4.3.2). The same file with the app's 24 partials appended compiles to 94,239 bytes.

## Geist font

Status: VERIFIED.

`neutral` sets `--font-sans: "Geist", sans-serif` and does not ship the font. `topcoat ui init` prints the setup; each step was verified (`tests/font_and_stylesheet.rs`).

| Step | Detail |
|---|---|
| Feature | `font-fontsource` on topcoat (implies `font`). The Fontsource catalog (2088 families) is vendored in `topcoat-font`; resolving a family needs no network at build time. |
| Declaration | `GEIST` is the Fontsource id `geist`, family name `Geist`. `fontsource_font!(GEIST)` with no arguments creates one face per weight and style of the default subset (latin): 9 weights x 2 styles = 18 faces, and `link` preloads each of the 18 files. Choose what is used. |
| Registration | The font must be registered on the router builder: `.font(GEIST)` from `RouterBuilderFontExt`. `web` does not enable `discover`, so nothing registers it automatically. Rendering `link` without it fails the request: the renderer panics on the missing `FontResolver` app context and the response is `500`. |
| Head | `topcoat::font::link(font: GEIST)` renders one `<link rel="preload" ... as="font" type="font/woff2" crossorigin>` per face (`preload: false` disables them) and `<link rel="stylesheet" href="/_topcoat/fonts/Geist-<hash>.css">`. |
| Served CSS | `GET /_topcoat/fonts/Geist-<hash>.css` returns `200`, `content-type: text/css; charset=utf-8`, `cache-control: public, max-age=31536000, immutable`, and one `@font-face` per face with `font-display: swap` and the latin `unicode-range`. |
| Font files | By default the browser loads `https://cdn.jsdelivr.net/fontsource/fonts/geist@latest/latin-<weight>-<style>.woff2` from jsDelivr (`@latest`, unpinned). `host: Asset` makes each face an `asset!` URL instead: `topcoat asset bundle` downloads the files into the asset bundle and the app serves them. |

The registry components use weights 400 (body), 500 (`font-medium`) and 600 (`font-semibold`) and no italics (no `font-bold`, `italic` or `font-light` in the 31 sources). Three faces cover them:

```rust
/// Only the weights the registry components use (400 body, 500 medium, 600 semibold).
const GEIST: Font = fontsource_font!(GEIST, weight: [400, 500, 600], style: Normal);
```

```rust
fn app_router() -> Router {
    Router::builder().font(GEIST).page(head_page).build()
}
```

```rust
#[page("/head")]
async fn head_page() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" class="dark">
            <head>
                topcoat::font::link(font: GEIST)
            </head>
            <body></body>
        </html>
    })
}
```

Probe: `scratch/topcoat-kb/ui/tests/font_and_stylesheet.rs`. The `<html>` in the head example carries `class="dark"`; see [Dark mode](#dark-mode).

## Icons

Status: VERIFIED.

Eight registry files call `iconify_icon!("lucide:<name>")` in code (the macro appears in other files only inside doc comments):

| Icon | Used by |
|---|---|
| `lucide:chevron-down` | `accordion`, `select` |
| `lucide:chevron-right` | `breadcrumb`, `dropdown_menu`, `pagination` |
| `lucide:chevron-left` | `pagination` |
| `lucide:ellipsis` | `breadcrumb`, `pagination` |
| `lucide:check` | `checkbox`, `select` |
| `lucide:panel-left` | `sidebar` |
| `lucide:loader-circle` | `spinner` |

`iconify_icon!` reads the icon set from the calling crate's `OUT_DIR` (`topcoat-icon-iconify/lucide.json`), written there by that crate's own `build.rs`, so `web/build.rs` must call `stage()` for `lucide` before the components compile. A missing set is a compile error with the snippet to add. Each icon expands to a `const`-compatible `IconData::unescaped_unchecked(ViewBox::new(0, 0, 24, 24), "<path fill=\"none\" stroke=\"currentColor\" .../>")`, so nothing is fetched at run time.

Three ways to supply the icons, all verified in probes:

| Option | Features | Network | Source edits | Probe |
|---|---|---|---|---|
| A. Iconify, default cache | `icon-iconify` | Build script downloads `https://cdn.jsdelivr.net/npm/@iconify-json/lucide@latest/icons.json` (about 610 KB) when `target/topcoat/cache/iconify/lucide.json` is missing; no checksum, `latest` unless pinned with `icon_set_version` | none | `scratch/topcoat-kb/ui` |
| B. Iconify, vendored set | `icon-iconify` | none: a committed `icons/lucide.json` is used as is (`cache_dir("icons")`) | none | `scratch/topcoat-kb/ui/vendored` (built and tested in a network-less namespace) |
| C. Local `IconData` constants | `icon` only | none | the 8 files | `scratch/topcoat-kb/ui/local-icons` |

Option B: the vendored file may contain only the seven icons. A hand-trimmed `{"prefix":"lucide","width":24,"height":24,"icons":{...}}` of the seven bodies is 1,343 bytes (the full set is 610,753 bytes); the build step checks that the `prefix` equals the set name and that every alias resolves.

Option C replaces each call with a constant from a new module, for example `components/ui_icons.rs`:

```rust
use topcoat::{icon::IconData, view::svg::ViewBox};

pub const CHEVRON_DOWN: IconData = IconData::unescaped_unchecked(
    ViewBox::new(0.0, 0.0, 24.0, 24.0),
    r#"<path fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="m6 9l6 6l6-6"/>"#,
);
```

and edits `use topcoat::icon::{icon, iconify::iconify_icon};` to `use topcoat::icon::icon;` and `iconify_icon!("lucide:chevron-down")` to `super::ui_icons::CHEVRON_DOWN` in `accordion`, `breadcrumb`, `checkbox`, `dropdown_menu`, `pagination`, `select` (also `IconData` in its import and `CHECKMARK`), `sidebar` and `spinner`. The probe's `gen.py` performs exactly these substitutions from the vendored set and renders the same markup as option A. The module can also hold the app's existing hand-written paths from `web/src/components/icons.rs`.

`icon(...)` renders `<svg viewBox width="1em" height="1em" style="vertical-align: -0.125em" aria-hidden="true">` (or `role="img" aria-label=...` when `label` is given) and `attrs` go on the `<svg>`.

## Using it in this app

Status: VERIFIED (steps 1 to 7 in a probe crate that mirrors `web`).

State at the time of writing (checked in the repository): `web/Cargo.toml` enables `asset`, `compression`, `cookie`, `router`, `runtime`, `serve`, `view` on topcoat and `tailwind`, `ui` in `[build-dependencies]`; `web/build.rs` uses `style/input.css`; `web/src/components/mod.rs` has no registry modules; the workspace `Cargo.toml` pins `topcoat = { version = "0.10", default-features = false }`. Nothing in `web/src` or `web/tests` imports a registry component. The build script, stylesheet, font, icon and lint steps below were run in probe crates that mirror these files; nothing in `web/` was changed.

### 1. Features

`web/Cargo.toml`:

```toml
topcoat = { workspace = true, features = [
  "asset",
  "compression",
  "cookie",
  "font-fontsource",
  "icon-iconify",
  "router",
  "runtime",
  "serve",
  "view",
] }

[build-dependencies]
topcoat = { workspace = true, features = ["icon-iconify", "tailwind", "ui"] }
```

- Use `"icon"` instead of `"icon-iconify"` (and drop it from the build dependency) for option C.
- `cargo machete` is unaffected: no crate is added to `web/Cargo.toml`.
- `Cargo.lock` gains `topcoat-icon`, `topcoat-icon-grammar` and `topcoat-icon-macro`; the font crates, `ureq` and the TLS crates are already locked (they come from `topcoat-tailwind` and from other topcoat features). CI uses `--locked`, so commit the lock change.
- Dependency weight is in [tooling.md](tooling.md#what-enabling-fonts-and-icons-changes).

### 2. Stylesheet and `build.rs`

Make `styles.css` the Tailwind input and fold the app's partials into it so there is a single entry point. Verified end to end in `scratch/topcoat-kb/ui/web-build`:

1. In `web/styles.css` change `@import "tailwindcss";` to `@import "tailwindcss" source(none);` so only `@source "./src/**/*.rs"` is scanned, as `style/input.css` does today.
2. Append the 24 `@import "./style/partials/<name>.css";` lines from `style/input.css` at the end of `styles.css`. Tailwind resolves `@import` after other rules. Appending keeps the app's tokens last, so its unlayered `:root` values (`--border`, `--radius-lg`, `--radius-xl`) win over the theme's.
3. Delete `style/input.css` (its `@source` line is already in `styles.css`) and update the README line that names it.
4. `web/build.rs`: use `.input("styles.css")`, write the icon set and track the new files:

```rust
use topcoat::{icon::iconify, tailwind::BuildConfig};

const TAILWIND_CLI: &str = "TAILWIND_CLI";
const TAILWIND_VERSION: &str = "4.3.2";
const TAILWIND_LINUX_X64_SHA256: &str =
    "sha256:5036c4fb4328e0bcdbb6065c70d8ac9452e0d4c947113a788a8f94fd390425c1";

fn main() {
    let config = BuildConfig::new().input("styles.css");
    let config =
        if std::env::var_os(TAILWIND_CLI).is_some_and(|value| !value.is_empty()) {
            config.executable_env(TAILWIND_CLI)
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            config.version_checksum(TAILWIND_VERSION, TAILWIND_LINUX_X64_SHA256)
        } else {
            config.version(TAILWIND_VERSION)
        };

    if let Err(error) = config.render() {
        println!("cargo::error=tailwind failed: {error}");
    }

    if let Err(error) =
        iconify::BuildConfig::new().cache_dir("icons").icon_set("lucide").stage()
    {
        println!("cargo::error=iconify failed: {error}");
    }

    println!("cargo::rerun-if-env-changed={TAILWIND_CLI}");
    println!("cargo::rerun-if-changed=styles.css");
    println!("cargo::rerun-if-changed=style");
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=assets");
    println!("cargo::rerun-if-changed=icons");
}
```

   Printing any `rerun-if-*` replaces Cargo's default change detection, so every input must be listed. `styles.css` is new; `icons` is listed only when the set is vendored (option B), and the `cache_dir("icons")` call and its `rerun-if-changed` line are dropped for option A.
5. Option B: create `web/icons/lucide.json` (the trimmed set or the full set) and commit it.
6. `web/src/document.rs`: add `class="dark"` to `<html>`, the font `<link>` before the stylesheet link, and register the font in `web/src/router.rs` (`base.font(GEIST)` before `.build()`), or the page render fails:

```rust
#[page("/head")]
async fn head_page() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" class="dark">
            <head>
                topcoat::font::link(font: GEIST)
            </head>
            <body></body>
        </html>
    })
}
```

7. `web/tests`: any test that renders `root_layout` through its own `Router::builder()` also needs `.font(GEIST)`; tests that go through `router(base)` inherit it.

### 3. Declare the components

In `web/src/components/mod.rs` add the registry modules in alphabetical order, as `topcoat ui add` would:

```rust
pub mod accordion;
pub mod alert;
pub mod alert_dialog;
pub mod avatar;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod card;
pub mod checkbox;
pub mod dialog;
pub mod dropdown_menu;
pub mod field;
pub mod hover_card;
pub mod input;
pub mod kbd;
pub mod label;
pub mod pagination;
pub mod progress;
pub mod radio_group;
pub mod separator;
pub mod sheet;
pub mod sidebar;
pub mod skeleton;
pub mod spinner;
pub mod switch;
pub mod table;
pub mod tabs;
pub mod textarea;
pub mod toggle;
pub mod tooltip;
```

(`select` stays the app's until it is resolved, below.) Declare only what is used if the others are not wanted, but keep the dependency sets together: `alert_dialog` needs `dialog`, `field` needs `label`, `pagination` needs `button`, `sheet` needs `dialog`, `sidebar` needs `button`, `input`, `separator`, `sheet`, `skeleton`.

The registry sources do not pass the workspace lints and formatters as copied. Under the lint set in the workspace `Cargo.toml` with `-D warnings` (probe `lint-wired`, same lints as the probe `Cargo.toml`):

| Finding | Count | Fix |
|---|---|---|
| `unreachable_pub` | 115 | `#![expect(unreachable_pub, reason = "...")]` at the top of `components/mod.rs`, with the reason the existing component files use. A module-level `expect` covers the child modules and is fulfilled. |
| `clippy::allow_attributes`, `allow_attributes_without_reason` | 16 each | Delete the 16 `#[allow(dead_code)]` lines on the public enums; no `dead_code` warning follows. |
| `clippy::missing_const_for_fn` | 19 | `const fn` on the 19 private `classes`, `motion`, `aria`, `name`, `sheet` and `input_type` methods, then on `button_variants`, `badge_variants` and `sidebar_menu_button_variants`, which clippy flags once the methods are `const`. |
| `cargo fmt --check` (workspace `rustfmt.toml`, `max_width = 85`) | all 31 files | Run `topcoat fmt` on the directory, then `cargo fmt`. |
| `topcoat fmt --check` after `cargo fmt` | `card.rs` | The one-line `Ok(view! { <div class=(class!(CARD, ...)) (attrs)>(child)</div> })` bodies in `card` and `card_content` are 86 to 89 columns: `topcoat fmt` (fixed 89-column margin) keeps them on one line, `rustfmt` (`max_width = 85`) wraps them, and the two keep undoing each other. Bind `let class = class!(...)` first so the line stays under 85 columns. |

`scratch/topcoat-kb/ui/lint-wired/fix.py` applies all of these (plus the `mod.rs` attribute); after it, then `topcoat fmt` and `cargo fmt` in that order, `cargo clippy -- -D warnings` is clean, `cargo fmt --check` and `topcoat fmt --check` pass, and the crate builds.

### Registry `select` next to the app's `select`

`web/src/components/select.rs` is imported by 12 source files as `crate::components::select::{...}` and by two test files (`web::components::select`). Options:

1. Keep the app's module name and give the registry file another one. Copy `topcoat-ui-registry-0.10.0/src/components/select.rs` to `web/src/components/native_select.rs` (it has no `super::` imports), add `pub mod native_select;`, and edit `components.toml`: either delete the `[registries.topcoat.components.select]` table, or change its `file` to `src/components/native_select.rs`. The second form keeps `list` accurate and makes `topcoat ui remove select` delete the copy instead of the app's file. In both cases never run `topcoat ui add select --overwrite` or `topcoat ui add --all --overwrite`; they write `select.rs`.
2. Rename the app's module (for example to `select_field`), update those 12 imports and two tests, then `topcoat ui add select --overwrite` installs the registry file as `select.rs` and appends `pub mod select;`.

With option 1 the two `select` items never share a path: the registry function is `crate::components::native_select::select`, the app's items are `crate::components::select::{select_field, channel_select, ...}`.

### Conflicts with the app stylesheet (`web/style/**`)

Compared by compiling the theme with the 24 partials (`scratch/topcoat-kb/ui/combined`):

| Conflict | Detail | What to do |
|---|---|---|
| `--border` | Both define it. The app: `rgb(255 255 255 / 0.08)` in `:root` (15 of the 24 partials read it: `card.css`, `forms.css`, `layout.css`, ...). The theme: `oklch(0.9 ...)` in `:root` and `oklch(0.3 ...)` in `.dark`. Whichever is later wins on `<html>` because the specificity is equal. | Keep the partials last (step 2) so the app value wins for both worlds. Components then draw `border-border` with the app's dark hairline. |
| `--radius-lg`, `--radius-xl` | The app sets `10px` and `12px` in `:root`. Tailwind's theme layer sets `0.5rem` and `0.75rem` and the registry uses `rounded-lg` and `rounded-xl` (`var(--radius-lg)`). The unlayered app values win, so registry radii become 10px and 12px. | Accept it, or rename the app's tokens (`--radius-2xl` and `--radius-full` are not used by the registry). |
| `--sidebar-width` | App `:root`: `248px` (`layout.css` `.app-sidebar`). Registry: `[--sidebar-width:16rem]` on `sidebar_provider`. | No clash outside a provider; inside it the registry value applies. |
| Color scheme | The app is dark-only (`--bg-base: #0a0908`, `--text-primary: #f2f1ef`). The theme is light by default and its `.dark` values are cool greys (hue 260), the app's are warm. | Add `class="dark"` to `<html>`, and optionally map the theme tokens onto the app tokens in `:root`/`.dark` (`--background: var(--bg-base)`, `--foreground: var(--text-primary)`, `--card: var(--bg-card)`, `--popover: var(--bg-elevated)`, `--muted-foreground: var(--text-secondary)`, `--primary: var(--accent)`, `--primary-foreground: var(--on-accent)`, `--destructive: var(--error)`). The app defines no `--ring`. |
| `body` and `a` rules | `base.css` has unlayered `body { background-color; color; font-family: -apple-system... }` and `a { color: var(--accent); text-decoration: none }` with `a:hover`. Unlayered rules beat the layered utilities, so links styled with `button_variants`, `pagination_link`, `breadcrumb_link`, `tabs_trigger` and `sidebar_menu_button(href)` get the accent color and hover color instead of the component's `text-*` classes, and `body` keeps the system font instead of Geist. | Move those rules into `@layer base { }` (or delete the `font-family`) when the registry components are adopted. |
| Class names | The 263 class selectors in the partials do not overlap the 485 classes Tailwind generates for the registry (`.card`, `.alert`, `.toggle`, `.label`, `.input`, `.icon` and `.select` exist only as app selectors; the registry emits no semantic class names). | None. |
| Custom properties | 42 custom properties declared by the partials; only `--border` has the same name as a theme token. `--ring` and the `--sidebar-*` tokens other than `--sidebar-width` are not defined by the app. | None. |
| Existing guards | `web/tests/legacy_style_tokens.rs` and `web/tests/components_stylesheet.rs` read `style/partials/*.css` only; they do not read `styles.css` or `style/input.css`. | None. |

## Probe index

All under `scratch/topcoat-kb/ui/` (local only; `CARGO_TARGET_DIR=.../scratch/topcoat-kb/target`, `TAILWIND_CLI=.../scratch/topcoat-probe/scaffold/tools/tailwindcss-4.3.2-linux-x64`).

| Path | Verifies |
|---|---|
| `src/components/` | the 31 registry sources, verbatim |
| `tests/examples.rs` | one compiled and rendered example per component (embedded above) |
| `tests/form_controls.rs`, `overlays.rs`, `navigation.rs`, `data_display.rs` | emitted HTML and hooks per kind; HTML dumps are written to `dump/` |
| `tests/font_and_stylesheet.rs` | Geist faces, `link`, router registration, font route headers, 500 without registration, `tailwind::stylesheet!` |
| `build.rs`, `styles.css` | the theme as a Tailwind input, iconify staging |
| `vendored/` | option B, built and tested without network |
| `local-icons/` | option C (`gen.py`) |
| `lint-wired/` | lint and formatter fixes (`fix.py`) |
| `web-build/` | the proposed `build.rs` and combined `styles.css` |
| `combined/` | stylesheet conflict analysis |
| `cli-sandbox/` | `topcoat ui init`, `add --all`, `remove` behavior |
| `deps/` | dependency trees per feature set |
