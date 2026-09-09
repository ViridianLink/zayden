# TECHNICAL RECONNAISSANCE AUDIT: LEPTOS & AXUM HEALTH

## Context

The `dashboard/` crate (~9.3k lines) is a Leptos SSR/hydrate operator console fronted by
Axum, sharing `zayden-app`'s `AppState` with the Discord bot. It has grown module-by-module
(one settings tab per bot-module), and the audit was commissioned to find where that
iterative growth left performance, UX, and reactive-state debt before the next feature lands.

This is a **fact-finding dossier**. No source file was modified. Every finding below was
verified by reading the file at the cited lines; several initial hypotheses were **disproved
and dropped** (recorded in §5) rather than padded into the report.

**Scope audited:** all 8 Axum route handlers + middleware, all 16 `dashboard/src/server/`
modules (51 `#[server]` fns), all 21 UI pages/components, all 20 CSS partials, the
`zayden-app` HTTP client layer, and the external-request boundaries of the bot-module crates.
Leptos version confirmed as **0.8.20**.

---

## 1. Executive Findings Matrix

| ID | Vector | Sev | Location | Primary Root Cause |
| :--- | :--- | :--- | :--- | :--- |
| NET-01 | Performance | **P0** | `dashboard/src/server/auth.rs:23-25,191-197` | Fresh `twilight_http::Client` + Discord round-trip per authorization call |
| NET-02 | Performance | **P0** | `dashboard/src/ui/pages/guild_settings/mod.rs:77-105` | ~8 `/users/@me/guilds` + ~13 session SELECTs per page, serialized |
| DATA-01 | Correctness | **P0** | `dashboard/src/server/modules.rs:151-157` | Discord error → every module renders "Enabled"; toggle silently no-ops |
| RX-01 | Reactivity | **P0** | `dashboard/src/ui/pages/palworld_save.rs:131,145,280` | Trait-option `Vec` cloned per pal; O(pals × traits) DOM nodes |
| UI-01 | UI/UX | **P0** | `dashboard/src/web/routes_patreon.rs:38` | OAuth redirect targets a route that does not exist → 404 |
| SEC-01 | Data exposure | P1 | `dashboard/src/dto/guild.rs:68`; `server/guild.rs:183` | `faq_wiki_api_key` serialized into the SSR payload of *every* settings tab |
| DATA-02 | Correctness | P1 | `dashboard/src/server/greetings.rs:106-113` | Permission-read failure reports the **inverse**: "works in every channel" |
| NET-03 | Performance | P1 | `dashboard/src/ui/components/layout.rs:38,118,137` | App chrome fires 5 extra resources; decorative badge costs a Discord call |
| NET-09 | Performance | P1 | `dashboard/src/server/auth.rs:55-72` + 5 more | `session_cache` exists but is unreachable from all 51 server fns |
| NET-10 | Performance | P1 | `dashboard/src/server/greetings.rs:90,105` | `get_greetings` performs the same authorization twice (6 Discord calls total) |
| NET-11 | Performance | P1 | `levels.rs:56-62`; `guild.rs:455-466` | N+1 sequential Discord calls per leaderboard row / per helper link |
| UI-02 | UI/UX | P1 | `routes_patreon.rs:38` + `ui/` | `?patreon={outcome}` never read — 7 outcomes silently discarded |
| UI-03 | UI/UX | P1 | `guild_settings/patreon.rs:55,56,58,72`, `support/faq.rs:110` | 6 classes used in markup are undefined in CSS → unstyled controls |
| UI-04 | UI/UX | P1 | 34 mutation sites across `dashboard/src/ui/` | 32 of 34 mutations have no `.pending()` — no in-flight feedback anywhere |
| UI-05 | UI/UX | P1 | `guild_settings/mod.rs:66-76,120`; `levels.rs:18,48`; +3 | `<Suspense>` + action-version keys → mutations destroy unsaved form input |
| UI-06 | UI/UX | P1 | `layout.rs:48,121,140`; `server_switcher.rs:32`; +3 | Six empty `fallback=\|\| ()`; **zero skeletons** anywhere in the product |
| UI-07 | UI/UX | P1 | `palworld_save.rs:46,67` | `AppShell` nested *inside* `<Suspense>` — whole chrome absent while loading |
| RX-02 | Reactivity | P1 | `guild_settings/mod.rs:131-138`; `support/mod.rs:55-71`; `select.rs:70` | Channel/role vectors deep-cloned ~10× per settings render, across 3 layers |
| RX-08 | Reactivity | P1 | `dashboard/src/app.rs:56-69` | `AppShell` is a child, not a `ParentRoute` → 5 shell resources remount per nav |
| NET-05 | Performance | P1 | `reaction_roles.rs:12-24` | Three sequential server fns behind one resource |
| NET-04 | Performance | P1 | `dashboard/src/main.rs:96,168-196` | No `TimeoutLayer` (`tower-http` absent); OAuth client has no timeout at all |
| RX-09 | Reactivity | P1 | `palworld_save.rs:122-128,151-154,204-213` | Per-keystroke whole-`HashMap` clones — `.get()` where `.with()` belongs |
| UI-13 | UI/UX | P1 | `dashboard/src/ui/components/server_switcher.rs:35-40` | Operator-accessed guild shows "Select a server" beside "Operator access" |
| NET-06 | Performance | P2 | `dashboard/src/server/guild.rs:132-142` | 11 sequential settings lookups (cold-cache path) |
| NET-07 | Performance | P2 | `dashboard/src/server/operator.rs:59-81` | Unbounded Discord pagination loop, no iteration cap |
| NET-08 | Performance | P2 | `dashboard/src/server/patreon.rs:73-76` | `can_manage_patreon` — dead but publicly reachable; burns a Discord call |
| UI-08 | UI/UX | P2 | `patreon.rs:57`, `faq.rs:110`, `palworld_save.rs:163` | No confirmation on any destructive action; no toast; no `aria-live` |
| UI-09 | UI/UX | P2 | `dashboard/style/partials/*.css` | 39 ad-hoc rem values, no `--space-*`; `.card` ≡ `.settings-section` |
| UI-10 | UI/UX | P2 | `dashboard/src/web/routes_login.rs:115-119` | Session cookie has no `max_age`; browser-session only vs 7-day server session |
| UI-11 | UI/UX | P2 | `dashboard/src/ui/pages/levels.rs:67,108` | "Next" enabled on a full page with no total → dead-end empty page |
| UI-12 | UI/UX | P2 | `palworld_save.rs:48`; `operator_servers.rs:43` | `Err(_) => <NotFound/>` masks 403/409/500, and nests a hero card in the shell |
| RX-03 | Reactivity | P2 | `guild_settings/mod.rs:66-76`; `greetings.rs:44-55` | Mutations invalidate Discord data they cannot affect |
| RX-04 | Reactivity | P2 | `levels.rs:101` vs `levels.rs:108` | Sibling `prop:disabled` — one reactive, one bound once |
| RX-05 | Reactivity | P2 | `layout.rs:159-160,183-204` | Caret toggle rebuilds 13 links; context fallback creates an orphan signal |
| RX-06 | Reactivity | P2 | `guild_settings/mod.rs:78-86` | `?` on one call, `.unwrap_or_default()` on five → silently empty dropdowns |
| RX-07 | Reactivity | P2 | `dashboard/src/main.rs:44-64` | `WebState`: no handler uses >5 of 12 fields; ~10 allocs per request |
| RX-10 | Reactivity | P2 | `operator_servers.rs:86-113` | Whole guild list cloned + grid rebuilt per keystroke |
| SRV-01 | Data flow | P2 | `dashboard/src/dto/guild.rs:29-73` | 42-field `GuildSettings` shipped to every tab; Family tab uses 1 |

---

## 2. Deep-Dive Findings

### Vector 0 — Correctness & Data Integrity
*(Emerged during the sweep. Outside the three requested vectors, but reported first because two
of these make the UI assert the opposite of the truth.)*

#### [DATA-01] A Discord error makes every module report "Enabled", and the fix-up toggle no-ops
- **Severity**: **P0**
- **File**: `dashboard/src/server/modules.rs:151-157`, `:242-252`;
  `dashboard/src/server/command_permissions.rs:177-189,228-232,248-252`
- **Mechanism**: `command_ids()` swallows every Discord failure into an empty `HashMap`
  (`let Ok(resp) = list else { return HashMap::new() }`). That empty map flows into `view()`,
  where `known` is then empty and the expression `known.is_empty() || …` short-circuits to
  **`true`**. So on any Discord 429, token expiry, or permission loss, the Modules page renders
  all nine command-backed modules as *Enabled* — with total confidence and no error. It gets
  worse on the write path: toggling one calls `set_module_enabled`, which re-fetches (also
  empty), finds no `cmd_id`, `continue`s, and returns `Ok(())`. `ModuleCard`'s optimistic state
  (`module_card.rs:63`) then marks it synced. The operator sees a successful toggle that changed
  nothing, on a page that was already lying about current state.
