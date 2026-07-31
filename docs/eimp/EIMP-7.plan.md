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

- [ ] STOP — preconditions: `cargo test --workspace`, `cargo clippy
      --all-targets -- -D warnings`, `cargo fmt --check` all clean. Do not
      begin while any test is broken (`EIMP-0` §8).
- [ ] Read `docs/eimp/EIMP-7.md` in full — in particular §S.3
      (`EinmoCase`/`StageAgreement`) and §S.7 (the comparison
      unification), which carry the design decisions the rest of this
      plan assumes.
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
- [ ] Set `EIMP-7.md` frontmatter `status: Implementing` and `begun: [x]`;
      commit both EIMP files stating that work has commenced.
- [ ] **S.0 — rename `einmo_suite::EinmoSuite` → `EinmoTestRunner`.**
      Mechanical, no behavior change, must compile as one commit. 51
      occurrences across 6 files: `src/einmo_suite.rs` (31),
      `src/review.rs` (12), `src/cli.rs` (4), `src/review_server.rs` (2),
      `src/lib.rs` (1, the `pub use` at line 47), `src/bin/
      einmo_review_server.rs` (1).
  - [ ] Rename the type, its `impl` blocks, and the `pub use` export.
  - [ ] Update `einmo_suite.rs`'s module doc comment (line 1-2 names
        `EinmoSuite` as "the test runner" — that sentence becomes correct
        again only after the rename).
  - [ ] `cargo test --workspace` green, `clippy`/`fmt` clean. This is a
        pure rename: the test count must be **unchanged at 356**, and no
        test body should need editing beyond the type name.
  - [ ] Commit: "EIMP-7 S.0: rename EinmoSuite to EinmoTestRunner".

## Phase A — `EinmoStorage` + `EinmoDirectory` (§S.1, §S.2)

- [ ] Read §S.1 and §S.2 of `EIMP-7.md`. Note especially §S.2's constraint:
      **the `input/`/`output/`/`checked/`/`flagged/`/`verified/` split is
      deliberately preserved** — hand-authored suites are browsed and
      edited in place, so `EinmoDirectory` resolves every `(EinmoId,
      ArtifactLocation)` to exactly the path it resolves to today.
- [ ] Write tests first: round-trip `read`/`write`/`remove`; `read` of an
      absent artifact is `Ok(None)`, never an error; `remove` of an absent
      artifact is a no-op, not an error; `list_ids` for `Input` and for
      each `Stage`.
- [ ] Add `ArtifactLocation` (`Input` | `Stage(Stage)`), deriving
      `Debug, Clone, Copy, PartialEq, Eq, Hash`.
- [ ] Define the `EinmoStorage` trait exactly as §S.1 specifies
      (`read`/`write`/`remove`/`list_ids`).
- [ ] Implement `EinmoDirectory`. **Reuse, do not reimplement**, the
      existing path primitives: `EinmoId::to_stage_path`,
      `mirror_input_path`, `ensure_parent_dir` (`src/stage.rs`),
      `TestConfig::stage_dir`/`input_path`/`walk_depth_limit`
      (`src/config.rs`). `list_ids(Input)` wraps `walk_input_tree` +
      `EinmoId::from_input_rel`; `list_ids(Stage(s))` wraps
      `walk_input_tree(stage_dir(s))` + `EinmoId::from_stage_artifact_path`.
      No new path-construction logic is written in this phase.
- [ ] Add the in-memory `EinmoStorage` test fake (`HashMap<(EinmoId,
      ArtifactLocation), Vec<u8>>`), `#[cfg(test)]` but `pub(crate)` so
      Phases B and C can use it without a tempdir.
- [ ] Parity test: a fixture suite on disk, read through `EinmoDirectory`,
      yields the same ids and bytes as the same fixture loaded into the
      in-memory fake.
- [ ] Phase A tests green; `clippy`/`fmt` clean; commit.

## Phase A2 — `flagged/` moves inside each stage (§S.2a)

