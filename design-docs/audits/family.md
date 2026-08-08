# Audit: family

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Clean `commands/` + `components/` + manager + `relationships.rs` structure for a
relationship-graph feature (~1.5k LOC). Concrete `PgPool` manager with
integration tests under `tests/`.

## Findings

_None outstanding._

## Clean
- #1 Architecture: clean command/component/manager/relationships split.
- #1 DB access: concrete impl uses compile-time macros (no runtime SQL).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no locks across `.await`.
- #7 Lint: one `#[expect]` at `commands/tree.rs:71` (CC-3).

## Deep-sweep findings

_Deep sweep: 2026-07-17 · lenses: silent-failure, state-machine/invariant, concurrency._

### DS-2. `marry`/`adopt` accept handlers re-run no invariant checks → `MAX_PARTNERS`/already-adopted bypass  ·  Pass 7  ·  low
- **Status:** `unclear`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-22):** Both `accept` handlers now re-validate the invariants
  against the *freshly-read* rows, before the write:
  `components/marry.rs::accept` rejects with `AlreadyRelated` if the pair is
  already related and `MaxPartners` if **either** party is `at_partner_limit()`;
  `components/adopt.rs::accept` rejects with `AlreadyAdopted` if the child
  `is_adopted()` and `AlreadyRelated` if the pair is already related. Two new
  pure guards on `FamilyRow` — `at_partner_limit(max)` / `is_adopted()` —
  encapsulate the checks. **Folded into the guild-scope design change
  (2026-07-22):** the partner cap is no longer a `const`; it is the guild's
  configured `family_settings.max_partners` (default 1), read via
  `Manager::settings(pool, guild_id)` and passed to `at_partner_limit(max)` at
  both propose and accept time. Regression test `tests/invariants.rs` pins the
  guards against the configured cap (incl. a raised cap permitting another
  partner, and negative-cap clamping); they didn't exist pre-fix, so the accept
  handler had nothing to consult.
  **Residual:** this closes the *sequential* accept-both scenario (the second
  accept re-reads the updated row and is rejected). The *same-tick concurrent*
  double-accept is still the [CC-9](_cross-cutting.md#cc-9) read-modify-write
  race (both reads see the stale pre-image); a truly atomic guard needs the
  conditional-write / [CC-1](_cross-cutting.md#cc-1) concrete-`PgPool` migration
  of the additive `save`, out of scope for this low-sev surgical fix.
- **Where:** `bot-modules/family/src/components/marry.rs:8-33` (`accept`),
  `bot-modules/family/src/components/adopt.rs:8-37` (`accept`). The guards
  (`MAX_PARTNERS`, "already adopted", "already related") live only in the
  *command* (`marry.rs:44-63`, `adopt.rs:46-60`), evaluated at proposal time.
- **What:** Between proposal and accept, state can change; the accept handler
  blindly `add_partner`/`add_child` + additive save with no recheck.
- **Failure scenario:** X sends `/marry @Z`; Y sends `/marry @Z` (both pass the
  command-time check because Z has 0 partners). Z clicks accept on both. Each
  accept adds a distinct partner via `ON CONFLICT DO NOTHING` on different pairs,
  so Z ends with 2 partners despite `MAX_PARTNERS = 1`. Same shape for two pending
  adoptions of one free child → child gets two parents.
- **Why it matters:** Invariant (`MAX_PARTNERS = 1`, single-parent adoption) is
  bypassable by anyone with two pending proposals; low severity because it needs
  cooperating/duplicate proposals and only corrupts the social graph, not economy.
- **Confidence:** confirmed (logic traced; no recheck exists).
- **Suggested fix:** Re-validate the invariants inside `accept` within the same
  transaction as the write (or make the write conditional: insert only if the
  partner/parent count is still under the cap), mirroring the CC-9 remediation.