- **Evidence**:
  ```rust
  // modules.rs:151-157
  let enabled = match self.backing {
      Backing::Commands(names) => {
          let known: Vec<_> = names.iter().filter_map(|c| name_to_id.get(*c)).collect();
          known.is_empty() || known.iter().any(|id| !denied.contains(*id))
      },
  ```
- **Remediation Blueprint**: Propagate the Discord failure instead of defaulting. Change
  `command_ids` to return `Result<HashMap<..>, ServerFnError>`; make `ModuleView::enabled` an
  `Option<bool>`/tri-state so "unknown" is representable, and render that as a disabled card
  with "Couldn't reach Discord" rather than as Enabled. Make `set_module_enabled` return `Err`
  when the command id is missing, so the optimistic toggle rolls back (`module_card.rs:67`
  already implements the rollback — it is simply never triggered).

#### [DATA-02] Permission-read failure reports the inverse of the restriction
- **Severity**: P1
- **File**: `dashboard/src/server/greetings.rs:106-113`;
  `dashboard/src/server/command_permissions.rs:228-230`
- **Mechanism**: The `Err(_e) => Vec::new()` arm carries a comment naming one cause ("the command
  is not registered for this guild yet") but catches *all* of them — rate limit, token expiry,
  lost permission. An empty allowlist renders as "With nothing listed, `/good` works in every
  channel" (`greetings.rs:275`). An admin who has restricted the command to one channel is told
  it is unrestricted everywhere. `fetch` independently swallows its own errors the same way, so
  even the `Ok` branch can report "unrestricted" falsely.
- **Remediation Blueprint**: Distinguish "no restriction configured" from "could not read the
  restriction". Return `Option<Vec<Id>>`; render `None` as "Couldn't read channel restrictions"
  rather than as the permissive default. Narrow the `Err(_e)` arm to the specific not-registered
  error and propagate the rest.

#### [SEC-01] A wiki API key is embedded in the SSR payload of every settings tab
- **Severity**: P1
- **File**: `dashboard/src/dto/guild.rs:29,68`; `dashboard/src/server/guild.rs:183`;
  `dashboard/src/ui/pages/guild_settings/support/settings.rs:243-244`
- **Mechanism**: `GuildSettings` is `#[derive(Serialize, Deserialize)]` and carries
  `faq_wiki_api_key: String`. `get_guild_settings` populates it unconditionally, and
  `guild_settings/mod.rs:66` fetches the whole 42-field struct for **every** tab. Because the
  resource is `Resource::new_blocking`, its resolved value is serialized into the HTML for
  hydration — so the key is present in the page source of the Family, Music, AI, LFG, Honeypot,
  Temp Voice, General, and Patreon tabs, none of which display it. Only the Support tab renders
  it, and there it is a plain `value=` on a text input, not `type="password"`.
  **Calibration**: the viewer is already an authenticated guild admin, so this is not a
  privilege escalation — it is an unnecessary widening of the exposure surface (browser cache,
  view-source, SSR HTML in any intermediary, and any future XSS on an unrelated tab).
- **Evidence**:
  ```rust
  // dto/guild.rs:28-29,68 — Serialize + secret in the same struct
  #[derive(Clone, Default, Serialize, Deserialize)]
  pub struct GuildSettings { … pub(crate) faq_wiki_api_key: String, … }
  // server/guild.rs:183
  faq_wiki_api_key: faq.wiki_api_key.clone().unwrap_or_default(),
  ```
- **Remediation Blueprint**: Never ship the key to the client. Return a
  `faq_wiki_api_key_set: bool` in the DTO and render the input as an empty
  `type="password"` with a "configured" indicator; treat an empty submission as "leave
  unchanged". This pairs naturally with SRV-01's per-section projection, which stops the Support
  DTO reaching the other eight tabs at all.

---

### Vector 1 — Performance & Network Orchestration

#### [NET-01] Per-authorization Discord client allocation and round-trip
- **Severity**: P0
- **File**: `dashboard/src/server/auth.rs:23-25`, `:180-197`, `:231-244`
- **Mechanism**: `bearer_client()` calls `Client::builder().build()`, which constructs a
  **complete `twilight_http::Client`** — its own hyper connection pool, TLS config, and
  rate-limiter state — and then throws it away after one request. Because the pool is new
  every time, **no TLS session or TCP connection is ever reused** for user-token calls, and
  the discarded rate-limiter means the process cannot track its own 429 budget across calls.
  `guild_admin_for` then issues a real `GET /users/@me/guilds` to Discord. Since
  `guild_admin_context()` is the entry point for **every guild-scoped server fn**, the unit
  cost of *any* guild request is: 1 DB session lookup + 1 client construction + 1 TLS
  handshake + 1 rate-limited Discord round-trip.
- **Evidence**:
  ```rust
  // auth.rs:23-25
  pub(crate) fn bearer_client(access_token: &str) -> Client {
      Client::builder().token(format!("Bearer {access_token}")).build()
  }
  // auth.rs:191-197
  let all_guilds = bearer_client(&identity.access_token)
      .current_user_guilds().await.map_err(server_err)?
      .model().await.map_err(server_err)?;
  ```
  Call sites: `auth.rs:191`, `auth.rs:297`, `guild.rs:99`, `command_permissions.rs:171,268`.
- **Remediation Blueprint**:
  1. Add a `moka::future::Cache<(i64, i64), GuildAccess>` (user_id, guild_id) with a 60s TTL
     to `WebState`, mirroring the `session_cache` already built at `main.rs:110-114`. Have
     `guild_admin_for` consult it before hitting Discord.
  2. Separately cache `current_user_guilds()` per session token (~60s) — `list_manageable_guilds`
     (`guild.rs:99`) and `guild_admin_for` request the identical payload on the same page load.
  3. Keep one `twilight_http::Client` per *session token*, not per call — or drop to a
     `reqwest` call issued on the shared `state.app.http` client, which already carries the
     workspace timeout budget (see NET-04).

#### [NET-02] Six-call authorization waterfall on the primary settings page
- **Severity**: P0
- **File**: `dashboard/src/ui/pages/guild_settings/mod.rs:66-106`
- **Mechanism**: The resource `.await`s six server fns **strictly in sequence**. They are
  mutually independent — every one takes only `gid` — so the latency is the *sum*, not the max.
  Compounding it, each independently calls `guild_admin_context()`, so NET-01's cost is paid
  once per call. Counting the six page fns plus the five chrome fns from NET-03, one render of
  `/guild/:id/settings` costs **≈8 `GET /users/@me/guilds` calls and ≈13 `web_sessions`
  SELECTs**, plus `guild_channels`, `roles`, and an N+1 `guild_member` per helper link
  (NET-11). Because the resource is `new_blocking`, the SSR response is withheld until all six
  finish. Discord rate-limits `/users/@me/guilds` aggressively, so the busiest page in the app
  is also the one most likely to trip a 429 — which, per DATA-01, the UI then misreports.
- **Evidence**:
  ```rust
  // guild_settings/mod.rs:77-86
  |(gid, ..)| async move {
      let settings = get_guild_settings(gid.clone()).await?;
      let support_roles = list_support_roles(gid.clone()).await.unwrap_or_default();
      let helper_links = list_helper_links(gid.clone()).await.unwrap_or_default();
      let channels = list_guild_channels(gid.clone()).await.unwrap_or_default();
      let patreon = get_patreon_status(gid.clone()).await.unwrap_or_default();
      let roles = list_guild_roles(gid).await.unwrap_or_default();
  ```
- **Remediation Blueprint**:
  1. **Authorize once, fetch many.** Add a single `get_settings_bundle(guild_id) -> SettingsBundle`
     server fn in `dashboard/src/server/guild.rs` that calls `guild_admin_context()` **one**
     time, then runs the six fetches under `tokio::try_join!` using the already-resolved
     `ctx.guild_id: i64`. This collapses 6 authorizations → 1 and 6 serial fetches → 1 RTT.
  2. Have the page's `Resource` call only that bundle fn.
  3. Fix NET-01 first — with the authorization cache in place, step 1's win compounds.

#### [NET-03] App chrome adds five resources to every page, one costing a Discord call
- **Severity**: P1
- **File**: `dashboard/src/ui/components/layout.rs:38,118,137`; `server_switcher.rs:29`;
  `tier_badge.rs:8`; `dashboard/src/server/operator.rs:35-48`
- **Mechanism**: `AppShell` wraps every page and independently mounts `check_session`,
  `guild_operator_access`, `is_operator`, `list_manageable_guilds`, and `get_user_tier`.
  `guild_operator_access` is the expensive one: it runs `current_user_id()` (DB) +
  `has_role()` (DB) + `guild_admin_context()` (DB + **Discord round-trip**) — all to decide
  whether to render a decorative "Operator access" chip that is `false` for nearly every user.
  `is_operator` then repeats `current_user_id` + `has_role` in the same sidebar. Combined with
  NET-02, one visit to `/guild/:id/settings/general` is **~11 server fn invocations and ~7
  Discord round-trips**.
- **Evidence**:
  ```rust
  // operator.rs:35-48 — cost of a decorative badge
  pub async fn guild_operator_access(guild: String) -> Result<bool, ServerFnError> {
      let Ok(user_id) = current_user_id().await else { return Ok(false) };
      let pool = db_pool()?;
      if !has_role(&pool, user_id, WebRole::Operator).await? { return Ok(false) }
      let ctx = guild_admin_context(&guild).await?;   // ← Discord round-trip
  ```
- **Remediation Blueprint**: Merge `is_operator` + `guild_operator_access` into one
  `operator_context(guild) -> OperatorContext { is_operator, guild_access }` server fn; return
  early *before* `guild_admin_context()` when `has_role` is false (already the shape at
  `:41-43` — the Discord call only needs to run for actual operators, which is already true;
  the win is deduplicating the two fns and reusing NET-01's cache).

#### [NET-04] The twilight path bypasses the workspace's own timeout discipline
- **Severity**: P1
- **File**: `dashboard/src/server/auth.rs:23`; `dashboard/src/main.rs:55,96,168-196`
- **Mechanism**: `zayden-app` gets this *right* — `services/http.rs:11-14` defines
  `ClientBuilderExt::with_timeouts()` (30s total / 10s connect) and `app_state.rs:13-22`
  builds one shared client with it. Three paths opt out:
  1. `bearer_client()` — a different HTTP stack (twilight, not reqwest), no budget at all.
  2. `http_oauth: oauth2::reqwest::Client::new()` (`main.rs:96`) — shared correctly, but
     `new()` sets neither `timeout` nor `connect_timeout`, and its only consumer is the
     interactive Discord token exchange at `routes_login.rs:52`. A hung Discord token endpoint
     parks the login request forever. One-line fix: `ClientBuilder::new().with_timeouts()`.
  3. The router itself. `tower-http` is **not a dependency** of `dashboard`, `bot`, or
     `zayden-app`, so there is no `TimeoutLayer`, `RequestBodyLimitLayer`, or `TraceLayer` —
     `main.rs:194` carries only `CookieManagerLayer`.

  Also worth a comment: `app_state.rs:20`'s fallback branch silently yields a client with **no
  timeout at all** if the builder ever fails, recording only a `warn!`.
- **Evidence**:
  ```rust
  // zayden-app/src/services/http.rs:11-14 — the discipline that exists
  impl ClientBuilderExt for reqwest::ClientBuilder {
      fn with_timeouts(self) -> Self {
          self.timeout(HTTP_TIMEOUT).connect_timeout(HTTP_CONNECT_TIMEOUT)
      }
  }
  ```
- **Remediation Blueprint**: Wrap every `bearer_client()` await in
  `tokio::time::timeout(HTTP_TIMEOUT, ...)` (reuse the constant — do not introduce a second
  budget), and map elapse to a typed `ServerFnError` the UI can render as "Discord is slow,
  retry". Optionally add `tower-http`'s `TimeoutLayer` to the router at `main.rs:189` as a
  process-wide backstop.

#### [NET-05] Reaction-roles page repeats the waterfall
- **Severity**: P1 · **File**: `dashboard/src/ui/pages/reaction_roles.rs:12-24`
- **Mechanism**: Same shape as NET-02 at smaller scale — three independent server fns awaited
  in sequence, each re-authorizing. Additionally keyed on `add.version()`/`remove.version()`,
  so adding one mapping re-fetches the full Discord channel *and* role lists.
- **Evidence**:
  ```rust
  let maps = list_reaction_roles(gid.clone()).await?;
  let channels = list_guild_channels(gid.clone()).await.unwrap_or_default();
  let roles = list_guild_roles(gid).await.unwrap_or_default();
  ```
- **Remediation Blueprint**: `tokio::try_join!` the three; split the mutation-driven refetch
  so only `list_reaction_roles` is keyed on the action versions (channels/roles do not change).

#### [NET-06] Cold-path settings fan-out
- **Severity**: P2 · **File**: `dashboard/src/server/guild.rs:132-142`
- **Mechanism**: 11 sequential `s.<module>.get(guild_id).await` calls. These are moka-backed
  (`SettingsRegistry`), so the steady state is cheap — but on cold cache, a restart, or a
  LISTEN/NOTIFY invalidation this is 11 serial DB round-trips inside an already-serial chain.
- **Remediation Blueprint**: `tokio::try_join!` all 11; they share no state.

#### [NET-07] Unbounded Discord pagination
- **Severity**: P2 · **File**: `dashboard/src/server/operator.rs:59-81`
- **Mechanism**: `loop { ... }` paging `current_user_guilds()` 200 at a time with no iteration
  cap and no timeout. For a bot in thousands of guilds this is dozens of serial Discord calls
  inside one server fn; if the API ever returns a persistently full page the loop never exits.
- **Remediation Blueprint**: Add a `MAX_PAGES` guard, wrap the loop in `tokio::time::timeout`,
  and cache the result (bot guild membership changes rarely).

#### [NET-09] The session cache exists and no server function can reach it
- **Severity**: P1
- **File**: `dashboard/src/main.rs:62,116-119,181-187`; `dashboard/src/server/auth.rs:55-72`
- **Mechanism**: `WebState.session_cache` (moka, 1-minute TTL, 1024 entries) is built correctly
  and consumed by the Axum middleware at `middleware/auth.rs:28` — which guards only the six
  `/kofi`, `/patreon`, `/admin` routes. It is **not** passed to `provide_context` at
  `main.rs:181-187`, so none of the 51 `#[server]` fns can see it; every one re-queries Postgres.
  The same `SELECT … FROM web_sessions WHERE token = $1 AND expires_at > now()` is written out
  **six times**: `auth.rs:61-68`, `auth.rs:160-167`, `auth.rs:262-266`, `auth.rs:284-289`,
  `guild.rs:86-93`, `tier.rs:44-50` — five of them bypassing the cache built for exactly this.
  This is the mechanical cause of the ~13 session SELECTs per settings page render.
- **Remediation Blueprint**: `provide_context(web_state.session_cache.clone())` alongside the
  existing five, then collapse the six duplicated queries into one cache-aware
  `session_identity()` that every other helper calls.

#### [NET-10] `get_greetings` performs the same authorization twice
- **Severity**: P1 · **File**: `dashboard/src/server/greetings.rs:89-113`
- **Mechanism**: `admin_guild_id(&guild)` at `:90` and `guild_context(&guild)` at `:105` both
  funnel into `guild_admin_context()` — so the session SELECT and the `/users/@me/guilds` call
  each happen twice in one function, purely because the first call discarded the context and
  kept only the id. Adding `guild_server_tier` and the command-permission reads, this single
  server fn makes six sequential Discord round-trips, two of them exact duplicates.
  `save_greeting_cooldowns` (`:183-187`) repeats the `admin_guild_id` + `guild_server_tier` pair.
- **Remediation Blueprint**: Call `guild_context()` once at the top and use `ctx.guild_id`;
  delete the `admin_guild_id` call. Once NET-01's cache lands, the duplicate becomes cheap — but
  the redundancy should go regardless.

#### [NET-11] N+1 sequential Discord calls in two server functions
- **Severity**: P1 · **File**: `dashboard/src/server/levels.rs:56-73`; `guild.rs:455-466`
- **Mechanism**: `levels.rs` awaits `http.user(Id::new(user_id))` once per leaderboard row — ten
  strictly sequential `GET /users/{id}` calls per page render, uncached, on a page whose pager
  makes re-renders frequent (UI-05). `guild.rs:455-466` does the same with `display_name` →
  `GET /guilds/{g}/members/{u}` per helper link.
- **Evidence**:
  ```rust
  // levels.rs:56-58
  for (rank, row) in (offset + 1..).zip(rows) {
      let user_id = row.user_id.cast_unsigned();
      let user = match http.user(Id::new(user_id)).await { … };
  ```
- **Remediation Blueprint**: Collect the futures and drive them with `FuturesOrdered` (rank order
  matters) or `futures::future::join_all`. Add a small moka cache for user/member display names —
  leaderboard membership changes far more slowly than the page is viewed.

#### [NET-08] A dead server function is still a public, Discord-burning endpoint
- **Severity**: P2 · **File**: `dashboard/src/server/patreon.rs:73-76`
- **Mechanism**: `can_manage_patreon` has zero call sites (verified by grep across
  `dashboard/src/`). `#[server]` nonetheless registers a reachable POST endpoint, and each hit
  performs a session lookup plus a `GET /users/@me/guilds` — an unauthenticated-traffic
  amplifier against the process's Discord rate limit, for a function nothing calls.
- **Remediation Blueprint**: Delete it.

---

### Vector 2 — UI/UX & Design Cohesion

#### [UI-01] The entire Patreon OAuth flow dead-ends on a 404
- **Severity**: **P0**
- **File**: `dashboard/src/web/routes_patreon.rs:37-39` vs `dashboard/src/app.rs:56-69`
- **Mechanism**: `settings_url()` builds `/guilds/{guild_id}/patreon`. The router declares
  `/guilds` (exact), `/guild/:id`, `/guild/:id/settings`, and `/guild/:id/settings/:section` —
  **singular** `guild` for the parameterised routes. `/guilds/123/patreon` matches none of
  them and falls through to `<Routes fallback>` → `NotFound`. Every terminal state of the flow
  routes here: `connected`, `disconnected`, `forbidden`, `error`, `declined`, `state_mismatch`,
  `unconfigured` (`:74,80,90,136,141,145,151,159,170,204,207,223,247,250`). A creator who
  successfully authorizes Patreon is shown a Not Found page.
- **Evidence**:
  ```rust
  // routes_patreon.rs:37-39
  fn settings_url(guild_id: &str, outcome: &str) -> String {
      format!("/guilds/{guild_id}/patreon?patreon={outcome}")
  }
  // app.rs:62-63 — the routes that actually exist
  <Route path=path!("/guild/:id/settings") view=GuildSettingsPage/>
  <Route path=path!("/guild/:id/settings/:section") view=GuildSettingsPage/>
  ```
  Corroborated: `grep '"/guilds/'` across `dashboard/src/` returns this **one** line — every
  other link in the codebase correctly uses `/guild/{id}/settings/{slug}` (`nav.rs:18`).
- **Remediation Blueprint**: Change to
  `format!("/guild/{guild_id}/settings/patreon?patreon={outcome}")`. Better: call
  `nav::section("patreon").href(guild_id)` (`nav.rs:15-22`) so the URL is generated from the
  same table the sidebar uses and can never drift again. Add a route-coverage test asserting
  every `settings_url` outcome resolves to a non-fallback route.

#### [UI-02] Seven OAuth outcomes are computed, transmitted, and discarded
- **Severity**: P1 · **File**: `routes_patreon.rs:38` + all of `dashboard/src/ui/`
- **Mechanism**: The handler carefully distinguishes seven outcomes in `?patreon=`. Grepping
  `patreon=` across `dashboard/src/ui/` returns **nothing** — no `use_query`, no param read.
  Even with UI-01 fixed, the user gets no success confirmation and no failure reason; a
  `forbidden` and a `connected` land on a visually identical page.
- **Remediation Blueprint**: In `PatreonTab`, read the param via
  `leptos_router::hooks::use_query_map()`, map the seven values to a message + severity, and
  render through the shared feedback surface introduced in UI-04/UI-08.

#### [UI-03] Six classes are used in markup that no stylesheet defines
- **Severity**: P1
- **File**: `dashboard/src/ui/pages/guild_settings/patreon.rs:56,58`;
  `dashboard/src/ui/pages/guild_settings/support/faq.rs:110`
- **Mechanism**: A full cross-check of every `class="…"` token in `src/ui/` against every
  selector in `style/partials/` yields **six classes used in markup that no stylesheet
  defines**:

  | Class | Used at | Consequence |
  | :--- | :--- | :--- |
  | `btn-danger` | `support/faq.rs:110` | Delete gets base `.btn` only — transparent background and border, so it reads as neutral text, visually identical to "Save" |
  | `button` | `patreon.rs:56,73` | Connect / Reconnect render as bare unstyled anchors |
  | `danger` | `patreon.rs:58` | Disconnect renders as a default OS-grey browser button on `#0a0908` |
  | `settings-actions` | `patreon.rs:55,72` | No-op div — the controls stack instead of sitting in a row |
  | `save-pal-id` | `palworld_save.rs:244` | Unstyled |
  | `save-pending` | `palworld_save.rs:150` | Unstyled |

  `buttons.css` defines only `.btn`, `.btn-lg`, `.btn-primary`, `.btn-secondary`, `.btn-ghost`
  and `.btn:disabled`. The Patreon tab is the most visually broken surface in the product, and
  this is the clearest surviving fingerprint of bolted-on iterative patching. Related:
  `palworld_save.rs:101,225,267` emit bare `<input type="number">` and `<select multiple>` with
  no `.setting-field` ancestor, so none of `settings.css:42`'s rules apply — they render as
  white browser-default spinners on the dark page, unlike every other input in the app.
- **Evidence**:
  ```rust
  // patreon.rs:56-59 — neither class exists in the stylesheet
  <a class="button" href=connect_href>"Reconnect Patreon"</a>
  <form method="post" action=disconnect_href>
      <button type="submit" class="button danger">"Disconnect"</button>
  ```
- **Remediation Blueprint**: (a) rewrite `patreon.rs:56,58` to `btn btn-secondary` /
  `btn btn-danger`; (b) add a `.btn-danger` rule to `style/partials/buttons.css` using the
  existing `--error` token (`tokens.css:19`); (c) add a CI grep asserting every `btn-*` used
  in `src/` has a matching definition in `style/` — this class of drift is mechanically
  detectable and will recur otherwise.

#### [UI-04] Thirty-four mutations, two with in-flight feedback
- **Severity**: P1
- **File**: 30 declaration sites; canonical offender `dashboard/src/ui/components/settings.rs:23-30`
- **Mechanism**: `grep -rn '\.pending()' dashboard/src/` returns **nothing**. A full enumeration
  finds 34 mutation sites; **32 have no pending state**. The shared `SaveButton` — used by every
  settings tab — emits a plain `<button type="submit">` with no `disabled` binding and no prop
  through which one could be passed. Between click and response the UI is visually inert, so
  users re-click and fire duplicate mutations. The irony: `.btn:disabled` **is** already styled
  at `buttons.css:65-70` — the design system anticipated pending states; no component wired them.

  Two further gaps in the same surface:
  - **One mutation has no error surface at all** — the Patreon disconnect (`patreon.rs:57-61`)
    is a raw `<form method="post">`, the only non-`ActionForm` form in the app: full-page
    navigation, no pending, no error, no confirmation.
  - **Feedback renders far from its trigger.** `support/faq.rs:35-36` renders one shared
    `SaveFaqArticle` result at the top of the pane, though the action is shared by the new-article
    form *and* every article row — saving article #14 inside a collapsed `<details>` writes
    "Saved." off-screen with no indication of which article. `greetings.rs:143-161` has the same
    shape: one `add`/`remove` pair shared by both `ImageSection`s, so adding a night image prints
    success above the morning section.
  - **No `aria-live` / `role="status"` anywhere** in `ui/` — screen-reader users get no submit
    confirmation at all. Messages also never auto-dismiss, so a stale "Saved." sits above a form
    the user has since edited.

  The one correct implementation is `ModuleCard` (`module_card.rs:81-91`), which does optimistic
  update, in-flight coalescing, and rollback — all hand-rolled and factored out nowhere.
- **Evidence**:
  ```rust
  // settings.rs:23-30 — one component, used by every settings tab
  pub(crate) fn SaveButton() -> impl IntoView {
      view! {
          <div class="form-actions">
              <button type="submit" class="btn btn-primary">"Save"</button>
  ```
- **Remediation Blueprint**: Give `SaveButton` a `#[prop(into)] pending: Signal<bool>` and bind
  `prop:disabled=pending` plus a label swap to "Saving…". Thread `action.pending()` from each
  of the 30 call sites. Because every tab already routes through this one component, this is a
  ~30-line change that fixes the whole surface.

#### [UI-05] `<Suspense>` where `<Transition>` is required — mutations collapse the page
- **Severity**: P1
- **File**: `guild_settings/mod.rs:66-76,120-122`; `levels.rs:18-21,48-50`;
  `reaction_roles.rs:12-13,43-45`
- **Mechanism**: The codebase contains **one** `<Transition>` (`modules.rs:33`) against ~12
  `<Suspense>`. `<Suspense>` re-shows its fallback whenever its resource returns to pending;
  `<Transition>` holds the previous view. Two patterns make this constant:
  1. Resources keyed on `action.version()` (`guild_settings/mod.rs:69-74`) go pending after
     **every mutation** — so clicking "Add support role" replaces the entire settings form
     with the one-line `<p class="loading">"Loading settings…"</p>`, collapsing the page to a
     single text row before re-expanding.
  2. Resources keyed on filter/page signals (`levels.rs:19`) go pending on every paginate and
     every "This server"/"Global" toggle — blanking a 10-row table on each click.
- **Evidence**:
  ```rust
  // guild_settings/mod.rs:69-74 — any mutation re-triggers the fallback
  create_creator.version().get(), add_support_role.version().get(),
  remove_support_role.version().get(), add_helper_link.version().get(),
  remove_helper_link.version().get(),
  // :120-122
  <Suspense fallback=|| view! { <p class="loading">"Loading settings…"</p> }>
  ```
- **Remediation Blueprint**: Swap `<Suspense>` → `<Transition>` at all three sites. Replace
  the one-line text fallbacks with skeletons that reserve the final layout's height. Pair with
  UI-04 so the in-flight signal moves to the button, where it belongs, rather than the page.

#### [UI-06] Six empty Suspense fallbacks, and not one skeleton in the product
- **Severity**: P1
- **File**: `layout.rs:48,121,140`; `server_switcher.rs:32`; `tier_badge.rs:11`
- **Mechanism**: `fallback=|| ()` reserves zero space. Six sit in persistent chrome or gating
  logic: the navbar's Sign-in/Log-out button, the tier badge, the server switcher, the operator
  badge, the operator sidebar link, and `public_layout.rs:32`. Each resolves independently, so
  the navbar's right edge and the sidebar's link list reflow several times per navigation — the
  sidebar's item *count* changes, pushing "Upgrade to Pro" down after `is_operator` resolves.

  The broader finding: **there is not one skeleton in the product.** Every one of the other ten
  fallbacks is a single italic line (`.loading`, `pages.css:80-83`, which sets only colour and
  font-style — no height, no shape). The worst deltas are `guilds.rs:23` (one line → a multi-row
  48px card grid) and `guild_settings/mod.rs:120` (one line → an entire multi-fieldset form).
  Related: `login.rs:14-24` puts only the redirect inside `<Suspense>` and the login card
  outside it, so an already-authenticated user watches the full "Sign in with Discord" card
  paint before being yanked to `/guilds`.
- **Evidence**:
  ```rust
  // layout.rs:48 (navbar), :121 (operator badge), :140 (operator link)
  <Suspense fallback=|| ()>
  ```
- **Remediation Blueprint**: Give each a fixed-dimension skeleton matching the resolved
  element (the CSS already fixes `.server-switcher-avatar` at 30×30, `layout.css:136-142` — the
  same discipline just needs extending to the containers). For the operator link, reserve the
  row height unconditionally so the list length never changes.

#### [UI-07] Save editor nests the entire app shell inside its Suspense boundary
- **Severity**: P1 · **File**: `dashboard/src/ui/pages/palworld_save.rs:46,67,175`
- **Mechanism**: Every other page renders `<AppShell>` *outside* `<Suspense>` (compare
  `guild_settings/mod.rs:112` and `levels.rs:25`). Here the order is inverted: `<Suspense>` at
  :46 wraps `<AppShell>` at :67. Until the roster resolves there is no navbar and no sidebar at
  all — just a bare "Loading world…" on an empty page — then the whole application chrome
  appears at once. On the error path (:48) it renders `<NotFound/>`, also chrome-less.
- **Remediation Blueprint**: Hoist `<AppShell>` outside `<Suspense>` to match every sibling
  page; keep only the roster body inside the boundary.

#### [UI-08] No confirmation on destructive actions, and no dismissible feedback
- **Severity**: P2 · **File**: `patreon.rs:57-61`; `faq.rs:110`; `reaction_roles.rs`;
  `settings.rs:5-21`
- **Mechanism**: There is no `<dialog>`, modal, drawer, or toast anywhere in the codebase — the
  only overlay primitive is native `<details>` (`server_switcher.rs:44`, `palworld_save.rs:92`,
  `faq.rs:37`), which is a consistent and defensible choice. But it leaves destructive actions
  unguarded: "Disconnect" (which unregisters a Patreon webhook and deletes the connection),
  "Delete FAQ article", and "Remove reaction role" all fire on a single click. Feedback is a
  static `<p class="success">"Saved."</p>` (`settings.rs:7`) that never auto-dismisses and
  persists until the next render — so a stale "Saved." can sit above a form the user has since
  edited. Note `patreon.rs:57` also uses a raw `<form method="post">`, a full-page navigation
  that bypasses the `ServerAction` pattern every other mutation uses.
- **Remediation Blueprint**: Add one `<ConfirmButton>` component wrapping a native `<dialog>`;
  apply to the three destructive sites. Convert `save_feedback`/`create_feedback` into a
  single dismissible inline-alert component with an auto-clear timeout, and migrate
  `patreon.rs:57` to `ServerAction` for consistency with the other 30 mutations.

#### [UI-09] Spacing is the one un-tokenized axis of an otherwise strong design system
- **Severity**: P2 · **File**: all 20 partials under `dashboard/style/partials/`
- **Mechanism**: `tokens.css` tokenizes color, radius, shadow, easing, and layout dimensions
  properly, including a four-bot accent-theme system (`:41-73`). Spacing is the gap: measured
  across every `padding`/`margin`/`gap` declaration there are **39 distinct rem values** and no
  `--space-*` scale. The distribution is the tell — `0.15`, `0.22`, `0.34`, `0.42`, `0.45`,
  `0.55`, `0.65`, `0.85`, `0.95`, `1.35` rem are hand-tuned per component, not steps on a
  scale. This is precisely how sibling panels drift out of vertical rhythm.
  Four corroborating drifts in the same layer:
  - **`.card` and `.settings-section` are byte-identical rules** with different names in
    different files (`card.css:3-9`, `settings.css:3-9`). `upgrade.rs:117` reaches for one;
    every settings tab reaches for the other. Five more panel variants (`.module-card` 1.35rem,
    `.plan-card` 1.4rem, `.feature-card` 1.5rem, `.guild-card` 1.1rem, `.greet-card` 0.7rem)
    share the same border/radius/background and differ only in padding. No `<Panel>` component.
  - **Input styling is copy-pasted into three partials** (`settings.css:42-52`,
    `upgrade.css:137-152`, `operator.css:11-29`) and has already drifted — `settings.css` sets a
    monospace `font-family` and a transition; the other two set one or neither. The same control
    renders in a different typeface on `/upgrade` than in `/guild/:id/settings`.
  - **`--content-max: 1080px` is defined and then bypassed**: `.page` hardcodes `max-width: 960px`
    (`pages.css:3-6`), so the public column and the dashboard column disagree and the token meant
    to unify them is used only by `landing.css`.
  - **`#fff` is hardcoded in 10 places** because there is no `--on-accent` token, and
    `--radius-full: 999px` is bypassed by literal `999px` / `50%` / `9px` in five rules.
- **Remediation Blueprint**: Add a `--space-1..8` ramp and an `--on-accent` token to
  `tokens.css`, then migrate partials one at a time, snapping each ad-hoc value to the nearest
  step. Highest-value first: `landing.css` (30 declarations), `layout.css` (23), `pages.css`
  (20), `upgrade.css` (20). Collapse `.card`/`.settings-section` into one `<Panel>` component
  with a density prop, and extract the triplicated input block into a single `.input` primitive.

#### [UI-10] Session cookie expires on browser close despite a 7-day server session
- **Severity**: P2 · **File**: `dashboard/src/web/routes_login.rs:115-119` vs `:16,89-91`
- **Mechanism**: `SESSION_TTL_HOURS = 24 * 7` and the `web_sessions` row is written with a
  7-day `expires_at`, but the `Set-Cookie` omits `max_age`/`expires`, making it a *session*
  cookie the browser drops on close. Users are silently logged out every browser restart while
  a valid week-long session lingers server-side until the hourly prune. Note the OAuth *state*
  cookie does set `max_age` correctly (`main.rs:222-232`), so the omission is inconsistent
  rather than deliberate.
- **Remediation Blueprint**: Add `.max_age(Duration::hours(SESSION_TTL_HOURS))` to the builder
  at `:115-119`, deriving from the same constant that sets the DB row.

#### [UI-11] Pagination dead-end
- **Severity**: P2 · **File**: `dashboard/src/ui/pages/levels.rs:67,105-110,55-65`
- **Mechanism**: `has_next = entries.len() == PAGE_SIZE` — with no total count, a board whose
  size is an exact multiple of 10 leaves "Next" enabled onto an empty page, where the user
  meets "No more entries on this page." and must click Previous to escape.
- **Remediation Blueprint**: Have `get_leaderboard` return a total (or fetch `PAGE_SIZE + 1`
  and render only `PAGE_SIZE`) so `has_next` is exact.

#### [UI-12] Real errors masked as Not Found
- **Severity**: P2 · **File**: `dashboard/src/ui/pages/palworld_save.rs:47-48`
- **Mechanism**: `Err(_) => view! { <NotFound/> }` discards the error. The export path can
  legitimately return 403 (not an admin), 409 (`"no world save is configured"`,
  `routes_palworld_save.rs:39-44`), and 422 (corrupt save) — all three render as "Not Found",
  sending an operator to look for a routing bug instead of a configuration one.
- **Remediation Blueprint**: Match on the error variant; render `NotFound` only for a genuine
  authorization miss and surface the server message otherwise, as `levels.rs:52-54` already does.

---

### Vector 3 — Reactive State Management & Data Flow

#### [RX-01] Save editor allocates and renders O(pals × traits)
- **Severity**: **P0**
- **File**: `dashboard/src/ui/pages/palworld_save.rs:52-63,130-132,144-146,280-285`
- **Mechanism**: `options: Vec<(String, String)>` holds **every** Palworld trait/passive
  (order ~250 entries, 500 `String`s). It is `.clone()`d **once per pal** at both `:131` and
  `:145`, then each clone is expanded into a `<select multiple>` at `:280-285`. For a world
  with 1,000 pals that is ~500,000 `String` allocations and ~250,000 `<option>` DOM nodes on a
  single page — enough to stall hydration and pin browser memory. The cost is entirely
  incidental: every pal renders the *same* option list.
- **Evidence**:
  ```rust
  // :130-132 — per pal, in a loop over the whole roster
  {player.pals.into_iter().map(|pal| {
      pal_row(pal, pending, options.clone())
  }).collect_view()}
  // :280-285 — and each clone becomes ~250 DOM nodes
  {options.into_iter().map(|(value, label)| {
      let selected = current.contains(&value);
      view! { <option value=value selected=selected>{label}</option> }
  ```
- **Remediation Blueprint**:
  1. Change `pal_row`'s parameter to `StoredValue<Arc<[(String, String)]>>` (or
     `Signal<Arc<...>>`) so the option table is allocated **once** and shared by reference —
     removing the per-pal clone outright.
  2. Eliminate the per-pal DOM cost: render the option list once into a `<datalist>` and
     reference it by `id`, or replace the always-rendered `<select multiple>` with a
     trait-picker mounted lazily on focus (only one pal is ever being edited at a time).
  3. Virtualize the pal list, or paginate it per player group, so DOM size is bounded by
     viewport rather than by save-file size.

#### [RX-02] Six collection clones on every section switch
- **Severity**: P1 · **File**: `dashboard/src/ui/pages/guild_settings/mod.rs:128-138`
- **Mechanism**: The inner closure reads `active` (a `Memo`), so it re-runs on **every sidebar
  tab click**. On each run it deep-clones all six loaded collections — including `channels` and
  `roles`, which for a large guild are hundreds of `ChannelInfo`/`RoleInfo` structs — even
  though the tab being rendered consumes at most three of them. The comment at `:129-130`
  correctly notes the *resource* is not refetched; the *clone* cost was not considered.
- **Evidence**:
  ```rust
  // :131-138
  (move || {
      let guild_id = gid.clone();
      let s = s.clone();
      let support_roles = support_roles.clone();
      let helper_links = helper_links.clone();
      let channels = channels.clone();
      let roles = roles.clone();
      let patreon_status = patreon.clone();
  ```
- **Remediation Blueprint**: Store the bundle in a `StoredValue` (or wrap the collections in
  `Arc`) once when the resource resolves, and have each tab component take `Arc<[ChannelInfo]>`
  instead of `Vec<ChannelInfo>`. Tab switching then costs a pointer clone.

#### [RX-03] Mutations invalidate data they cannot affect
- **Severity**: P1 · **File**: `guild_settings/mod.rs:66-76`; `reaction_roles.rs:12-13`
- **Mechanism**: Keying the resource on five `action.version()` signals means adding a helper
  link re-fetches the Discord channel list, the Discord role list, the Patreon status, and all
  11 settings groups — none of which that mutation touches. Each refetch re-pays NET-01/NET-02.
- **Remediation Blueprint**: Split into two resources: a *static* one keyed on `guild_id` only
  (channels, roles — Discord-sourced, changes rarely) and a *mutable* one keyed on the action
  versions (settings, support roles, helper links). Only the second should invalidate.

#### [RX-04] Sibling `prop:disabled` bindings disagree on reactivity
- **Severity**: P2 · **File**: `dashboard/src/ui/pages/levels.rs:101` vs `:108`
- **Mechanism**: `:101` binds `prop:disabled=move || page.get() <= 1` (reactive closure);
  `:108` binds `prop:disabled=!has_next` (a plain `bool` evaluated once at render). The Next
  button currently works only because the whole block is rebuilt when the resource reloads —
  it is correct by accident, and becomes a live bug the moment UI-05's `<Transition>` fix or
  any memoization stops that rebuild.
- **Remediation Blueprint**: Make `has_next` a `Memo`/derived signal and bind it as a closure,
  matching `:101`.

#### [RX-05] Caret toggle destroys and rebuilds the module list; orphan context fallback
- **Severity**: P2 · **File**: `dashboard/src/ui/components/layout.rs:159-160,183-204`
- **Mechanism**: `sublist` reads `open.get()` at the top of the closure, so collapsing or
  expanding the sidebar group tears down and reconstructs all 12 `<A>` nodes — each with its
  own `current`-reading class closure — instead of toggling visibility. Separately, `:159-160`
  falls back to `RwSignal::new(true)` when the `ModulesOpen` context is missing, silently
  creating an orphan signal that no other component observes; a context wiring regression would
  manifest as a toggle that appears to work but does not persist across navigation.
- **Remediation Blueprint**: Render the sublist unconditionally and drive visibility from a CSS
  class bound to `open` (the `.app-sidebar-caret.open` pattern at `:179-181` already does this
  for the chevron). Replace the fallback with an explicit error or a `provide_context` at the
  shell root so a missing context is loud rather than silent.

#### [RX-06] Inconsistent error propagation yields silently empty dropdowns
- **Severity**: P2 · **File**: `dashboard/src/ui/pages/guild_settings/mod.rs:78-86`
- **Mechanism**: `get_guild_settings` propagates with `?`, while the other five swallow failure
  via `.unwrap_or_default()`. A Discord outage or a 429 from NET-01 therefore does not surface
  an error — it renders the settings form with **empty channel and role dropdowns**. An admin
  sees "(not set)" as the only option, concludes their channels vanished, and may save the form,
  writing nulls over valid configuration.
- **Remediation Blueprint**: Return a per-section `Result` in the bundle DTO (NET-02) and render
  a scoped inline warning — "Couldn't reach Discord; channel list unavailable" — with the
  dependent `<select>` disabled rather than empty, so no destructive save is possible.

#### [RX-08] `AppShell` is a child component, not a layout route — five resources remount per navigation
- **Severity**: P1 · **File**: `dashboard/src/app.rs:56-69`; `layout.rs:17-27`
- **Mechanism**: The router declares twelve flat `<Route>`s, and each page renders `<AppShell>`
  itself (`modules.rs:21`, `levels.rs:26`, `greetings.rs:69`, `guild_settings/mod.rs:112`, …).
  Because the shell lives *below* the route boundary, navigating `/guild/:id` →
  `/guild/:id/settings` disposes and reconstructs `AppNavBar`, `TierBadge`, `ServerSwitcher`,
  `OperatorBadge` and `OperatorLink` — and every `Resource` they own. Compounding it,
  `layout.rs:24` returns `AnyView` from a closure reading the whole param map, so switching
  between two guilds also tears down `GuildSidebar` and refetches `list_manageable_guilds()`
  (a Discord call) even though the guild list is identical for both.
- **Remediation Blueprint**: Convert to a nested layout:
  `<ParentRoute path=path!("/guild/:id") view=AppShell>` with the per-guild pages as children,
  and pass `guild_id` down as `Signal<String>` rather than `String` so leaves update reactively
  instead of remounting. This is the structural fix that makes NET-03's five chrome resources
  fetch once per session rather than once per navigation.

#### [RX-09] Per-keystroke whole-`HashMap` clones in the save editor
- **Severity**: P1 · **File**: `palworld_save.rs:122-128,151-154,204-213`
- **Mechanism**: `RwSignal::<HashMap<..>>::get()` clones the entire map. Three hot paths do this
  to read one field: the pending-count label clones **both** maps just to sum two `.len()`s; the
  per-player grant summary clones the player map once per row per edit; and `edit_for` — called
  from the `on:change` of every numeric input and every trait `<select>` — clones the whole pal
  map to look up a single entry, then writes it back. That is an O(n) read for an O(1) lookup in
  the app's most interactive path.
- **Evidence**:
  ```rust
  // :204-206 — clones the whole map to read one key
  let edit_for = move |id: &str, pending: RwSignal<HashMap<String, PalEdit>>| {
      pending.get_untracked().get(id).cloned().unwrap_or_else(|| PalEdit { … })
  ```
- **Remediation Blueprint**: Mechanical substitution — `pending.with(|m| m.len())` and
  `pending.with_untracked(|m| m.get(id).cloned())`. No structural change needed.

#### [RX-10] Guild list cloned and grid rebuilt on every keystroke
- **Severity**: P2 · **File**: `operator_servers.rs:86-113`
- **Mechanism**: The filter closure reads `filter`, so each keystroke clones or rebuilds a
  `Vec<GuildInfo>` sized to the bot's entire guild list and returns a fresh `AnyView` — tearing
  down and rebuilding every `<A>`/`<img>` in `GuildGrid`, including a `format!` for each guild's
  CDN URL (`guild_grid.rs:10-36`).
- **Remediation Blueprint**: A `Memo<Vec<GuildInfo>>` keyed on the needle, rendered through a
  keyed `<For>`, turns a full-subtree teardown into per-node add/remove.

#### [UI-13] The server switcher contradicts the badge two lines below it
- **Severity**: P1 · **File**: `server_switcher.rs:29,35-40`; `layout.rs:103-104`
- **Mechanism**: The switcher derives "which guild is selected" from membership in
  `list_manageable_guilds()`, which returns only guilds where the *user* holds
  `ADMINISTRATOR | MANAGE_GUILD` (`server/guild.rs:107-118`). An operator viewing a guild via
  `GuildAccess::Operator` is legitimately on `/guild/:id` but absent from that list — so the
  switcher renders **"Select a server"** with no avatar, while `OperatorBadge` directly beneath
  renders "Operator access" for that same guild. Two components, two answers, same screen.
- **Remediation Blueprint**: Resolve the active guild's name from the URL-resolved guild (the
  server fn already has it) with the id as fallback, rather than from list membership.

