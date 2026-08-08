# Audit Remediation Workflow

> A **repeatable Claude Code workflow** for turning the read-only audit in this
> directory into landed fixes — **one finding at a time, with a mandatory human
> review pause after every task.**
>
> The audit ([`README.md`](README.md) playbook, [`_cross-cutting.md`](_cross-cutting.md),
> and the per-module `*.md` files) is the *backlog*. This document is the
> *procedure* for working that backlog down safely. It does not itself fix
> anything.

---

## Operating principles

1. **One finding → one task.** Never batch findings. The audit was written to
   keep fixes reviewable in isolation ([`README.md:20-22`](README.md)); this
   workflow preserves that. Work happens **directly on `main`** — a single agent
   works at a time, and the human reviews each task's diff **before it is
   committed**, so no branch/PR ceremony is needed.
2. **Stop after each task.** Every task ends at a **HUMAN REVIEW PAUSE** with the
   change **uncommitted**. Do not commit it yourself, and do not start the next
   finding, until the human has reviewed and explicitly said go. The review does
   not have to happen at the pause: the human may review a completed task **while
   the next task's agent is already running**. When they approve during that
   window they **stage** the approved files (`git add`) as the go-ahead signal and
   commit manually once the running agent finishes — so a task's files may be
   left *staged-and-approved* rather than committed. Never unstage or clobber
   changes a prior approved task left staged; treat them as belonging to that
   already-approved task, not the one you are working.
