# In-memory family tree graphics for `/tree`

> **Status: implemented.** This began as the approved design and has been
> updated to describe what actually shipped. Deviations found during
> implementation are listed in *Changes from the original design* at the end;
> that section is the interesting part if you already read the plan.

## Context

`/tree` currently posts a flat text list. `TreeCmd::run` (`bot/src/bindings/family.rs:442`)
calls `row.tree(&db)` and hands the result to a local `format_tree` that prints one
`⬆`/`◆`/`⬇` line per depth, truncated at ~1990 chars. It conveys almost nothing about who
is married to whom or who adopted whom.

A graphics path was clearly intended and abandoned: `family::commands::Tree::run`
(`bot-modules/family/src/commands/tree.rs:70`) builds a `charming::series::GraphData` and
**is never called** — nothing in the workspace references `Tree::run` or `GraphData`.
`charming` is declared `default-features = false`, so it has no rasteriser and no JS
engine; it is a pure ECharts-JSON serialiser pointed at a browser that does not exist in
this deployment. `error.rs:73-74` still carries commented-out `JoinError` / `CharmingError`
variants from that attempt.

Two structural problems sit underneath the feature:

1. **The fetch is N+1.** `FamilyRow::build_tree` (`manager.rs:189`) recurses one user at a
   time and each hop calls `FamilyRow::get`, which issues **5 queries per person**. It also
   linear-scans the accumulated map for cycle detection (O(n²)), pins a person to whichever
   depth reached them first, and deliberately refuses to expand partners' parents. A
   renderer built on that output would draw a wrong graph.
2. **The data is a graph, not a tree.** `family_partners` is undirected with a configurable
   `max_partners` (polygamy is a supported setting), `family_parent_child` permits multiple
   parents per child and only forbids self-loops, so parent cycles are representable. Any
   layout must survive that.

Outcome: `/tree` posts a PNG showing people as boxes, partners joined by a bar, children
hanging off a sibling bus, with the invoking/target user highlighted and carrying their
avatar. Everything is produced in memory — no temp files, no browser, no JS engine.

### Decisions taken

| Axis | Decision |
| --- | --- |
| Scale | Render the whole connected component when ≤ 60 nodes; above that, collapse to a focus-centred view |
| Collapse rule | Cap at ±2 generations, then admit neighbours breadth-first until the budget fills; hidden neighbours become a `+N` chip |
| Renderer | Build an SVG string → `usvg` parse → `resvg`/`tiny-skia` raster → PNG |
| Fonts | Install Noto in the Docker runtime image; load explicit paths so `fontdb` mmaps them |
| Avatars | Focus user only; everyone else name-only |
| DB | Replace `FamilyRow::tree` / `build_tree` outright with a 3-query graph fetch |
| Failure | Fail loudly with a user-facing `FamilyError`; delete `format_tree` |
| Premium | Tier-scaled quotas on the expensive axes; the command itself is never gated |

### This is a replacement, not an addition

The new renderer is the **only** `/tree` implementation when this lands. There is no text
tree, no charming, and no remnant of the abandoned render attempt. Every item below must be
gone — the greps in **Verification** are the acceptance check.

| Removal | Where |
| --- | --- |
| `charming` workspace dependency | `Cargo.toml:164` |
| `charming` module dependency | `bot-modules/family/Cargo.toml:15` |
| `use charming::series::{...}` | `bot-modules/family/src/commands/tree.rs:1` |
| `struct Node` + `impl From<&Node> for GraphNode` | `bot-modules/family/src/commands/tree.rs:13-65` |
| `// CharmingError(charming::EchartsError),` and `// JoinError(tokio::task::JoinError),` | `bot-modules/family/src/error.rs:73-74` |
| `bot-modules/family/Cargo.lock` (tracked orphan pinning `charming 0.4.0`) | delete the file |
| `fn format_tree` and the text-tree response | `bot/src/bindings/family.rs:460, 473-514` |
| `FamilyRow::tree` | `bot-modules/family/src/manager.rs:260-271` |
| `FamilyRow::build_tree` | `bot-modules/family/src/manager.rs:189-258` |
| now-unused imports (`HashMap`, `FamilyRow`, `as_i64`, `UserId`) left behind in the binding | `bot/src/bindings/family.rs` |

There is deliberately **no fallback path**: if the renderer cannot produce an image the
command returns a `FamilyError`. Do not reintroduce a text rendering as a safety net —
that was the decision, and keeping one alive would leave two divergent representations of
the same data, which is the bug this change exists to remove.

### Two design choices worth understanding before implementing

**Fixed-size node boxes, names truncated to fit.** Sizing boxes to *measured* text would
make layout depend on whichever font the host resolved — geometry would differ between a
dev Mac and the container, and could not be unit-tested. Inverting it means width
estimation error can only shift a truncation by a character; it can never move geometry.
Layout therefore has **no font dependency at all** and is fully hermetic.

**Font memory.** `resvg`'s `memmap-fonts` feature routes `fontdb` through `memmap2`, so
font files are page-cache backed and only touched glyphs become resident. This is only true
if we load explicit font *paths*. Do **not** call `load_system_fonts()` — it would read
every face on the box into the heap, and it drags in the `fontconfig` parser we are
deliberately excluding.

---

## Architecture & system design

