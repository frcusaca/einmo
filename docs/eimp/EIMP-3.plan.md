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

## Phase A — `Status::Drifted` + the content/key decision table (EIMP-3.md §Specification)

- [ ] Write the unit tests FIRST (`EIMP-3.md` §Test Plan "Unit — write_output
      drift-vs-append-vs-noop-vs-fresh") against the intended behavior:
      absent→fresh, same-signer-match→untouched, different-signer-match→append,
      content-differs→`Drifted`+untouched, corrupt-existing→treated-as-absent
- [ ] Add `Status::Drifted` (wherever `Status` is defined today — locate it
      first; `einmo_suite.rs` and `format.rs`/`metadata` are the likely
      homes) and thread it through existing `Status` match arms (serialization,
      `einmo list`/`einmo body` status rendering, suite-run pass/fail
      aggregation) so it is treated as a failing status everywhere
      `OutputError`/`InputError` already are
- [ ] Add an exact-pubkey lookup for `stage:<name>` stamps (new method on
      `Stamps`, e.g. `stamped_by_exact(&self, pubkey_hex: &str) -> bool`,
      alongside the existing prefix-based `stamped_by`) in `src/signature.rs`
- [ ] Rewrite `write_output`'s comparison block (`einmo_suite.rs:1219-1261`
      today) to implement the decision table: replace the fixed 3-entry
      `expected_keys == existing_keys` check with "existing has content
      match AND (my key already present → no-op) OR (my key absent → append
      my `stage:output` stamp to the existing file, preserving all other
      stamps) OR (content differs → `Status::Drifted`, leave `output/`
      untouched, do not write anything)"
- [ ] New `Stamps` append-in-place support if it doesn't already exist:
      given an already-serialized existing file's stamp chain plus one new
      `stage:output` stamp computed over the SAME prior-bytes prefix the
      existing file's own last stamp signed, append and re-serialize without
      touching earlier stamps or content sections
- [ ] Phase A tests green; `cargo fmt` / `cargo clippy -D warnings` clean

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