#### [SRV-01] One 42-field DTO serves nine tabs; the Family tab uses one field
- **Severity**: P2 · **File**: `dashboard/src/dto/guild.rs:29-73`; `server/guild.rs:129-188`
- **Mechanism**: `get_guild_settings` reads all 11 settings stores and serializes all 42 fields
  on every settings page load, regardless of section. Measured consumption: Family 1 field, AI 2,
  Temp Voice 2, LFG 3, Music 4, Honeypot 4, General 6, Patreon 0, Support ~20. Rendering the
  Family tab therefore ships 41 unused fields — and is the delivery mechanism for SEC-01.
- **Remediation Blueprint**: Per-section projections (`get_general_settings`,
  `get_music_settings`, …) selected by the active slug. Combines cleanly with NET-02's bundle fn:
  the bundle takes the section as a parameter and fetches only that tab's stores.

#### [RX-07] `WebState` bundles twelve unrelated concerns
- **Severity**: P2 · **File**: `dashboard/src/main.rs:44-64`
- **Mechanism**: One `State` extractor carries the app state, OAuth client, OAuth HTTP client,
  invite URL, upgrade URL, Ko-fi token, Patreon app, Patreon webhook URI, Discord client,
  session cache, Leptos options, and the Palworld client. Every handler receives all twelve; the
  Ko-fi webhook handler needs three. The boundaries are invisible at the call site, so it is not
  statically apparent which handlers touch Discord, which touch Patreon, or which touch the save
  directory — the same opacity that let UI-01 ship unnoticed.