```
                      bot/src/bindings/family.rs :: TreeCmd::run
                             │  defer() ; server_tier() resolved once
                             ▼
    family::commands::Tree::run(interaction, pool, http, tier) -> TreeImage
                                     │
   ┌─────────────────────────────────┼──────────────────────────────────────┐
   │ family/src/tree/                │                                      │
   │                                 ▼                                      │
   │  fetch.rs   RawGraph::fetch(pool, guild, root)      3 SQL queries       │
   │                 │  people: Vec<(i64, String)>                           │
   │                 │  partners: Vec<(i64, i64)>   parents: Vec<(i64,i64)>  │
   │                 ▼                                                       │
   │  model.rs   FamilyGraph { people, by_id, unions }   union-find on       │
   │                 │                                   partner edges       │
   │                 ▼                                                       │
   │  compose.rs ┌── prune(budget) → model → layout → canvas_for ──┐        │
   │             │   repeat with a smaller budget while the scale    │        │
   │             └── would fall below MIN_LEGIBLE_SCALE ─────────────┘        │
   │                 │                                                       │
   │                 ▼                                                       │
   │  svg.rs     render(graph, layout, quota, avatars) -> TreeSvg            │
   │                 │   markup + canvas + avatar slots (device px)          │
   └─────────────────┼───────────────────────────────────────────────────────┘
                     ▼
   zayden_graphics::Renderer::shared()?.render(svg, canvas, overlays, limits)
        weighted semaphore → spawn_blocking → usvg → resvg → Pixmap
        → composite avatar overlays → png::Encoder(Fast) → PNG bytes
                                     │
                                     ▼
        CreateAttachment::bytes(png, "family-tree.png")
        + CreateEmbed .image("attachment://family-tree.png")
        → interaction.edit_response(...)
```

No new Discord components, no pager, no persistent state. One deferred response, one
attachment.

### Why a separate `zayden-graphics` crate

`bot-modules/zayden-core` is depended on by all ~19 modules, so putting the rasteriser
there would rebuild the world on every render tweak. A leaf crate that only `family`
depends on keeps rebuilds local, and — more importantly — keeps `family`'s layout tests
free of `resvg`. `bot-modules/zayden-graphics` matches the existing precedent that shared
non-module crates live under `bot-modules/` with a `zayden-` prefix, and is picked up
automatically by the `bot-modules/*` members glob.

### Dependency shape (verified against crates.io)

```toml
resvg = { version = "0.48", default-features = false, features = ["text", "memmap-fonts"] }
png   = "0.18"
```

`default-features = false` drops `system-fonts` (and its `fontconfig-parser`),
`raster-images` (`gif` + `image-webp` + `zune-jpeg`) and `svgz` (`flate2`).
`resvg` re-exports `usvg`, `usvg::fontdb` and `tiny_skia`, so **`resvg` is the only vector
dependency needed** — do not add `usvg`, `fontdb` or `tiny-skia` directly. In particular do
not add `tiny-skia` just for `Pixmap::encode_png()`; a version mismatch with resvg's copy
produces confusing type errors. Encode with the `png` crate, which is needed for avatar
decoding anyway and lets us pick a fast compression level.

`charming` is removed from the root and from `family` — nothing else in the workspace uses
it, and `cargo machete` is a gate.

---

## Affected code & files

### Create

| Path | Contents |
| --- | --- |
| `bot-modules/zayden-graphics/Cargo.toml` | `resvg`, `png`, `tokio`, `tracing`; `[lints] workspace = true` |
| `bot-modules/zayden-graphics/src/lib.rs` | re-exports; `pub use resvg::tiny_skia` |
| `bot-modules/zayden-graphics/src/renderer.rs` | `Renderer`, `Renderer::shared()`, `render()` |
| `bot-modules/zayden-graphics/src/fonts.rs` | explicit-path font discovery |
| `bot-modules/zayden-graphics/src/image.rs` | `decode_png`, circular masking, `encode_png` |
| `bot-modules/zayden-graphics/src/error.rs` | `GraphicsError` |
| `bot-modules/zayden-graphics/tests/render.rs` | font-free raster + overlay tests |
| `bot-modules/family/src/tree/mod.rs` | module root, tuning constants |
| `bot-modules/family/src/tree/quota.rs` | `TreeQuota` + `for_tier` |
| `bot-modules/family/src/tree/cooldown.rs` | moka-backed per-`(guild, user)` cooldown |
| `bot-modules/family/src/tree/fetch.rs` | `RawGraph::fetch` (3 queries) |
| `bot-modules/family/src/tree/model.rs` | `FamilyGraph`, `Person`, `Union` |
| `bot-modules/family/src/tree/prune.rs` | budget/generation collapse |
| `bot-modules/family/src/tree/layout.rs` | generations, ordering, coordinates |
| `bot-modules/family/src/tree/svg.rs` | SVG emission, XML escaping, name sanitising |
| `bot-modules/family/src/tree/avatar.rs` | focus-user avatar fetch (non-fatal) |
| `bot-modules/family/tests/tree_model.rs` … `tree_svg.rs` | see Testing |
| `bot-modules/family/tests/fixtures/family_graph.sql` | fixture for the fetch test |
| `design-docs/family_tree_graphics.md` | this document, committed alongside the code |

### Modify

| Path | Change |
| --- | --- |
| `Cargo.toml` (root) | remove `charming`; add `resvg`, `png` workspace deps |
| `bot-modules/family/Cargo.toml` | drop `charming`; add `zayden-app`, `zayden-graphics`, `moka` (`future`), `reqwest`, `tokio`, `tracing`; `sqlx` gains `migrate`; dev-dep `tokio` (`macros`). `bot-modules/greetings/Cargo.toml` is the exact template |
| `bot-modules/family/src/lib.rs` | `pub mod tree;` |
| `bot-modules/family/src/manager.rs` | **delete** `build_tree` (`:189-258`) and `tree` (`:260-271`) |
| `bot-modules/family/src/commands/tree.rs` | **delete** `Node` + `GraphNode` impl (`:1-65`); rewrite `run` |
| `bot-modules/family/src/error.rs` | add variants; update the **three** exhaustive matches; drop the dead `CharmingError`/`JoinError` comments |
| `bot/src/bindings/family.rs` | `TreeCmd::run` becomes thin; **delete** `format_tree` (`:473-514`) and its now-unused imports |
| `docker/Dockerfile.bot` | runtime stage: `fonts-noto-core fonts-noto-cjk` |
| `.sqlx/*.json` | regenerate |

### Delete outright

| File | Why it is safe to delete rather than deprecate |
| --- | --- |
| `bot-modules/family/Cargo.lock` | A tracked orphan. Workspace members must not carry their own lock; Cargo ignores it, and it pins `charming 0.4.0` against the root's `0.6.0`. It is the last place `charming` would survive a `grep`. |

