# Game server hosting module — `/server host <game>`

## Context

Zayden has no path from "someone in Discord wants a game server" to a running
server. Today that is manual: open the Pelican panel, create a user, pick a node
and allocation, create the server from an egg, copy the IP into chat, chase
payment out of band.

This adds a `hosting` bot module: `/server host <game>` opens a modal with
resource dropdowns and live-priced upgrades, provisions a real server through
Pelican's Application API, DMs panel credentials, runs a free trial, and converts
to monthly via Ko-fi — with Pro/Ultra subscribers getting an included server.

Decisions taken with the user:

| Question                   | Decision                                                              |
| -------------------------- | --------------------------------------------------------------------- |
| Provisioning               | Full auto-provision via Pelican Application API                       |
| Trial end / lapsed payment | Suspend, then delete after grace                                      |
| Ko-fi matching             | Ko-fi tier + email→Discord, claim code to disambiguate                |
| Access                     | Open to anyone, per-user cap scaled by tier                           |
| Spec UX                    | Dropdowns per resource; base price text component, `(+£X)` per option |
| Billing                    | Per-server rows; hosting **reads** entitlements but never grants them |
| Panel access               | Bot creates a Pelican user and DMs credentials                        |
| Pro                        | Small server included free, one rung off higher plans                 |
| Ultra                      | Medium server included free, two rungs off higher plans               |

## Verified infrastructure (`root@10.0.3.99`)

Proxmox host, Ryzen 7 5800X (8C/16T), 32 GB RAM. Pelican runs as two LXCs:
`pelican-panel` (106, `10.0.3.206`) and `pelican-wings` (108, `10.0.3.208`).

| Resource          | Reality                                                                                                                            | Consequence                                      |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| Host RAM          | 32 GB total, 11.4 GB used, **20.5 GB available**                                                                                   | ~16 GB safely sellable                           |
| Wings LXC RAM cap | 24 GiB                                                                                                                             | Above real availability — host is the true limit |
| Wings rootfs      | **30 GB (25 GB free)**                                                                                                             | **Blocker: node advertises 100 GB**              |
| LVM thin pool     | 1.71 TiB, 11% used (~1.5 TiB free)                                                                                                 | Resize is trivial                                |
| Wings CPU         | 8 cores, shared                                                                                                                    | ~8 × 100% CPU units to sell                      |
| Free allocations  | **13, all unassigned**                                                                                                             | Hard ceiling of 13 concurrent servers            |
| Eggs installed    | 18 (Paper, Vanilla/Forge/NeoForge MC, Rust, Palworld, Satisfactory, TF2, Gmod, Insurgency, Necesse, Space Engineers, Mumble, TS3…) | Catalog draws from these ids                     |

Wings currently hosts **zero** servers — this is a clean start.

### Ops prerequisites (not code; do these first)

1. **Resize the wings rootfs** — it is the binding storage constraint and the node
   is already over-committed (advertises 100 GB over a 30 GB filesystem):
   ```bash
   ssh root@10.0.3.99 'pct resize 108 rootfs +370G'
   ```
2. **Create more allocations** on node 1 if more than 13 concurrent servers are
   wanted (`POST /api/application/nodes/1/allocations`). Existing ones are aliased
   to specific games, so hosting should create its own range rather than consume
   them blindly.
3. **Create 4 Ko-fi membership tiers** matching the price ladder below, named
   exactly as the config expects.
4. **Add `KOFI_VERIFICATION_TOKEN` to Doppler** — it is absent from the `zayden/dev`
   config, so the existing Ko-fi webhook cannot currently verify anything.

## Corrections to earlier assumptions

- **One Pelican key is enough.** `PELICAN_API_KEY` in Doppler (prefix `pacc_`)
  returns 200 on both `/api/client` and `/api/application/users`. No
  `PELICAN_APP_API_KEY`, no new env var.
- **`[pelican]` config must stay.** The save _editor_ was removed, but the save
  _refresh_ is still live — `refresh_shared_if_stale()` is reached from the
  Palworld roster path ([client.rs:273](bot-modules/palworld/src/client.rs:273)),
  and `Pelican` is still constructed in [state.rs:73](bot/src/state.rs:73).
  `server_id`/`save_path` stay as-is; hosting reuses only `base_url` + `api_key`.
  Lift `PelicanConfig` so both consumers share it rather than duplicating.
- **Ko-fi has no write API.** Webhooks are outbound-only; membership tiers are
  created by hand in the Ko-fi UI. There is no endpoint to create a tier, query a
  subscription, or link an account. This is _not_ a blocker — it reshapes matching
  for the better (below).