- **Remediation Blueprint**: Group into `AuthState`, `IntegrationsState` (Ko-fi + Patreon), and
  `PalworldState`, then implement `FromRef<WebState>` for each — the pattern `LeptosOptions`
  already uses at `:118-122`. Handlers then declare exactly what they consume.

---

## 3. Adjacent Findings — bot-module crates

Outside the Axum/Leptos brief, but they share the same tokio runtime and the same
`AppState.http` client, and two are severe enough to name.

- **Five identical sequential fan-out loops** in `marathon/src/client/{weapon,runner,map,faction,build}.rs`
  (e.g. `weapon.rs:133-137`) do `for slug in &slugs { self.weapon(slug).await }`, where each
  iteration is itself a `tokio::join!` over six external sources. ~30 slugs × a 30s per-source
  ceiling is a worst case in the tens of minutes, on a path with no aggregate deadline. The
  codebase already uses `buffer_unordered` correctly in `patreon/src/announce.rs:75` and
  `family/src/tree/avatar.rs:79` — the idiom is established, just not applied here.
- **Full channel-history scan per reaction** — `suggestions/src/reaction.rs:83-110` paginates the
  *entire* review channel (100 messages per Discord call) on every reaction event to locate one
  message. On a 10k-message channel that is 100 sequential API calls per reaction. Storing the
  review message id on the suggestion row removes it outright.