`FamilyRow::tree` and `FamilyRow::build_tree` have exactly two callers — the dead
`Tree::run` and `TreeCmd::run` — and **zero test coverage** (`family/tests/manager.rs:7`
explicitly defers them). Both callers are rewritten here, so delete rather than deprecate.
Update the doc comment at `family/tests/manager.rs:7` that names `tree` as a deferred
DB-touching path; it is stale once the function is gone.

---

## Data schema / state changes

**No migrations.** The existing tables in `migrations/0015_family_guild_scope.up.sql` are
sufficient. `family_blocks` is not read by the renderer — the current code fetches it once
per person for nothing.

### Query 1 — connected component (replaces the N+1 walk)

A recursive self-reference may not appear inside a sub-SELECT in PostgreSQL, so flatten the
edges into a non-recursive CTE first and plain-join the recursive term:

```sql
WITH RECURSIVE edges AS (
    SELECT user_id AS a, partner_id AS b FROM family_partners       WHERE guild_id = $1
    UNION ALL SELECT partner_id, user_id FROM family_partners       WHERE guild_id = $1
    UNION ALL SELECT parent_id,  child_id FROM family_parent_child  WHERE guild_id = $1
    UNION ALL SELECT child_id,   parent_id FROM family_parent_child WHERE guild_id = $1
),
component AS (
        SELECT $2::bigint AS id
    UNION
        SELECT e.b FROM component c JOIN edges e ON e.a = c.id
)
SELECT c.id AS "id!", u.username AS "username!"
FROM component c JOIN users u ON u.id = c.id
ORDER BY c.id
LIMIT $3
```

`UNION` (not `UNION ALL`) dedupes on `id`, so termination is guaranteed even with parent
cycles — no depth counter needed. `$3` = `quota.fetch_limit` is a blast-radius guard, not
the render budget. The `as "col!"` overrides are required: sqlx infers CTE columns as
nullable.

### Queries 2 & 3 — edges within the component

The component is closed under both edge kinds, so filtering one endpoint suffices:

```sql
SELECT user_id, partner_id FROM family_partners
WHERE guild_id = $1 AND user_id = ANY($2) ORDER BY user_id, partner_id
```
```sql
SELECT parent_id, child_id FROM family_parent_child
WHERE guild_id = $1 AND parent_id = ANY($2) ORDER BY parent_id, child_id
```

Every `ORDER BY` here is load-bearing: it is what makes the rendered image byte-identical
for identical input, which the determinism test depends on.

### In-memory state

Only three things outlive a request:

- `OnceLock<Option<Renderer>>` in `zayden-graphics` — an `Arc<fontdb::Database>` holding
  mmap'd font handles (kilobytes of heap; glyph data stays in page cache) and a
  `tokio::sync::Semaphore`.
- The moka cooldown cache — a few bytes per recently active `(guild, user)`, 5-minute TTL.
- Nothing else. **No PNG cache** — caching rendered images would trade the exact resource
  we are trying to protect. Revisit only if CPU shows up in profiling.

Transient peak is bounded by the weighted semaphore at `RENDER_BUDGET_MP` × 4 bytes ≈
48 MB of pixmap across all in-flight renders, plus encoder output — independent of how many
Ultra-tier guilds hit the bot at once.

---

## Premium tiers

**The feature is fully functional on `Tier::Free`.** A free server with a normal family
graph gets the complete image with no watermark, no nag and no degradation — the free
budget of 60 nodes already covers the overwhelming majority of servers. Premium buys
*headroom on the expensive axes*: bigger graphs rendered whole, a larger canvas, more
avatars, and a shorter cooldown. Nothing is unlocked; everything is widened.

### Do not gate the command

`CommandRegistry::run_command` (`bot/src/registry/mod.rs:144-146`) applies a **binary**
gate on `CommandMetadata::required_tier` and replies with an upgrade prompt instead of
running. `TreeCmd::metadata()` must therefore keep the default `required_tier: Tier::Free`.
All scaling happens inside `Tree::run`.

> `CommandMetadata::cooldown` (`bot-modules/zayden-core/src/scope.rs:19`) is declared but
> **never read anywhere in the workspace** — it is a dead field. Do not wire the cooldown
> through it expecting the harness to enforce it; implement it in the module as below.

### `TreeQuota` — `family/src/tree/quota.rs`

Follows the established idiom of `palworld::UploadQuota` (`palworld/src/upload.rs:14-39`)
and `greetings::GreetingsSettingsRow::floors_for`: a `Copy` struct with `FREE`/`PRO`/`ULTRA`
consts and a `const fn for_tier`.

```rust
pub struct TreeQuota {
    pub node_budget:       usize,          // whole component at or below this
    pub generation_span:   i32,            // ± generations kept when collapsing
    pub fetch_limit:       i64,            // SQL LIMIT guard
    pub max_canvas_pixels: u32,            // pixmap ceiling
    pub max_canvas_dim:    u32,            // binds first at realistic sizes
    pub cooldown:          Option<Duration>,
}
```

| | Free | Pro | Ultra |
| --- | --- | --- | --- |
| `node_budget` | 15 | 40 | 120 |
| `generation_span` | ±2 | ±3 | ±4 |
| `fetch_limit` | 300 | 800 | 2000 |
| `max_canvas_pixels` | 2.4 MP | 5.0 MP | 9.0 MP |
| `max_canvas_dim` | 2400 | 3200 | 4096 |
| `cooldown` | 60s | 20s | none |

Every field replaces a would-be global constant, so the quota threads from `Tree::run`
through fetch → prune → layout → raster. The layout code takes `TreeQuota` (or the two
numbers it needs); it must not reach for globals.

### Why these axes

| Axis | Cost it controls |
| --- | --- |
| `node_budget` | The master knob. Drives canvas area, and therefore pixmap bytes and PNG encode time — the dominant costs. |
| `max_canvas_pixels` / `max_canvas_dim` | The pixmap allocation itself, at 4 bytes/px. Must rise with `node_budget` or a bigger graph merely gets scaled down to mush. |
| `generation_span` | How far the collapsed view reaches; multiplies effective graph size. |
| `avatars` | Genuinely expensive *and* visibly better: N CDN fetches + N PNG decodes + N composites per invocation. The clearest premium perk. |
| `fetch_limit` | Database work. Must stay ≥ `node_budget` or collapsing becomes meaningless. |
| `cooldown` | The answer to "servers that make heavy use of the feature" — it bounds *sustained* cost rather than peak cost. |

