---
eimp: 8
title: Code-review findings — einmo library, review server, and zweimomo
author: opencode (z-ai/glm-5.2) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-31
supersedes: []
begun: [ ]
---


# EIMP-8: Code-review findings — einmo library, review server, and zweimomo

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

## Abstract

A read-only code review of three surfaces — the `einmo` library (focusing on
`src/review.rs`, the `EinmoReview` session object), the review server
(`src/review_server.rs` + `src/bin/einmo_review_server.rs`), and the
`zweimomo` companion crate — surfaced **twenty-five findings** (P0–P24),
grouped into three severity tiers. One finding (P0) currently breaks the
hard toolchain gate (`cargo clippy --all-targets -- -D warnings`) and
therefore blocks all substantive work per AGENTS.md. The rest are
correctness/soundness, concurrency/async, scalability, and
convention-hygiene items, each with a concrete file:line pointer and a
recommended remediation. This EIMP exists to hand the review to a second
agent for triage and improvement: nothing here is specified as in-scope to
implement yet — the `Draft` status records the findings; `begun: [ ]` stays
until a maintainer decides which P-items to accept and promotes this EIMP
to `Implementing`.

## Motivation

`EIMP-1`'s maintainer-review pass produced the P0–P12 "maintainer-found
defects" record, and `EIMP-7` was spun out of that record's own P1
recommendation. That established the repo's convention that a thorough
read of an already-"complete" surface produces a **numbered, location-stamped
findings list**, which is then triaged into "fix in place" vs "spin into its
own EIMP" — rather than being silently patched or dropped. This EIMP
continues that discipline for three surfaces that have not yet had such a
pass since reaching `complete`:

1. **`src/review.rs`** — `EinmoReview`, the thread-safe review-session
   object (`EIMP-1`/`EIMP-2`, `complete`). Well-encapsulated, well-tested,
   but carries a `Drop`-impl panic risk, a drift-detection error-swallowing
   path, and a `plan()`/`execute()` action-shape race the existing comments
   slightly overstate as resolved.
2. **`src/review_server.rs` + `src/bin/einmo_review_server.rs`** — the axum
   HTTP app and its CLI binary (`EIMP-2`, `complete`). Strong typed-extractor
   discipline and good authz separation, but blocks the tokio runtime inside
   `post_execute`, leaks sessions unboundedly, and ships a default socket
   mode that is not hardened the way `--private` is.
3. **`zweimomo/`** — the JavaScript-only companion crate (`EIMP-2` §8,
   `complete`). Small and mostly clean, but `lib.rs` `pub mod`s an internal
   module (a `rust_instructions` §"Don't" violation) and `BoaEvaluator`
   mints a fresh `Context` per call without documenting the cost.

A review that finds nothing is suspicious; this one found twenty-five
items, of which one blocks the build and five are real correctness or
soundness concerns. None of the twenty-five is a "the design is wrong"
finding — the architectures of all three surfaces are sound. The findings
are localized defects, each with a recommended fix, so a second agent can
triage and apply them without re-deriving the review.

## Specification

This EIMP's "specification" is the findings list itself: each P-item is a
self-contained record with **Location**, **Finding**, **Severity**, and
**Recommended remediation**. The implementing agent's job is to read each
record, decide accept/reject/defer, and apply the accepted ones in the order
the plan file prescribes (highest-severity, lowest-blast-radius first).

Severity tiers used below:

- **Blocker** — breaks a hard toolchain gate; no substantive work may
  proceed per AGENTS.md until fixed.
- **High** — a real correctness or soundness defect; should be fixed before
  the next release.
- **Medium** — a robustness, concurrency, or scalability concern; fix when
  the affected code is next touched, or when a profile shows it matters.
- **Low** — a hygiene, documentation, or minor-convention deviation; fix at
  leisure, ideally in the same commit as a nearby change.

### S.1 — Library findings (`src/review.rs` and friends)

#### P0 — `verify.rs:451` breaks `cargo clippy --all-targets -- -D warnings`

- **Location:** `src/verify.rs:451`
- **Finding:**
  ```rust
  let gate_fails_with_override = flagged_count > 0 && !true; // !flag_is_not_failure
  ```
  The `--flag-is-not-failure` override is dead code: `!true` is always
  `false`, so `gate_fails_with_override` is always `false` regardless of
  `flagged_count`. clippy denies this (`clippy::overly_complex_bool_expr` +
  `clippy::nonminimal_bool`) and the **entire** `cargo clippy --all-targets
  -- -D warnings` gate fails, not just the verify module. This is not in the
  review/zweimomo scope the review was scoped to, but it blocks the
  toolchain gate for the whole crate and therefore blocks every other
  finding's verification.
