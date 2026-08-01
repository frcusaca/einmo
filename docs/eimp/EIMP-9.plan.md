# EIMP-9.plan — the test-tooling contract

Read `docs/eimp/EIMP-9.md` before executing this plan. Each task below names
the finding (T1–T12) it closes; the evidence for every finding is in §S.1.

Order matters: the documentation corrections (Phase A) are the ones that
actively mislead a reader today and are safe to land immediately. The
tooling repairs (Phase B) change what the gates do. The code hygiene
(Phase C) touches two test-only call sites. Phase D re-verifies everything
through the repaired commands.

---

- [x] Begin work: commit `EIMP-9.md` and `EIMP-9.plan.md`, check `begun: [x]`
      in the frontmatter, add the row to `docs/eimp/INDEX.md`
      (2026-08-01 06:05)

## Phase A — stop the document from misleading (T6, T7, T8, T9, T10)

- [x] (read §S.1 T6–T10 and §S.3 of `EIMP-9.md`)
      (2026-08-01 06:05)
- [x] `rust_instructions.md`: delete the "`just` accepts unambiguous
      prefixes" paragraph (T7 — `just --dry-run cov` errors on `just 1.57.0`)
      (2026-08-01 06:05)
- [x] `rust_instructions.md`: correct the toolchain claim (T6). Until
      `rust-toolchain.toml` exists (Phase B), "Don't change the toolchain"
      must not assert that a pin file does the pinning — say that
      `[workspace.lints.clippy]` in `Cargo.toml` is what survives an upstream
      lint regrouping, and that the toolchain itself is currently unpinned
      (2026-08-01 06:05)
- [x] `rust_instructions.md`: give "Use `just test`, not `cargo test`" its
      reason (T9) — process-per-test isolation is why a single panic cannot
      poison `JOURNAL_ENV_LOCK` and cascade; cite the 96-of-158 measurement
      (2026-08-01 06:05)
- [x] `rust_instructions.md`: add the "When results look impossible" rule
      (T8), verbatim from §S.3 item 4 — shared `CARGO_TARGET_DIR`, never two
      concurrent `cargo` invocations, `cargo clean -p einmo` before believing
      a surprising failure
      (2026-08-01 06:05)
- [x] `rust_instructions.md`: extend "Machine-readable output" (T10, T4) —
      `<failure>` has no message, the panic is in `<system-err>`, check the
      test count against the workspace total first; include the Python
      extractor from §S.3 item 5
      (2026-08-01 06:05)
- [x] `rust_instructions.md`: add the two-tier table (§S.2) and state that
      `just test <filter>` is the inner development loop
      (2026-08-01 06:05)
- [x] `rust_instructions.md`: state the suite's real cost — 394 tests,
      ~345s, 18 SLOW, dominated by Argon2id — so a six-minute run does not
      read as a hang
      (2026-08-01 06:05)
- [ ] Commit Phase A

## Phase B — make the gates runnable (T1, T2, T3, T4, T5, T6, T11, T12)

- [ ] (read §S.1 T1–T5 and T11–T12 of `EIMP-9.md`)
- [ ] `justfile`: fix `pr` (T1) — replace `git diff main...HEAD` with
      `git diff $(git merge-base jia HEAD)..HEAD`, and add `--no-fail-fast`
      to its nextest line so the gate reports every failure, not the first
- [ ] `justfile`: add `mutants-wip` for the uncommitted working tree (T2),
      and comment `mutants` to say that an unscoped run is never useful
- [ ] `justfile`: add `--workspace` to `ci-test` (T3)
- [ ] `.config/nextest.toml`: `[profile.ci]` sets `fail-fast = false` (T4) —
      a consumed artifact must describe the whole suite
- [ ] `.config/nextest.toml`: raise `[profile.default] slow-timeout`
      `terminate-after` from 4 to 8 (T5). Re-measure
      `suite::tests::update_corpus_signature_re_signs_when_stale` and record
      the new figure here — the baseline was **116.053s against a 120s
      kill**
- [ ] `rust_instructions.md`: correct "`just pr` already scopes to your
      diff" (T1/T2) — on `jia` the diff is empty by construction, so
      mutation testing is scheduled per-EIMP, not incidental to `just pr`
- [x] Declare the MSRV floor (T6, first half). `[workspace.package]
      rust-version = "1.88"` in `Cargo.toml`, inherited by both members via
      `rust-version.workspace = true` *inside* `[package]` (the opposite form
      from `lints`). 1.88 is the max declared `rust-version` in the resolved
      graph — `time 0.3.54` and the `boa_*` crates — not the edition floor of
      1.85; the recompute one-liner is commented in `Cargo.toml`. Verified by
      temporarily setting 1.99: `error: rustc 1.97.1 is not supported by the
      following packages`. `cargo check --workspace` clean, no
      `unused manifest key`. Documented in `rust_instructions.md`
      (floor/lints/pin table + troubleshooting row)
      (2026-08-01 07:20)
