# Audit: ai

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Tiny (~173 LOC, 4 files) OpenAI chat wrapper (`chat.rs`, `openai.rs`, `error.rs`).
Uses the `async-openai` `Client` with an injected `http_client`. One `tests/`
file present. Clean; only worth confirming the HTTP client sets a timeout.
_(Confirmed 2026-08-04: it did not. See finding #1 — the one open question in
this module turned out to be a real defect.)_

## Findings

### 1. Confirm outbound request timeout  ·  #3  ·  low
- **Status:** `complete — df39b21d`            <!-- open | in-progress | in-review | complete | wontfix -->
- **Confirmed, then fixed (2026-08-04).** The finding asked us to *confirm* the
  injected client had a timeout. It did not. `openai.rs:44` built its own
  `reqwest::ClientBuilder::new().default_headers(headers).build()?` — headers
  only — and `reqwest`'s default is **no** timeout, so a provider that accepted
  the connection and then went silent left `AiClient::chat` pending forever,
  wedging the command task holding it. The audit's suggested fix ("ensure the
  *shared* client has a timeout") does not apply as written: `AppState::http` is
  already bounded (30s/10s), but `ai` cannot use it — it needs the OpenRouter
  `http-referer`/`x-title` default headers a shared client cannot carry, which is
  why it builds its own.
- **Fix.** Gave the budget a single owner instead of a second policy.
  `zayden-app/src/services/http.rs` (new) holds `HTTP_TIMEOUT` (30s),
  `HTTP_CONNECT_TIMEOUT` (10s) and a `ClientBuilderExt` extension trait whose
  `with_timeouts()` applies both — method position, so it chains into an
  existing builder rather than wrapping it (owner's call, 2026-08-04).
  `AppState::http_client` now builds through it (the durations moved out of
  `state/app_state.rs`, unchanged in value), and `AiClient::new` chains it onto
  its header-carrying builder. `ai` gains a direct
  `zayden-app` dep — already in its graph transitively via `zayden-core`, so no
  new crate and no cycle (`zayden-app` does not depend on `ai`).
- **Regression test** `tests/openai.rs::chat_gives_up_on_a_hung_upstream_instead_of_waiting_forever`:
  a `spawn_black_hole_server` accepts, drains the request, then holds the socket
  open silently forever (never answers, never hangs up — a closed socket would
  surface as a connection error rather than the hang the finding describes).
  Runs `#[tokio::test(start_paused = true)]`, so the client's real 30s budget is
  virtual and costs the suite nothing; the assertion is that the client's own
  timer fires before a 60s backstop. **Fails-before** (`Elapsed(())` at the
  backstop — the client never gave up), **passes-after** in 0.74s, asserting
  `AiError::OpenAI(OpenAIError::Reqwest(e)) if e.is_timeout()`. Needed
  `tokio/test-util` in `[dev-dependencies]`.
- **Gates:** `cargo +nightly clippy --workspace --all-targets -- -D warnings`
  clean; `cargo test --no-fail-fast` 604 passed / 0 failed / 7 ignored;
  `cargo machete` clean (dep lists changed); `cargo +nightly fmt`;
  `-p dashboard --features ssr` clean (`zayden-app` is a dashboard dep). No SQL
  touched, so no `.sqlx` regen. No new `#[allow]`/`#[expect]`. Two clippy lints
  pull against each other on the trait method: `return_self_not_must_use`
  demands `#[must_use]`, while `double_must_use` rejects a bare one because
  `ClientBuilder` is itself `#[must_use]`. Both are satisfied by giving the
  attribute an explicit reason (`#[must_use = "the budget applies to the
  returned builder, not to this one"]`) rather than suppressing either.
- **Where:** `src/openai.rs:48`
  (`Client::with_config(config).with_http_client(http_client)`).
- **What:** The completion call goes to a remote LLM API; verify the injected
  `http_client` sets a `.timeout(...)` so a slow/hung upstream can't wedge the
  request future.
- **Why it matters:** No timeout on an LLM call is a foot-gun (they can be slow).
- **Suggested fix:** Ensure the shared `reqwest::Client` has a timeout; add one
  if not.

## Clean
- #1 Architecture: minimal, single-responsibility wrapper.
- #1 DB access: n/a (no DB).
- #2 Dead code: none found.
- #3 Async: no blocking I/O; no `unwrap()`/`expect()` on the call path.
- #6 Tests: one `tests/` file present.
- #7 Lint: no `#[expect]`/`#[allow]`.
