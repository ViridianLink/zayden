# Audit: gambling

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

The largest module (~10.7k LOC, 63 src files). Now on concrete `PgPool` with
compile-time SQL macros, typed component ids, no inline test module, and 13
`tests/` files. The residual debt is the remaining `#[expect]` cluster (CC-3),
one reserved shop-item const, and the dashboard read-view direction.

## Findings

### 2b. `WEAPON_CRATE` is the same dead stub, hidden by `pub` instead of `#[expect]`  ·  #2  ·  low
- **Status:** `open`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Ruling (2026-07-29):** Do **not** delete reserved shop items — they are
  planned features. The standing rule the owner set here: *comment the const out
  only when it is flagged by an `#[expect]`; if it is already commented out,
  leave it.* `WEAPON_CRATE` satisfies neither trigger — it carries no `#[expect]`
  (being `pub` through a public module chain, the `dead_code` lint never fires)
  and its `SHOP_ITEMS` entry is already commented out (`items.rs:392`). So the
  finding is closed with no code change. The analysis below stands as the record
  of *why* the gate cannot see it, which is the part worth remembering.
- **Pre-checks run before the ruling (read-only, 2026-07-29):**
  `git log -S '    WEAPON_CRATE,'` on `items.rs` returns **no commit** — the
  `SHOP_ITEMS` entry was never uncommented, so no `gambling_inventory` row can
  hold `"weaponcrate"`. A workspace grep for `WEAPON_CRATE` / `weaponcrate` /
  `"Weapon Crate"` matches only `items.rs:157` and `:392`; nothing resolves the
  id and no use-handler exists.
- **Found:** 2026-07-29, while deleting `RIGGED_LUCK` for finding #2 / CC-4. Not
  part of that finding's text, so recorded separately rather than folded in
  (one finding → one task). Note that `RIGGED_LUCK` was *deleted* under the
  earlier approval; the ruling above supersedes that approach for any future
  reserved item.
- **Where:** `src/common/shop/items.rs:157` (the const) and `:393` (its
  commented-out `SHOP_ITEMS` entry).
- **What:** `pub const WEAPON_CRATE` is dead in exactly the way `RIGGED_LUCK`
  was — declared, commented out of `SHOP_ITEMS`, never purchasable. It carries
  **no** `#[expect(dead_code)]` only because it is `pub` and the module chain is
  public all the way out (`lib.rs:11 pub mod common` → `common/mod.rs:2 pub mod
  shop` → `shop/mod.rs:31 pub use items::*`), so rustc considers it reachable
  API and never fires the lint.
- **Why it matters:** This is the more dangerous shape of CC-4. The `#[expect]`
  on `RIGGED_LUCK` was at least a visible marker that something was dead;
  `WEAPON_CRATE` is dead code that is **structurally invisible to the gate** —
  no lint, no suppression to grep for. It also leaks a non-feature into the
  crate's public API, where an external `pub use items::*` consumer could
  legitimately depend on it.
- **Suggested fix:** Delete both lines, same as `RIGGED_LUCK`. Verify first with
  `git log -S '    WEAPON_CRATE,'` that it was never active in `SHOP_ITEMS`
  (`RIGGED_LUCK`'s check came back empty; this one was not run).
- **Follow-up worth considering:** a `tests/` catalogue-integrity check —
  `SHOP_ITEMS` ids are unique, and no `ShopItem` const exists outside
  `SHOP_ITEMS` — would make this whole class fail loudly instead of silently.
  Fits [CC-6](_cross-cutting.md#cc-6).

### 3. `#[expect]` cluster  ·  #7  ·  med
- **Status:** `open`
- **Where:** `src/utils.rs:85`, `src/models/mod.rs:74` (`cast_sign_loss`),
  `src/commands/tictactoe.rs:136,151`, `src/commands/gift.rs:37`,
  `src/common/shop/items.rs:47`, `src/games/lotto.rs:118`.
- **What / Why / Fix:** See [CC-3](_cross-cutting.md#cc-3). The
  `cast_sign_loss` on stamina points at a domain-type opportunity (an unsigned
  stamina type) rather than a cast suppression.

### 6. Leaderboard / profile are better as dashboard read-views  ·  #8  ·  low
- **Status:** `open`
- **Where:** `src/commands/leaderboard.rs`, `src/commands/profile.rs` (+ their
  `components/*` pagers).
- **What:** Data-dense, paged displays that a Discord embed renders poorly
  (button pagination, field limits).
- **Why it matters:** A web leaderboard/profile page is a strictly better view and
  offloads the pager-component complexity.
- **Suggested fix:** Add read-only dashboard views; **keep all games/economy
  actions in-bot** (they are live interactions). Lower priority than
  de-duplicating config writes. See [CC-8](_cross-cutting.md#cc-8).

## Clean
- #1 Architecture: concrete `PgPool`; no DB-generic manager trait.
- #1 DB access: compile-time `query!`/`query_as!` only (no runtime SQL).
- #3 Async: no blocking I/O; no locks across `.await` observed.
- #6 Tests: 13 integration files covering the economy paths (stamina, lotto,
  prestige, work, dig, craft, shop buy/sell, effects, custom ids).
- Wager games (`/blackjack`, `/higherorlower`) are safe from **intra-game**
  double-submit: `GameCache::check_and_set` (`game_cache.rs`) atomically rejects a
  repeat within 5s, and the bet is taken via the **atomic** guarded `bet`
  decrement (`coins = coins - $2 WHERE coins >= $2`).