3. **The audit doc is the source of truth for status.** Task state lives inline
   next to each finding (see [Status convention](#status-convention)), so a fresh
   session can resume with zero external context.
4. **Evidence in, evidence out.** Each finding already carries a concrete failure
   scenario. The fix must neutralise *that* scenario, and the task must record
   *how it was verified* against it — ideally a regression test that fails before
   and passes after.
5. **Respect the workspace guardrails.** All of [`CLAUDE.md`](../../CLAUDE.md)
   applies — manual serenity framework (no poise), compile-time `sqlx` macros,
   no new `#[allow]`/`#[expect]`, `tests/` integration files (never inline
   `#[cfg(test)]`), disk-hygiene rules, and the SQLx offline-cache regeneration
   step.

---

## Status convention

Findings currently have **no status marker**. Before first use of this workflow,
each finding heading gets a one-line status tag appended directly beneath it, so
progress is visible in the audit docs themselves:

```markdown
### DS-8. Stamina cron `UPDATE` has no `WHERE` ...
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
```

State meanings:

| Status        | Meaning |
|---------------|---------|
| `open`        | Not started. Eligible to be picked up. |
| `in-progress` | Work is underway on `main`. |
| `in-review`   | Fix complete, validation green, awaiting the human's review. The review may land later — during a subsequent task's agent phase — not only at the pause. |
| `complete`    | Reviewed and approved. The human has committed it, or approved-and-**staged** it pending the current agent run's completion. Append the commit once it exists: `complete — <sha>`. |
| `wontfix`     | Deliberately declined. Append a one-line reason. |

Only **one** finding may be `in-progress` at a time.

---

## Step 0 — Intake (once per session)

Before touching code, load the audit into context in this order:

1. [`README.md`](README.md) — the playbook, the 8-point checklist vocabulary, and
   the "audit, don't fix; fixes are separate scoped branches" ground rule.
2. [`_cross-cutting.md`](_cross-cutting.md) — `CC-1…CC-9` themes and the six+ deep-
   sweep pass indexes. **Read this even for a module-specific fix:** most module
   `DS-#` findings are instances of a `CC-#` class, and the cross-cutting note
   often records *why* a naive fix is wrong (e.g. CC-9's absolute-overwrite vs.
   atomic-increment distinction).
3. The per-module `<module>.md` for the finding you intend to work.
4. Confirm you are on `main` then confirm which finding is next per
   the [priority queue](#priority-queue).

---

## Step 1 — Select the next task

Pick the **top `open` finding** in the [priority queue](#priority-queue). If the
human named a specific finding, that wins. Announce the choice and its failure
scenario in one or two sentences, then set its status to `in-progress` in the
audit doc. Work proceeds on `main`; no branch is cut.

> ### ⏸ HUMAN REVIEW PAUSE — task selection
> State: **which finding**, **the failure scenario it fixes**, and **the one-line
> fix direction** you intend to take. **Wait for the human to confirm the finding
> and the approach before writing any code.** This is the cheapest place to catch
> a wrong direction (e.g. "absolute overwrite" vs. "atomic increment", or "this
> is dead code, delete it instead").

---

## Step 2 — Reproduce, then fix

1. **Pin the failure.** Re-read the cited `path:line`. Confirm the scenario still
   holds against current code (findings are timestamped; code may have moved).
   If it no longer reproduces, mark the finding `wontfix — no longer reproduces
   as of <sha>` and return to the pause in Step 1.
2. **Write the regression test first** where feasible — in a `tests/` integration
   file, **never** inline `#[cfg(test)]` in `src/` (project convention; see
   [`README.md:66-70`](README.md) checklist item #6). It must fail for the reason
   the finding describes.
3. **Apply the proper fix** that neutralises the scenario and honours the
   cross-cutting guidance. Prefer scoped iteration — `cargo +nightly clippy -p
   <crate>` and `cargo +nightly check -p <crate>` while working — not the full
   workspace gate on every edit ([`CLAUDE.md` disk-hygiene §3](../../CLAUDE.md)).
4. **If the change touches SQL** (`query!`/`query_as!` added, removed, or
   changed): regenerate the offline cache with **all features** and stage the
   `.sqlx/` diff:
   ```
   cargo sqlx prepare --workspace -- --all-features
   ```
5. **If the change touches a `Cargo.toml` dependency list:** run `cargo machete`.

---

## Step 3 — Validate (the CLAUDE.md gate)

Run the full mandated gate before declaring the task done. **All must pass with
no new `#[allow]`/`#[expect]`:**

```
cargo +nightly clippy --workspace --all-targets -- -D warnings
cargo test
```

Plus, conditionally:

```
cargo sqlx prepare --workspace -- --all-features   # if SQL changed (then commit .sqlx/)
cargo machete                                        # if a Cargo.toml dep list changed
```

If a finding lives in the dashboard's feature-gated code, also run the relevant
feature check(s) from [`CLAUDE.md`](../../CLAUDE.md) (`-p dashboard --features
ssr`, and the wasm/hydrate check where hydration code changed).

Record the **actual** results. If a gate fails, it failed — fix it or report it;
do not narrate green over red.

---

## Step 4 — Record (do **not** commit)

1. Update the finding's status to `in-review` in its audit doc.
2. Leave the change **on `main`, uncommitted**. **Do not run `git commit`** — the
   human reviews the working-tree diff and either commits, or (if reviewing while
   a later task's agent is already running) stages the approved files and commits
   once that agent run finishes.
3. Prepare a short **review packet** for the human (see Step 5).

---

## Step 5 — ⏸ HUMAN REVIEW PAUSE — task complete (mandatory)

**Stop here. The change is uncommitted. Do not commit and do not pick up the
next finding.** Present:

- **Finding:** id + one-line title.
- **Root cause & fix:** 2–3 sentences.
- **Diff surface:** files touched, and the `.sqlx/` / `Cargo.toml` deltas if any.
- **Verification:** the regression test (fails-before / passes-after) and the
  exact gate results from Step 3.
- **Residual risk / follow-ups:** anything the fix deliberately left (e.g. "DS-8
  scoped the WHERE clause but the 40P01 retry wrapper is a separate follow-up"),
  recorded as a new finding if warranted.
- **Suggested next task** from the queue — as a *proposal*, not an action.

Then wait. On the human's go:
- **approved** → the human commits themselves, or — if they are reviewing while a
  later task's agent is still running — **stages** the approved files and commits
  once that run completes. Update the finding to `complete — <sha>` once the
  commit exists. A resuming agent may find a prior task already committed (or
  approved-and-staged) with its status still `in-review`; reconcile that stale
  marker to `complete` rather than treating the finding as unfinished. Confirm
  the working tree is clean of *your* task's changes before Step 1.
- **changes requested** → back to Step 2 with their notes.
- **declined** → set the finding to `wontfix — <reason>` and `git restore` the
  working tree.

Only after the tree is clean of the current task, return to **Step 1** for the
next finding.

---

## Priority queue

Ordering rule: **prod-confirmed** first, then **severity** (high → med → low),
then **blast radius** (data loss / economy integrity > user-visible breakage >
hygiene), then **structural enablers** that unblock later fixes. The three
`confirmed (prod)` findings from the log-driven seventh pass lead.

| # | Finding | Sev | Why it's ranked here |
|---|---------|-----|----------------------|
| 1 | [gambling DS-8](gambling.md) — stamina `UPDATE` no `WHERE` | high | **Prod deadlock (40P01) + slow-statement, now.** Full-table churn on the hottest table. |
| 2 | [gambling DS-1](gambling.md) — `/send` non-atomic transfer | high | Coins minted from nothing; economy integrity. |
| 3 | [gambling DS-2](gambling.md) — `/gift` double-submit double-mint | high | Free mint via double-submit; economy integrity. |
| 4 | [destiny2 DS-1](destiny2.md) — `RwLock` write guard held across upload+sleep | high | Global bot-state stall up to ~50 s; async foot-gun. |
| 5 | [destiny2 DS-3](destiny2.md) — parse-drop + `TRUNCATE` replace | med | **Prod parse failures**; each drop is destructive (tierlist erodes). |
| 6 | [gambling DS-6](gambling.md) — lotto `WeightedIndex` rebuilt after final pick | med | **Prod cron failure** at exactly 3 participants; whole draw rolls back. |
| 7 | [CC-9](_cross-cutting.md) — absolute-overwrite race class | high | Umbrella for DS-5/DS-7 (gambling), gold-star DS-1, temp-voice DS-2, etc. Fix the *pattern*; see note below. |
| 8 | [gambling DS-5](gambling.md) — `bet` decrement no `WHERE coins>=bet` | med | Overdraft via cross-command race. |
| 9 | [suggestions DS-1](suggestions.md) — flipped demote threshold | med | Feature inert (downvoted never leave review) + full-channel scan. |
| 10 | [config DS-1](config.md) — `grant` writes tier not aggregate max | med | Silent entitlement downgrade. |
| 11 | [family DS-1](family.md) — `/block` never enforced | med | Whole block feature inert. |
| 12 | [temp-voice DS-1](temp-voice.md) — claim/transfer keeps old owner perms | med | Stale permission grants. |
| 13 | [lfg DS-1](lfg.md) — fireteam capacity race | med | Overfills past `fireteam_size`. |
| 14 | [ticket DS-1](ticket.md) — FAQ select >25 options breaks | med | Hard Discord limit; `/support list` 400s. |
| 15 | [gold-star DS-1](gold-star.md) — `/give_star` RMW races | med | Star mint/loss/cap-bypass (instance of CC-9). |
| 16 | [gambling DS-3](gambling.md), [DS-4](gambling.md), [DS-7](gambling.md) | med / low-med | Prestige/lotto `ON CONFLICT` + confirm double-submit + daily/work overwrite. |
| 17 | [bot DS-1](bot.md) — level-up coin reward lost on co-future error | med | Reward silently dropped. |
| 18 | [bot DS-2](bot.md) — orphaned `moderation` tree | med | **Decide first: revive or delete.** Dead feature + 3 latent bugs. Likely a `wontfix`/delete, not a fix. |
| 19 | Remaining `DS-#` (music, marathon, reaction-roles, family DS-2, temp-voice DS-2, ticket DS-2) | low-med → low | Work down by severity. |
| 20 | ~~Structural `CC-1` (generic `async_trait` managers → concrete `PgPool`)~~ | — | **Done.** Closed piecemeal by the eight per-module migrations; umbrella reconciled 2026-07-29. Its `bot` #3 sub-item closed with it. |
| 21 | `CC-8` dashboard migrations, ~~`CC-2`/`CC-5`/`CC-3` hygiene~~, per-module test gaps (`#6` / `CC-6`) | med → low | Lowest urgency; batch by theme, still one finding per task. `CC-2` (`de1238d8`), `CC-5` (`6049775d`) and `CC-3` (`3d787146`) are done. |

**Queue status as of 2026-07-29 (`bf0d90ff`):** rows **1–20 are complete** — every
deep-sweep `DS-#` finding in the workspace now carries a commit sha. What remains
is the first-pass structural/hygiene backlog, which was never given status tags —
plus (2026-07-31) one **new** finding that the CC-6 close-out surfaced:

> **Intake note (2026-08-05, `33281cd4`):** the tagged backlog reached **zero
> `open` findings** on this date, so selection now has to re-derive the queue
> from the *untagged* first-pass items and check each against the tree. Doing
> that closed four on sight — [music #1](music.md) and
> [dashboard #2](dashboard.md) by CC-3 (the login-route `#[expect]` is one of its
> 17 *justified* keeps, not a survivor), [dashboard #1](dashboard.md) by CC-5
> (`middleware/auth.rs:34` is a `query_scalar!` now), [palworld #1](palworld.md)
> by CC-2 — leaving palworld #2 as the only untagged *defect* that was neither
> feature work nor a human call. **An empty `open` list is not an empty backlog:**
> it means the remaining work is untagged, and two findings (`CC-10`,
> [palworld #4](palworld.md)) came out of that pass. Tag what you close.

> **Intake note (2026-08-06, `2503adc1`):** the untagged list is now down to
> **feature work and documentation**. Of the 20 untagged headings, all but seven
> are already closed by a cross-cutting fix (CC-2, CC-3, CC-5, CC-6, CC-7); of the
> survivors, `destiny2 #1`, `CC-8`, `destiny2 #4`, `gambling #6`, `palworld #3`
> and `dashboard #3` are feature work, `destiny2 #3` and `marathon #1` are
> documentation, and `gambling #5` closed on sight. bot #4 was the last untagged
> **defect-shaped** item, and it is now `in-review`. **What that means for the
> next selection:** the "confirm X" seam that produced five straight findings is
> exhausted. The remaining defect surface is the unaudited
> [honeypot](README.md) crate — the only code in the workspace that has never had
> an 8-point pass, and therefore the only place a *new* `DS-#` can come from
> without re-auditing. Everything else in the queue is a build, not a fix.

> **Intake note (2026-08-07, `7e75cc79`):** the tagged backlog was at **zero
> `open` findings** again, and this time the untagged list held nothing
> defect-shaped — the 2026-08-06 note's prediction held exactly. So the pass ran
> the thing that note pointed at: the **8-point audit of `honeypot`**, the last
> unaudited crate. That is a [`README.md`](README.md) task, not a FIX_WORKFLOW
> one, and it is worth recording that the distinction is now load-bearing: **an
> empty fix queue is a signal to audit, not a signal to stop.** The workspace is
> now at full 8-point coverage, and the queue has nine `open` findings again.
>
> **What the pass found, and what it says about the audit method.** The crate is
> structurally the cleanest new code in the workspace — no CC-1/CC-2/CC-3/CC-5
> debt, zero `#[expect]`, settings correctly on the shared `SettingsStore`, and
> a `tests/policy.rs` whose comments record *why* each exemption case is what it
> is. It would pass a checklist skim. Both `med` defects
> ([#1](honeypot.md), [#2](honeypot.md)) sit in the ~40 lines that test file
> deliberately stops short of: the **action path**. The lesson is the inverse of
> [bot #4](bot.md)'s — that one showed a self-flagged TODO is a hypothesis; this
> one shows **a good test file marks its own blind spot.** `tests/policy.rs`
> covers the decision and not the act, so the act is where two defects lived.
> Read what a test suite *declines* to cover as the audit's first lead.
>
> **Second lesson, on scoping a finding.** [#3](honeypot.md) presents as a
> honeypot bug (the bot command seeds the `guilds` parent row, the dashboard path
> does not) but every `*_settings` table FKs `guilds(id)`, so all nine share the
> hazard — honeypot is merely where the **asymmetry between two writers** makes
> it visible. Fixing it here would be the low-churn path and would leave eight
> tables exposed. Where a module finding's root cause is workspace-shaped, open
> the CC first and let the module finding close with it; the `guilds` seed wants
> one owner (`SettingsStore` or `guild_create`), which is the CC-5 / ticket #2
> "one owner for the SQL" pattern again.
>
> **Also reconciled:** [bot.md](bot.md) records bot #4 as `complete — 12f6b01e`,
> a sha not on `main`. The commit was amended into **`7e75cc79`** with identical
> content (verified: `git show --stat` on both lists the same 11 paths and the
> same `src/` deltas; only the audit-doc line counts differ). Marker corrected —
> and note this is the 2026-07-29 "reconcile against the tree, not the record"
> lesson catching a *sha*, not a claim: a fix note can name a commit that no
> longer exists without anything being wrong with the fix.

| Finding | Sev | Note |
|---------|-----|------|
| ~~[`CC-4`](_cross-cutting.md) — `RIGGED_LUCK` dead const + its `#[expect(dead_code)]`~~ | low | **Done (`a2c6f652`).** `GameState` half died with CC-1 (`83930148`); the `items.rs:192` half was deleted here. Its follow-up [gambling #2b](gambling.md) (`WEAPON_CRATE`) is **`wontfix`** — see the policy note below. |
| ~~[`CC-6`](_cross-cutting.md) — test-coverage gaps~~ | med | **3 of 3 done; `in-review`.** [`gold-star`](gold-star.md) (`9a7b8795`) built the workspace's first `#[sqlx::test]` harness — so `cargo test` needs a throwaway `DATABASE_URL`; [`llamad2`](llamad2.md) (`b5cc3faf`) split DB-backed from offline tests; [`verify`](verify.md) is **`wontfix`** — no independently testable surface, ruled explicitly 2026-07-31 rather than closed with trivia. That pass recorded a **new** finding, [verify #2](verify.md) (`open`). |
| ~~[verify #2](verify.md) — `VERIFIED_ROLE` hardcoded + duplicated~~ | med | **Done (`882dfec6`).** Fixed as directed: `migrations/0023_verified_role` adds `guild_settings.verified_role_id`, read through the roles `SettingsRow`/`SettingsStore` by a single shared `verify::verified_role()` that both the button and `/manverify` call; both constants deleted; `VerifyError::RoleNotConfigured` covers the null column; dashboard Roles field added. Status marker was left stale at `in-progress` and reconciled 2026-08-04. |
| ~~[`CC-7`](_cross-cutting.md) — `custom_id` string routing~~ | low | **Done (`c794fe8f`).** Four `*CustomId` enums in `bot-modules/gambling/src/components/custom_id.rs` following the `LevelsCustomId` (`04a8ab2b`) precedent; producers and routers now share one source. It also closed a live defect the untyped ids were hiding (non-owner tictactoe cancel falling through to the coordinate catch-all → `GamblingError::NotYourGame`). Status marker was left stale at `in-review` and reconciled 2026-08-04. |
| ~~[ai #1](ai.md) — outbound LLM call had no timeout~~ | low | **Done (`df39b21d`).** Not in the queue before: a first-pass "confirm X" item that was never status-tagged, and the confirmation failed — `AiClient` built its own header-carrying `reqwest` client with no timeout, so a silent provider left the chat future pending forever. Fixed by giving the workspace's timeout budget one owner (`zayden-app::services::http::ClientBuilderExt::with_timeouts`) that both `AppState::http_client` and `AiClient::new` chain onto their builders. Status marker was left stale at `in-review` and reconciled 2026-08-04. |
| ~~[music #2](music.md) — resolver calls had no timeout~~ | low | **Done (`33281cd4`).** Status marker was left stale at `in-review` and reconciled 2026-08-05 (verified against the tree: `resolve/http.rs`, `tests/resolver_timeouts.rs`, and the fallible `RadioResolver::new` at `bot/src/main.rs:95` are all present). The second untagged "confirm X" item, picked up on the `ai` #1 precedent — and it failed the same way, twice over: both streaming `reqwest` clients were bare `Client::new()`, and the `yt-dlp` child was awaited with no budget. Its lesson is the inverse of `ai` #1's: **one owner for a budget does not mean one budget.** Reusing `with_timeouts()` here would have capped every track at 30 s, because a streaming body needs a *per-read* budget where a request/response needs a *total* one. A test now guards that distinction. Spotify's third of the finding checked out clean (`rspotify` sets its own 10 s). |
| ~~[palworld #2](palworld.md) — upload sweep unlinked on the reactor~~ | low → **med in practice** | **Done (`916dbcbf`).** Status marker was left stale at `in-review` and reconciled 2026-08-05 (verified against the tree: `916dbcbf` carries the `cron.rs` rewrite and `tests/upload_sweep.rs`). The third untagged "confirm X" item, and the third to fail its confirmation. Its lesson: a confirm finding's *cited surface* is its weakest part. The audit sized this as "a single `remove_file`, usually tolerable"; the code had since moved to `remove_dir_all` on the uploader's whole directory (≤100 MB), on the reactor, every minute — and `bot/src/cron.rs` `join_all`s every job due in the same tick onto one task, so the stall was never confined to palworld. The finding's *other* half (`save/mod.rs`) confirmed clean exactly as predicted, so a failed confirm is not a reason to distrust the whole finding. Surfaced one new finding, [palworld #4](palworld.md) (`open`). |
| ~~[`CC-10`](_cross-cutting.md) — `.sqlx` cache drifted on `main`~~ | med | **Done (`c294c4b6`).** Status marker was left stale at `in-review` and reconciled 2026-08-06 (verified against the tree: the commit touches exactly `.sqlx/query-905f7d2f…json` — the single entry the fix note names — plus the audit docs, and no Rust source, matching its "1 modified, 0 added, 0 deleted" claim). The collaborative task it was billed as: the human authorised a throwaway `postgres:18-alpine`, and `--check` went red against an empty migrated DB exactly as claimed. **1 file modified, 0 added, 0 deleted** — one inverted `nullable` array on lfg's `LEFT JOIN` (`edit.rs:50`), not the three entries CC-5 estimated. Its lesson: **a hash absent from `.sqlx` is not evidence of drift.** This finding's own offline evidence — "no entry starts with `query-895e6b8`" — was wrong in the way that matters: the regen added nothing, so that query no longer exists in the source. Only a regen distinguishes *missing* from *retired*. Second lesson: the fix changed **no behaviour and could not have**, since every column carries an explicit `!`/`?` override that supersedes the cached inference. A structural enabler is allowed to fix zero bugs — restoring the gate *is* the deliverable — but it must be recorded that way rather than dressed up as a defect fix. |
| ~~[palworld #4](palworld.md) — `stat`/`read_dir` on async command paths~~ | low | **Done (`6d51f6fc`).** Status marker was left stale at `in-review` and reconciled 2026-08-05 (verified against the tree: the commit carries `save/dps.rs`, `save/mod.rs`, `tests/cache_key_offload.rs`, and `client.rs:330` now awaits an async `file_mtime`; the surviving `std::fs::metadata` at `:634` is `local_mtime_secs`, already inside `spawn_blocking` as the fix note predicted). Recorded the same day by the palworld #2 close-out. Same class, far smaller: bounded metadata syscalls, but on user-facing `/pal` paths and on every call including cache hits, since the mtime *is* the cache key. Its lesson: **one finding does not mean one corrective shape.** The two cited sites got different fixes — a single stat that gates whether a blocking task is spawned at all became `tokio::fs` (`level_mtime`'s precedent), while the unbounded `read_dir` + per-file stat got its own single `spawn_blocking` for the whole batch. Folding the one-stat case into a `spawn_blocking` would have meant spawning a blocking task to decide whether to spawn one. Also note the *test* needed an idea the fix did not: `player_record`'s downstream offload masks the defect on the happy path, so the guard asks for a **missing** save, where the lookup short-circuits and nothing downstream can hide the inline stat. |
| ~~[temp-voice #4](temp-voice.md) — `actions` layer untested~~ | med | **Done (`2503adc1`).** Status marker was left stale at `in-review` and reconciled 2026-08-06 (verified against the tree: the commit carries `tests/actions_authz.rs` and the `tokio` dev-dep, and touches no `src/` file, matching its "pure regression net" claim). First untagged item picked by *severity + blast radius* rather than the "confirm X" pattern: an untested authz surface outranked gambling #5's generic thinness and destiny2 #1's port-to-DB feature work. Never in CC-6's scope — CC-6's `Where` list was *zero-test* crates, and temp-voice had four test files, none touching `actions/`. Unlike the four preceding untagged items, **the confirmation held: no defect.** The gate mapping was already correct, so this is a pure regression net (15 tests, no `src/` change). Two lessons. **(1)** Assert the exact `PermissionError` variant, not "an error" — an outsider is rejected by *either* guard, so only a **trusted-non-owner** caller can tell `require_owner` from `require_trusted`, which is the actual escalation. **(2)** In the guard-removal matrix CC-6 prescribes, a **build error is an invalid mutant, not a catch** — the first pass scored a bogus 21/21 by counting 10 non-compiling mutations as passes; repairing their `use` lines and re-running turned them into 10 real catches. |
| ~~[bot #4](bot.md) — orphaned `reset.rs` + cron `retain` deletes jobs~~ | low → **med in practice** | **Done (`12f6b01e`).** The fifth untagged "confirm X" item and the fifth to fail its confirmation — this time on **both** halves, in opposite directions. Half 1 evaporated: all 6 cited `unwrap()`s live in `bot/src/reset.rs`, never `mod`-declared, never compiled — nothing to audit, so the file was deleted (orphaned-tree class, same as [bot DS-2](bot.md)). Half 2 was the reverse: the TODO guessed both retain conditions were redundant, but `includes(t)` is load-bearing — `jiff_cron 0.3.0`'s `next_after` fast path skips the day-of-week check, so a `Fri`-only schedule sampled at Thu 16:59:59.5 proposes *Thursday* 17:00, and the old code fed that rejection into `jobs_mut().retain(...)`, **permanently deleting the job**. Four weekday-restricted jobs were exposed (gambling `lotto`/`higherlower`, destiny2 `endgame`, marathon `announce`). Its lesson: **a self-flagged TODO is a hypothesis, not a finding.** This one named the right line and got the conclusion backwards; had it been closed as "verified redundant" the retain would have stayed. Reproduce the predicate against the library before believing either the TODO or the audit — the third pass had already traced this scheduler "clean", so two prior records agreed on the wrong answer. **Second lesson, and this one cost a review round:** deleting the thing you proved wrong is not the same as deleting the thing it was *doing*. The `retain` was both a broken guard and the registry's **only** garbage collector — LFG reminders are year-pinned one-shots (`reminders.rs:28-106`) that nothing else ever removes — so removing it wholesale fixed the guard and silently leaked four `CronJob`s per LFG post. The human caught it at the Step 5 pause; no gate could have. The fix splits the two predicates: `earliest_pending` (shared slice, cannot remove) and `prune_exhausted` (removes only on `.next().is_none()`, never on an `includes` rejection). Third: the extraction that makes `bot` testable at all (no `[lib]` target) is the CC-2 `DispatchMap` move. |
| ~~[gambling #5](gambling.md) — thin test coverage for size~~ | med | **Closed on sight (2026-08-06)**, during this pass's queue re-derivation. `tests/` holds **11** files now, not the one the finding counted, covering exactly the payout/odds/stamina math it prescribed. No task worked it — the CC-9 deep-sweep tasks (DS-9…DS-15) and CC-7 each shipped a test with their fix. The counter-case to the 2026-07-29 lesson: that one caught a note claiming work never done, this catches work done without a note. Both need the tree. |
| [`CC-8`](_cross-cutting.md) — dashboard config pages | med | Largest. The active-duplication half is closed (both `setup` commands removed); what's left is *building* music/ticket/suggestions/reaction-roles pages — feature work, not a defect fix. |
| [`CC-9`](_cross-cutting.md) umbrella | high | Every enumerated site closed. `in-review` — **the human's call to close**, not an agent task. |
| ~~[honeypot](README.md) — never audited~~ | ? → **known** | **Done (2026-08-07, uncommitted).** The audit pass ran; see [honeypot.md](honeypot.md) and the intake note below. Its nine findings are now the queue's only `open` items and are listed immediately below. |
| [`CC-11`](_cross-cutting.md) — `.sqlx` entry hand-edited, `SQLX_OFFLINE` builds broken | med | **Top of the queue — structural enabler, and it blocks the `cargo test` gate for every task.** Recorded 2026-08-07 by the honeypot #2 task, which hit it as a hard blocker. Needs a regen against an empty migrated DB (the human's step). |
| ~~[honeypot #1](honeypot.md) — failed `unban` leaves a permanent ban~~ | med | **Done (`9b034aa6`).** Fixed by the human directly rather than as an agent task, in the same commit that landed the audit file; marker reconciled `in-review` → `complete` 2026-08-07 against the tree. Extracted the retry into `zayden-core::retry` and gave `guild_create` the same owner. |
| ~~[honeypot #2](honeypot.md) — `GUARD.claim` non-atomic check-then-act~~ | med | **`in-review` (2026-08-07, uncommitted).** CC-9/double-submit shape on a `moka` cache. Fails-before was **221 winners across 128 floods**, not a marginal race. |
| ~~[`CC-11`](_cross-cutting.md) — `.sqlx` drifted again; `SQLX_OFFLINE` builds broken~~ | **high** | **`in-review` (2026-08-07, uncommitted).** `docker/Dockerfile.bot` could not build; regenerated against an empty migrated DB (1 added, 1 deleted, 237 total). The finding's stated mechanism was **wrong and is corrected in place** — `c76ea93c` *did* regen, but the regen raced a reformat of the `.sql`, so cache and source were committed disagreeing over whitespace. Left open as a follow-up: **nothing in CI checks this**, which is why it reached `main`. |
| ~~[honeypot #3](honeypot.md) — dashboard settings write can violate the `guilds` FK~~ | med | **Done (`14784257`).** Fixed at the cross-cutting level as the finding asked: one `SettingsStore::seed_guild` owner covering all ten settings tables and both writers; honeypot's ad-hoc `INSERT` deleted. 4 tests, 3 fails-before. Marker was left stale at `in-review` and reconciled 2026-08-07 against the tree. |
| [honeypot #7](honeypot.md) — action path + authz gate untested | med | The untested surface is exactly where #1 and #2 survived. Sequence it *after* #1/#2, not before — #2 ships its own guard test and #1's fix determines what seam `message_create` can be tested through. |
| ~~[honeypot #5](honeypot.md) — `/honeypot set` duplicates the dashboard form~~ | med | **`in-review` (2026-08-08, uncommitted).** Owner ruled **keep both surfaces, converge the write** — the reaction-roles #3 shape, against the finding's own "retire `set`/`disable`" recommendation (arming is setup, but *dis*arming is incident response). One owner in `honeypot::settings`; both editors call it. **It was not behaviour-neutral:** the two writers *disagreed*, and the dashboard's `parse_id` mapped garbage and blank alike to `None`, so a garbled channel id silently disarmed the trap and reported success. 10 tests; fails-before by mutation (4/10, then 1/10). |
| [honeypot #4](honeypot.md) — ban reason literal duplicated across two crates | low-med | [verify #2](verify.md) class, smaller blast radius (desyncs the Discord audit log from `/logs`, breaks no feature). |
| [honeypot #6](honeypot.md), [#8](honeypot.md), [#9](honeypot.md) | low | Dead `HoneypotHit.channel_id` (gambling #2b blind spot); `GUARD.forget` clearing the wrong cache; hardcoded 24 h `PURGE_WINDOW`. Batch or fold into adjacent work. |

> **Policy recorded 2026-07-29 (reserved/dead consts):** do **not** delete
> reserved catalogue items (shop items and the like) — they are planned features.
> Comment the declaration out **only** when an `#[expect]` is flagging it; if it
> is already commented out of its registry, leave it alone and move to the next
> finding. This closed [gambling #2b](gambling.md) as `wontfix`. It does not
> retroactively apply to `RIGGED_LUCK`, deleted under the earlier approval in
> `a2c6f652`.

> **Lesson recorded 2026-07-29:** the CC-1 reconcile found a fix note asserting a
> deletion that never happened (CC-3 ↔ `RIGGED_LUCK`). A satisfied
> `#[expect(dead_code)]` keeps the clippy gate green, so the gate cannot catch a
> stale suppression — **reconcile fix records against the tree, not against the
> record.** Prefer citing a `path:line` or `git log -S` check in a fix note over
> asserting an outcome.

> **Note on #7 (CC-9):** do not "fix" a race by swapping one absolute overwrite
> for another. The cross-cutting record distinguishes **read-modify-write +
> absolute `save`** (racy) from **atomic `col = col + $n` / guarded
> `WHERE`** (correct). Each CC-9 site is its own task, but they share the same
> corrective pattern — reference [`_cross-cutting.md` CC-9](_cross-cutting.md)
> in each.

---

## Definition of done (per task)

- [ ] Failure scenario reproduced, then neutralised.
- [ ] Regression test added in a `tests/` integration file (fails-before /
      passes-after), or an explicit note on why a test isn't feasible.
- [ ] `cargo +nightly clippy --workspace --all-targets -- -D warnings` clean, no
      new `#[allow]`/`#[expect]`.
- [ ] `cargo test` green.
- [ ] `.sqlx/` regenerated (`--all-features`) **iff** SQL changed.
- [ ] `cargo machete` clean **iff** a dependency list changed.
- [ ] Finding status updated to `in-review` in its audit doc.
- [ ] Change left **on `main`, uncommitted**.
- [ ] **HUMAN REVIEW PAUSE reached; waiting for go.**