- **Timeout budget inversion** — `palworld/src/transport/cloudflare.rs:45-51` and
  `marathon/src/transport/mobalytics/clearance.rs:72-78` both send `"maxTimeout": 60_000` to
  FlareSolverr while the client aborts at `HTTP_TIMEOUT` (30s). The solver keeps working; the
  caller always sees a timeout on hard challenges. Separately, `palworld/src/transport/pelican.rs:163-196`
  downloads up to `MAX_SAVE_BYTES = 64 MiB` under that same 30s total budget.
- **Serial per-player Pelican refresh** — `palworld/src/client.rs:577-624` loops
  mtime-check → download → validate/write per player, each download being two round-trips, all
  under `refresh_lock` (`:509`) so every `roster()` caller blocks behind it. Contrast
  `client.rs:399-449`, which does the parallel-spawn-then-collect version correctly.
- **PNG decode on the reactor** — `family/src/tree/avatar.rs:55-58` calls `decode_avatar`
  directly in async, with `AVATAR_FETCH_CONCURRENCY` of them in flight. Every other
  `zayden-graphics` path is behind `spawn_blocking` (`renderer.rs:93-107`); this one is not.
- **DB N+1** — `patreon/src/cron.rs:124-128` does one `insert_post` per post inside a 25-page ×
  20-post loop (up to 500 sequential inserts); `family/src/manager.rs:218-276` has four
  consecutive N+1 loops, 2 queries per relation, not visibly transactional.
