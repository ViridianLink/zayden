# Audit: ai

_Audited: 2026-07-17 · Commit: `2833ce8`_

## Summary

Tiny (~173 LOC, 4 files) OpenAI chat wrapper (`chat.rs`, `openai.rs`, `error.rs`).
Uses the `async-openai` `Client` with an injected `http_client`. One `tests/`
file present. Clean; only worth confirming the HTTP client sets a timeout.

## Findings

### 1. Confirm outbound request timeout  ·  #3  ·  low
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