Deliberately **not** tier-scaled: PNG compression level (giving free users worse
compression is perverse), layout refinement passes and ordering sweeps (near-linear and
cheap next to rasterisation — scaling them would degrade free output for no real saving).

### Weighted render semaphore

A fixed permit count no longer bounds memory once canvas size varies by tier, so make
permits proportional to megapixels. In `zayden-graphics`:

```rust
pub const RENDER_BUDGET_MP: u32 = 12;   // total concurrent megapixels across all renders
```

`Semaphore::new(RENDER_BUDGET_MP as usize)`; each render calls
`acquire_many(pixels.div_ceil(1_000_000))`. A free render takes 3 permits (4 concurrent);
an Ultra render takes 9 and largely serialises against everything else — which is the
correct behaviour. Total transient raster memory is capped at ~48 MB regardless of the mix
of tiers hitting the bot.

> **Invariant:** `ULTRA.max_canvas_pixels.div_ceil(1_000_000) <= RENDER_BUDGET_MP`.
> `acquire_many` for more permits than the semaphore will ever hold blocks forever — a
> silent hang, not an error. `tree_quota.rs` must assert this.

### Cooldown

`moka::future::Cache<(GuildId, UserId), Instant>` with a fixed 5-minute TTL, storing the
last successful render; compare `elapsed()` against `quota.cooldown`. One cache serves all
tiers, so the per-tier duration needs no per-entry expiry. This mirrors how
`zayden_core::tier::OWNERS` already uses moka, needs no migration, and costs a few bytes
per active user.

Keyed per `(guild, user)` rather than per guild so one person spamming cannot lock out the
whole server. If aggregate load ever becomes the problem, add a second per-guild bucket —
do not change this one.

Record the timestamp **only on a successful render**, so a failed attempt does not burn the
user's cooldown.

### Avatar selection

`quota.avatars` is a cap, not a target. Selection order is deterministic: focus user first,
then their partners, parents, children, then remaining nodes in BFS order. Fetch with
`futures::stream::iter(..).buffer_unordered(6)` — `futures` is already a `family`
dependency. Every failure is non-fatal and yields no avatar for that node.

### Upsell

Reuse the `palworld` idiom (`palworld/src/commands/upload.rs:272-274`):

```rust
fn upsell_url(app: &AppState, tier: Tier) -> Option<&str> {
    (tier < Tier::Pro).then_some(app.upgrade_url.as_deref()).flatten()
}
```

Show the upsell in the embed footer **only when the tree was actually collapsed**
(`shown < total`) **and** `tier < Tier::Pro`. A free server whose tree fit inside the budget
sees no mention of premium at all — nagging a user whose output was complete is exactly the
failure mode "fully functional for free" is meant to prevent.

Footer when collapsed: `Showing 60 of 214 members · Pro renders up to 140 · <url>`

### Tier resolution

`zayden_core::server_tier(&ctx.http, &app.entitlements, guild_id)` — the guild's tier, as
`greetings` does (`greetings/src/commands/greeting.rs:35`). A family tree is a property of
the server, not of whoever typed the command, so do **not** use `ai`'s
`author_tier.max(server_tier)` here; that would let one Pro member silently raise the
render cost for a free server. `server_tier` goes through a cached owner lookup, so it is
cheap, but resolve it **once** at the top of `Tree::run` — the fetch limit depends on it.

---

## Tuning constants

Tier-varying values live in `TreeQuota` above. These are fixed:

```rust
// family/src/tree/mod.rs
pub const NODE_W: f32 = 168.0;  pub const NODE_H: f32 = 46.0;
pub const NODE_GAP: f32 = 24.0; pub const ROW_PITCH: f32 = 130.0;
pub const MARGIN: f32 = 32.0;   pub const MAX_NAME_CHARS: usize = 18;
pub const REFINE_PASSES: usize = 8;  pub const ORDER_SWEEPS: usize = 4;
pub const GEN_RELAX_PASSES: usize = 8;
pub const AVATAR_FETCH_CONCURRENCY: usize = 6;
```
```rust
// zayden-graphics
pub const RENDER_BUDGET_MP: u32 = 12;
pub const AVATAR_PX:        u32 = 64;
pub const AVATAR_MAX_BYTES: usize = 128 * 1024;
```

---

## Algorithms

### Union construction (`model.rs`)

1. Union-find over partner edges → partner groups. This handles `max_partners > 1`
   naturally; do not assume pairs.
2. For each child, collect its parent set, sort by id, and use that sorted set as a key.
   Group children by key → one `Union { partners, children }` per distinct parent set.
   This single rule covers married couples, unmarried co-parents, single parents and
   polygamous groups without special cases. A couple with children by different partners
   correctly yields several unions.
3. Order `partners` within a union: focus user first if present, then ascending id.

### Generation assignment (`layout.rs`)

1. BFS from the focus: partner → same generation, parent → −1, child → +1.
2. Relaxation, up to `GEN_RELAX_PASSES`: for each union, `g = max(gen(p))` over partners;
   assign `g` to every partner; for each child `gen(child) = max(gen(child), g + 1)`.
   Stop early when stable.
3. If still unstable after the cap, a genuine parent cycle exists: freeze generations and
   mark the offending parent→child edges `back_edge = true`. Back edges are drawn as dashed
   curves and skip bus routing. **This path must not loop or panic** — it is reachable from
   real data, since the schema only forbids self-loops.
4. Normalise so the minimum generation is 0.

### Pruning (`prune.rs`) — only when `people.len() > quota.node_budget`

1. BFS from the focus over undirected adjacency, recording `(gen_offset, hops)`.
2. Reject anything with `gen_offset.abs() > quota.generation_span`.
3. Admit in order `(hops asc, edge_priority asc, id asc)` — priority partner < parent <
   child < other — until the budget fills.
