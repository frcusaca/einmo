# EIMP-8.plan — Code-review findings (einmo library, review server, zweimomo)

The plan is derived from `docs/eimp/EIMP-8.md` (the specification). Read
both files before executing any checkbox. The plan is **suggested order**,
not a mandate — the maintainer should triage each P-item
(accept/reject/defer) before `begun: [x]`, and the implementing agent marks
rejected items `[x] rejected — see <reason>` rather than silently skipping.

**Status as of creation:** `Draft`, `begun: [ ]`. No checkbox below is
checked. The first checkbox flips `begun` to `[x]` only after the
maintainer promotes the EIMP to `Implementing` and accepts at least the
first phase.

## Phase 0 — Unblock the toolchain gate (do this first, alone)

`AGENTS.md`: "NEVER start substantive work while any test is broken."
P0 breaks `cargo clippy --all-targets -- -D warnings` for the whole
crate. Fix it before touching any other P-item, so every later fix's
verification can run a clean clippy.

- [ ] (read §S.1 / P0 of EIMP-8.md) — understand the `verify.rs:451` dead
      code and the Open Question about whether `--flag-is-not-failure` is
      implemented elsewhere
- [ ] grep `flag_is_not_failure` / `flag-is-not-failure` across `src/` to
      answer P0's Open Question (feature exists vs. aspirational)
  - [ ] if the feature exists: wire `verify.rs:451`'s test to the real
        config path so `gate_fails_with_override` is computed honestly
  - [ ] if the feature is unimplemented: delete the dead assertion and
        leave a `// TODO(--flag-is-not-failure): …` note
- [ ] `cargo clippy --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] `cargo test --workspace` green (no regressions from the verify.rs
      edit)

## Phase 1 — Library correctness/soundness (high severity)

- [ ] (read §S.1 / P1 of EIMP-8.md) — `EinmoReview::Drop` panic-on-poison
  - [ ] Write the test first: a separate thread panics inside `log_at`
        (poisoning the journal mutex), then the main thread drops the
        `EinmoReview` and asserts the process does not abort
  - [ ] Fix `Journal::log_at` (and any sibling in `src/journal.rs`) to
        take the mutex with poison-recovery:
        `lock().unwrap_or_else(|e| e.into_inner())`
  - [ ] Test green; `cargo clippy` clean
- [ ] (read §S.1 / P2 of EIMP-8.md) — `decide` swallows basis stat errors
  - [ ] Write the test first: inject a stat failure (e.g. point the basis
        path at a parent dir with no read permission, or mock the path)
        and assert `decide` returns `Err` rather than silently recording
        a `None` basis
  - [ ] Fix `Fingerprint::of` to distinguish `NotFound` (→ legitimate
        `None`) from other I/O errors (→ propagate as `EinmoError::Io`);
        fix `decide` to propagate the error instead of `.ok()`-ing it
  - [ ] Test green
- [ ] (read §S.1 / P3 of EIMP-8.md) — `execute` applies stale action shape
  - [ ] Write the test first: `decide(Promote{Checked})`, `plan()`,
        `decide(Retract{Checked})` (same basis fingerprint), `execute` →
        assert the id is in `skipped`, not `executed` with the stale
        promote
  - [ ] Fix `execute`'s drift filter to re-check the *kind* (not just the
        basis) against the live `DecisionBook`; push to `skipped` on
        kind mismatch. Correct the comment at `review.rs:896-898` to
        match what's actually checked
  - [ ] Test green; the existing `execute_*` tests unchanged

## Phase 2 — Library concurrency/scalability (medium severity)

- [ ] (read §S.1 / P4 of EIMP-8.md) — `refresh()` holds the read lock
      across stat calls
  - [ ] Write the test first: hold `refresh` for a large pending set
        while a `decide` thread contends; assert the `decide` completes
        promptly
  - [ ] Fix `refresh` to snapshot `(id, basis)` pairs under the lock into
        a `Vec`, drop the guard, then stat outside the lock
  - [ ] Test green
- [ ] (read §S.1 / P5 of EIMP-8.md) — `items()` re-verifies every artifact
      per call, bypassing `VerifiedCache`
  - [ ] Benchmark `items()` on a 1000-case synthetic suite (before)
  - [ ] Resolve the Open Question (share `VerifiedCache` vs. sibling
        `agreement_cache`); sketch both, pick one
  - [ ] Route `case.agreement`'s verify-on-inspect through the chosen
        cache
  - [ ] Benchmark after; assert no regression on the small `day.1` suite
        and a measurable improvement on the 1000-case suite
  - [ ] `comprehensive_multi_reviewer_end_to_end` still passes unchanged
- [ ] (read §S.1 / P6 of EIMP-8.md) — `shuffle` modulo bias
  - [ ] Decide: accept-and-comment vs. uniform sampler
  - [ ] Either add the `// non-uniform; ordering only, not security`
        comment, or switch to a uniform sampler; no test needed (ordering
        is non-deterministic)

