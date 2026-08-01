# Audit: verify

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Tiny (~112 LOC, 2 files: `lib.rs` + `error.rs`). A thin verification-gate
module. Nothing structurally wrong; no `tests/`, but the surface is small enough
that coverage is low-priority. _(Re-read 2026-07-31 while closing
[CC-6](_cross-cutting.md#cc-6): the coverage gap is accepted as `wontfix` — see
#1 — but the pass surfaced a **new** finding, the hardcoded single-guild role id
in #2, which the original pass recorded as clean under #4.)_

## Findings

### 1. No integration tests  ·  #6  ·  low
- **Status:** `wontfix — no independently testable surface (2026-07-31)`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Ruling (2026-07-31, CC-6's last crate).** Accepted as-is, per this finding's
  own "otherwise accept as-is" and [CC-6](_cross-cutting.md#cc-6)'s requirement
  that the judgement be made explicitly rather than by writing trivia. The whole
  crate is three items and none is reachable from a test:
  - `Panel::run_command` (`src/lib.rs:24-52`) — builds a constant embed + button
    and makes two `&Http` calls. No branch, no input.
  - `Panel::register` (`src/lib.rs:54-58`) — command metadata. Asserting the name
    and description equal the literals two lines above is the trivia checklist #6
    warns against.
  - `Panel::run_component` (`src/lib.rs:60-82`) — the crate's **only** branch,
    `interaction.member.is_none()` (`:64`). Both arms are unreachable offline: the
    `Some` arm is `member.add_role(http, …)`, the `None` arm returns
    `NotGuildMember`. Constructing a `ComponentInteraction` to drive it means
    fabricating a serenity payload, and the assertion would only re-state the
    `let-else` — it would pin serenity's deserialization, not this crate's logic.
  - No DB, so the `#[sqlx::test]` harness built for `gold-star`/`llamad2`
    (`9a7b8795`, `b5cc3faf`) buys nothing here — this crate has no SQL at all.

  The one genuinely non-trivial property available is
  `VerifyError::Discord => user_message() == None` (`src/error.rs:16-23`): raw
  serenity errors are never surfaced to the user. It was **deliberately not
  added** — no other `Respond` impl in the workspace is tested, so a lone
  assertion here would be an inconsistent one-off rather than coverage; if that
  invariant is worth pinning it should be pinned once, across all `Respond`
  impls, as its own finding.
- **Where:** no `tests/` directory.
- **What:** No coverage, though behaviour is minimal.
- **Suggested fix:** Add a single happy-path/deny-path test if the gate logic has
  any branching worth pinning; otherwise accept as-is. See
  [CC-6](_cross-cutting.md#cc-6).

### 2. `VERIFIED_ROLE` hardcoded, and duplicated across two crates  ·  #4 / #5  ·  med
- **Status:** `in-progress`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Recorded:** 2026-07-31, while closing #1. The original pass (2026-07-17)
  listed #4 as clean; that was wrong — checklist #4 covers hardcoded IDs, not
  just string matching.
- **Where:** `bot-modules/verify/src/lib.rs:19` and
  `bot/src/bindings/verify/mod.rs:80` — the same literal
  `RoleId::new(1_404_640_603_848_839_299)`, written out twice.
- **What:** The role the module grants is a compile-time constant naming one
  specific guild's role, and there are **two independent copies** of it: the
  verify button (`Panel::run_component`) reads the module-crate copy, `/manverify`
  reads the binding-crate copy.
- **Why it matters:** Two failure modes.
  1. **Drift.** Nothing ties the copies together; editing one leaves the button
     and `/manverify` granting different roles, and neither the compiler nor the
     clippy gate can see it.
  2. **Single-guild by construction.** The bot is multi-guild — `guild_settings`
     already carries per-guild `artist_role_id` / `sleep_role_id` /
     `lfg_role_id` (`migrations/0001_v1_init.up.sql:155-161`) and the dashboard
     already edits them (`dashboard/src/server/guild.rs:254` `save_role_settings`).
     Verify is the outlier: invoked in any other guild it asks Discord to add a
     role id that does not exist there, so the interaction fails with an opaque
     `Discord(_)` error — which, per #1's note, maps to `user_message() == None`
     and so surfaces to the user as a generic failure.
- **Suggested fix:** Add a `verified_role_id BIGINT` column to `guild_settings`,
  read it through the existing roles `SettingsRow`/`SettingsStore` (the pattern
  `artist_role_id`/`sleep_role_id` already use), and delete both constants so the
  two call sites share one source. Surface it on the dashboard's Roles section
  alongside the existing two — the [CC-8](_cross-cutting.md#cc-8) precedent, and
  cheap here since `save_role_settings` already exists. Return a typed
  "verification role not configured" error rather than a bare `add_role` failure
  when the column is null.
- **Related, not folded in:** `bot-modules/llamad2/src/behind_the_scenes.rs:8`
  holds a hardcoded `BEHIND_THE_SCENES_ROLE` too, but `llamad2` is explicitly a
  crate of server-specific novelty handlers (see [llamad2.md](llamad2.md)), so
  that one is arguably by design. `verify` is not framed that way. Worth a
  deliberate ruling if the workspace ever adopts a blanket no-hardcoded-ids rule.

## Clean
- #1 Architecture: minimal, single-responsibility.
- #2 Dead code: none found.
- #3 Async: no blocking I/O.
- ~~#4 Stringly typing: none.~~ No string matching, but a hardcoded role id —
  also checklist #4 — was missed; see finding #2 (recorded 2026-07-31).
- #7 Lint: no `#[expect]`/`#[allow]`.