- **Retry infra exists and is barely wired.** `zayden-core/src/retry.rs:38-64` implements backoff
  with a transient-status predicate, but is used only by `marathon/src/news.rs:52` and
  `patreon/src/api.rs:64`. No marathon or palworld transport, no Pelican, no wiki client, and no
  dashboard Discord call goes through it — including the FlareSolverr and Mobalytics paths, which
  are the ones most likely to be throttled.

---

## 4. High-Leverage Strategic Interventions

Ordered by return on human review effort.

### Initiative A — Collapse the authorization tax *(NET-01, NET-02, NET-03, NET-05, NET-06, NET-09, NET-10, NET-11, RX-08)*
The single highest-value change. One visit to the main settings page currently costs ~11 server
fn calls, **≈8 `/users/@me/guilds` round-trips and ≈13 session SELECTs**, almost all redundant
re-authorization. Four moves, in order: (1) provide the existing `session_cache` to the Leptos
context and collapse the six duplicated session queries into one cache-aware helper (NET-09 —
the cache is already built, it is simply not wired); (2) cache the
`(user_id, guild_id) -> GuildAccess` decision the same way; (3) introduce `get_settings_bundle`
that authorizes once and `try_join!`s its fetches; (4) convert `AppShell` to a `ParentRoute` so
the five chrome resources stop remounting per navigation. Expected: ~8 Discord calls → 1, and
serial latency → roughly one round-trip. Everything else in this dossier gets faster as a
side effect.

