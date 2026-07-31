# EIMP-7.plan — EinmoCase / EinmoSuite / EinmoDirectory / EinmoStorage

Read `docs/eimp/EIMP-7.md` before acting on any task below. Tasks run top to
bottom. This plan executes directly on `jia` with regular commits
(`EIMP-0` §8) — einmo has no worktree stage.

**Standing rules for this plan** (they apply to every phase, restated once
here rather than repeated per checkbox):

- **Tests first.** Every phase writes its tests before its implementation.
- **Behavior preservation is the whole point.** This EIMP unifies
  implementations; except where the spec names an intentional change
  (§S.4's CLI promote co-signing, §S.6's two `differing` bug fixes), any
  observable change in output is a **defect in this refactor**, not an
  improvement. When in doubt, assert the old behavior.
- **Commit at every phase boundary**, and within a phase whenever a
  logical unit completes. Do not batch.
- **Never leave `cargo test` broken between commits.** The rename (Phase
  0) in particular must land as one complete, compiling commit.
- A full `cargo test --workspace` takes ~5.5 minutes (356 tests as of
  2026-07-31). Run it at phase boundaries; use `cargo test --lib
  <module>::` while iterating inside a phase.

---

## Phase 0 — preconditions, design freeze, and the rename

- [x] STOP — preconditions: `cargo clippy --all-targets -- -D warnings`
      and `cargo fmt --check` clean; `cargo test --workspace` confirmed
      clean immediately prior to this (356/356, the jia-merge
      verification commit `82d1231`). Do not begin while any test is
      broken (`EIMP-0` §8).
      (2026-07-31)
- [x] Read `docs/eimp/EIMP-7.md` in full — in particular §S.3
      (`EinmoCase`/`StageAgreement`) and §S.7 (the comparison
      unification), which carry the design decisions the rest of this
      plan assumes.
      (2026-07-31)
- [x] Resolve `EIMP-7.md`'s three original §Open Questions with the human;
      record each answer in the spec body.
      (2026-07-31) — all three resolved in conversation and written into
      the spec: **module layout** → three new files, one type each
      (§S.10); **`agreement`'s argument** → takes *stages*, never a
      *level*, with the `stage`/`level`/`section` vocabulary pinned in
      §S.9 (and the non-concept "slice" retired from the design
      vocabulary); **who receives an instruction** → the suite is told
      and the suite tells the cases; `transitions.rs`'s
      `promote`/`flag`/`retract` free functions are removed (§S.10).
- [x] Resolve the fourth Open Question: **where `flagged/` lives on
      disk**.
      (2026-07-31) — decided: one `flagged/` inside **each** of
      `output/`, `checked/`, `verified/`. `Stage` drops to three variants
      and "flagged" becomes a location modifier
      (`ArtifactLocation::Flagged(Stage)`). Rationale recorded in §S.2a:
      it is fine-grained state **the reviewer needs** — a flag against an
      output-stage artifact and one against a verified-stage artifact
      mean different things, and the transition table has always
      distinguished them while the filesystem discarded it. Lands inside
      this EIMP (Phase A2) because it shapes `ArtifactLocation`, which
      Phase A defines.
- [x] `EIMP-7.md` §Open Questions is now empty — the design is frozen.
      Reopen explicitly rather than deciding otherwise mid-implementation.
      (2026-07-31)
- [x] Set `EIMP-7.md` frontmatter `status: Implementing` and `begun: [x]`;
      commit both EIMP files stating that work has commenced.
      (2026-07-31)
- [x] **S.0 — rename `einmo_suite::EinmoSuite` → `EinmoTestRunner`.**
      Mechanical, no behavior change, must compile as one commit. 51
      occurrences across 6 files: `src/einmo_suite.rs` (31),
      `src/review.rs` (12), `src/cli.rs` (4), `src/review_server.rs` (2),
      `src/lib.rs` (1, the `pub use` at line 47), `src/bin/
      einmo_review_server.rs` (1).
      **Correction found while executing**: the plan's own survey missed
      a 7th file in a *different crate* — `zweimomo/tests/suites.rs` (6
      occurrences) — caught by the post-rename `cargo test --workspace`
      failing to compile with `E0432: unresolved import`, exactly the
      kind of miss a compile-and-test gate exists to catch. Fixed;
      recorded here so the blast-radius count above is now known
      incomplete (57 total, not 51) rather than silently left wrong.
      (2026-07-31)
  - [x] Rename the type, its `impl` blocks, and the `pub use` export.
        (2026-07-31)
  - [x] Update `einmo_suite.rs`'s module doc comment (line 1-2 names
        `EinmoSuite` as "the test runner" — that sentence becomes correct
        again only after the rename).
        (2026-07-31)
  - [x] `cargo test --workspace` green, `clippy`/`fmt` clean. This is a
        pure rename: the test count must be **unchanged at 356**, and no
        test body should need editing beyond the type name.
        (2026-07-31) — 356/356 (318 einmo lib + 31 einmo-review-server
        bin + 4 zweimomo lib + 3 zweimomo tests/suites.rs), identical
        composition to pre-rename. `cargo fmt --check` / `cargo clippy
        --all-targets -- -D warnings` both clean.
  - [x] Commit: "EIMP-7 S.0: rename EinmoSuite to EinmoTestRunner".
        (2026-07-31)

