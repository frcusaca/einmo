# EIMP-1.plan — einmo-review-session

Read `docs/eimp/EIMP-1.md` before acting on any task below. Tasks run top to
bottom; each phase lands value on its own. This plan is adapted from the
original `FOOP-25.plan.md` (in `foolish-rust`), with worktree/branch
mechanics removed: einmo is a small, single-maintainer repository, so this
plan executes directly on `main` with regular commits (`EIMP-0` §8).

- [x] STOP — preconditions: all workspace tests pass (`cargo test`,
      `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`). Do
      not begin while any test is broken.
      (2026-07-30 06:05) — 190 workspace tests, clippy/fmt clean.
- [x] Sanity check: consult human to resolve `EIMP-1.md` §Open Questions
      (HTTP stack, journal location, differing default) enough to start
      Phase A. Remind them: "Above message comes from EIMP-1 working to
      build the EinmoReview session object; changes are on `main`. PTAL"
      (2026-07-30 06:05) — all six Open Questions resolved via direct
      conversation with the human (not just the three named above): HTTP
      stack (keep axum), journal location (scratch/state dir), claim TTL
      (5 min, auto-reclaim, shown in `plan()`), quorum (out of scope
      entirely, not just deferred), `ReviewOpts` mode (runtime-selectable
      `Full`/`Random`/`NewOrBroken`, not a boolean default), and Phase A2's
      parallel-read worker pool (`tokio`, not `rayon`/hand-rolled).
      Additionally resolved and written into `EIMP-1.md` §S.4a: the
      multi-signer content-then-key decision table for `checked`/`verified`
      promotion (paired with the new `EIMP-3` for `output`'s analogue).
- [x] Begin work: check `begun: [x]` in `EIMP-1.md` frontmatter, commit
      `EIMP-1.md` stating that work has commenced
      (2026-07-30 06:05)

## Phase 0 — drift re-survey (EIMP-1.md §S.10)

- [ ] Read §S.10 of `EIMP-1.md`, then re-survey `src/einmo_suite.rs`,
      `transitions.rs`, `signature.rs`, `verify.rs`, `format.rs`,
      `compare.rs` for API drift since 2026-07-19 (the date this design was
      originally written, as `FOOP-25`)
- [ ] Touch up `EIMP-1.md` §S.2–§S.7 sketches to match current einmo shapes;
      record notable drift in this plan as sub-tasks

## Phase A — the session library (EIMP-1.md §S.2–§S.6)

- [ ] Write the unit tests FIRST (`EIMP-1.md` §Test Plan: decisions, cache,
      signer, execute, journal) as failing tests against the intended
      `einmo::review` surface
- [ ] Implement `review::Decision` + `DecisionBook` (per-item, per-reviewer,
      versioned; replace-not-stack)
- [ ] Implement `review::VerifiedCache` (fingerprint → `Arc<OnceLock<VerifiedBody>>`,
      single-flight; verify-count test hook)
- [ ] Implement `review::Signer` / `SignerSet` (Argon2id→Ed25519
      derive-once, zeroize on drop, computer key constructor) — §S.4 is the
      authority for what does NOT go in `EinmoReview`
- [ ] Implement `EinmoReview` (open/items/body/diff/decide/undecide/decision/refresh)
      over the above
- [ ] Implement `ExecutionPlan` + `execute`/`execute_one` (exec mutex,
      fingerprint re-check, skip-and-report drift, retract cascade, confirm
      token plumbed but enforced by frontends)
- [ ] Flag = plaintext, concatenating (§S.3): `flagged/` is
      PLAINTEXT/unsigned/transient; execute writes the annotated note as
      plaintext and CONCATENATES a dated block on top when re-flagging;
      concurrent flags serialize under the exec mutex; `flagged/` stays
      exempt from verification; journal records each.
- [ ] New signed `notes/` stage (§S.3): a durable, attributed sibling to
      `flagged/`; a note is a valid signed `.einmo` (stamped,
      verify-on-inspect, participates in signature checks); support
      promoting a flag's concatenated content into `notes/` as a signed
      note body.
- [ ] Flags break tests by default (§S.3): a flagged artifact fails the run
      (non-zero / red gate); `--flag-is-not-failure` downgrades to
      non-fatal but stderr STILL announces the flag count (no silent
      config); wire into the goal-state check (green = zero flags + signed
      + matching + valid signatures). Tests per §Test Plan "flag breaks
      tests".
- [ ] Implement `Journal` (append-only JSONL, replay, truncated-tail
      tolerance)
- [ ] All Phase A tests green; `cargo fmt` and `cargo clippy -D warnings`
      clean

## Phase A2 — `CorpusSigner` (section PQ attestation), CRYPTO CORE ONLY (EIMP-1.md §S.11)

Self-contained `CorpusSigner` object — NOT mixed into `EinmoReview` (§S.11).
NO real-corpus writes and NOT wired into the live promotion flow in this
EIMP (that integration is a later step). Prove the object in isolation;
`EinmoReview` will merely hold and call it later.

- [ ] Read §S.11 of `EIMP-1.md`; add `fips205` dep (feature
      `slh_dsa_sha2_256s` — conservative set) to `Cargo.toml`
- [ ] Write tests FIRST (§Test Plan "CorpusSigner read strategies" +
      section attestation): deterministic manifest; digest changes on
      add/remove/alter/reorder; SLH-DSA sign→verify round-trip; tamper
      fails; same-passphrase dual-derivation determinism; empty-section
      manifest; the two read strategies agree bit-for-bit — all exercising
      `CorpusSigner` standalone (no `EinmoReview`)
- [ ] Implement `CorpusSigner` skeleton (`new`/`manifest`/`digest`/`sign`/`verify`)
      + the manifest builder (stage name + param-set id + sorted
      mirror-path list via the existing deterministic walk)
- [ ] Implement `ReadStrategy::ParallelBuffer` (DEFAULT): metadata→offsets→one
      allocation; parallel `read_exact` into disjoint slices via `tokio`
      `spawn_blocking` tasks bounded by `read_workers` (resolved: `tokio`,
      not `rayon`/hand-rolled — `EIMP-1.md` §S.11); short/long-read hard
      error; hand the whole buffer to the signer
- [ ] Implement `ReadStrategy::Stream` (alternative): sequential
      manifest-order read feeding the hasher incrementally, bounded memory;
      assert byte-identical digest to `ParallelBuffer`
- [ ] Extend `Signer` (§S.4) to derive BOTH the Ed25519 stamp key and the
      section SLH-DSA key from one passphrase (Argon2id output expanded to
      the SLH-DSA seed; deterministic keygen)
- [ ] Implement `sign`/`verify` over the digest; `.section.sig` file shape
      defined but written only to fixtures/tempdirs in tests, never the
      real corpus
- [ ] Phase A2 tests green; `cargo fmt` / `cargo clippy -D warnings` clean;
      `#![forbid(unsafe_code)]` still holds (fips205 is pure Rust)

## Phase B — CLI verbs

- [ ] `einmo review plan|list|decide|undecide|execute` one-shot subcommands
      (journal-backed session identity) with endpoint-equivalent semantics;
      unit tests
- [ ] Byte-for-byte equivalence test: `review execute` promotion == existing
      `einmo promote`

## Phase C — the server (EIMP-1.md §S.7)

- [ ] `einmo review serve <suite>`: UDS listener by default; TCP 127.0.0.1 +
      bearer token behind a flag; suite lockfile (second server refuses)
- [ ] JSON endpoints per §S.7 table incl. If-Match/409 and SSE events;
      endpoint tests against a tempdir suite; passphrase handled only
      inside POST execute (derive-use-drop)
- [ ] Concurrency tests: N verifiers, single-flight verify counts, no lost
      updates, claims expire

## Phase D — reduce `scripts/experimental_reviewer.sh` (EIMP-1.md §S.8)

- [ ] Add server discovery + `fetch_body`/decision/plan/execute thin-client
      paths; keep the direct `einmo` fallback
- [ ] Delete the superseded state machinery (decision arrays, undo/answer
      bookkeeping, results rendering, stats computation)
- [ ] Pty-driven end-to-end tests (stub-vim technique): promote,
      note→flag, u-revisit keeps answer, gate confirm/skip,
      fallback-without-server
- [ ] Measure and record here: line count and per-test spawn/verification
      counts, before vs after

## Phase E — dhtml frontend (EIMP-1.md §S.9)

- [ ] Single embedded page: 4-pane view, server diff hunks, verb buttons,
      notes→Flag, plan view with typed-PROMOTE gate, SSE refresh
- [ ] Browser-path integration test (HTTP+token mode) reusing Phase C
      fixtures

## Comprehensive test + completion

- [ ] Comprehensive test, per `EIMP-1.md` §Test Plan: scripted
      multi-verifier end-to-end session (two reviewers, mixed
      individual/batch signing, crash-resume, drift) over a fixture suite;
      stamp chains asserted with `einmo verify`.
- [ ] Verify all work is committed on `main` and all tests pass
      (`cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`)
- [ ] Update `EIMP-1.md` frontmatter `status: complete`
- [ ] Update `docs/eimp/INDEX.md` to reflect EIMP-1's completed status