Lands here, right after `ArtifactLocation` exists and before `EinmoCase`
builds on it, so nothing downstream is written against the old
four-variant `Stage`.

- [ ] Read §S.2a and §S.10's blast-radius note in `EIMP-7.md`.
- [ ] Write tests first: `ArtifactLocation::Flagged(stage)` resolves to
      `<stage>/flagged/<id>.einmo` for each of the three stages; flagging
      from the checked stage lands in `checked/flagged/` and **not** in
      `output/flagged/`; the origin stage is recoverable from the
      location alone (the property this change exists for).
- [ ] Remove `Stage::Flagged`. `Stage` becomes `Output | Checked |
      Verified`. Work through the 27 references across
      `transitions.rs` (15), `stage.rs` (4), `einmo_suite.rs` (3),
      `verify.rs` (2), `config.rs` (2), `bin/einmo_review_server.rs` (1).
  - [ ] `Stage::ALL` (4 → 3) and `dir_name()`.
  - [ ] Delete `stamp_key()`'s `Flagged` arm — dead by construction:
        flagging appends an unsigned advisory, never a stamp.
  - [ ] `is_legal_transition`: drop the three `* → Flagged` rows.
        Flagging is no longer a stage transition at all; it is a move
        within a stage, so it leaves the transition table entirely.
  - [ ] `promote`'s `to == Stage::Flagged` delegation to `flag` — remove;
        `promote` can no longer be asked to flag.
  - [ ] `count_flagged` (`einmo_suite.rs:508`): walk all three
        `<stage>/flagged/` directories; decide and document whether it
        returns a flat count or a per-stage breakdown (a per-stage
        breakdown is the point of this change — prefer it, and update
        `einmo verify`'s gate message to name the stages).
  - [ ] The R2 orphan exemption ("`flagged/` is exempt") — re-express
        per-stage.
  - [ ] The review server's flag/retract endpoints and
        `bin/einmo_review_server.rs`.
- [ ] `StageDirs::flagged` stays a configurable *name*; only its parent
      changes. Confirm a suite configuring a custom flagged name still
      works.
- [ ] Migrate the live fixture `zweimomo/suites/javascript/day.1/flagged`
      → `output/flagged/`, per §S.2a: old top-level flags have no
      recorded origin stage, so they move to the most conservative
      reading rather than being guessed per-file. **Say so in the commit
      message** — a reader finding these in `output/flagged/` later must
      know they were migrated, not flagged there.
- [ ] Phase A2 tests green; `clippy`/`fmt` clean; commit (message must
      call out both the on-disk break and the fixture migration).

## Phase B — `EinmoCase` (§S.3)

- [ ] Read §S.3 of `EIMP-7.md`, including the `StagePairAgreement` enum
      rationale and the **deliberate asymmetry** between `agreement`
      (policy-driven) and `promote`'s destination-match check
      (all-non-STAMPS-sections).
- [ ] Write tests first for `agreement`, covering every
      `StagePairAgreement` variant: `Agree`; `Differ` (assert the
      **section names**, not just that it differs); `OneSided` (both
      directions, asserting which stage is `present`); `BothAbsent`;
      `Tampered` (assert a tampered artifact is `Tampered`, **never**
      folded into `Differ` — the distinction `scan_tests` loses today).
  - [ ] Include the **P1 repro** as a named test: a fresh suite where
        `output` and `checked` agree and `verified/` is empty →
        `pair(Output, Checked)` is `Agree`. Today's `differing` bool is
        `true` here; this test is the fix's proof.
  - [ ] Include the **COMMENTS repro** as a named test (the third finding
        in §Motivation): two stages agreeing on INPUT/OUTPUT but
        differing in COMMENTS → `Agree` under
        `MatchSections::InputOutput`, `Differ { sections: ["COMMENTS"] }`
        under `InputOutputComments`. This is the inconsistency between
        `einmo test` and `einmo review` that this EIMP closes.
- [ ] Implement `StagePairAgreement`, `StageAgreement` (with its recorded
      `policy` field and the `pair(left, right)` accessor).
- [ ] Implement `EinmoCase` (`read`, `stages`, `agreement`).
      `agreement` reuses `compare.rs`'s existing required-section rules
      (`required_sections`/`is_required_section`/`compare_sections`) —
      make them `pub(crate)` if needed, but **do not fork them**; §S.7
      folds `compare::compare` onto this same core, and two copies would
      recreate the exact defect this EIMP exists to remove.
- [ ] Write tests first for `promote`: all three `PromoteOutcome`
      variants (`Promoted`, `CoSigned`, `AlreadySigned`), plus the
      `non_human` flag on verified-stage promotion with a computer key.
- [ ] Implement `EinmoCase::promote` + `PromoteOutcome` by **moving**
      `review.rs`'s `promote_one_accumulating` (`src/review.rs:1179`)
      here, converting its `(String, bool)` return into the enum. Keep
      its logic byte-identical; only the return shape changes.
  - [ ] Byte-for-byte equivalence test: `EinmoCase::promote` produces
        output identical to the pre-move `promote_one_accumulating` for
        each of the three cases. (`review.rs` already has an equivalence
        test of this shape — `execute_promote_matches_cli_promote_byte_
        for_byte` — model it on that, and note P0's deadlock lesson: do
        not hold two `TestContext` journal locks on one thread.)
- [ ] Implement `EinmoCase::flag` / `retract` as per-id delegations to
      `transitions::flag` / `transitions::retract` (already correct and
      already shared — behavior must not change).
- [ ] Phase B tests green; `clippy`/`fmt` clean; commit.

## Phase C — `EinmoSuite` (§S.5)

- [ ] Read §S.5 of `EIMP-7.md`.
- [ ] Write tests first: `scan` against both the in-memory fake and a
      real `EinmoDirectory` fixture (parity); the id union covers cases
      present only in `input/`, only in a stage, and in both; `filter`
      behaves as `scan_tests`'s existing filter does; `cases()` ordering
      matches today's `rels.sort()`.
- [ ] Implement `EinmoSuite::scan` / `cases` / `case`.
- [ ] Write tests first for `directory_tree()` against a multi-level
      fixture using real-shaped ids (`foop/23/sub_feature/test1`): every
      case appears in exactly one node at the right depth; a case at the
      root (`test1.foo`, no directory components) is handled; no node
      exists for a component with neither cases nor children.
- [ ] Implement `directory_tree()` + `DirectoryNode`. Pure and on-demand
      — no persisted tree state (§Rejected Alternative D).
- [ ] Phase C tests green; `clippy`/`fmt` clean; commit.

## Phase D — one pairwise-comparison implementation (§S.7)

- [ ] Read §S.7 of `EIMP-7.md`.
- [ ] Before changing anything: capture the **current** `ComparisonResult`
      for every existing `compare.rs` fixture as a baseline assertion, so
      the fold below is provably behavior-preserving rather than
      apparently so.
- [ ] Reimplement `compare::compare`'s body as a fold over
      `EinmoSuite::cases()` + `EinmoCase::agreement(&[a, b], policy)`,
      bucketing each `StagePairAgreement` into the existing
      `matching`/`differing`/`only_in_a`/`only_in_b`/`tampered` vectors.
      **Public shapes do not change**: `ComparisonResult`, `DiffEntry`,
      `compare`'s signature, and `root_causes` all stay as they are.
  - [ ] Preserve the `files: Option<&[PathBuf]>` argument's meaning — it
        becomes a filter over `cases()`, not a substitute id list.
        `cli.rs:546` (`einmo compare`) passes it; its behavior must not
        change.
  - [ ] Confirm the independent input-tree walk at `compare.rs:104` is
        now gone (the fourth such walk in the crate).
- [ ] `einmo_suite.rs`'s two `compare::compare` call sites (line 640
      `stage_pair_problems`, line 987 `require_correspondence`) keep
      calling it unchanged — verify by running their existing tests
      untouched.
- [ ] Phase D tests green (`compare::tests` and `einmo_suite::tests` in
      particular); `clippy`/`fmt` clean; commit.

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