## Phase A — `EinmoStorage` + `EinmoDirectory` (§S.1, §S.2)

- [x] Read §S.1 and §S.2 of `EIMP-7.md`. Note especially §S.2's constraint:
      **the `input/`+per-stage directory split is deliberately
      preserved** — hand-authored suites are browsed and edited in
      place, so `EinmoDirectory` resolves every `(EinmoId,
      ArtifactLocation)` to exactly the path it resolves to today.
      **Sequencing note**: §S.1's `ArtifactLocation` is shown in its
      FINAL, post-§S.2a shape (`Input | Stage(Stage) | Flagged(Stage)`,
      3-variant `Stage`). This phase builds it in its PRE-migration
      shape instead — `Input | Stage(Stage)` only, 2 variants, matching
      `Stage` as it still stands here (4 variants, `Flagged` included;
      `ArtifactLocation::Stage(Stage::Flagged)` covers today's top-level
      `flagged/` exactly as it exists on disk right now). Phase A2 is
      what evolves both `Stage` and `ArtifactLocation` to their final
      shape — building the final shape here would leave a `Flagged(Stage)`
      variant with nothing constructing it for one whole phase, which is
      the kind of unreachable-code gap a compile-and-test gate should
      never have to paper over.
      (2026-07-31)
- [x] Write tests first: round-trip `read`/`write`/`remove`; `read` of an
      absent artifact is `Ok(None)`, never an error; `remove` of an absent
      artifact is a no-op, not an error; `list_ids` for `Input` and for
      each `Stage` (including `Stage::Flagged` — still a real stage here).
      (2026-07-31) — `src/storage.rs`'s `assert_round_trip` helper, run
      against both `EinmoDirectory` and `InMemoryStorage`; separate
      `list_ids` tests confirming per-location independence (Output vs
      Checked don't leak into each other) and empty-when-nothing-there.
- [x] Add `ArtifactLocation` (`Input` | `Stage(Stage)`, 2 variants — see
      sequencing note above), deriving
      `Debug, Clone, Copy, PartialEq, Eq, Hash`.
      (2026-07-31)
- [x] Define the `EinmoStorage` trait exactly as §S.1 specifies
      (`read`/`write`/`remove`/`list_ids`).
      (2026-07-31)
- [x] Implement `EinmoDirectory`. **Reuse, do not reimplement**, the
      existing path primitives: `EinmoId::to_stage_path`,
      `mirror_input_path`, `ensure_parent_dir` (`src/stage.rs`),
      `TestConfig::stage_dir`/`input_path`/`walk_depth_limit`
      (`src/config.rs`). `list_ids(Input)` wraps `walk_input_tree` +
      `EinmoId::from_input_rel`; `list_ids(Stage(s))` wraps
      `walk_input_tree(stage_dir(s))` + `EinmoId::from_stage_artifact_path`.
      No new path-construction logic is written in this phase.
      (2026-07-31) — confirmed no new path logic: `artifact_path`'s two
      match arms are direct calls to `config.input_path()`/
      `id.to_stage_path`, nothing else.
- [x] Add the in-memory `EinmoStorage` test fake (`HashMap<(EinmoId,
      ArtifactLocation), Vec<u8>>`), `#[cfg(test)]` but `pub(crate)` so
      Phases B and C can use it without a tempdir.
      (2026-07-31) — `InMemoryStorage`. Simpler than the plan's own
      sketch: `ArtifactLocation` already derives `Hash`/`Eq`, so it's the
      map key directly — no separate key-wrapper type needed. (Wrote one
      first, then removed it on review; recorded here as a note for
      later phases: check whether a "helper type for map-key purposes"
      is actually necessary before adding it.)
- [x] Parity test: a fixture suite on disk, read through `EinmoDirectory`,
      yields the same ids and bytes as the same fixture loaded into the
      in-memory fake.
      (2026-07-31) — `einmo_directory_and_in_memory_storage_agree_on_the_
      contract`: both backends driven through the identical `EinmoStorage`
      sequence (absent → write → read-back) and asserted equal at each
      step, via `&dyn EinmoStorage` so the same assertions run against
      both without duplicating them.
- [x] Phase A tests green; `clippy`/`fmt` clean; commit.
      (2026-07-31) — 7 new tests, all passing. Full
      `cargo test --workspace`: 363/363 (356 + 7), `cargo fmt --check` /
      `cargo clippy --all-targets -- -D warnings` both clean.

## Phase A2 — `flagged/` moves inside each stage (§S.2a)

Lands here, right after `ArtifactLocation` exists and before `EinmoCase`
builds on it, so nothing downstream is written against the old
four-variant `Stage`.

- [x] Read §S.2a and §S.10's blast-radius note in `EIMP-7.md`.
      (2026-07-31)
- [x] Write tests first: `ArtifactLocation::Flagged(stage)` resolves to
      `<stage>/flagged/<id>.einmo` for each of the three stages; flagging
      from the checked stage lands in `checked/flagged/` and **not** in
      `output/flagged/`; the origin stage is recoverable from the
      location alone (the property this change exists for).
      (2026-07-31) — `storage.rs`:
      `einmo_directory_flagged_resolves_under_its_own_origin_stage_only`
      (Checked's flag is absent from Output's and Verified's sinks, and
      confirmed on disk at `checked/flagged/`) and
      `einmo_directory_list_ids_stage_excludes_its_own_nested_flagged_sink`.
- [x] Remove `Stage::Flagged`. `Stage` becomes `Output | Checked |
      Verified`. Worked through the 27 originally-surveyed references
      across `transitions.rs` (15), `stage.rs` (4), `einmo_suite.rs` (3),
      `verify.rs` (2), `config.rs` (2), `bin/einmo_review_server.rs` (1).
      (2026-07-31)
  - [x] `Stage::ALL` (4 → 3) and `dir_name()`.
  - [x] Delete `stamp_key()`'s `Flagged` arm — dead by construction:
        flagging appends an unsigned advisory, never a stamp.
  - [x] `is_legal_transition`: drop the three `* → Flagged` rows.
        Flagging is no longer a stage transition at all; it is a move
        within a stage, so it leaves the transition table entirely.
  - [x] `promote`'s `to == Stage::Flagged` delegation to `flag` — removed;
        `promote` can no longer be asked to flag (the branch is now
        impossible to construct, not just untaken).
  - [x] `count_flagged` (`einmo_suite.rs:509`): walks all three
        `<stage>/flagged/` directories via a new `FlaggedArtifact { stage,
        rel_path }` type — **chose the per-stage breakdown**, per the
        plan's own preference. `cli.rs`'s `cmd_verify` gate message now
        names each artifact's stage (`"a.foo.einmo (checked stage)"`).
  - [x] The R2 orphan exemption doc language — re-expressed per-stage in
        `orphans_of`'s doc comment.
  - [x] `bin/einmo_review_server.rs`'s `parse_decidable_stage`: dropped
        the dead `Stage::Flagged` arm from its exhaustive match.
  - [x] The review server's flag/retract endpoints themselves needed no
        code change (they already called `EinmoReview::flag_now`/
        `retract_now` generically over `Stage`, never matching
        `Stage::Flagged` literally) — confirmed, not assumed, by their
        own passing test suite (below).
  - [x] **Correction found while executing, beyond the original 27-site
        survey**: removing `Stage::Flagged` alone is not sufficient —
        `EIMP-7` §S.2a nests the flagged sink INSIDE the stage directory
        being walked for that stage's own artifacts, and four call sites
        walk a stage directory directly (not via input-derived
        existence-checks, which are naturally immune): `einmo_suite.rs`'s
        `orphans_of` and `scan_tests`, and (retrofitted into Phase A's
        code) `storage.rs`'s `EinmoDirectory::list_ids(Stage(s))`. Without
        exclusion, every flagged artifact would double as a phantom
        ordinary one — a false orphan, a false `TestRow`, a false
        `EinmoCase` id. Added `stage::is_in_flagged_sink` (filters by
        first path component) and applied it at all three sites, plus
        `TestConfig::flagged_dir`/`flagged_dir_name` and
        `stage::DEFAULT_FLAGGED_DIR_NAME` (for `corpus_signer.rs`, which
        holds no `TestConfig` and already assumes default directory
        names) to support it. `verify()` (`verify.rs`) needed the inverse
        fix — it does NOT walk stage directories directly (input-derived,
        naturally immune) but therefore stopped visiting flagged content
        at all once `Stage::ALL` narrowed to 3; fixed by having it check
        both `stage_dir(s)` and `flagged_dir(s)` per stage. Each fix has
        its own regression test:
        `flagged_orphan_is_not_a_violation` (rewritten to prove the
        recursion is filtered, not merely that the concept is exempt),
        `einmo_directory_list_ids_stage_excludes_its_own_nested_flagged_sink`,
        and `verify.rs`'s `flagged_artifact_still_passes_verify` (with an
        added non-empty-report assertion, so the test cannot pass
        vacuously by verify silently skipping the file).
      - [x] **A second correction, found only by the FULL workspace test
            run**: `review.rs` and `review_server.rs` each had hardcoded
            `tmp.path().join("flagged/a.foo.einmo")` path literals in
            their own tests — invisible to the original grep (it searched
            for `Stage::Flagged`, not string literals) and invisible to
            per-module test runs of `transitions`/`einmo_suite`/`storage`/
            `verify` (all green) because the failure only surfaces when
            `review::`'s own tests run. First appeared as 72 cascading
            `PoisonError` failures (one real panic in
            `execute_flag_moves_and_writes_advisory_no_signing` poisoned a
            shared static mutex, then every other `review::` test sharing
            it panicked on lock too) — traced to the one non-poison root
            panic, not guessed from the cascade. Fixed 4 occurrences (3 in
            `review.rs`, 1 in `review_server.rs`) to
            `"output/flagged/a.foo.einmo"`. `review::` (62 tests) and
            `review_server::` (47 tests) both fully green after.
            (2026-07-31)