4. For each admitted node, count its rejected neighbours into `hidden: HashMap<NodeIdx, u32>`.
   These render as a `+N` chip on the node's lower-right corner.
5. The `id asc` final tiebreak is what makes the output deterministic; do not drop it.

Pruning produces the *same* `FamilyGraph` type, so layout has exactly one code path.

### Ordering and coordinates (`layout.rs`)

- A **block** is a union's partner row, or a lone person. Blocks are atomic during ordering
  — partners must never be separated.
- Initial per-generation order: deterministic DFS from the focus's block (child unions
  first, then parent unions), appending unseen blocks.
- `ORDER_SWEEPS` alternating down/up median-heuristic sweeps on block positions.
- `REFINE_PASSES` iterations of:
  - *down*: block desired-x = mean of its parent-union anchor x's;
  - *up*: union anchor x = mean of its child block centres; block desired-x = mean of the
    anchors it participates in;
  - after each, `resolve_overlaps(gen)`: left-to-right enforce
    `x[i] >= x[i-1] + (w[i-1] + w[i])/2 + NODE_GAP`, then a right-to-left relaxation to
    re-centre the run. Order is fixed at this point; only positions move.
- Translate so `min x == MARGIN`; canvas height = `(gens - 1) * ROW_PITCH + NODE_H + 2*MARGIN`.

### SVG emission (`svg.rs`)

Plain `String` building — no XML library. Draw order: background rect → sibling-bus paths →
partner bars → dashed back edges → node rects → focus ring → clipped labels → `+N` chips →
avatar placeholder circle for the focus.

Dark palette matching Discord: background `#2b2d31`, node `#404249`, focus `#5865f2`,
text `#f2f3f5`, edges `#6d6f78`.

**Name sanitising** — apply in this order, and test it:
1. Strip C0/C1 control characters and Unicode bidi controls (`U+202A`–`U+202E`,
   `U+2066`–`U+2069`) — these can otherwise scramble the rendered line.
2. Strip pictographic/emoji codepoints. Noto Core + CJK cover essentially every script but
   not emoji, and resvg's COLR/CBDT support is not something to depend on; unstripped they
   render as tofu.
3. If the result is empty or whitespace-only, substitute `user-<last 4 digits of id>`.
4. Truncate to `MAX_NAME_CHARS` **grapheme-aware at char boundaries** (`string_slice` is a
   workspace lint — use `chars().take(n)`), appending `…`.
5. XML-escape `& < > " '` last.

Each label additionally sits inside a per-node `clipPath`, so an underestimated width
clips rather than overflowing into a neighbour.

### Rasterisation (`zayden-graphics/src/renderer.rs`)

```rust
pub struct Overlay { pub pixmap: tiny_skia::Pixmap, pub x: i32, pub y: i32 }

/// Caller-supplied ceilings. `zayden-graphics` stays tier-agnostic: it must NOT
/// depend on `zayden-app` or know that `Tier` exists.
pub struct RasterLimits { pub max_pixels: u32, pub max_dim: u32 }

impl Renderer {
    pub fn shared() -> Result<&'static Self, GraphicsError>;   // OnceLock<Option<Self>>
    pub async fn render(&self, svg: String, overlays: Vec<Overlay>, limits: RasterLimits)
        -> Result<Vec<u8>, GraphicsError>;
}
```

`render` computes the canvas size, acquires `pixels.div_ceil(1_000_000)` semaphore permits,
then `tokio::task::spawn_blocking`:

1. `usvg::Tree::from_str(&svg, &opt)` with `opt.fontdb = Arc::clone(&self.fonts)` and
   `opt.font_family = self.family.clone()`.
2. Reject `w * h > limits.max_pixels` or either dimension `> limits.max_dim` **before**
   allocating or acquiring permits; `Pixmap::new` returns `Option` — map `None` to
   `GraphicsError::OverBudget`.
3. `resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut())`.
4. Composite overlays with `draw_pixmap` (circular masking is applied at decode time).
5. Encode with `png::Encoder`, `Compression::Fast`, demultiplying tiny-skia's premultiplied
   RGBA into a **single reused row buffer** — do not materialise a second full-frame buffer.

Font resolution order (`fonts.rs`): explicit configured paths → the known Noto paths under
`/usr/share/fonts/truetype/noto/` → `GraphicsError::NoFont`. Log the resolved face once at
init so a packaging mistake is diagnosable from the logs.

> `panic = "abort"` is set on the release profile, so a panic inside `spawn_blocking` takes
> the whole bot down. Validate every dimension before it reaches `Pixmap`/`resvg`, and keep
> the generated SVG machine-escaped. The workspace already denies `unwrap_used`,
> `expect_used`, `panic`, `unreachable` and warns on `indexing_slicing` — lean on `.get()`,
> `try_from` and `checked_*` throughout the layout math rather than reaching for `#[expect]`.

### Avatar (`avatar.rs`) — up to `quota.avatars`, non-fatal

Fetch `https://cdn.discordapp.com/avatars/{user_id}/{hash}.png?size=64` (or
`embed/avatars/{index}.png` when unset) through the shared `AppState.http` client, cap each
body at `AVATAR_MAX_BYTES`, decode with `png`, apply a circular alpha mask, return an
`Overlay` at that node's coordinates.

Select in deterministic order — focus, partners, parents, children, then BFS order — and
take at most `quota.avatars`. Fetch concurrently with
`futures::stream::iter(..).buffer_unordered(AVATAR_FETCH_CONCURRENCY)`.

**Any failure here logs at `warn` and yields `None` for that node** — the tree still
renders. The "fail loudly" decision covers render failures, not a CDN hiccup, and one dead
avatar must never cost the whole image.

---

## Error handling

Add to `FamilyError` and update **all three** exhaustive matches (`Display`,
`Respond::user_message`, `Error::source`):

| Variant | `user_message` | `source` |
| --- | --- | --- |
| `TreeEmpty(UserId)` | "X has no family yet. Try `/marry` or `/adopt`." | `None` |
| `TreeRender(GraphicsError)` | "I couldn't draw that family tree right now." | `Some(e)` |
| `TreeCooldown { retry_at: Timestamp }` | "You can draw another tree <t:…:R>." + upsell URL when `tier < Pro` | `None` |