- **Severity:** Blocker.
- **Recommended remediation:** The test clearly *intends* to assert that
  `--flag-is-not-failure` downgrades flags to advisory — but the test's own
  harness never actually flips that config bit, so the override is
  untestable from inside this test as written. Either (a) drive the
  config through `TestConfig::with_flag_is_not_failure(true)` (or
  whatever the real accessor is) and recompute the gate honestly, or (b)
  if no such config path exists yet, delete the dead assertion and leave a
  `// TODO(--flag-is-not-failure): …` note rather than shipping a boolean
  tautology. Verify with `cargo clippy --all-targets -- -D warnings` after.

#### P1 — `EinmoReview::Drop` can panic-on-poison and abort the process

- **Location:** `src/review.rs:1128-1140` (`impl Drop for EinmoReview`),
  depending on `src/journal.rs`'s `Journal::log_at` taking its internal
  `Mutex<Option<File>>` via `.expect("…poisoned")`.
- **Finding:** `Drop::drop` calls `self.journal.log_at(Terse, SessionClose)`.
  `log_at` is documented as infallible ("logging degrades silently rather
  than ever failing the review"), but its implementation `.expect()`s the
  journal's internal `Mutex`. If a panic during the session poisoned that
  mutex, `drop` panics-on-unwind → aborts the process (no `Drop` for the
  journal, no clean shutdown, no further `Drop` runs). The documented
  "infallible" contract is therefore not actually upheld under poison.
- **Severity:** High.
- **Recommended remediation:** Take the journal's internal mutex with
  poison-recovery: `lock().unwrap_or_else(|e| e.into_inner())` (the
  standard `std` poison-recovery pattern). This makes `log_at` truly
  infallible and `Drop` panic-free. Add a unit test that poisons the
  journal mutex (via a deliberate panic in a `log_at` call on a
  separate thread) and then drops the `EinmoReview` without aborting.

#### P2 — `decide` swallows transient fingerprint I/O errors, silently disabling drift detection

- **Location:** `src/review.rs:643-644`
  ```rust
  let basis = decision_basis_path(&self.config, &id, &decision)
      .and_then(|p| Fingerprint::of(&p).ok());
  ```
- **Finding:** `Fingerprint::of` returns `Err` on a transient `metadata()`
  failure (e.g. a brief NFS hiccup, an EBUSY on Windows). The `.ok()`
  turns that into `None`, which `execute` later treats as "no recorded
  basis → nothing to compare → proceed" (`review.rs:905-907`). A transient
  stat failure during `decide` therefore silently disables drift detection
  for that decision — exactly the failure mode `EIMP-1` §S.2's
  fingerprint re-check exists to prevent. `None` is a *legitimate* value
  (the basis file genuinely doesn't exist for some decision kinds), so the
  fix must distinguish "file absent" (legitimate `None`) from "stat
  errored" (should propagate, or record a sentinel that forces `execute`
  to skip).
- **Severity:** High.
- **Recommended remediation:** Have `Fingerprint::of` distinguish
  `NotFound` (→ legitimate `None`) from other I/O errors (→ propagate as
  `EinmoError::Io`). In `decide`, propagate that error instead of
  `.ok()`-ing it: a `decide` that can't stat its basis should fail loudly,
  not silently record a decision that `execute` will then apply without a
  drift check. Add a test that injects a stat failure (e.g. by pointing
  the basis path at a directory's parent with no read permission, or by
  mocking) and asserts `decide` returns `Err`.

#### P3 — `execute` applies the caller's `plan.actions` but only re-checks the *basis* against the live `DecisionBook`

- **Location:** `src/review.rs:888-919` (the drift filter), with the
  documented claim at `review.rs:896-898`:
  > Checked against the LIVE DecisionBook (not `plan` itself, which carries
  > no fingerprint) so a decision changed or cleared between plan() and
  > execute() is caught the same way.
- **Finding:** The claim is half-true. The drift filter re-derives the
  *basis fingerprint* from the live `DecisionBook` (`decisions.get_entry(id)`),
  but the *action* it appends to `actions` is the one from `plan.actions`
  — the caller's snapshot. If a concurrent `decide` *replaces* the decision
  with a different *kind* (e.g. `Promote` → `Retract`) whose basis
  fingerprint happens to match (both `Decision::Promote { to: Checked }`
  and `Decision::Retract { from: Checked }` fingerprint the same `checked/`
  path), `execute` will apply the stale *promote* action against the
  reviewer's fresh *retract* intent. The exec mutex serializes
  `execute`/`flag_now`/`retract_now` against each other, but **not**
  against `decide`/`undecide`. The HTTP and CLI paths call `plan()` →
  `execute()` back-to-back so the race window is tiny, but the comment's
  "caught the same way" overstates the guarantee.
- **Severity:** Medium (high if multi-session review lands).
- **Recommended remediation:** Either (a) re-derive the actions from the
  live `DecisionBook` inside `execute` (ignoring `plan.actions` for the
  action list, but still using the plan as the *set* of ids to act on —
  so a `decide` between `plan()` and `execute()` that replaces the kind
  surfaces as "the live decision disagrees with what plan() promised,
  skip and report"), or (b) keep `plan.actions` authoritative but
  re-check the *kind* (not just the basis) against the live book and push
  to `skipped` on mismatch. Either way, correct the comment to match
  what's actually checked. Add a test that `decide`s, calls `plan()`,
  then `decide`s a *different kind* for the same id, then `execute`s, and
  asserts the id is `skipped` (not executed with the stale kind).

#### P4 — `refresh()` holds the decisions read lock across per-entry filesystem stat calls

- **Location:** `src/review.rs:667-678`
- **Finding:** `refresh()` takes `self.decisions.read()` and holds the guard
  for the whole body, which calls `Fingerprint::of(&current_path)` (a
  filesystem `metadata()` call) once per pending decision. For a large
  pending set this blocks writers (`decide`/`undecide`/`execute` all take
  the write lock) for the duration of N stat calls. The lock is a
  `RwLock`, so concurrent readers are fine, but any decider is stalled.
- **Severity:** Medium.
- **Recommended remediation:** Snapshot the `(id, basis)` pairs under the
  read lock into a `Vec`, drop the guard, then stat each path outside the
  lock. The journal/decisions stay consistent because `refresh` is
  read-only; a `decide` that lands mid-stat just isn't reflected in this
  `refresh` call's report (which is acceptable — `refresh` is advisory).
  Add a test that holds `refresh` for a long pending set while a
  `decide` thread contends and asserts the `decide` completes promptly.

#### P5 — `items()` re-walks the suite and re-runs `case.agreement` (verify-on-inspect) on every call, bypassing `VerifiedCache`

- **Location:** `src/review.rs:538-574` (esp. `case.agreement(&[Output,
  Checked], …)` at `review.rs:549-552`), which goes through `EinmoCase` →
  `EinmoFile::from_file` directly, not through `VerifiedCache`.
- **Finding:** `GET /einmo/{session}/cases` re-scans the suite directory and
  re-runs `case.agreement` for every case on every request.
  `case.agreement` verify-on-inspects the `output/` and `checked/`
  artifacts. The single-flight `VerifiedCache` (used by `body()`) is **not**
  consulted here, so each `items()` call re-verifies every output+checked
  artifact from scratch. For a large suite this is a hot path: a reviewer's
  worklist refresh after every `decision-made` SSE event re-verifies the
  whole corpus.
- **Severity:** Medium (will become High once multi-session / large-corpus
  review lands).
- **Recommended remediation:** Route `agreement`'s verify-on-inspect
  through `VerifiedCache` (or a sibling agreement cache keyed by the same
  `Fingerprint`), so a `body()` call and an `items()` call for the same
  artifact share one verification. Alternatively, add a per-session
  `agreement_cache: VerifiedCache<StagePairAgreement>` populated lazily.
  Benchmark before/after on a 1000-case suite; the cache self-invalidates
  on edit via `Fingerprint` (path+len+mtime), so correctness is preserved.

#### P6 — `shuffle` uses modulo reduction, introducing negligible bias

- **Location:** `src/review.rs:274-280`
  ```rust
  let j = (OsRng.next_u32() as usize) % (i + 1);
  ```
- **Finding:** `next_u32() % (i+1)` is non-uniform when `i+1` does not
  divide `2^32`. For a review worklist ordering this is irrelevant (not
  security-sensitive, just a reordering), but it is technically biased.
- **Severity:** Low.
- **Recommended remediation:** Either accept and add a `// non-uniform;
  // ordering only, not security — modulo bias is acceptable here` comment,
  or use `rand_core`'s `fill_uniform`-style API if a dependency-light
  uniform sampler is available without adding `rand`.

### S.2 — Review server findings (`src/review_server.rs` + binary)

#### P7 — `post_execute` runs synchronous `review.execute(...)` on the tokio worker thread

- **Location:** `src/review_server.rs:732` (`review.execute(&plan, &keys)`),
  called from `async fn post_execute` (`review_server.rs:705`).
- **Finding:** `EinmoReview::execute` is synchronous: it does filesystem I/O
  (promote/retract/flag) and Argon2id key derivation (~1.8s by design, per
  `signature.rs`'s pinned parameters) and holds a `std::sync::Mutex` (the
  `exec` lock) for the whole duration. Called from an `async fn` on the
  tokio runtime, this blocks the worker thread for ~2s per execute and
  holds a std `Mutex` across the async context (not across an `.await`, so
  it's not the canonical "Mutex across await" footgun, but it is blocking
  the runtime). `rust_instructions` §Concurrency: "never hold a lock across
  `.await`" and "make concurrency explicit and testable."
- **Severity:** Medium (acceptable for single-user loopback today; becomes
  a real problem under concurrent multi-session load).
- **Recommended remediation:** Wrap the `review.execute(&plan, &keys)` call
  in `tokio::task::spawn_blocking`, returning a `JoinHandle` whose result
  the handler `.await`s. The `exec` `Mutex` then lives entirely on the
  blocking thread, never touching the async runtime. Note this changes
  nothing about `execute`'s own serialization (the `exec` mutex still
  serializes execute/flag_now/retract_now) — it just moves the blocking off
  the runtime. Add a test that fires two concurrent `POST … /execute`
  requests against the same session and asserts they serialize (the second
  blocks until the first completes) rather than corrupting state.

#### P8 — Default `--socket .einmo-review.sock` is not hardened; only `--private` mode hardens

- **Location:** `src/bin/einmo_review_server.rs:309-319` (the `else` branch
  uses `args.socket` verbatim) and `src/review_server.rs:971-998` (`serve_uds`
  binds whatever path it's given).
- **Finding:** The standalone `serve` mode binds `--socket` (default
  `.einmo-review.sock` in CWD) without hardening the socket file or its
  containing directory. The suite lock (`SuiteLock::acquire`) prevents a
  *second server* from binding the same suite, but does **not** prevent an
  unauthorized *client* from connecting to a world-traversable socket in a
  CWD with a permissive umask. The spec (`EIMP-1` §S.7) frames "directory
  permissions are the access control" as if it always holds, but it only
  holds if the directory is restrictive; a CWD socket in a world-readable
  directory is exposed. `--private` (`private_socket_path`) hardens both
  the base and the per-session leaf to 0700; the default mode does not.
- **Severity:** Medium (depends on deployment umask; could be High in a
  shared dev box).
- **Recommended remediation:** Either (a) harden the socket file's
  containing directory in `serve_uds` (reuse `journal::harden_dir` on the
  socket's parent, with a comment that this is the default-mode equivalent
  of `private_socket_path`'s hardening), or (b) document explicitly in the
  `--socket` help text that the default mode is only safe under a
  restrictive umask / a user-owned directory, and `--private` is the
  hardened alternative. Prefer (a). Add a test that binds a default-mode
  socket in a temp dir and asserts the parent directory is 0700.

#### P9 — `AppState::sessions` grows unboundedly; no `close_session`/eviction

- **Location:** `src/review_server.rs:155-175` (`AppState` + `create_session`).
- **Finding:** `create_session` inserts but nothing removes. The standalone
  `serve` mode opens one session so it's moot today, but the doc frames
  `AppState` as shaped for "eventual multi-session support" (`EIMP-1` §S.7's
  session-scoped routes). A long-lived server (the future multi-session
  shape) leaks memory across sessions.
- **Severity:** Low today, Medium once multi-session lands.
- **Recommended remediation:** Add `pub fn close_session(&self, id:
  SessionId)` that removes from the map, and a `Drop for AppState` (or a
  periodic reaper task) that drops all sessions. Alternatively, an
  idle-session TTL reaper. Add a test that creates and closes N sessions
  and asserts the map is empty. Document the lifecycle in `AppState`'s
  doc comment.

#### P10 — `SessionId` is minted predictably (incrementing `u64`); doc claims "opaque"

- **Location:** `src/review_server.rs:33-40` (`SessionId(u64)` + `Display` as
  `{:016x}`), `review_server.rs:164-167` (`create_session` uses
  `AtomicU64::fetch_add`), and the doc at `review_server.rs:31-32`:
  > A session identifier — opaque from the client's perspective, minted by
  > `AppState::create_session`.
- **Finding:** The id is `0`, `1`, `2`, … — easily guessable. The doc's
  "opaque from the client's perspective" overstates reality. For UDS,
  directory permissions are the real gate (so predictability is fine); for
  TCP, the bearer token is the real gate. The predictability is therefore
  *acceptable* under the current trust model, but the doc should match
  reality.
- **Severity:** Low.
- **Recommended remediation:** Either (a) change the doc to "sequential,
  minted by `AppState::create_session`; not a secret — the access control is
  the socket's directory permissions (UDS) or the bearer token (TCP)," or
  (b) if true opacity is desired, mint `SessionId` from `OsRng` (128 bits,
  matching `review.rs::random_session_id`). (a) is cheaper and matches the
  trust model; (b) is the defense-in-depth option.

#### P11 — `serve_review_dhtml` and `serve_review_dhtml_root` are byte-identical; `/review/{session}` ignores the `{session}` segment

- **Location:** `src/review_server.rs:806-833` (routes), `review_server.rs:1104-1118`
  (two identical handlers).
- **Finding:** Both handlers serve `REVIEW_DHTML` with no `Path` extractor.
  The `/review/{session}` route therefore ignores the `{session}` segment
  entirely — any value (including a malformed one) serves the shell. A
  `Path<SessionId>` extractor on `/review/{session}` would 400 on a
  malformed segment and 404 on an unknown session before serving the shell,
  matching the typed-extractor discipline the rest of the server follows.
- **Severity:** Low.
- **Recommended remediation:** Either (a) dedupe to one handler and add a
  `Path<SessionId>` extractor on `/review/{session}` that 404s on unknown
  sessions (so a browser hitting `/review/bogus` gets a 404, not the
  shell), or (b) if the DHTML client genuinely reads the session from the
  URL client-side and the server shouldn't validate, document that intent
  in a comment and keep one handler serving both routes. Prefer (a) for
  consistency with the rest of the typed-extractor surface.

#### P12 — `case_detail` calls `items()` and filters — full suite re-scan to fetch one case

- **Location:** `src/review_server.rs:285-297` (`case_detail`).
- **Finding:** `GET /einmo/{session}/cases/{id}` re-scans the whole suite
  (via `review.items()`) and then linear-filters for the one id. Combined
  with P5, this re-verifies every output+checked artifact per
  single-case fetch.
- **Severity:** Low (shares P5's fix).
- **Recommended remediation:** Add a `review.case(&id)` (or have `items()`
  accept an optional single-id filter) that scans only the one case's
  stage directories, not the whole suite. This pairs naturally with P5's
  cache routing. Alternatively, accept the re-scan for now and document
  that `case_detail` is O(suite) not O(1).

#### P13 — `delete_decision` publishes `DecisionMade` (not a distinct `decision-cleared` event)

- **Location:** `src/review_server.rs:488-501` (`delete_decision`).
- **Finding:** Both recording and clearing a decision emit
  `event: decision-made`. Documented as intentional ("recorded, replaced,
  or cleared"), but a DHTML that wants to skip a full re-fetch on clear
  can't distinguish "a decision was just made" from "a decision was just
  cleared" without re-fetching the case.
- **Severity:** Low.
- **Recommended remediation:** Add a `DecisionCleared` variant (or a
  `cleared: bool` field on `DecisionMade`) so a subscriber can skip the
  re-fetch on clear. Alternatively, accept the current shape and document
  that any `decision-made` event means "re-fetch the affected case" —
  which is the DHTML's current behavior anyway.

#### P14 — `CaseSummary.decision` embeds the flag `reason` verbatim in the worklist response

- **Location:** `src/review_server.rs:267-264` (`decision_tag` →
  `format!("flag {stage}: {reason}")`), surfaced in the `GET /cases`
  listing.
- **Finding:** A reviewer-entered flag `reason` travels in the worklist
  payload. Reasons are advisory, but if a reason could contain sensitive
  context (a bug number, a customer name), it's exposed to every reader of
  the listing.
- **Severity:** Low.
- **Recommended remediation:** Document in `decision_tag`'s doc comment
  that the reason is included verbatim and advisory-only; if a future
  deployment needs to redact reasons in the listing, add a `reason_summary`
  accessor that truncates. No change needed for the current trust model
  (local-only review), but the data flow should be explicit.

#### P15 — `tcp_guard` bearer-token comparison is not constant-time

- **Location:** `src/review_server.rs:875-878`:
  ```rust
  v.strip_prefix("Bearer ")
      .is_some_and(|presented| presented == guard.token)
  ```
- **Finding:** The bearer token is compared with `==` (short-circuiting,
  timing-sensitive). The token is a loopback access token, not a secret
  key, so timing attacks aren't practically exploitable over a
  loopback-only TCP listener — but any authz comparison benefits from
  constant-time discipline.
- **Severity:** Low.
- **Recommended remediation:** Use `subtle::ConstantTimeEq` (or a manual
  byte-by-byte compare that doesn't short-circuit) for the token
  comparison. Document that this is defense-in-depth, not a real attack
  surface given the loopback-only constraint. If `subtle` is not already
  a dependency and adding it is heavyweight, a 4-line manual
  constant-time compare is fine.

#### P16 — `ExecuteResponse`/`PlanResponse` DTOs `derive(Deserialize)` though only ever returned

- **Location:** `src/review_server.rs:558`, `631` (and several others).
- **Finding:** These response DTOs are only ever serialized outbound; the
  `Deserialize` derive is unused and widens the surface.
- **Severity:** Low.
- **Recommended remediation:** Drop `Deserialize` from pure-response DTOs.
  Trivial, do it in the same commit as a nearby change.

### S.3 — zweimomo findings (`zweimomo/`)

#### P17 — `BoaEvaluator` mints a fresh `Context::default()` per `evaluate()` call; cost is undocumented

- **Location:** `zweimomo/src/evaluators.rs:19` (`let mut context =
  Context::default();`).
- **Finding:** `Context` is `!Send` and constructed fresh per call —
  required for thread-safety, but the per-call construction cost (Boa's
  context initialization builds the global object, built-in prototypes,
  etc.) is paid for every input evaluation. For `day.1`'s handful of
  inputs this is invisible; for a large corpus it's measurable. The
  serialization choice (`to_std_string_escaped`) is documented; the
  `Context`-per-call choice is not.
- **Severity:** Low.
- **Recommended remediation:** Add a doc comment on `BoaEvaluator`
  explaining: (1) `Context` is `!Send` so it cannot be reused across
  `evaluate` calls on different threads (the `Evaluator` trait is
  `Sync`); (2) a `thread_local!`-cached `Context` *could* amortize the
  cost across same-thread calls — sketch as a rejected alternative if not
  implemented, or implement if a profile shows it matters on a large
  corpus. No change required for `day.1`.

#### P18 — `zweimomo/Cargo.toml` has no `[lints.rust] unsafe_code` gate

- **Location:** `zweimomo/Cargo.toml` (entire file, 22 lines).
- **Finding:** `rust_instructions` §2f says crypto-touching crates should
  declare `[lints.rust] unsafe_code = "deny"`. `zweimomo` is not
  crypto-touching directly (it's an evaluator harness), but it's part of
  the signed-baseline pipeline (its `boa_engine` output text lands in
  signed `output/` baselines). The crate has no `unsafe` today, so
  `deny` would be a no-op gate that prevents `unsafe` creeping in later.
- **Severity:** Low.
- **Recommended remediation:** Add `[lints.rust] unsafe_code = "deny"` to
  `zweimomo/Cargo.toml` (or, if a workspace `[workspace.lints]` exists,
  ensure `zweimomo` inherits it). Verify with `cargo clippy` showing no
  change.

#### P19 — `crash_crumb_survives_stack_overflow` relies on runtime stack-overflow detection

- **Location:** `zweimomo/tests/suites.rs:90-140` (esp. `recurse(usize::MAX)`
  at line 99).
- **Finding:** The test forces a stack overflow via infinite recursion and
  asserts the crash-crumb survived. This depends on the runtime's
  stack-overflow detection (the default thread stack size and the
  allocator's behavior). If the runtime ever changes the stack guard or
  the test binary is built with a non-default `RUST_MIN_STACK`, the test
  could become flaky or fail to crash. The test is exercising a defensive
  path (crash-crumb survival), so the brittleness is acceptable, but the
  dependency should be documented.
- **Severity:** Low.
- **Recommended remediation:** Add a comment at the `recurse(usize::MAX)`
  line noting the test depends on the default thread stack size triggering
  a guard-page SIGSEGV, and that if it ever becomes flaky the fix is to
  spawn the recursion on a thread with an explicitly small stack
  (`std::thread::Builder::new().stack_size(64 * 1024)`).

#### P20 — `run_tier` exercises generation+self-verification but not the `output==checked` gate

- **Location:** `zweimomo/tests/suites.rs:59-80` (`run_tier`).
- **Finding:** The test asserts each output was `written_and_verified` but
  does not assert `output==checked` correspondence — that gate is enforced
  by the `einmo` CLI after human review, per the README. The test is
  therefore exercising the "dog-food" half (does the runner produce
  verifiable output) but not the "reviewed baseline" half. Acceptable per
  the doc, but the gap is worth recording so a future test doesn't assume
  the checked baseline is verified by this test.
- **Severity:** Low.
- **Recommended remediation:** Add a comment in `run_tier` noting the
  checked-baseline gate is intentionally not exercised here (it's a
  human-review action, not an automated one), and that
  `eimp3_output_drift_comprehensive` is the test that covers the
  output/checked relationship. No code change.

#### P21 — `eimp3_output_drift_comprehensive` hardcodes a checked passphrase in the test body

- **Location:** `zweimomo/tests/suites.rs:194-196`:
  ```rust
  std::fs::write(scratch.join("einmo.toml"), "[signing]\noutput = \"zweimomo second signer\"\nchecked = \"We unanimously, …\"\n").unwrap();
  ```
- **Finding:** The passphrase is a test fixture string, not a real secret,
  but the "passphrase in test source" pattern is worth a note — there's no
  `#[cfg(test)]`-gated secret management, and the string is checked into
  the repo. Not a vulnerability (it's a demo passphrase for a `publish =
  false` crate), but the data flow should be explicit.
- **Severity:** Low.
- **Recommended remediation:** Add a comment noting the passphrase is a
  fixture string, not a real secret, and that `zweimomo`'s `day.1/einmo.toml`
  (the real fixture config) is the source of truth for the production
  passphrase. No code change.

#### P22 — `copy_dir_recursive` uses `.unwrap()` on every fs op

- **Location:** `zweimomo/tests/suites.rs:145-157`.
- **Finding:** `rust_instructions` §"Don't" prohibits `.unwrap()` in
  "library, protocol, parser, interpreter, FFI, or production paths" —
  tests are exempt. This is test-only, acceptable.
- **Severity:** Low (no change needed).
- **Recommended remediation:** None — record as "reviewed, acceptable for
  test code."

#### P23 — `BoaEvaluator`'s `to_std_string_escaped()` quotes string OUTPUT

- **Location:** `zweimomo/src/evaluators.rs:23-26`.
- **Finding:** `to_std_string_escaped()` renders a JS string value `"hello"`
  as `"hello"` (with quotes) in OUTPUT, not `hello`. This is a deliberate
  serialization choice (documented: "the idiomatic JS rendering of a value
  is `String(value)`"), but it means string-producing inputs have
  quote-wrapped OUTPUT bodies. Worth confirming this is the intended
  fixture convention across `day.1` (and that a reviewer editing a fixture
  knows to expect the quotes).
- **Severity:** Low.
- **Recommended remediation:** Confirm `day.1`'s string-producing inputs
  (if any) have quote-wrapped OUTPUT in their `checked/` baselines; add a
  test case to `evaluators.rs`'s unit tests asserting `evaluate("'hi'")`
  yields `"'hi'"` (or whatever the actual Boa rendering is) so the
  serialization choice is pinned. No production change.

#### P24 — `zweimomo/Cargo.toml`'s `boa_engine = "=0.21.1"` exact pin is correct but unverified-by-test

- **Location:** `zweimomo/Cargo.toml:19`.
- **Finding:** The exact pin is correct (interpreter output text lands in
  signed baselines; a version bump implies a corpus re-review, per the
  comment). But there's no test that *asserts* the pin — a `cargo update
  -p boa_engine` (which the pin should refuse) would silently succeed if
  someone weakened the pin. The comment is load-bearing but not
  machine-checked.
- **Severity:** Low.
- **Recommended remediation:** Either (a) accept the comment as the gate
  (the pin is the gate; the comment explains why), or (b) add a
  `[dependencies] boa_engine = { version = "=0.21.1", ... }` test that
  fails `cargo build` if the version doesn't match a recorded constant.
  (a) is the lighter touch and matches the repo's existing pinning
  discipline.

#### P25 — `zweimomo/src/lib.rs` `pub mod evaluators;` violates `rust_instructions` §"Don't `pub mod` internal modules"

- **Location:** `zweimomo/src/lib.rs:11`.
  ```rust
  pub mod evaluators;
  pub use evaluators::BoaEvaluator;
  ```
- **Finding:** `rust_instructions` §"Don't" says "Don't `pub mod` internal
  modules from `lib.rs`. → private `mod` + curated `pub use` re-exports."
  `evaluators` is an internal module (only `BoaEvaluator` is the public
  surface), but it's `pub mod`, exposing the module's entire contents
  (including future internal helpers) as the public API.
- **Severity:** Low.
- **Recommended remediation:** Change to:
  ```rust
  mod evaluators;
  pub use evaluators::BoaEvaluator;
  ```
  Verify `cargo test` still passes (the `evaluators.rs` tests are under
  `#[cfg(test)] mod tests` inside the module, so `mod evaluators;` still
  compiles them). This is the smallest real fix in the whole EIMP.

## Test Plan

Each P-item above carries its own verification step ("Add a test that …").
The EIMP-wide verification is:

- `cargo fmt --check` clean.
- `cargo clippy --all-targets -- -D warnings` clean (this is P0's own gate;
  fixing P0 unblocks the rest).
- `cargo test --workspace` green, with the new per-finding tests added
  alongside the fixes.
- The `comprehensive_multi_reviewer_end_to_end` test
  (`src/review.rs:2906`) and `eimp3_output_drift_comprehensive`
  (`zweimomo/tests/suites.rs:167`) still pass unchanged (they are the
  regression sentinels for the library and zweimomo respectively).
- A new EIMP-8 comprehensive test (see plan) that, in one pass, exercises:
  - a `decide` whose basis transiently fails to stat (P2) → error
    propagates, not silently `None`;
  - a `decide` → `plan()` → `decide`-different-kind → `execute` (P3) →
    the id is `skipped`, not executed with the stale kind;
  - a poisoned journal mutex survived by `Drop` (P1);
  - `POST … /execute` serialized under `spawn_blocking` (P7);
  - a default-mode socket bound in a temp dir whose parent is 0700 (P8).

## Rejected Alternatives

### A. Do nothing — the surfaces are `complete` and tested

The existing test suites (`cargo test --workspace`: 356 passing) are
green, and all three surfaces carry `complete` EIMPs (`EIMP-1`, `EIMP-2`,
`EIMP-2 §8`). But "tested" is not "reviewed": `EIMP-1`'s own
maintainer-review pass found P0–P12 *despite* `EIMP-2` being green, and
`EIMP-7` was spun out of that finding. The review discipline this repo
has adopted is that a thorough read of a `complete` surface produces a
findings list, not a silent patch. P0 alone (which breaks the clippy gate
the repo mandates) makes "do nothing" unacceptable; P1–P3 are real
correctness/soundness defects that the existing tests do not cover because
the tests document the *happy* path. Doing nothing is worse than fixing.

### B. Spin each finding into its own EIMP (one EIMP per P-item)

`EIMP-7` was spun out of `EIMP-1`'s P1 because its blast radius spanned
six modules. None of P1–P25 here has a blast radius beyond one or two
modules; most are one-line to one-function fixes. Twenty-five EIMPs for
twenty-five localized fixes would drown the index and the maintainer's
triage queue. One EIMP cataloguing all twenty-five, with the plan file
prescribing the order, is the right granularity — the implementing agent
works top-to-bottom and the maintainer triages accept/reject/defer per
P-item in one pass.

### C. Implement every finding before triage

This EIMP is `Draft`, not `Implementing`. Some findings may be rejected
after maintainer review (e.g. P10's "opaque" doc fix could be deemed
fine-as-is; P19's stack-overflow test could be deemed acceptable). The
plan file's checkboxes are a *suggested* order, not a mandate; the
maintainer should promote this EIMP to `Implementing` only after deciding
which P-items to accept, and the implementing agent should skip rejected
ones (marking them `[x] rejected — see <reason>` in the plan rather than
silently dropping).

## Open Questions

- **P0's intent:** The `--flag-is-not-failure` override at `verify.rs:451`
  is dead code, but is the *feature* (`--flag-is-not-failure`) implemented
  elsewhere (a config path `TestConfig` exposes), or is the feature itself
  unimplemented and the test aspirational? The fix depends on the answer:
  if the feature exists, wire the test to it honestly; if not, delete the
  dead assertion and leave a TODO. The implementing agent should grep for
  `flag_is_not_failure` / `flag-is-not-failure` to decide before touching
  the test.
- **P5's cache shape:** Should `agreement` share `VerifiedCache` (one
  cache, two consumers) or have its own sibling cache (separate invalidation
  concerns)? The implementing agent should sketch both and pick the one
  that doesn't require `VerifiedCache` to know about `StagePairAgreement`.
- **P8's hardening scope:** Should `serve_uds` *always* harden the
  socket's parent directory (even for `--private`, which already hardens),
  or only in the default `--socket` path? Hardening unconditionally is
  safer but changes the default-mode directory's perms in a way a caller
  might not expect if they passed a shared dir.
- **P10's direction:** Doc fix (a) vs. `OsRng` mint (b) — which does the
  maintainer want? (a) is cheaper and matches the trust model; (b) is
  defense-in-depth.
- **P13's event taxonomy:** Is a `DecisionCleared` variant worth a
  separate SSE event, or is "re-fetch on any `decision-made`" the
  DHTML's permanent contract?

## References

- Prior EIMPs:
  - `EIMP-1` (`EinmoReview`, `Implementing`) — the session object under
    review here; its own "maintainer-found defects" P0–P12 record is the
    convention this EIMP follows.
  - `EIMP-2` (review server, `complete`) — the HTTP surface under review
    here; §8 ports `zweimomo`.
  - `EIMP-7` (`EinmoCase`/`EinmoSuite`/`EinmoDirectory`, `complete`) —
    spun out of `EIMP-1`'s P1; the precedent for "spin a finding into its
    own EIMP when the blast radius warrants." This EIMP does *not* spin
    findings out (Rejected Alternative B).
- External docs:
  - `rust_instructions.md` §1a (optimization order), §"Don't" (`pub mod`,
    `.unwrap()`), §Concurrency (locks across `.await`), §HTTP services
    (typed extractors), §2f (crate hygiene, `[lints.rust] unsafe_code`).
  - `AGENTS.md` "Development Rules" (no substantive work while tests are
    broken — P0 blocks).
- Code locations:
  - `src/review.rs` (P1–P6)
  - `src/verify.rs:451` (P0)
  - `src/review_server.rs` (P7, P9–P16)
  - `src/bin/einmo_review_server.rs` (P7's binary side, P8)
  - `zweimomo/src/lib.rs`, `zweimomo/src/evaluators.rs`,
    `zweimomo/tests/suites.rs`, `zweimomo/Cargo.toml` (P17–P25)

## Last Updated

**Date**: 2026-07-31
**Updated By**: opencode (z-ai/glm-5.2)
**Changes**: Created EIMP-8 — a read-only code review of the einmo library
(`src/review.rs`), the review server (`src/review_server.rs` + binary),
and `zweimomo`, cataloguing twenty-five findings (P0–P25) with locations,
severities, and recommended remediations. `status: Draft`, `begun: [ ]` —
awaiting maintainer triage before any P-item is implemented.