## Phase 3 — Review server concurrency/hardening (medium severity)

- [ ] (read §S.2 / P7 of EIMP-8.md) — sync `execute` on the tokio thread
  - [ ] Write the test first: fire two concurrent `POST … /execute` at the
        same session; assert they serialize (the second blocks until the
        first completes) and no state corrupts
  - [ ] Wrap `review.execute(&plan, &keys)` in
        `tokio::task::spawn_blocking`; `.await` the `JoinHandle`
  - [ ] Test green; the existing `execute_with_confirm_promotes_to_checked`
        HTTP test unchanged
- [ ] (read §S.2 / P8 of EIMP-8.md) — default socket not hardened
  - [ ] Resolve the Open Question (harden unconditionally vs. only default
        mode); pick one
  - [ ] Write the test first: bind a default-mode socket in a temp dir and
        assert the parent directory is 0700
  - [ ] Harden the socket's parent in `serve_uds` (reuse
        `journal::harden_dir`), OR document the umask caveat in the
        `--socket` help text
  - [ ] Test green
- [ ] (read §S.2 / P9 of EIMP-8.md) — `AppState::sessions` leaks
  - [ ] Add `pub fn close_session(&self, id: SessionId)`; add a test that
        creates and closes N sessions and asserts the map is empty
  - [ ] Document the lifecycle in `AppState`'s doc comment
- [ ] (read §S.2 / P10 of EIMP-8.md) — predictable `SessionId`, "opaque"
      doc
  - [ ] Resolve the Open Question (doc fix vs. `OsRng` mint); pick one
  - [ ] Either correct the doc to "sequential; not a secret" or mint from
        `OsRng` (128 bits, matching `random_session_id`)

## Phase 4 — Review server hygiene (low severity)

- [ ] (read §S.2 / P11 of EIMP-8.md) — duplicated DHTML handlers,
      ignored `{session}` segment
  - [ ] Resolve the Open Question (typed extractor vs. client-side
        reads-URL); pick one
  - [ ] Dedupe to one handler; if typed extractor chosen, add
        `Path<SessionId>` on `/review/{session}` that 404s on unknown
        sessions
- [ ] (read §S.2 / P12 of EIMP-8.md) — `case_detail` re-scans the suite
  - [ ] Add `review.case(&id)` (or single-id filter on `items()`); pairs
        with Phase 2 / P5's cache routing
- [ ] (read §S.2 / P13 of EIMP-8.md) — `delete_decision` event taxonomy
  - [ ] Resolve the Open Question (`DecisionCleared` variant vs. re-fetch
        contract); implement or document
- [ ] (read §S.2 / P14 of EIMP-8.md) — flag `reason` in the listing
  - [ ] Document the data flow in `decision_tag`'s doc comment
- [ ] (read §S.2 / P15 of EIMP-8.md) — non-constant-time bearer compare
  - [ ] Switch to `subtle::ConstantTimeEq` (or a 4-line manual compare);
        document as defense-in-depth
- [ ] (read §S.2 / P16 of EIMP-8.md) — unused `Deserialize` on response
      DTOs
  - [ ] Drop `Deserialize` from pure-response DTOs (`ExecuteResponse`,
        `PlanResponse`, etc.)

## Phase 5 — zweimomo (low severity)

- [ ] (read §S.3 / P17 of EIMP-8.md) — `BoaEvaluator` `Context`-per-call
      cost undocumented
  - [ ] Add a doc comment on `BoaEvaluator` explaining `Context` is
        `!Send` (required for thread-safety) and the per-call cost; sketch
        `thread_local!`-caching as a rejected alternative or implement if
        a profile shows it matters
