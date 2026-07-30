# EIMP-3.plan — output-stage drift fails, explicit regenerate, multi-signer output stamps

Read `docs/eimp/EIMP-3.md` before acting on any task below. Tasks run top to
bottom. Work happens directly on `main` (`EIMP-0` §8).

- [x] STOP — preconditions: `cargo test`, `cargo clippy --workspace --all-targets
      -- -D warnings`, `cargo fmt --check` all clean. Do not begin while any
      is broken.
      (2026-07-30 06:05) — confirmed clean (190 workspace tests, clippy/fmt
      clean) before drafting EIMP-3.
- [x] Begin work: check `begun: [x]` in `EIMP-3.md` frontmatter, commit
      `EIMP-3.md` + `EIMP-3.plan.md` stating that work has commenced
      (2026-07-30 06:05)

## Phase A — `FileResult.drifted` + the content/key decision table (EIMP-3.md §Specification)

- [x] Write the unit tests FIRST (`EIMP-3.md` §Test Plan "Unit — write_output
      drift-vs-append-vs-noop-vs-fresh") against the intended behavior:
      absent→fresh, same-signer-match→untouched, different-signer-match→append,
      content-differs→drifted+untouched, corrupt-existing→treated-as-absent
      (2026-07-30 06:20) — 4 tests in `einmo_suite.rs`'s test module:
      `write_output_unchanged_content_same_signer_is_byte_identical_noop`,
      `write_output_unchanged_content_different_signer_appends_stamp`,
      `write_output_differing_content_marks_drifted_and_leaves_existing_untouched`,
      `write_output_corrupt_existing_is_treated_as_absent_fresh_write`.
- [x] Add `FileResult.drifted: bool` — **not** a new `Status` variant; see
      `EIMP-3.md`'s "Corrected during implementation" note (`status` is the
      envelope's own recorded harness status and a drifted case never writes
      a new envelope; the existing `written_and_verified`/`ignored` fields
      already drive suite-run pass/fail, so `drifted` only needed to be a
      sibling boolean, not a new failure-status wired through parse/render
      paths)
      (2026-07-30 06:25)
- [x] Add an exact-match lookup for `stage:<name>` stamps: `Stamps::
      has_stage_stamp_from(stage_key, pubkey_hex) -> bool` in
      `src/signature.rs`, alongside the existing prefix-based `stamped_by`
      (2026-07-30 06:25)
- [x] Rewrite `write_output`'s comparison block to implement the decision
      table: content mismatch → `drifted: true`, `written_and_verified:
      false`, restore `existing`'s bytes (see crash-crumb note below); my
      key already present → true no-op restore; my key absent → append via
      `EinmoFile::append_stage_stamp_with` on a clone of `existing`; absent/
      corrupt/crash-crumb existing → fresh write (unchanged from before)
      (2026-07-30 06:30)
- [x] Append-in-place support: `EinmoFile::append_stage_stamp_with`
      (`format.rs`, already existed from before this EIMP — used by
      `transitions::promote`) works unmodified on an already-stamped file
      cloned from `existing`; no new primitive needed
      (2026-07-30 06:30)
- [x] Found and fixed during implementation: `write_crash_crumb` (called at
      the top of every `evaluate()`) unconditionally clobbers `out_path`
      with a transient placeholder *before* the real evaluator runs, so (a)
      the drift branch must actively restore `existing`'s bytes rather than
      "do nothing" (the crumb already overwrote the file this run), and (b)
      a *stale* crash crumb from a previous interrupted run must never be
      compared against as if it were a real baseline (its placeholder OUTPUT
      is always empty, which would misreport every case as drifted) — fixed
      by filtering `existing` to `None` whenever its own `status_detail`
      starts with `"TEST IN PROGRESS"`, before the decision table runs.
      Caught by `catastrophe_crumb_rerun_overwrites`, a pre-existing
      regression test.
      (2026-07-30 06:35) — see `EIMP-3.md`'s "Crash crumbs are not a
      baseline" note.
- [x] Phase A tests green; `cargo fmt` / `cargo clippy -D warnings` clean
      (2026-07-30 06:38) — 192 workspace tests (188 einmo + 4 zweimomo),
      clippy/fmt clean.

## Phase B — `einmo regenerate-output` verb (EIMP-3.md §Specification)

- [ ] Write endpoint/CLI tests FIRST: a suite with one drifted case; normal
      run reports `Drifted` and leaves `output/` untouched; `regenerate-output`
      replaces it, re-verifies, re-signs; a subsequent normal run then
      reports it clean
- [ ] Implement the `einmo regenerate-output <suite> [--filter <glob>]
      [--files <path>...]` CLI subcommand (`cli.rs`): re-evaluate matching
      inputs; for `Drifted` cases only, perform today's pre-EIMP-3
      unconditional-overwrite (fresh content + fresh stamp chain); all other
      cases behave exactly as a normal run (no-op / append-stamp / fresh-if-absent)
- [ ] Phase B tests green; `cargo fmt` / `cargo clippy -D warnings` clean

## Comprehensive test + completion

- [ ] Comprehensive test, per `EIMP-3.md` §Test Plan: one suite fixture run
      (reuse `zweimomo`'s ported suites) exercising in one pass: a no-op
      rerun, a second-signer co-sign (stamps accumulate, content untouched),
      a drifted case (fails, `output/` untouched), `regenerate-output` on
      the drifted case, then confirm the regenerated case is a normal
      `output/` candidate a subsequent run reports as clean
- [ ] All tests pass: `cargo test` (workspace), `cargo clippy --workspace
      --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] Update `EIMP-3.md` frontmatter to `status: complete`
- [ ] Update `docs/eimp/INDEX.md` to add EIMP-3 and reflect its completed
      status