- [ ] Decide whether `rust-toolchain.toml` is wanted *on top of* the floor
      (T6, second half — see the first Open Question, now narrowed), then add
      it and restore the guide's original wording
  - [ ] sub-agent: consult the human on exact-pin (`1.97.1`) vs floating
        `stable` + a `rust-version` floor. Remind them: "Above message comes
        from EIMP-9 working to repair the test tooling contract; changes are
        on `jia`. PTAL"
  - [ ] Add `rust-toolchain.toml` per that decision
  - [ ] Verify: `rustup show active-toolchain` names it; `cargo clippy
        --version` is stable across a `RUSTUP_TOOLCHAIN`-cleared shell
- [ ] `install-dev-tools.sh` becomes the single bootstrap (T11) — it
      installs `just`, then delegates to `just setup` for the pinned three;
      no second version list
- [ ] Untrack the mutation outputs (T12): `git rm -r --cached mutants.out
      mutants.out.old`, and add both `mutants.out` and `mutants.out.*` to
      `.gitignore`
- [ ] Commit Phase B

## Phase C — make `cargo test` output honest (T9)

- [ ] `src/review.rs:1244` and `src/review_server.rs:1142`: replace
      `JOURNAL_ENV_LOCK.lock().unwrap()` with
      `.unwrap_or_else(std::sync::PoisonError::into_inner)`. The guarded
      value is `()`; there is no invariant a poisoning panic could break
- [ ] Check the other `ENV_LOCK` sites for the same pattern —
      `src/einmo_suite.rs:2767`, `src/journal.rs:320`,
      `src/suite_lock.rs:126`, `src/review_server.rs:2273`,
      `src/review.rs:1216` — and fix each that unwraps a poisoned guard
- [ ] Verify the fix: temporarily `panic!()` inside one `test_context()`
      consumer, run `cargo test --lib`, confirm **one** failure rather than a
      cascade, then revert the injection
- [ ] Commit Phase C

## Phase D — the EIMP-9 comprehensive verification

The subject under test is the tooling, so the comprehensive test is an
execution of every repaired command, checked mechanically rather than by eye.

- [ ] Write and verify the EIMP-9 comprehensive check:
  - [ ] `just test crash_crumb_survives_stack_overflow` — the inner loop
        still returns in seconds
  - [ ] `just test --no-fail-fast` — 394 passed, 0 failed, and no SLOW line
        within 20% of the new kill threshold
  - [ ] `just ci-test` — then assert against `target/nextest/ci/junit.xml`
        with the §S.3 extractor: `tests="394"`, `failures="0"`, and a
        `<testsuite>` for `zweimomo` as well as `einmo`
  - [ ] `just lint` and `just fmt --check` clean
  - [ ] `just pr` — runs to completion; record its wall-clock time here
  - [ ] `just mutants --file src/verify.rs` — completes and writes
        `mutants.out/outcomes.json`; record survivors
  - [ ] `just coverage` — completes and writes an HTML report
  - [ ] `grep -n "unambiguous prefixes" rust_instructions.md` is empty (T7)
- [ ] Update the plan skeleton so future EIMPs schedule the expensive gates:
      add the four gate checkboxes from §S.2 to `eimp.md` ("Comprehensive
      EIMP Tests") and to
      `.claude/skills/eimp-write-plan/SKILL.md` ("Minimal Plan Skeleton"),
      replacing the line that names `cargo test`
- [ ] Update `docs/eimp/INDEX.md` — status, and a "Last Updated" entry
      recording the corrected baseline
- [ ] Update `EIMP-9.md` frontmatter `status: complete`
- [ ] Commit Phase D

---

## Notes for the executing agent

- **The tree is green.** 394/394 passed on 2026-08-01 at `ac873c3` via
  `just test --no-fail-fast` in 345s. If a run disagrees, suspect the build
  before the code — §S.1 T8 documents a 158-failure run on this same green
  source.
- **Never run two `cargo` commands at once.** `CARGO_TARGET_DIR` is shared
  (`/yolo/target` in the reference environment) and `cargo install` honours
  it. This is what produced T8.
- **Phase B changes what the gates mean.** After it lands, a plan checkbox
  reading "tests pass" is a claim about 394 tests across two packages, not
  55 across one. Re-run anything ticked earlier in the session.