`TreeCooldown` formats the relative timestamp with the `<t:{unix}:R>` idiom already used by
`palworld`'s `cooldown_label` (`palworld/src/commands/upload.rs:277-284`).

`TreeRender` carries a user-facing message per the fail-loudly decision while keeping the
real cause in the source chain for `respond_with_error`'s logging. Uncomment the existing
`Reqwest` variant only if the avatar path ends up needing it — it should not, since avatar
failures are swallowed.

---

## Security & performance considerations

- **Injection into the SVG.** Display names are attacker-controlled. Escaping is the whole
  defence; it must be applied to every interpolated string, and `tree_svg.rs` must assert
  it. A name containing `"/>` must not be able to close a node rect.
- **Unbounded canvas.** Node count is bounded by `quota.node_budget`, the fetch by
  `quota.fetch_limit`, and the pixmap by `quota.max_canvas_pixels`/`max_canvas_dim`. All
  three are independent guards; keep all three. None of them is attacker-controlled — they
  are chosen by the server's tier, not by anything in the interaction.
- **Concurrent renders.** Without the semaphore, N simultaneous `/tree` calls each allocate
  their own pixmap, and tier-varying canvas sizes mean a fixed permit count no longer bounds
  bytes. Weighting permits by megapixels caps total in-flight raster memory at
  `RENDER_BUDGET_MP` × 4 bytes ≈ 48 MB. Rendering is CPU-bound, so it must be off the async
  runtime via `spawn_blocking` regardless.
- **Premium cannot be self-granted.** Tier comes from `server_tier`, which resolves through
  `EntitlementService` — there is no path from the interaction payload to a larger quota.
- **Query cost.** 3 queries with fixed shape, down from `5 × component_size`. The
  `edges` CTE is materialised server-side; if a guild's family tables ever grow enough for
  that to matter, add a partial index on `family_parent_child (guild_id, parent_id)` — but
  measure before adding it.
- **Docker image growth.** `fonts-noto-core` + `fonts-noto-cjk` add roughly 70 MB of layer.
  RSS impact is close to zero because of `memmap-fonts`; if image size becomes a problem,
  drop `fonts-noto-cjk` first and accept tofu for CJK names.
- **Build time.** `resvg` + `harfrust` + `skrifa` under `opt-level = "z"`, `lto = "fat"`,
  `codegen-units = 1` will lengthen release builds noticeably. Use
  `cargo +nightly clippy -p family -p zayden-graphics` while iterating and save the full
  workspace gate for the end, per `CLAUDE.md`.

---

## Testing

Integration tests in `tests/`, never inline `#[cfg(test)]`. Note the existing convention
that test helpers needing `.expect()` are written as `macro_rules!` — `clippy.toml`'s
`allow-expect-in-tests` only covers code inside a `#[test]` item, not free helper fns.

| File | Asserts |
| --- | --- |
| `family/tests/tree_model.rs` | union grouping for polygamy, unmarried co-parents, single parent; generation assignment including a deliberate parent cycle terminating with `back_edge` set |
| `family/tests/tree_prune.rs` | budget respected at each tier's quota; generation cap honoured; hidden counts correct; a graph that collapses under `FREE` renders whole under `ULTRA`; **same input rendered twice gives identical block order** |
| `family/tests/tree_quota.rs` | monotonicity across tiers — `node_budget`, `generation_span`, `fetch_limit`, `max_canvas_pixels`, `avatars` strictly increase and cooldown strictly decreases from Free → Pro → Ultra; `fetch_limit >= node_budget` at every tier; and the semaphore invariant `ULTRA.max_canvas_pixels.div_ceil(1_000_000) <= RENDER_BUDGET_MP`. Mirrors `palworld/tests/upload.rs` and `greetings/tests/cooldown.rs` |
| `family/tests/tree_layout.rs` | no two boxes overlap within a generation; partners stay adjacent; a graph at each tier's `node_budget` stays within that tier's `max_canvas_pixels` |
| `family/tests/tree_svg.rs` | `<`, `&`, `"`, `'` escaped; control and bidi chars stripped; emoji-only name falls back to `user-NNNN`; truncation lands on a char boundary |
| `family/tests/tree_fetch.rs` | `#[sqlx::test(migrations = "../../migrations", fixtures("family_graph"))]` over a fixture with two disjoint components plus a cycle — the returned id set is exactly the focus's component |
| `zayden-graphics/tests/render.rs` | a **text-free** SVG rasterises to a PNG with the expected magic bytes and IHDR dimensions; an overlay changes a pixel at a known coordinate; an over-budget size returns `OverBudget` rather than allocating |

Keeping the `zayden-graphics` test text-free is deliberate: it must pass on a dev machine
that has no Noto installed.

Add an `#[ignore]`d test in `family/tests/tree_layout.rs` that writes
`target/tree-sample-{2,12,200}-{free,ultra}.png` for eyeballing. Those sizes are exactly
the regimes this feature has to span, and geometry assertions will not tell you whether it
*looks* right — in particular whether a 200-node graph is still readable at `ULTRA`, which
is the entire premise of selling the larger budget.

---

## Implementation steps

1. Scaffold `bot-modules/zayden-graphics` (Cargo.toml with `[lints] workspace = true`,
   `error.rs`, `fonts.rs`, `image.rs`, `renderer.rs`, `lib.rs`). Add the `resvg` and `png`
   workspace deps to the root `Cargo.toml`.
2. Implement `fonts.rs` (explicit-path discovery, no `load_system_fonts`) and
   `image.rs` (`decode_png` with byte cap, circular mask, `encode_png` with `Compression::Fast`
   and a reused row buffer).
3. Implement `Renderer` — `OnceLock<Option<Self>>`, `Semaphore(RENDER_BUDGET_MP)` with
   `acquire_many` weighted by megapixels, `RasterLimits`, `spawn_blocking` render with the
   dimension guards checked *before* acquiring. Write `zayden-graphics/tests/render.rs` and
   get it green before touching `family`.
