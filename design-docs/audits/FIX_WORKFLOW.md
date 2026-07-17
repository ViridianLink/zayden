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
   change staged but **uncommitted**. Do not commit, and do not start the next
   finding, until the human has reviewed and explicitly said go.
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
- **Status:** `open`            <!-- open | in-progress | in-review | fixed | wontfix -->
```

State meanings:

| Status        | Meaning |
|---------------|---------|
| `open`        | Not started. Eligible to be picked up. |
| `in-progress` | Work is underway on `main`. |
| `in-review`   | Fix complete, validation green, awaiting the human pause. |
| `fixed`       | Committed. Append the commit: `fixed — <sha>`. |
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
4. Confirm you are on `main` with a clean working tree (`git status`) — no
   leftover changes from a prior task — then confirm which finding is next per
   the [priority queue](#priority-queue).

Do **not** re-run the audit or hunt for new findings during a remediation
session. New defects discovered incidentally get **recorded** (append to the
relevant audit doc), not fixed inline — same discipline the audit itself used.

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
3. **Apply the smallest fix** that neutralises the scenario and honours the
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

## Step 4 — Record & stage (do **not** commit)

1. Update the finding's status to `in-review` in its audit doc.
2. Leave the change **on `main`, uncommitted**. Optionally `git add -A` to stage
   it so the human reviews a clean `git diff --staged`, but **do not run `git
   commit`** — the human reviews the working-tree diff first and commits
   themselves (or tells you to). Draft the commit message now so it's ready to
   hand over, e.g.:
   ```
   fix(gambling): scope stamina regen UPDATE with WHERE stamina < max (DS-8)

   Eliminates the full-table rewrite that deadlocked (40P01) against
   concurrent gameplay writes every 10 min. Adds regression test …

   Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
   ```
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
- **Proposed commit message** (from Step 4).
- **Suggested next task** from the queue — as a *proposal*, not an action.

Then wait. On the human's go:
- **approved** → the human commits (or tells you to `git commit` with the drafted
  message). Once committed, update the finding to `fixed — <sha>` and confirm
  the tree is clean before Step 1.
- **changes requested** → back to Step 2 with their notes.
- **declined** → set the finding to `wontfix — <reason>` and `git restore` the
  working tree.

Only after the tree is clean again, return to **Step 1** for the next finding.

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
| 20 | Structural `CC-1` (generic `async_trait` managers → concrete `PgPool`) | high (effort) | Large, cross-crate; schedule deliberately — it *camouflages* race hazards, so pairs well with the CC-9 work. |
| 21 | `CC-8` dashboard migrations, `CC-2`/`CC-5`/`CC-3` hygiene, per-module test gaps (`#6`) | med → low | Lowest urgency; batch by theme, still one finding per task. |

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
- [ ] Change left **on `main`, uncommitted**, with a drafted commit message.
- [ ] **HUMAN REVIEW PAUSE reached; waiting for go.**

---

## Initiating prompt

Paste this into a fresh Claude Code session (from the repo root) to start or
resume the workflow:

> Run the audit remediation workflow in
> `design-docs/audits/FIX_WORKFLOW.md`. Do the **intake** (Step 0): read the
> workflow doc, `README.md`, and `_cross-cutting.md`, then scan the per-module
> audit files for finding statuses.
>
> Work on **`main`** — a single agent, no branches. Handle exactly **one**
> finding: the top `open` item in the priority queue (or the one I name).
>
> **Stop at the two pause points.** First, at **task selection** (Step 1): tell
> me the finding, its failure scenario, and your one-line fix direction, then
> **wait for my go before writing any code.** Second, at **task completion**
> (Step 5): leave the change staged but **uncommitted** on `main`, present the
> review packet (root cause, diff surface, fails-before/passes-after test, the
> Step 3 gate results, residual risk, and a drafted commit message), and
> **wait** — I review the diff and commit myself.
>
> Enforce the `CLAUDE.md` gates before you call a task done: `cargo +nightly
> clippy --workspace --all-targets -- -D warnings`, `cargo test`, plus
> `cargo sqlx prepare --workspace -- --all-features` if SQL changed and
> `cargo machete` if a `Cargo.toml` dep list changed. No new
> `#[allow]`/`#[expect]`. Do not fix a second finding in the same run.