### Existing prior art to reuse

- Ko-fi webhook receiver already runs at `POST /webhooks/kofi`
  ([routes_kofi.rs](dashboard/src/web/routes_kofi.rs)); it verifies the token and
  already maps Ko-fi email → Discord via `kofi_links`.
- `EntitlementService` / `Tier::{Free,Pro,Ultra}` exists and is the standard gate.
- Sweep idiom: `WITH due AS (… FOR UPDATE SKIP LOCKED) UPDATE … RETURNING`
  ([sweep.rs](bot-modules/ticket/src/idle/sweep.rs)).
- **Delete the stale `servers-at-home` path dep** (`Cargo.toml:161`) — the
  directory no longer exists.

## Pricing model

Budget positioning: unverified operation, one box, friend groups. Commercial
hosts charge ~£3–5/GB/month; this sits near **£1.25/GB**.

Every reachable price lands on a **4-rung ladder**, so exactly four Ko-fi tiers
cover every combination — which is what makes fixed-amount Ko-fi subscriptions
workable at all:

**Ladder: £2.50 · £5.00 · £7.50 · £10.00**

| Plan   | RAM  | CPU  | Disk  | Free   | Pro          | Ultra        |
| ------ | ---- | ---- | ----- | ------ | ------------ | ------------ |
| Small  | 2 GB | 100% | 10 GB | £2.50  | **included** | **included** |
| Medium | 4 GB | 150% | 20 GB | £5.00  | £2.50        | **included** |
| Large  | 6 GB | 200% | 30 GB | £7.50  | £5.00        | £2.50        |
| XL     | 8 GB | 250% | 40 GB | £10.00 | £7.50        | £5.00        |

Pro drops one rung, Ultra two; the included plan costs £0. Concurrent-server cap:
Free 1, Pro 2, Ultra 3 — bounded globally by 13 allocations and ~16 GB RAM.

**Consequence:** when the computed price is £0 the server is billed against the
user's existing Pro/Ultra subscription, not Ko-fi. `hosted_servers` therefore
carries `billing_source` (`kofi` | `entitlement` | `trial`), and entitlement-billed
servers take their expiry from `EntitlementService`, not `paid_until`. Hosting
reads entitlements; it still never grants them.

## Architecture

New crate `bot-modules/hosting`, module crate stays Serenity-agnostic;
`bot/src/bindings/hosting/` implements `ModuleCommand` / `ModuleModal` /
`ModuleComponent` / `ModuleAutocomplete`.

Pelican credentials stay **in the bot process only**. The dashboard's Ko-fi
handler writes rows and `pg_notify`s; the bot reacts — mirroring the existing
Patreon path and keeping the admin-scoped key out of the web tier.

```
bot-modules/hosting/src/
  lib.rs  error.rs  catalog.rs  pricing.rs  store.rs
  sweep.rs  cron.rs  provision.rs  modal.rs  components.rs
  commands/mod.rs (host/list/info/cancel)
  pelican/{client.rs,model.rs}      # Application API
```

### Command flow

1. `/server host <game>` — autocompletes from the catalog; checks the tier-scaled
   cap, free allocations and RAM headroom before anything else.