4. Add `docker/Dockerfile.bot` runtime packages `fonts-noto-core fonts-noto-cjk`.
5. `family/src/tree/quota.rs`: `TreeQuota` with the three consts and `for_tier`. Update
   `family/Cargo.toml` per the Modify table (use `bot-modules/greetings/Cargo.toml` as the
   template). Write `tree_quota.rs` — including the `RENDER_BUDGET_MP` invariant, which is a
   silent hang if it ever regresses.
6. `family/src/tree/fetch.rs`: the three queries above, taking `quota.fetch_limit`. Delete
   `FamilyRow::build_tree` and `FamilyRow::tree` from `manager.rs`. Add the fixture and
   `tree_fetch.rs`.
7. `model.rs`: union-find partner groups, parent-set-keyed unions, deterministic ordering.
   Write `tree_model.rs`.
8. `layout.rs` part 1: generation assignment with bounded relaxation and `back_edge`
   marking. Extend `tree_model.rs` with the cycle case.
9. `prune.rs`: generation cap then BFS budget with hidden counts, both from `quota`.
   Write `tree_prune.rs`.
10. `layout.rs` part 2: block ordering (median sweeps) and coordinate refinement with
    overlap resolution. Write `tree_layout.rs`.
11. `svg.rs`: sanitising, escaping, emission. Write `tree_svg.rs`.
12. `cooldown.rs`: moka cache, checked before any work and recorded only after a successful
    render.
13. `avatar.rs`: deterministic selection capped at `quota.avatars`, concurrent fetch →
    `Vec<Overlay>`, all failures non-fatal.
14. Rewrite `family/src/commands/tree.rs` — delete `Node`/`GraphNode`, resolve
    `server_tier` once at the top, honour the `user` option via
    `zayden_core::{parse_options, optional_option}` (the current `Tree::run` ignores it
    while the binding honours it), return
    `TreeImage { png, target, shown, total, tier }`.
15. Add the `FamilyError` variants and update all three exhaustive matches.
16. Slim `TreeCmd::run` in `bot/src/bindings/family.rs` to defer → `Tree::run` →
    `CreateAttachment::bytes` + `CreateEmbed::image("attachment://family-tree.png")`, with
    the collapse/upsell footer when `shown < total`. Leave `metadata()` at its default so
    the registry's binary tier gate never fires. Delete `format_tree` and its now-unused
    imports.
17. **Removal sweep.** Work the removal table in *This is a replacement, not an addition*
    top to bottom: `charming` out of both `Cargo.toml`s, the import and `Node`/`GraphNode`
    out of `tree.rs`, the dead `CharmingError`/`JoinError` comments out of `error.rs`,
    `bot-modules/family/Cargo.lock` deleted, the stale `tree` reference in
    `family/tests/manager.rs:7` corrected. Then run the acceptance greps in
    **Verification** — they must all come back empty.
18. Render the sample PNGs and look at them. Iterate on `ROW_PITCH`, `NODE_GAP` and the
    palette until the 12-node case reads cleanly, the 200-node case collapses sensibly under
    `FREE`, and the same 200-node case renders whole and legibly under `ULTRA`.
19. Copy this document to `design-docs/family_tree_graphics.md`.

---

## Verification

### Acceptance greps — the old path is gone

All four must return **no output**. `cargo machete` will separately fail if `charming`
survives in a manifest, and `cargo +nightly clippy -D warnings` will fail on any import
left dangling by the removals.

```bash
grep -rn "charming" --include="*.rs" --include="*.toml" --include="*.lock" . --exclude-dir=target
```
```bash
grep -rn "format_tree\|build_tree\|GraphData\|GraphNode" --include="*.rs" . --exclude-dir=target
```
```bash
grep -rn "fn tree(" bot-modules/family/src/manager.rs
```
```bash
ls bot-modules/family/Cargo.lock
```

### Build and test gates

