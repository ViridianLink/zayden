# Audit: reaction-roles

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Small (~500 LOC), clean `command/` + `reaction/` + manager split with a concrete
`PgPool` manager and a `tests/` directory. Otherwise unremarkable.

## Findings

### 3. `add`/`remove` mapping CRUD belongs on the dashboard  ·  #8  ·  med
- **Status:** `unclear`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Scope decision (owner, 2026-07-25):** *Partial* move, not the wholesale
  migration the finding proposed — the same per-field lens the music [#3](music.md)
  ruling established, applied to per-*operation*. **`/reaction_role add` stays in
  Discord:** picking an emoji is far easier against the client's native picker
  than typing `<:name:id>` into a web text field, and that ergonomic beats the
  "single editor" tidiness. **Listing and removing move to the web**, where a row
  already carries its message, emoji and role, so removal is one button and needs
  no emoji entry at all — strictly easier than the `/reaction_role remove` it
  replaces. So the heuristic here is *not* "config → dashboard" but **"pick an
  emoji → client, see and manage the set → web"**.
- **Fix (2026-07-25):** Built the dashboard side and deleted only the operation
  the web genuinely does better.
  - **New page** `/guild/:id/reaction-roles`
    (`dashboard/src/ui/pages/reaction_roles.rs`, sidebar entry in
    `ui/components/layout.rs`): a table of every `channel → emoji → role`
    mapping in the guild — the whole set visible at once, which the paired
    slash commands never showed — with a jump-to-message link, a per-row
    Remove, and an add form. Channel/role **names** are resolved from the
    existing `list_guild_channels`/`list_guild_roles` calls, so no extra
    Discord round-trip; custom emoji render from the CDN.
  - **Server fns** `dashboard/src/server/reaction_roles.rs`:
    `list_reaction_roles` / `add_reaction_role` / `remove_reaction_role`, all
    behind the same `guild_admin_context` authz as every other guild mutation.
    `add` mirrors the removed command exactly: blank message id → post a new
    panel embed, given id → attach to that message (round-tripped via
    `http.message` so a typo fails before a row is written); either way the
    bot seeds the reaction. `remove` deletes the row, then clears the seeded
    reaction *best-effort* — the row is already gone, so a deleted message
    must not fail the removal.
  - **Duplicate guard (both add paths).** `reaction_roles` carries no unique
    constraint and the handler's lookup is a single-row `fetch_optional`, so a
    second row for the same `(message, emoji)` pair breaks *both* mappings.
    This was reachable before by re-running `/reaction_role add`, and is a
    double-click away now that a web form exists, so both the command
    (`ReactionRoleError::DuplicateMapping`) and `add_reaction_role` reject it.
  - **No new SQL.** The three existing concrete-`PgPool` queries
    (`rows`/`create`/`delete`) are reused as-is, so `.sqlx` is untouched.
  - **Emoji normalisation** is the one real hazard: the reaction handler looks
    a mapping up by `reaction.emoji.to_string()`, so anything the dashboard
    stores must match that rendering byte-for-byte or the mapping is silently
    inert. Rather than reimplement it web-side, `ParsedEmoji` (`src/emoji.rs`)
    normalises through serenity's `ReactionType` and exposes the `stored` form
    plus the `custom_id`/`name` split the twilight `RequestReactionType` needs.
    `reaction-roles` is therefore an **ssr-only** dashboard dependency (with
    the four snowflake id types re-exported from its `lib.rs`) so the wasm
    hydrate bundle is unaffected — one implementation of the contract, not two.
  - **Removed:** only `src/command/remove.rs` and the `remove` subcommand from
    `command/mod.rs`'s `register`/dispatch. `command/add.rs` and the
    `bot/src/bindings/reaction_roles/` shim stay per the scope decision above;
    `/reaction_role`'s description narrows to "Adds a reaction role". The
    reaction **event handler** stays in-bot untouched — it needs the live
    reaction.
  - **Two writers, one contract.** Keeping `add` in both places is the
    duplication [CC-8](_cross-cutting.md#cc-8) warns about, so the *divergence*
    is designed out rather than tolerated: both paths land on the same
    `ReactionRole::create` and the same `ReactionType`-derived emoji string
    (the command uses `ReactionType` directly; the web uses `ParsedEmoji`,
    which `tests/emoji.rs` pins against it). Both also enforce the duplicate
    guard below. There is no second SQL statement and no second normalisation
    rule to drift.
  - **Verified:** `tests/emoji.rs` (4 tests) pins `ParsedEmoji::parse` against
    `ReactionType::to_string()` for unicode/custom/animated inputs, whitespace
    trimming, the `custom_id`/`name` split, and malformed-input rejection.
    `cargo +nightly clippy --workspace --all-targets -D warnings` clean, plus
    `-p dashboard --features ssr` and the wasm `--features hydrate` check;
    `cargo test` green; no new `#[allow]`/`#[expect]`.
    **Residual:** `cargo machete` reports a pre-existing `levels -- tokio`
    unused dep, untouched here.
- **Where:** `src/command/{add,remove}.rs`.
- **What:** Managing message→emoji→role mappings is admin CRUD of reference data
  — a browsable web list is far better than paired slash commands.
- **Why it matters:** CRUD of config data is the dashboard's sweet spot (cf. the
  destiny2 loadout move); a table view makes the whole mapping visible at once.
- **Suggested fix:** Build a reaction-roles page (list/add/remove maps) against the
  concrete manager; keep the reaction **event handler** in-bot (it needs the live
  reaction). See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: clean command/reaction/manager separation.
- #1 DB access: concrete impl uses compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
- #4 Stringly typing: none of note.