- [ ] (read §S.3 / P18 of EIMP-8.md) — no `[lints.rust] unsafe_code`
  - [ ] Add `[lints.rust] unsafe_code = "deny"` to
        `zweimomo/Cargo.toml` (or inherit from workspace); verify
        `cargo clippy` unchanged
- [ ] (read §S.3 / P19 of EIMP-8.md) — stack-overflow test brittleness
  - [ ] Add a comment at `recurse(usize::MAX)` documenting the
        stack-guard dependency; no code change
- [ ] (read §S.3 / P20 of EIMP-8.md) — `run_tier` doesn't exercise the
      checked gate
  - [ ] Add a comment noting the checked-baseline gate is intentionally
        not exercised here; no code change
- [ ] (read §S.3 / P21 of EIMP-8.md) — hardcoded passphrase in test
  - [ ] Add a comment noting the passphrase is a fixture string; no code
        change
- [ ] (read §S.3 / P22 of EIMP-8.md) — `.unwrap()` in test code
  - [ ] Record as "reviewed, acceptable for test code"; no change
- [ ] (read §S.3 / P23 of EIMP-8.md) — `to_std_string_escaped` quotes
      string OUTPUT
  - [ ] Confirm `day.1`'s string-producing inputs have quote-wrapped
        OUTPUT in `checked/` baselines; add a unit test to
        `evaluators.rs` pinning the serialization choice
- [ ] (read §S.3 / P24 of EIMP-8.md) — `boa_engine` pin is comment-gated
  - [ ] Resolve: accept the comment as the gate (no change) or add a
        version-assertion test
- [ ] (read §S.3 / P25 of EIMP-8.md) — `pub mod evaluators;` violates the
      `pub mod` Don't
  - [ ] Change `zweimomo/src/lib.rs:11` from `pub mod evaluators;` to
        `mod evaluators;` (keep `pub use evaluators::BoaEvaluator;`)
  - [ ] `cargo test -p zweimomo` green (the module's `#[cfg(test)] mod
        tests` still compiles)

## Phase 6 — EIMP-8 comprehensive test

- [ ] Write the EIMP-8 comprehensive test (one pass, exercising the
      interactions of the fixes from Phases 1–3):
  - [ ] a `decide` whose basis transiently fails to stat (P2) → error
        propagates, not silently `None`
  - [ ] a `decide` → `plan()` → `decide`-different-kind → `execute` (P3) →
        the id is `skipped`
  - [ ] a poisoned journal mutex survived by `Drop` (P1)
  - [ ] `POST … /execute` serialized under `spawn_blocking` (P7)
  - [ ] a default-mode socket bound in a temp dir whose parent is 0700
        (P8)
- [ ] Test green; placed alongside the relevant module's existing tests

## Phase 7 — Final verification

- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` green
- [ ] `comprehensive_multi_reviewer_end_to_end` (`src/review.rs:2906`)
      passes unchanged
- [ ] `eimp3_output_drift_comprehensive`
      (`zweimomo/tests/suites.rs:167`) passes unchanged
- [ ] Update `EIMP-8.md` frontmatter `status: complete`
- [ ] Update `docs/eimp/INDEX.md` (EIMP-8 row: `Draft` → `complete`, plus
      a one-line "Last Updated" entry summarizing what was fixed)

## Notes for the implementing agent

- **Triage before implementation.** Read every P-item in `EIMP-8.md` before
  touching code. Mark rejected items in this plan as
  `[x] rejected — <reason>` so the maintainer's triage is auditable.
- **Tests first, per `rust_instructions` §2a/§Testing.** Every P-item with
  a behavioral change has a "Write the test first" sub-task above — do
  the test before the fix.
- **Commit regularly.** Phase boundaries are natural commit points; commit
  after each phase (or sub-task cluster) with a message referencing
  `EIMP-8 P<n>`.
- **No worktree/branch mechanics.** This EIMP executes directly on `jia`
  per `eimp.md`; do not add branch-creation checkboxes.
- **P0 is the gate.** Do not start Phase 1 until Phase 0 is green — every
  later phase's `cargo clippy` verification depends on P0 being fixed
  first.