- [x] `StageDirs::flagged` stays a configurable *name*; only its parent
      changes (`StageDirs::flagged_name()`, `TestConfig::flagged_dir`).
      No public builder for a custom flagged name exists today (none did
      before this EIMP either — `StageDirs` has no `with_*` constructor
      reachable from `TestConfig`), so "configuring a custom name still
      works" has no live path to test; recorded here rather than silently
      dropped.
- [x] Migrate the live fixture `zweimomo/suites/javascript/day.1/flagged`
      → `output/flagged/`.
      (2026-07-31) — the directory existed but was **empty and untracked
      by git** (`git ls-files` confirmed nothing inside it); there was no
      content to migrate. Removed the stale empty directory rather than
      leaving a legacy top-level `flagged/` sitting next to the new
      per-stage layout. Confirmed no other suite anywhere in the repo has
      a `flagged/` directory.
- [x] Phase A2 tests green; `clippy`/`fmt` clean; commit (message must
      call out both the on-disk break and the fixture migration).
      (2026-07-31) — full `cargo test --workspace`: **366/366** (328
      `einmo` lib + 31 `einmo-review-server` bin + 4 `zweimomo` lib + 3
      `zweimomo` `tests/suites.rs`; up from 356 pre-Phase-A, +9 Phase A +
      1 net from Phase A2's new/changed tests). `cargo fmt --check` /
      `cargo clippy --all-targets -- -D warnings` both clean.

## Phase B — `EinmoCase` (§S.3)

- [x] Read §S.3 of `EIMP-7.md`, including the `StagePairAgreement` enum
      rationale and the **deliberate asymmetry** between `agreement`
      (policy-driven) and `promote`'s destination-match check
      (all-non-STAMPS-sections).
      (2026-07-31)
- [x] Write tests first for `agreement`, covering every
      `StagePairAgreement` variant: `Agree`; `Differ` (assert the
      **section names**, not just that it differs); `OneSided` (both
      directions, asserting which stage is `present`); `BothAbsent`;
      `Tampered` (assert a tampered artifact is `Tampered`, **never**
      folded into `Differ` — the distinction `scan_tests` loses today).
      (2026-07-31) — `src/case.rs`'s test module, all against
      `InMemoryStorage` (no tempdir needed).
  - [x] Included the **P1 repro** as a named test
        (`agreement_p1_repro_unpopulated_verified_does_not_affect_output_
        checked_agreement`): output and checked agree, verified never
        written → `Agree`.
  - [x] Included the **COMMENTS repro** as a named test
        (`agreement_comments_repro_policy_controls_whether_comments_only_
        divergence_counts`): the same two files read `Agree` under
        `InputOutput` and `Differ { sections: ["COMMENTS"] }` under
        `InputOutputComments` — one core, two consistent answers.
- [x] Implement `StagePairAgreement`, `StageAgreement` (with its recorded
      `policy` field and the `pair(left, right)` accessor).
      (2026-07-31)
- [x] Implement `EinmoCase` (`read`, `stages`, `agreement`).
      `agreement` reuses `compare.rs`'s existing required-section rules
      (`required_sections`/`is_required_section`/`compare_sections`) —
      made `compare_sections` `pub(crate)` rather than forking it; §S.7
      will fold `compare::compare` onto this same core.
      (2026-07-31)
- [x] Write tests first for `promote`: all three `PromoteOutcome`
      variants (`Promoted`, `CoSigned`, `AlreadySigned`), plus the
      `non_human` flag on verified-stage promotion with a computer key.
      (2026-07-31) — 9 tests in `case.rs`, including the deliberate
      asymmetry (`promote_deliberate_asymmetry_comments_only_difference_
      is_not_a_match`, which also asserts `agreement()` WOULD call the
      same two files `Agree` — proving the asymmetry directly, not just
      promote's own behavior in isolation).
- [x] Implement `EinmoCase::promote` + `PromoteOutcome` by **moving**
      `review.rs`'s `promote_one_accumulating` (`src/review.rs:1179`)
      here, converting its `(String, bool)` return into the enum. Keep
      its logic byte-identical; only the return shape changes.
      (2026-07-31) — moved AND rewired: `review.rs`'s `execute()` now
      constructs an `EinmoCase` over an `EinmoDirectory` and calls
      `case.promote(...)`, mapping `PromoteOutcome` back to the
      `(detail, non_human)` shape `Executed` needs.
      `promote_one_accumulating` is deleted from `review.rs`, not just
      superseded. **Found and fixed while porting**: the original tuple
      return included an accurately-computed `non_human` on the
      already-signed/no-op path too — my first cut of `PromoteOutcome::
      AlreadySigned` had no field and silently dropped it. Added
      `non_human: bool` to `AlreadySigned` to match exactly.
      `EinmoCase::promote` is `pub(crate)` (not `pub`): it takes
      `&StageKeypair`, itself `pub(crate)` — exposing it publicly would
      leak crypto internals across the crate's public boundary for no
      reason, since the public entry point is `EinmoSuite::promote`
      (Phase F), which takes `&KeySource` and derives once per batch.
  - [x] Byte-for-byte equivalence test: rather than a bespoke diff
        (nothing left to diff against — the old function is deleted),
        equivalence is proven the stronger way: `review.rs`'s own
        pre-existing test suite (62 tests, including
        `execute_promote_matches_cli_promote_byte_for_byte`,
        `execute_promote_appends_a_second_signers_stamp_when_content_
        matches`, `execute_promote_is_a_true_noop_when_dest_already_
        matches_and_is_mine`, `execute_promote_writes_a_fresh_baseline_
        when_content_genuinely_differs`,
        `execute_derives_stage_key_once_per_batch_not_per_case`) passes
        **completely unedited** with `EinmoCase::promote` underneath.
        (2026-07-31)
- [x] Implement `EinmoCase::flag` / `retract`.
      (2026-07-31) — **deviated from the plan's own sketch**: rather than
      delegating to `transitions::flag`/`transitions::retract` (which
      are `TestConfig`-shaped, not `EinmoStorage`-shaped — `EinmoCase`
      holds no `TestConfig`), ported their per-file logic directly onto
      `EinmoStorage` reads/writes/removes, matching `promote`'s own
      pattern. Necessary for `EinmoCase` to be genuinely storage-agnostic
      rather than secretly filesystem-only; `transitions.rs`'s free
      functions are unaffected here and still exist (Phase F removes
      them once `EinmoSuite` calls `EinmoCase` directly for the CLI path
      too). `retract` returns `Result<Vec<Stage>>` (which cascade targets
      were ACTUALLY removed), not `Result<()>` as first sketched — a
      suite-level caller building `RetractReport` needs to know this per
      id, and `EinmoStorage::remove` is silently a no-op on an absent
      target so the case has to check presence itself to report it.
- [x] Phase B tests green; `clippy`/`fmt` clean; commit.
      (2026-07-31) — `case::` module: 19/19. `review::` (the rewired
      consumer): 62/62, unedited. Full `cargo test --workspace`:
      **385/385** (347 `einmo` lib + 31 `einmo-review-server` bin + 4
      `zweimomo` lib + 3 `zweimomo` `tests/suites.rs`; up from 366 —
      +19 `case::` tests). `cargo fmt --check` / `cargo clippy
      --all-targets -- -D warnings` both clean (one `large_enum_variant`
      lint fixed along the way by boxing `StageRead::Present`'s
      `EinmoFile`).

## Phase C — `EinmoSuite` (§S.5)

- [x] Read §S.5 of `EIMP-7.md`.
      (2026-07-31)
- [x] Write tests first: `scan` against both the in-memory fake and a
      real `EinmoDirectory` fixture (parity); the id union covers cases
      present only in `input/`, only in a stage, and in both; `filter`
      behaves as `scan_tests`'s existing filter does; `cases()` ordering
      matches today's `rels.sort()`.
      (2026-07-31) — `src/suite.rs`'s test module, 7 tests.
      **Correction**: `scan_tests`'s filter matches against the
      **`.einmo`-suffixed mirror path** (`shown.contains(f)` where `shown`
      is the mirror-relative string); `EinmoSuite::scan`'s filter matches
      against the bare `EinmoId` (no `.einmo` suffix) instead — the same
      form `transitions.rs`'s own filter (`matching_mirror_paths`,
      matched against `input_rel`) already uses. A pattern that happened
      to include literal `.einmo` would behave differently between old
      and new. Documented in `scan`'s doc comment rather than silently
      matched; Phase E (the actual `cli.rs`/`einmo list` migration off
      `scan_tests`) must re-verify this against real CLI filter usage
      before treating it as behavior-preserving.
      Also added a test not originally listed: `scan_excludes_flagged_
      sinks` — confirms a stage's nested flagged sink never leaks into
      the ordinary suite listing (flagging is retirement, `EIMP-1` §S.3).
- [x] Implement `EinmoSuite::scan` / `cases` / `case`.
      (2026-07-31)
- [x] Write tests first for `directory_tree()` against a multi-level
      fixture using real-shaped ids (`foop/23/sub_feature/test1`): every
      case appears in exactly one node at the right depth; a case at the
      root (`test1.foo`, no directory components) is handled; no node
      exists for a component with neither cases nor children.
      (2026-07-31) — `directory_tree_groups_by_path_components_at_every_
      depth` (exact fixture from the plan) and
      `directory_tree_never_has_an_empty_node` (a recursive invariant
      check over an arbitrary fixture, not just eyeballing the one
      example).
- [x] Implement `directory_tree()` + `DirectoryNode`. Pure and on-demand
      — no persisted tree state (§Rejected Alternative D).
      (2026-07-31)
- [x] Phase C tests green; `clippy`/`fmt` clean; commit.
      (2026-07-31) — 7/7 new `suite::` tests. `cargo fmt --check` /
      `cargo clippy --all-targets -- -D warnings` both clean. Full
      `cargo test --workspace`: **392/392** (354 `einmo` lib + 31
      `einmo-review-server` bin + 4 `zweimomo` lib + 3 `zweimomo`
      `tests/suites.rs`; up from 385 — +7 `suite::` tests).

## Phase D — one pairwise-comparison implementation (§S.7)

- [x] Read §S.7 of `EIMP-7.md`.
      (2026-07-31)
- [x] Before changing anything: capture the **current** `ComparisonResult`
      for every existing `compare.rs` fixture as a baseline assertion, so
      the fold below is provably behavior-preserving rather than
      apparently so.
      (2026-07-31) — `compare.rs` already carried 9 tests covering
      exactly this (matching, only_in_a/only_in_b, differing single AND
      multi-section, tampered-vs-differing, STAMPS-ignored, files-filter
      single/absent-from-other-stage, `root_causes`). Read in full rather
      than duplicated: a second bespoke baseline harness would be
      redundant busywork when a comprehensive pre-existing suite already
      serves the exact purpose, and running it unedited post-refactor
      (below) is the same proof pattern Phase B used against `review.rs`.
- [x] Reimplement `compare::compare`'s body as a fold over
      `EinmoSuite::cases()` + `EinmoCase::agreement(&[a, b], policy)`,
      bucketing each `StagePairAgreement` into the existing
      `matching`/`differing`/`only_in_a`/`only_in_b`/`tampered` vectors.
      **Public shapes do not change**: `ComparisonResult`, `DiffEntry`,
      `compare`'s signature, and `root_causes` all stay as they are.
      (2026-07-31) — **note on candidate-set widening**: old `compare`
      derived its candidate rels from `input/` alone (when `files` is
      `None`), so a case present in a stage with NO matching input was
      never even considered. `EinmoSuite::cases()` unions input ∪ all
      three stages, per §S.7's own design — a case existing only in a
      stage (e.g. input deleted after promotion) is now compared where
      it silently wasn't before. This is a genuine, spec-intended
      widening of coverage (matching `einmo list`'s own broader union),
      not a preservation of the OLD narrower candidate set — verified
      that none of the 9 existing tests exercise that scenario, so it
      surfaces as new capability, not a regression.
  - [x] Preserve the `files: Option<&[PathBuf]>` argument's meaning — it
        becomes a filter over `cases()`, not a substitute id list.
        `cli.rs:546` (`einmo compare`) passes it; its behavior must not
        change.
        (2026-07-31) — confirmed via `compare_single_file` and
        `compare_files_only_in_one_stage`, both unedited and passing.
  - [x] Confirm the independent input-tree walk at `compare.rs:104` is
        now gone (the fourth such walk in the crate).
        (2026-07-31) — `walk_input_tree` no longer appears anywhere in
        `compare.rs` (grep-confirmed).
- [x] `einmo_suite.rs`'s two `compare::compare` call sites (line 640
      `stage_pair_problems`, line 987 `require_correspondence`) keep
      calling it unchanged — verify by running their existing tests
      untouched.
      (2026-07-31) — 43/43 `einmo_suite::` tests pass unedited.
- [x] Phase D tests green (`compare::tests` and `einmo_suite::tests` in
      particular); `clippy`/`fmt` clean; commit.
      (2026-07-31) — `compare::` 9/9 unedited, `einmo_suite::` 43/43
      unedited. `cargo fmt --check` / `cargo clippy --all-targets -- -D
      warnings` both clean, no new tests needed (existing suites ARE the
      proof). Full `cargo test --workspace`: **392/392** (354 `einmo`
      lib + 31 `einmo-review-server` bin + 4 `zweimomo` lib + 3
      `zweimomo` `tests/suites.rs`) — unchanged count from Phase C, as
      expected for a pure refactor with no new tests of its own.

## Phase E — the two consumers (§S.6)

- [ ] Read §S.6 of `EIMP-7.md`.
- [ ] `EinmoTestRunner::stage_pair_problems`: source its comparison from
      `EinmoCase::agreement` (mapping `Differ` → one
      `Problem::SectionDifference` per section, `OneSided` →
      `Right`/`LeftMissingEntirely`, `Tampered` → today's tampered
      handling). Behavior-preserving: every existing `Problem`-generation
      test must pass **unedited**.
- [ ] `EinmoReview::items` / `ReviewItem::differing`: recompute from
      `agreement(&[Stage::Output, Stage::Checked],
      config.match_sections())`, per §S.6.
  - [ ] Update `ReviewItem::differing`'s doc comment — it currently
        describes the old all-stages semantics.
  - [ ] Update `ReviewMode::NewOrBroken`'s doc comment and
        `scripts/einmo_review_client.sh`'s `-n` help text if either
        overstates or understates the now-correct behavior.
  - [ ] Assert the P1 bug is fixed end-to-end at the `EinmoReview` level
        (not just at `EinmoCase`): a fresh suite with empty `verified/`
        and agreeing output/checked yields **no** `NewOrBroken` items.
- [ ] `cli.rs:814` (`einmo list`): migrate off `scan_tests`/`TestRow` onto
      `EinmoSuite::cases()`. Its rendered output must be unchanged.
- [ ] Delete `scan_tests` and `TestRow` from `einmo_suite.rs` once no
      caller remains, and drop the `use crate::einmo_suite::{TestRow,
      body_sections, scan_tests}` import at `src/review.rs:15`.
- [ ] Phase E tests green; `clippy`/`fmt` clean; commit.

## Phase F — one promote implementation; the suite directs the cases (§S.4, §S.10)

- [ ] Read §S.4 and §S.10 of `EIMP-7.md`. **This phase contains the
      EIMP's one intentional behavior change** — treat it deliberately,
      not as cleanup.
- [ ] Write the test first: promote the same content to the same stage
      with two different keys → **both** stamps present afterward
      (co-signing). Today `transitions::promote` clobbers, so this test
      fails before the change and passes after; it is the change's
      definition.
- [ ] Implement `EinmoSuite::promote` / `flag` / `retract` (§S.10) —
      selection is the suite's job, per-case application is the case's.
      `PromotionReport` / `FlagReport` / `RetractReport` keep their
      public shapes so `cli.rs`'s formatting needs no change.
  - [ ] Preserve the "derive the `StageKeypair` ONCE per promotion, not
        per case" discipline (`transitions.rs:127-131`) — Argon2id is
        ~1.8s, and per-case derivation made a 161-case promotion take ~5
        minutes. The suite derives once and lends `&StageKeypair` to each
        case; `EinmoCase::promote` takes the already-derived keypair
        precisely so this stays possible. **Add a test that pins it**
        (count derivations, or assert elapsed time is not ~N×1.8s) — this
        is the kind of performance invariant a refactor silently loses.
  - [ ] Preserve `promote`'s `to == Stage::Flagged` delegation to `flag`.
- [ ] Remove `transitions.rs`'s `promote`/`flag`/`retract` free
      functions; migrate `cli.rs` and `verify.rs:408` to the
      `EinmoSuite` methods. `transitions.rs` keeps
      `is_legal_transition`, the report types, and `flag`'s
      advisory-block concatenation (§S.10).
- [ ] Audit every existing `transitions::promote` test for the changed
      semantics. Where a test asserted clobbering, convert it to assert
      co-signing and leave a comment naming `EIMP-7` §S.4, so a future
      reader auditing stamp history finds the reason.
- [ ] Update `review.rs`'s execute path to call `EinmoCase::promote`
      (removing the now-duplicated private path) and confirm
      `execute_promote_matches_cli_promote_byte_for_byte` still passes —
      with both sides now on one implementation, it becomes a much
      stronger assertion than it was.
- [ ] Phase F tests green; `clippy`/`fmt` clean; commit.

## Phase G — `EinmoSuite` drives `CorpusSigner` (§S.8)

- [ ] Read §S.8 of `EIMP-7.md`.
- [ ] Baseline first: record the current section digest for a fixture
      corpus. It **must not change** — this phase moves where the
      manifest's ids and bytes come from, never what the manifest
      contains.
- [ ] Write tests first for all three `CorpusSignatureUpdate` outcomes:
      `Created` (no `.section.sig` yet), `Updated` (corpus changed since
      signing), `AlreadyCurrent` (signature matches — **asserting
      nothing was written**, e.g. by mtime or by a storage-write
      counter). The `AlreadyCurrent` arm is the one that makes routine
      re-signing affordable, so it is asserted, not assumed.
- [ ] Implement `EinmoSuite::update_corpus_signature(stage, key)` +
      `CorpusSignatureUpdate`: build the manifest from the cases already
      scanned into the suite (no fourth walk), construct the
      `CorpusSigner` over it, read artifact bytes through
      `EinmoStorage`, and write only where absent or stale.
- [ ] Change `CorpusSigner::manifest_under` (`src/corpus_signer.rs:100`)
      to accept the suite's case list instead of walking independently;
      route `digest_for`'s per-artifact reads through `EinmoStorage` so a
      non-filesystem backend is signable (§S.8 consequence 2).
- [ ] Migrate `cli.rs`'s corpus-signing call site onto
      `update_corpus_signature`; the caller no longer assembles a signer
      or decides staleness.
- [ ] Assert the digest is **byte-identical** to the recorded baseline.
- [ ] Phase G tests green; `clippy`/`fmt` clean; commit.

## Comprehensive test + completion

- [ ] Write and verify the EIMP-7 comprehensive test, per `EIMP-7.md`
      §Test Plan: build a realistic multi-section, multi-depth fixture
      suite; drive `einmo test`-shaped FAE/FF validation **and** `einmo
      review`-shaped promote/flag/retract/list through the **same**
      `EinmoSuite` instance; assert both consumers' view of every case's
      presence and agreement facts agrees. This is the property the whole
      EIMP exists to guarantee, so it is asserted directly rather than
      inferred from the per-phase tests.
  - [ ] Include a case differing only in COMMENTS and assert `einmo
        test` and `einmo review` now answer **consistently** about it
        under a single `MatchSections` policy.
  - [ ] Include a tampered artifact and assert it is reported as
        tampered — not as "differing" — by both consumers.
- [ ] Confirm no third comparison, promotion, or suite-walk implementation
      survives: grep for `walk_input_tree` callers outside `stage.rs` and
      `storage.rs` and justify or remove each; confirm `scan_tests`,
      `TestRow`, and `promote_one_accumulating` are gone.
- [ ] Full verification: `cargo test --workspace`, `cargo clippy
      --all-targets -- -D warnings`, `cargo fmt --check` all clean.
      Record the final test count here (356 before this EIMP).
- [ ] Verify all work is committed on `jia`.
- [ ] Update `EIMP-7.md` frontmatter `status: complete`.
- [ ] Update `docs/eimp/INDEX.md` to reflect EIMP-7's completed status.
- [ ] Update `EIMP-1.plan.md`'s **P1** checkbox: mark it `[x]` with a
      timestamp, noting it was resolved by EIMP-7 rather than in place —
      P1 is the finding this EIMP implements, and leaving it open would
      misreport EIMP-1's own state.

## Documentation — the stage vocabulary (§S.9)

Not deferrable to "later": the phrasing should land while the types are
being renamed, so docs and code agree at every commit.

- [ ] Adopt **"the output stage" / "the checked stage" / "the verified
      stage"** as the standard phrasing across `AGENTS.md`, `README.md`,
      and doc comments — in preference to bare "output" (ambiguous with
      an envelope's `OUTPUT` section) and to "the output directory"
      (which names the storage, not the lifecycle step).
  - [ ] `AGENTS.md` — the stage lifecycle description.
  - [ ] `README.md` — the four-stage overview becomes three stages plus
        the per-stage flagged sink (§S.2a).
  - [ ] `src/stage.rs` module + `Stage` doc comments; `einmo_suite.rs`'s
        `ValidationLevel` docs (state the stage-vs-level distinction
        explicitly, per §S.9's table).
  - [ ] `cli.rs` help text and any user-facing strings that name stages.
- [ ] Confirm no doc still describes `flagged/` as a top-level directory
      or as a fourth stage.

## Post-EIMP follow-ups (recorded for later)

- [ ] `EIMP-5` (Merkle corpus signing) should revisit its §S.2 tree shape
      against `EinmoSuite::directory_tree()` — hashing at every directory
      level, mirroring the suite's real section structure, is now a
      supported option rather than a redesign. Recorded in `EIMP-5.md`
      §Open Questions on 2026-07-31.
- [ ] Consider whether `EinmoStorage` should gain a batch/streaming read
      for `CorpusSigner`'s benefit once `EIMP-5`'s parallel machinery
      lands — deliberately out of scope here (§S.1 keeps the trait
      minimal), but a parallel signer may want it.