### Initiative B — Stop the UI asserting the opposite of the truth *(DATA-01, DATA-02, RX-06, UI-12, SEC-01)*
The most dangerous class found, because it is silent. A Discord failure currently makes the
Modules page report every module Enabled with a toggle that no-ops; makes the greetings page
report a restricted command as unrestricted; and makes a connected Patreon campaign render as
disconnected with a Connect button. The common root is `unwrap_or_default()` / `Ok(HashMap::new())`
on a fallible read, which conflates "empty" with "unknown". Make unknown representable —
`Option<bool>`, `Result` in the DTO — and render it as a disabled control with a reason. Fold in
SEC-01: stop shipping `faq_wiki_api_key` to eight tabs that never display it.

### Initiative C — Repair the Patreon flow *(UI-01, UI-02, UI-03, UI-08, UI-13)*
The only end-to-end **broken** user journey. A creator who authorizes successfully lands on a
404; if they reach the tab, its controls are unstyled because four of its classes are undefined.
Small, self-contained, independently shippable: fix the URL (generate it from `nav.rs` so it
cannot drift again), read the `?patreon=` param, correct the class names, migrate the raw
`<form method="post">` to a `ServerAction`. Add the mechanical CI guards — a route-resolution
test and a used-vs-defined class grep — because both defects are exactly the kind that recur.