2. Modal `hosting_spec:<game>`: `CreateModalComponent::TextDisplay` for the rate
   card (showing the user's tier-adjusted prices) + three
   `CreateLabel::select_menu` dropdowns + one `input_text` name field. That is
   exactly Discord's 5-component modal limit — a fourth dropdown will not fit.
   Verified available on the pinned Serenity `next` branch
   (`CreateLabelComponent::SelectMenu`).
3. Submit → compute price against tier, insert a `provisioning` row, reply
   ephemerally with Confirm/Cancel carrying `hosting_confirm:<row_id>` (row id,
   not specs — keeps `custom_id` under 100 chars and leaves an audit trail).
4. Confirm → defer, then `provision.rs`: ensure Pelican user (create + generate
   password on first use) → `POST /api/application/servers` with a `deploy` block
   so the panel picks node/allocation → poll until it leaves `installing`
   (bounded; on timeout mark `failed` and roll back) → read allocation → set
   `trial`, `trial_ends_at`, address → post address + Ko-fi link, **DM the
   password separately**. If DMs are closed, fall back to a panel password reset.

### Payment path

Extend `kofi_webhook_handler` with a hosting branch, after the existing logic and
without altering it:

- Insert into `hosting_payments` keyed on `kofi_message_id` first — idempotency,
  since Ko-fi retries.
- Match in order: **`tier_name` → plan** (deterministic, from the ladder);
  **`email` → `kofi_links` → Discord user** (already built); **claim code in
  `message`** only to disambiguate between several servers on the same tier.
- Extend `paid_until` by a month, set `active`, `pg_notify('hosting_paid', id)`.
- Unmatched payments are logged for manual reconciliation and still return `200`,
  matching the file's existing no-retry-storm convention.

Bot side: a listener on `AppEvent::HostingPaid` unsuspends via Pelican.

### Cron

Registered in `BotState::setup_static_cron`, using the claim idiom above:

- `HostingReminderCron` — DM before expiry.
- `HostingExpireCron` (~5 min) — for `kofi` rows `coalesce(paid_until, trial_ends_at) < now()`;
  for `entitlement` rows, when the tier no longer covers the plan → suspend, set
  `delete_after = now() + grace`.
- `HostingDeleteCron` (hourly) — `delete_after < now()` → delete in Pelican.

Every Pelican call in a sweep must treat 404 as success, or a panel outage wedges
the sweep.

## Files

**Create** — `bot-modules/hosting/**`, `bot/src/bindings/hosting/**`,
`migrations/0040_hosting.{up,down}.sql`

**Modify** — `Cargo.toml` (add `hosting`, drop `servers-at-home`);
`bot/Cargo.toml`; `bot/src/bindings/mod.rs`;
`zayden-app/src/config/bot_config.rs` (`HostingConfig`, share `PelicanConfig`);
`zayden-app/src/events/` (`AppEvent::HostingPaid`); `bot/src/state.rs`;
`dashboard/src/web/routes_kofi.rs`; `config.toml` + `bot/config.toml.example`.

## Data model — `migrations/0040_hosting.{up,down}.sql`

- `hosted_servers` — owner, guild, game key, plan, Pelican server/user ids, specs,
  `price_cents`, `billing_source` (`kofi|entitlement|trial`), unique `claim_code`,
  `address`, `state` CHECK (`provisioning|trial|active|suspended|deleted|failed`),
  `trial_ends_at`, `paid_until`, `suspended_at`, `delete_after`, timestamps.
- `hosting_pelican_users` — `discord_user_id` PK → `pelican_user_id`.
- `hosting_payments` — `kofi_message_id` PK, server FK, amount, `matched_by`.

Partial indexes on the two sweep predicates, styled after
`migrations/0039_support_stale.up.sql`.

## Verification

`bacon` is running. After each change `cargo fmt`, then read `.bacon-locations` —
touch a file after migration- or config-only edits or bacon reports a stale run.
Delegate all build/lint/test runs to the `verifier` subagent.

1. **Migrations + query cache** against a throwaway DB, never the live one:
   ```bash
   docker run --rm -d --name zayden-prepare -p 55432:5432 -e POSTGRES_PASSWORD=postgres -e POSTGRES_DB=zayden_prepare postgres:18-alpine
   ```
   then `sqlx migrate run`, then, against the empty freshly-migrated DB:
   ```bash
   cargo sqlx prepare --workspace -- --features ssr
   ```
2. **Gate** — `cargo build --workspace --all-targets` (clippy never links), plus
   `cargo clippy -p dashboard --features ssr -- -D warnings` and the
   `hydrate`/wasm variant. `--all-features` is invalid here.
3. **Tests** (`bot-modules/hosting/tests/`, flat `#[test]`/`#[sqlx::test]`):
   price ladder per plan × tier (every result must be a rung); included-server
   credit; `tier_name`→plan mapping; replayed `kofi_message_id` is a no-op;
   sweeps select exactly the due rows; Pelican client against a mock — create,
   404-tolerant delete, install-poll timeout.
4. **Live integration** using real credentials, wings being empty:
   ```bash
   doppler run -- cargo test -p hosting --test pelican_live -- --ignored
   ```
   Provision a Small server, assert it reaches `running` and the address resolves,
   then delete it and assert the allocation is released.
5. **End to end**: `/server host` → modal → confirm → address live → force
   `trial_ends_at` into the past → expire cron suspends → replay a Ko-fi payload
   at `/webhooks/kofi` → unsuspends and `paid_until` advances.
6. `cargo machete` and `cargo deny check` after touching dependency lists.

## Out of scope

- `/server start|stop|console` (users get the panel)
- A dashboard settings page for hosting
- Patreon as a payment source; refunds, proration, currency conversion
