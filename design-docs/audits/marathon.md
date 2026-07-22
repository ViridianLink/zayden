# Audit: marathon

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Best-tested crate in the workspace (13 integration files + a committed
`tests/fixtures/` corpus) and concrete `PgPool` throughout. The cross-source
merge/precedence design is well isolated (`merge.rs`, per-transport parsers under
`transport/`). No CC-1, no runtime-SQL, no inline tests. Essentially clean; only
housekeeping notes.

## Findings

### 1. Fixture regeneration must stay documented  ·  #6  ·  low
- **Where:** `tests/fixtures/*.json`, transport parsers in `src/transport/`,
  `src/news.rs`.
- **What:** Fixtures are captured from live endpoints + FlareSolverr; there is a
  known `.gitignore *.json` gotcha (fixtures must be force-added) per project
  memory.
- **Why it matters:** If regeneration drifts from the documented procedure, a
  contributor can silently commit stale or zero fixtures.
- **Suggested fix:** Ensure the capture procedure is recorded in the crate
  (a `tests/fixtures/README` or module doc-comment) so it survives without the
  chat context.

## Deep-sweep findings

_Deep sweep, 2026-07-17 (third pass — prior passes skipped marathon as "essentially
clean"). Drilled the `merge.rs` consensus layer against the actual per-entity
source fan-out in `client/`._

### DS-1. `consensus` tiebreak is a `HashMap`-order coin flip when ≥2 sources collapse to the same rank → nondeterministic weapon/runner `description`  ·  Pass 2/9  ·  low-med  ·  confirmed
- **Status:** `complete — d6068291`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Fix (2026-07-22):** Took the finding's option (a). `consensus`'s per-value
  tally now carries a third field — the value's first-appearance index in
  `candidates` (`merge.rs:73-80`) — used as the final `max_by` tiebreaker after
  (count, best_rank) (`merge.rs:86-94`). Because each `candidates` slot holds one
  value, first-appearance indexes are unique across distinct values, so the
  comparator is a total order and `max_by`'s result no longer depends on
  randomized `HashMap` iteration order. Ties now resolve to the earlier-listed
  source (a stable, source-order-derived key), so a description provided only by
  two unlisted-in-`Lore` sources stops flipping across restarts/refreshes.
  Regression test `equal_rank_tie_is_deterministic_across_runs`
  (`tests/merge.rs`) constructs a two-way `Lore` tie (Mobalytics vs.
  MarathonGuide, both unlisted) and asserts the winner is invariant over 256
  fresh-seed calls — it failed before the fix (winner flipped) and passes after.
  **Residual:** unchanged option (b) alternative (making `Category::rank` order
  unlisted sources by a global source order) was not taken — option (a) is
  smaller and also covers the latent duplicate-listed-source tie the finding
  notes.
- **Where:** `bot-modules/marathon/src/merge.rs:86-89` (the `max_by`) combined with
  `bot-modules/marathon/src/source.rs:97-100` (`Category::rank`), reached via
  `merge.rs:149` / `merge.rs:177` (weapon/runner `description` → `Category::Lore`).
- **What:** `Category::rank` returns `prec.len()` for **any** source not present in
  that category's precedence list — so all unlisted sources share one identical
  rank. `Category::Lore`'s precedence lists only 3 sources
  (`Fandom, TauCeti, MarathonDb`), but `client/weapon.rs:43-82` and
  `client/runner.rs` fetch **six** sources (`MarathonDb, Mobalytics, CyberAcme,
  TauCeti, MarathonMeta, MarathonGuide`). Four of those six
  (`Mobalytics, CyberAcme, MarathonMeta, MarathonGuide`) are unlisted in `Lore`
  and therefore all get `rank == 3`. `consensus`'s tiebreak
  (`.then_with(|| b.1.1.cmp(&a.1.1))`) resolves equal-count candidates by best
  rank; when two of those collapsed-rank sources disagree, the count **and** the
  best rank tie, `max_by` returns "the last equal-maximum element," and that order
  is `HashMap` iteration order — randomized per call (each `consensus` call builds
  a fresh `HashMap` with a random `RandomState` seed).
- **Failure scenario:** For a weapon whose `description` is provided only by, say,
  Mobalytics (`"A rapid-fire assault rifle."`) and MarathonGuide
  (`"Standard-issue automatic."`) — neither Fandom/TauCeti/MarathonDb supplies one —
  both entries have `count = 1, best_rank = 3`. The merged `description` is chosen
  at random and **flips between the two strings across bot restarts / cron
  refreshes with no upstream data change**. Same for runner descriptions. It
  defeats change-detection (every refresh looks like an edit) and shows users an
  arbitrary source's text.
- **Why it matters:** low-med — cosmetic per field, but it makes the "stable
  merge" contract false and any future "did the data change?" diffing unreliable.
  Weapon/runner Stats fields are unaffected (all 6 fetched sources *are* listed in
  `Category::Stats`, so ranks are unique there); the defect is specific to
  categories whose precedence list is a strict subset of the fetched sources —
  today only `Lore`.
- **Suggested fix:** Make the tiebreak total and deterministic. Either (a) fall
  back to a stable key (e.g. lowest `SourceId` discriminant, or first appearance in
  the input `candidates` slice) after `count`/`rank`, or (b) have `Category::rank`
  break unlisted-source ties by a global source order instead of collapsing them
  all to `prec.len()`. Option (a) is the smaller change and also fixes the analogous
  latent tie among genuinely equal-rank listed sources should any category ever
  list a source twice.

## Clean
- #1 Architecture: `transport/` per-source parsers + `merge.rs` consensus layer;
  concrete `PgPool`.
- #1 DB access: compile-time macros; the `.query(&[...])` calls are HTTP query
  params (reqwest), not SQL.
- #2 Dead code: none found.
- #3 Async: network parsers async; no blocking on hot paths.
- #4 Stringly typing: `WeaponStat` enum + `FromStr` landed in M2 with tests.
- #6 Tests: comprehensive (cron, embeds, html, merge, per-source parsers).