### Initiative D — Make loading and in-flight states first-class *(UI-04, UI-05, UI-06, UI-07)*
32 of 34 mutations have no in-flight feedback, there is one `<Transition>` against ~12
`<Suspense>`, six zero-height fallbacks in the persistent chrome, and **not one skeleton in the
product**. Most of it routes through shared components, so the leverage is high: teaching
`SaveButton` about `pending()` fixes every settings tab at once, and the Suspense→Transition
swaps are five sites. The design system is already ready — `.btn:disabled` is styled and unused.
Add `aria-live` to the feedback component while it is being touched.

### Initiative E — Bound the save editor *(RX-01, RX-09, UI-12)*
The Palworld page is the only place with a data-volume cliff: O(pals × traits) in both
allocations and DOM nodes, plus per-keystroke whole-`HashMap` clones. It will degrade smoothly
until a large enough save file makes it unusable. Share the option table by reference, stop
rendering ~250 `<option>`s per pal, swap `.get()` → `.with()` in the three hot closures, and
bound the list. RX-09 is a mechanical substitution and can land immediately.

### Initiative F — Finish the design system *(UI-09, UI-10, UI-11, RX-05, RX-07, SRV-01)*
Housekeeping with real but diffuse payoff. Spacing is the one un-tokenized axis in an otherwise
disciplined token layer; add a `--space-*` ramp plus `--on-accent`, collapse the duplicated
`.card`/`.settings-section` and the triplicated input block into shared primitives, and migrate
the four heaviest partials. Bundle in the session-cookie `max_age` fix, the pagination dead-end,
the `WebState` split, and SRV-01's per-section DTO projection.

---

## 5. Hypotheses Tested and Rejected

Recorded so these are not re-investigated, and so the report is not read as uniformly negative.
Several of the rubric's assumed anti-patterns simply are not present in this codebase.

- **"`reqwest::Client` is re-allocated per request."** False, and the opposite of the truth.
  `zayden-app/src/services/http.rs:11-14` defines `ClientBuilderExt::with_timeouts()` (30s/10s)
  and `state/app_state.rs:13-22` builds **one** shared client with it, passed by reference
  everywhere. Every `reqwest::Client::new()` in the workspace is in `tests/`. The
  `bearer_client` problem (NET-01) is a *twilight* problem specifically, and it stands out
  precisely because the reqwest discipline around it is exemplary.
- **"Outbound HTTP lacks timeouts."** Largely false. Per-request budgets exist at
  `palworld/src/transport/paldex.rs:160`, `palcalc.rs:176`, `patreon/src/api.rs:65`,
  `patreon/src/thumbnail.rs:31`, and `palworld/src/autocomplete.rs:106`. Only the twilight path
  is uncovered (NET-04).
- **"Signals are misused; effects are syncing derived state."** False — there is **no
  `Effect::new` / `create_effect` anywhere in the crate**, and no `Signal::derive`. The two
  `Memo`s that exist (`layout.rs:174`, `guild_settings/mod.rs:55`) are both well-formed. The
  reactivity findings above are about *missing* memoization and cloning, not misuse.
- **"`use_context().unwrap()` will panic and blank the UI."** False. All five server-side context
  reads use `ok_or_else` with a message (`auth.rs:29-52`) and `kofi.rs:14-16` uses a `let-else`.
  No panic path was found. The failure mode here is the opposite — errors turned into empty
  values (RX-06, DATA-01, DATA-02).
- **"Local signals duplicate URL or context state."** False. The URL is the sole source of truth
  for guild selection; no signal or context shadows it. The problems are that the derivation is
  duplicated across six files and passed as a non-reactive `String` (RX-08), and that one
  component derives selection from the wrong *source* (UI-13) — not that state is duplicated.
- **"Blocking I/O runs on the async runtime."** False where it matters most. The heaviest
  operation in the app — reading and rewriting multi-hundred-MB Palworld saves — is correctly
  offloaded at `routes_palworld_save.rs:46-48`, and `zayden-graphics` wraps resvg + PNG encode in
  `spawn_blocking` behind a permit semaphore (`renderer.rs:93-107`). The `std::sync::Mutex` in
  `server/supersede.rs:15-54` is held only across non-await critical sections — correct, though
  it deserves a comment saying so. The two genuine exceptions are noted in §3.
- **"Images lack dimensions, causing layout shift."** False. Five of six `<img>` tags omit HTML
  `width`/`height`, but every corresponding CSS class sets explicit dimensions —
  `.server-switcher-avatar` 30×30 (`layout.css:136-142`), `.guild-icon` 48×48
  (`pages.css:137-142`), `.rr-emoji-img` 24×24, `.lb-avatar` 32×32, `.greet-thumb` 100%×9rem.
  The boxes are reserved before load. Residual nits only: `guild_grid.rs:12-16` and
  `server_switcher.rs:18-21` request `?size=64` for 48px and 30px elements, and `greetings.rs:439`
  is the only image with an `onerror`-worthy third-party source but has no fallback.
- **"Tailwind div-soup / arbitrary values / inline styles."** False. `grep` for
  `class="…[…]"` returns **zero** hits, and the only two `style=` attributes
  (`module_card.rs:14`, `landing.rs:91`) are legitimate CSS-custom-property injection. Styling is
  a hand-written, well-factored 2,044-line system across 20 partials with a real token layer and
  a four-bot theming scheme. (One caveat: `icons.rs:105-117`'s `module_tint` holds ten hardcoded
  Tailwind-palette hexes outside `tokens.css`, so a bot theme swap recolours the accent but
  leaves module tints unchanged.)
- **"Modal/flyout logic is fragmented ad-hoc per page."** False. There are no bespoke modals at
  all; every disclosure uses native `<details>` (`server_switcher.rs:44`, `palworld_save.rs:92`,
  `faq.rs:37`). That is *consistent* — the gaps are the absence of a confirmation primitive
  (UI-08) and the native element's lack of click-outside/Escape/focus-trap behaviour, not
  fragmentation of an existing pattern.

**Convention note (outside the three vectors, flagged once):** `palworld_save.rs:430-453` uses an
inline `#[cfg(test)] mod tests`, which `CLAUDE.md` explicitly forbids ("Tests are flat `#[test]`
fns never inline `#[cfg(test)] mod tests`"; "Dev files live outside `src/`"). It is the only such
block in `dashboard/src/`.

---

## 6. Verification

This audit mutated nothing. To confirm the baseline is green before remediation begins:

```bash
cargo clippy -p dashboard --features ssr -- -D warnings
```

```bash
cargo clippy -p dashboard --target wasm32-unknown-unknown --features hydrate -- -D warnings
```

Note `--all-features` must **not** be used — `ssr` and `hydrate` are mutually exclusive and do
not co-build. Clippy never codegens, so `cargo build --workspace --all-targets` (or `bacon build`)
is still required to catch link failures. Check `pgrep bacon` first; if bacon is running, touch a
file before reading `.bacon-locations`, since it only re-runs on file change.

Per-finding verification once fixes land:
- **UI-01**: `grep -rn '"/guilds/' dashboard/src/` must return no parameterised path; add a test
  asserting each `settings_url` outcome resolves to a real route.
- **UI-03**: every `btn-*` in `dashboard/src/` must have a match in `dashboard/style/` — the
  cross-check that surfaced this is one grep and belongs in CI.
- **UI-04**: `grep -rn 'pending()' dashboard/src/` should return ~30 hits, not zero.
- **NET-01/02**: instrument `bearer_client` with a `tracing` counter and assert one
  `/users/@me/guilds` call per settings page render, not seven.
- **RX-01**: load a save with 500+ pals and confirm DOM node count is bounded by viewport.