While iterating (scoped, per `CLAUDE.md`'s disk-hygiene guidance):

```bash
cargo +nightly clippy -p zayden-graphics -p family --all-targets -- -D warnings
```

Full gate before concluding:

```bash
cargo +nightly clippy --workspace --all-targets -- -D warnings
```
```bash
cargo test
```
```bash
cargo machete
```

SQLx cache — new `query!` invocations were added, so regenerate **last**, after
`cargo +nightly fmt` and after any hand-edit of SQL (sqlx keys on exact query text):

```bash
cargo sqlx prepare --workspace -- --all-features
```

Then the offline gate, which no CI job covers:

```bash
SQLX_OFFLINE=true cargo check --workspace --all-targets
```
```bash
SQLX_OFFLINE=true cargo check -p dashboard --features ssr
```

If either finishes in under a second it reused a fingerprint from the online configuration
and did not actually run — `touch` a source file in the crate and rerun.

Visual check (the one that actually decides whether this works):

```bash
cargo test -p family --test tree_layout -- --ignored --nocapture
```

then open `target/tree-sample-2.png`, `-12.png` and `-200.png`.

End-to-end, against a test guild with Noto installed in the image:

```bash
docker compose build bot && docker compose up bot
```

Invoke `/tree` on a user with no family (expect the `TreeEmpty` message), a small family
(expect a full component render with **no** premium mention anywhere), and a member of a
>60-node component (expect a collapsed render with `+N` chips and a
"Showing 60 of N · Pro renders up to 140" footer). Then grant the guild `Tier::Pro` via
`EntitlementService::grant` and re-run the last case — it should render more of the graph,
carry more avatars, and drop the upsell from the footer.

---

## Deliberately out of scope

- **No PNG cache.** It would trade the exact resource being protected. Revisit only if
  profiling shows render CPU mattering.
- **No pagination or interactive components.** One image, one response.
- **No text fallback.** Deliberate, per the fail-loudly decision — see *This is a
  replacement, not an addition*.
- **No new premium SKU or entitlement plumbing.** This reuses `Tier`, `EntitlementService`
  and `server_tier` exactly as `palworld` and `greetings` already do.
- **Not fixing `CommandMetadata::cooldown`.** The field at
  `bot-modules/zayden-core/src/scope.rs:19` is dead across the whole workspace — every
  module that wants a cooldown implements its own. Making the registry honour it would be a
  worthwhile cleanup that touches every module's metadata, and it does not belong in this
  change.


---

## Changes from the original design

Seven things turned out differently once the code existed. Each is a
correction, not a shortcut.

### 1. An adaptive node budget (`tree/compose.rs`) — the significant one

The plan assumed the node budget alone bounded the picture. It does not. A
graph can sit comfortably inside the budget by *count* and still be far too
wide to draw: a hundred siblings in one row is roughly 19,000 layout units
across, and squeezing that into the canvas gives text a few pixels tall.

The visual check made this obvious. A 200-person graph at `ULTRA` rendered at
**4096 × 110 px** — every person present, nobody readable, and only 450k of the
9,000,000 pixel budget used. The premium tier bought a bigger file of the same
unreadable image, which is precisely the opposite of the intent.

`compose` now walks the budget down (three quarters per round, floored at
`MIN_NODE_BUDGET`) until the render scale reaches `MIN_LEGIBLE_SCALE` (0.55).
The same graph now renders **4096 × 387 px showing 52 people**, against 26 at
`FREE` — still double, and legible at both. Premium buys *more people at a
readable size*, which is the claim worth making.

Covered by `a_graph_too_wide_to_read_loses_people_rather_than_legibility` and
`a_higher_tier_composes_more_people_while_staying_legible`.

### 2. Over-budget canvases scale instead of failing

The plan had the renderer reject a canvas over `max_canvas_pixels`. In practice
that would turn an ordinary wide family into a hard error. Instead the SVG keeps
layout units in `viewBox` and declares scaled pixel dimensions, so the drawing
shrinks to fit. `GraphicsError::OverBudget` remains, but now only fires on a
caller bug rather than on real data.

### 3. Blocks and unions are separate concepts

The plan modelled only unions. Two different jobs were hiding in that word:
keeping partners side by side (a **block** — a connected component of partner
edges, atomic during ordering) and hanging children below a parent set (a
**union**, keyed on the exact sorted parent set). Splitting them removed every
special case for polygamy, unmarried co-parents and single parents.

### 4. Escaping had to be split out of `sanitise`

`tree_svg.rs::a_name_cannot_break_out_of_the_markup` caught a real bug: width
fitting ran on the *escaped* string, so it could truncate `&amp;` to `&am` and
produce markup that no longer parses. `clean` (strip and truncate) is now
separate from `escape_xml`, and escaping is strictly last on both paths.

### 5. `zayden-graphics` modules are `pub`

`unreachable_pub` (deny) and `redundant_pub_crate` (nursery) directly
contradict each other for `pub(crate)` items inside a private module. Declaring
the modules `pub` satisfies both and matches how `zayden-core` is laid out. No
`#[expect]` was needed.

### 6. Avatar hashes are not fetched per member

Only the target's avatar hash is known from the interaction payload. Resolving
everyone else's would cost a REST call per member — exactly the per-person
round-trip pattern this change exists to remove. Non-target nodes fall back to
Discord's default avatar. Revisit only if `GuildMembersCache` can supply hashes
for free.

### 7. `Renderer::with_fonts` exists for tests

`Renderer::shared()` returns `NoFont` on a host with no fonts installed, which
would make `zayden-graphics`'s tests environment-dependent. `with_fonts` lets
them build a renderer over an empty database and rasterise text-free SVGs, so
the suite is hermetic.

### Unrelated fix picked up on the way

The root manifest's `reqwest` pin had drifted from `"0.13"` to `"*"`. Because
`music` aliases `reqwest` 0.12 (which has no `rustls` feature), `*` let the
resolver unify onto 0.12.28 and the whole workspace failed to resolve. Restored
to `"0.13"`.

---

## Test coverage as built

| Suite | Tests |
|---|---|
| `zayden-graphics/tests/render.rs` | 7 |
| `family/tests/tree_quota.rs` | 7 |
| `family/tests/tree_fetch.rs` | 7 (`#[sqlx::test]`) |
| `family/tests/tree_model.rs` | 12 |
| `family/tests/tree_prune.rs` | 9 |
| `family/tests/tree_layout.rs` | 12 + 1 ignored sample dump |
| `family/tests/tree_svg.rs` | 14 |


---

## Sizing the tiers against reality

The first cut of these numbers was wrong, and worth recording so it is not
repeated.

Free was originally 60 people. Observed reality on comparable bots is that the
**largest** family in one connected component is around 20, and the median is
far smaller. A free budget of 60 therefore never binds: `node_budget`,
`generation_span` and `fetch_limit` were all dead knobs, the collapse path was
unreachable outside tests, and the upsell footer would essentially never
render. Premium was decorative on every axis it was built around.

Rendering a realistic 20-person family made the second point: it lays out at
**2332 × 370**, which is 68px under Free's 2400px `max_canvas_dim`. So the axis
that actually binds at real sizes was `max_canvas_dim` — the one treated as
secondary — and it binds almost exactly at the top of the observed range.

The numbers above are set against that. Free at 15 renders a median family
whole and collapses the largest ones, which is where premium is supposed to
start paying. Pro at 40 covers the observed ceiling with headroom.

`tree_quota.rs::premium_engages_at_realistic_family_sizes` pins this directly:
Free must sit below 20 and Pro above it. If those move, the test says so.

### Avatars are not a tier axis

Originally 1 / 12 / 60 per tier. That could not be delivered honestly: only the
*target's* avatar hash arrives with the interaction. `GuildMembersCache` stores
bare `Vec<UserId>`, serenity's own cache is disabled, and `users` holds only
`(id, username)` — so resolving anyone else's avatar means a REST call per
member, exactly the pattern this change removed. Pro's "12 avatars" would have
rendered as one real face and eleven default-colour circles.

The field is gone; every tier draws the focus user's avatar and nobody else's.

To revive it as a real axis, add an `avatar` column to `users`.
`FamilyRow::save` already upserts that table on every marry and adopt, so
hashes would populate opportunistically and arrive free in the existing
component query — no new round trips.
