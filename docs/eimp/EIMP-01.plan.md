# EIMP-01.plan — generated-stage-and-predecessor-only-gates

Implementation plan for [EIMP-01](EIMP-01.md): a separate `einmo generate`
phase writing the uncommitted `generated/` stage, `output/` as a committed
baseline reached by an explicitly weak promotion, and three gates that each
compare only against their immediate predecessor.

**Read `EIMP-01.md` in full before executing this plan.** Section pointers
appear inline below. EIMP plans execute directly on `jia`, with regular
commits — there is no worktree stage.

---

## Phase 0 — Preconditions

- [x] Confirm the suite is green before changing anything:
      `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
      Record the passing test count here. **Never start substantive work when
      tests are broken.**
      (2026-08-11 16:17)
      **Baseline: 401 tests, 0 failed, exit 0** (`cargo test --workspace` at
      `c261179`; count from `cargo test --workspace -- --list`). For
      comparison, `EIMP-9` §S.0 measured 394 at `ac873c3`. `zweimomo` 4/4
      including `eimp3_output_drift_comprehensive` — the test Phase 7 must
      **rewrite, not delete**. `einmo-tools` 8/8.
      Clippy and fmt: deferred to the first phase that changes code; nothing
      had changed at this point but documentation.
- [x] Begin work: commit `EIMP-01.md` and `EIMP-01.plan.md`, check
      `begun: [x]` in the `EIMP-01.md` frontmatter
      (2026-08-11 16:18)
      Committed as `c261179` and pushed to `origin/jia`. Frontmatter set to
      `begun: [x]`, `status: Draft` → `Implementing`. The one Open Question
      (does `generated/` appear in the review worklist) is deliberately left
      open — it is scheduled for Phase 6 and blocks none of Phases 1–5; the
      reasoning is recorded in EIMP-01.md §Open Questions rather than left
      implicit.
- [x] Add `generated/` to `.gitignore` (read §S.1 of EIMP-01.md for the
      input-directory-name collision caveat, which is accepted, not guarded)
      (2026-08-11 16:18)
      Added with a comment naming EIMP-01 and stating the collision caveat
      inline, so the next reader of `.gitignore` does not have to find §S.1
      to know it was a considered trade.

---

## Phase 1 — `Stage::Generated`

- [x] Read §S.1 of EIMP-01.md
      (2026-08-11 16:20)
- [x] Write the tests first (`src/stage.rs`): `Stage::ALL` has four entries in
      lifecycle order; `Stage::parse("generated")`; `dir_name` / `stamp_key`;
      `Generated < Output < Checked < Verified`; `EinmoId::to_stage_path`
      round-trips through `Generated`
      (2026-08-11 16:22)
      Four new tests, confirmed RED first (`E0599: no variant ... named
      Generated`) before any implementation. `ALL` is asserted as an exact
      array equality rather than a length check, and
      `stage_ordering_follows_the_lifecycle` exists because a stage declared
      in the wrong position would still compile and still round-trip by name —
      nothing else would catch it. `einmo_id_round_trips_through_stage_path`
      already loops `Stage::ALL`, so it covers `Generated` unmodified.
- [x] Add the `Generated` variant to `src/stage.rs`, declared **first** so the
      derived `Ord` keeps lifecycle order
      (2026-08-11 16:25)
- [x] Confirm no code path persists or transmits a `Stage`'s **ordinal** (the
      `Deserialize` impl is by name via `Stage::parse`; check `journal.rs`,
      `review_server.rs`, and `corpus_signer.rs` wire formats). If one does,
      fix it to be by name and note it here.
      (2026-08-11 16:39)
      **Verified clean, no fix needed.** There is no `Serialize` impl for
      `Stage` at all; every wire format stores `stage: String` and recovers it
      through `Stage::parse` (`journal.rs:60,150,159`,
      `corpus_signer.rs:527,549`, `review_server.rs:580`). No `stage as usize`
      or equivalent ordinal cast exists anywhere in `src/` or `src/bin/`, and
      nothing compares or sorts `Stage` values, so the derived `Ord` is used
      only by the new test that pins it. Reordering the variants therefore
      cannot corrupt persisted data.
- [x] Write the tests first (`src/config.rs`): `StageDirs::default` includes
      `generated`; `validate` and `ensure_stage_dirs` cover all four; the
      generation stage is renameable from `einmo.toml`
      (2026-08-11 16:30)
      Four new tests. **The third clause of this checkbox was wrong and the
      spec was corrected instead**: stage directory names are not
      `einmo.toml`-settable for *any* stage — `TestConfig` always builds
      `StageDirs::default()`. EIMP-01.md §S.1 claimed otherwise; it now
      records the correction, and the `.gitignore` comment that inherited the
      same false claim (telling a reader to rename the stage in `einmo.toml`)
      was fixed too. Stage *passphrases* genuinely are per-stage configurable,
      so `[signing] generated` is real plumbing and is tested.
      One test was written too strong and weakened deliberately:
      `ensure_stage_dirs_creates_the_generation_stage_too` also asserted each
      stage's flagged sink exists. It failed correctly — sinks are created
      lazily on first flag, which was already true of all three pre-existing
      stages. The reasoning is written into the test's doc comment rather than
      left in a chat log.
- [x] Add the `generated` field to `StageDirs` and its `einmo.toml` plumbing
      (2026-08-11 16:32)
      `StageDirs.generated` (default `"generated"`), `StagePassphrases.generated`,
      `SigningConfig.generated`, and the `[signing] generated` key in
      `parse_toml_content`. Unset follows the `output`/`checked` deployment
      convention — present and empty, i.e. the computer key — not `verified`'s
      absent.
- [x] Fix every `Stage::ALL` consumer that no longer compiles or whose test
      expectations assumed three stages (`src/case.rs`, `src/verify.rs`,
      `src/storage.rs`, `src/suite.rs`, `src/transitions.rs`,
      `src/einmo_suite.rs`, `src/config.rs`)
      (2026-08-11 16:35)
      **Four sites, not the seven anticipated.** `Stage::ALL` *iterations*
      absorb a new variant silently and correctly; only `match stage { … }`
      sites break. The four: `case.rs:436` (`retract`'s cascade),
      `config.rs:43` (`StageDirs::name`), `config.rs:175`
      (`StagePassphrases::get`), and `bin/einmo_review_server.rs:673`
      (`parse_decidable_stage` — missed by `cargo build --lib`, caught by
      `--all-targets`).
      Two of the four already have their final EIMP-01 answer rather than a
      placeholder: `retract` refuses `Generated` (§S.6 — it is regenerated
      every run; flipping `Output` to *allowed* is Phase 5's job and is
      commented as such), and `parse_decidable_stage` refuses `Generated`
      (§S.3 — `promote generated to output` is a CLI act, never a reviewer's
      decision). The latter deliberately does not prejudge Phase 6's open
      question: a stage can be inspectable without being decidable, which is
      already true of `output`.
- [x] Run all tests — old and new — and make sure they all pass correctly
      (2026-08-11 16:38)
      **409 tests, 0 failed** (`cargo test --workspace`), against the Phase 0
      baseline of 401 — exactly the 8 added. `cargo fmt --check` clean;
      `cargo clippy --workspace --all-targets -- -D warnings` clean.
      zweimomo 4/4, including `eimp3_output_drift_comprehensive`, which is
      still green because nothing about drift has changed yet — Phase 2
      removes it and Phase 7 rewrites that test.
- [x] Commit
      (2026-08-11 16:41)
      `03f9cac` — "EIMP-01 Phase 1: Stage::Generated, the fourth stage".

---

## Phase 2 — The generation phase writes `generated/`

- [x] Read §S.2 and §S.5 of EIMP-01.md
      (2026-08-12 09:35)
- [x] Write the tests first: generation writes only `generated/` and leaves
      `output/` byte-untouched; the crash crumb lands in `generated/`;
      generation fails on an evaluator error; fails on an unsound written
      artifact; fails on an extraneous `input/` file; fails on an empty suite;
      **does not** fail when `generated` differs from `output`
      (2026-08-12 09:40)
      Five new tests. `generation_writes_generated_and_leaves_output_byte_-
      untouched` asserts on BYTES, not existence: a committed baseline
      rewritten with identical content is still a violation, because the stamp
      chain and metadata header would churn.
      Two clauses were already covered and are not duplicated: evaluator error
      (`err_becomes_input_error_status`, `panic_becomes_output_error_status`)
      and the shape preconditions (`check_integrity`'s O1/O2 tests). "Fails on
      an unsound written artifact" is asserted indirectly via
      `written_and_verified`, which is the re-verify-what-we-wrote result;
      there is no way to make einmo write an unsound artifact on purpose
      without breaking the signer, so it is not directly triggerable.
- [x] Write the tests first: pruning removes a `generated/` artifact whose
      input was deleted, and leaves `generated/flagged/` alone
      (2026-08-12 09:41)
- [x] Retarget `write_output` at `generated/` (rename it to match) and sign
      with the generation stage's key
      (2026-08-12 09:52)
      Renamed `write_generated`. **`Stamps::generate` turned out to be
      specialized to `"stage:output"`** — passing it the generated keypair
      produced a file stamped `stage:output` signed by the generation key,
      which made the no-op fast path miss and 14 tests fail with an opaque
      byte-diff. Fixed by calling `Stamps::generate_for_stage(...,
      Stage::Generated.stamp_key(), ...)`, the general form `EIMP-1` §S.3
      already added for `notes/`. Worth recording: the failure surfaced as a
      1000-element byte-array assertion, not as anything naming the stamp key.
- [x] Move catastrophe-crumb creation into `generated/`
      (2026-08-12 09:45)
      Follows from the retarget — the crumb is written to the same `out_path`.
      Recorded in `write_crash_crumb`'s doc comment, including the consequence
      for EIMP 6: the crumb no longer pollutes a committed tree.
- [x] Implement pruning of input-less `generated/` artifacts, skipping the
      flagged sink via `is_in_flagged_sink`
      (2026-08-12 09:47)
      `prune_generated`, called at the top of `evaluate_all` — before
      evaluating, so a crash mid-run leaves a pruned tree rather than a
      half-pruned one.
- [x] Remove the drift path: `FileResult::drifted`, its `detail` message,
      `EinmoTestRunner::regenerate_output`
      (2026-08-12 09:50)
      All three gone. The `force` parameter threaded through `evaluate_impl`
      and `write_output` went with them.
- [x] Remove the multi-signer co-sign path from the runner (§S.5 — co-signing
      is `promote`'s job)
      (2026-08-12 09:50)
      `write_generated`'s `existing` handling is now a single fast path: same
      sections AND already carrying this signer's stamp → restore the original
      bytes; anything else → fresh write. Both removed branches are documented
      at the site with where their requirement now lives.
- [x] Retire EIMP 3's tests that covered the removed surface, with a comment
      naming EIMP-01; rewrite the tests covering EIMP 3's **intent** (drift
      must not be silently accepted) against the Output gate in Phase 4
      (2026-08-12 09:55)
      Three unit tests retired, replaced by a block naming each one, what it
      covered, and where its surviving requirement now lives — so nothing is
      dropped silently. One (`..._different_signer_appends_stamp`) is replaced
      by a test pinning the *new* behavior rather than leaving a hole.
      Two survive, retargeted: the byte-identical no-op and the corrupt-existing
      fresh write.
- [x] **Pulled forward into Phase 2 by the compiler** — recorded rather than
      done silently, because each belongs to a later phase's checkbox:
      (2026-08-12 10:05)
  - [x] **Phase 3's CLI verb.** Removing `regenerate_output` broke `cli.rs`
        immediately, so `Generate` (alias `evaluate`) replaced `Evaluate`, and
        `RegenerateOutput` was removed, in this phase.
        (2026-08-12 09:58)
  - [x] **Phase 5's `(Generated, Output)` transition.** Seven `review.rs`
        tests and two zweimomo tests used `regenerate_output` to refresh
        `output/` after changing an input. Under EIMP-01 that act *is*
        generate-then-promote, so the transition had to exist for them to
        compile. One line in `is_legal_transition`; the rest of Phase 5
        (retraction inversion, help text) is untouched.
        (2026-08-12 10:00)
  - [x] **Phase 7's zweimomo rewrite.** `eimp3_output_drift_comprehensive`
        could not compile against the removed surface. Rewritten as
        `eimp01_generate_promote_comprehensive` — generate → no-op → distinct
        signer → changed result does NOT fail → promote signs → clean rerun —
        against the real `BoaEvaluator` and a scratch copy of `day.1`. The
        Output-gate assertion is the one part it cannot make yet; Phase 4 adds
        it. `crash_crumb_survives_stack_overflow` repointed at `generated/`
        and given a second assertion that `output/` stays clean.
  - [x] Seven `review.rs` sites collapsed into one `generate_and_accept`
        helper, and `seeded_suite` now promotes — tests that want a committed
        baseline must ask for one, because evaluation no longer produces it as
        a side effect.
        (2026-08-12 10:02)
  - [x] The same fixture change in `review_server.rs` (its own `seeded_suite`
        plus one inline site). **Found only by the full workspace run** — the
        earlier `review::tests`-scoped runs were all green.
        (2026-08-12 10:15)
  - [x] And again in `src/bin/einmo_review_server.rs`'s own `seeded_suite`.
        **Three layers of the same fixture**, each invisible to the previous
        layer's scoped run: `review.rs` → `review_server.rs` → the binary.
        Lesson for the remaining phases: a scoped `cargo test --lib <module>`
        is not evidence; only `cargo test --workspace` is.
        (2026-08-12 10:22)
- [x] **EIMP-9's T9 poisoning cascade, observed live.** Recorded because it
      shaped how this phase was debugged and is worth the next reader knowing.
      (2026-08-12 10:12)
      The workspace run reported **40 failures; 3 were real.** The other 37
      were `PoisonError` from `JOURNAL_ENV_LOCK.lock().unwrap()`
      (`review.rs:1271`) after the first genuine panic poisoned it — and every
      one of the 3 real failures **passed in isolation**, so per-test reruns
      diagnosed nothing. The method that worked: filter the panic lines,
      discard everything pointing at the lock site, and read only what
      remained. A later run showed the same 40-for-1 pattern in
      `review_server::tests`.
      This is EIMP-9 §S.1 T9 exactly, and the concrete argument for its
      nextest recommendation: process-per-test isolation makes a failure
      *count* mean something. Staying on `cargo test` per maintainer
      direction; recording the cost rather than re-learning it.
- [x] Run all tests — old and new — and make sure they all pass correctly
      (2026-08-12 10:21)
      **412 tests, 0 failed** (`cargo test --workspace`): 368 lib + 31
      review-server binary + 8 einmo-tools + 4 zweimomo + 1. Against the
      Phase 1 baseline of 409: +5 generation tests, +1 replacing the retired
      co-sign test, −3 retired EIMP-3 tests = +3. `cargo fmt --check` clean;
      `cargo clippy --workspace --all-targets -- -D warnings` clean.
      zweimomo 4/4 including the rewritten
      `eimp01_generate_promote_comprehensive` against the real `BoaEvaluator`.
- [x] Commit
      (2026-08-12 10:25)

---

## Phase 3 — The `generate` CLI verb

**Executed inside Phase 2** — removing `regenerate_output` broke `cli.rs`
immediately, so the verb could not wait. Recorded here rather than left
looking undone.

- [x] Read §S.2 of EIMP-01.md
      (2026-08-12 09:35)
- [x] Write the tests first for the verb's exit codes: zero when every input
      evaluated to a sound artifact, non-zero on an evaluator error
      (2026-08-12 09:58)
      Covered by the existing exit-code path plus the parser smoke test, which
      now asserts `generate` and its `evaluate` alias parse **and that
      `regenerate-output` does not** — the removal is asserted, not merely
      done, so re-adding the verb has to be a deliberate act.
- [x] Rename the `Evaluate` subcommand to `Generate`, keeping `evaluate` as a
      clap alias
      (2026-08-12 09:58)
- [x] Remove the `RegenerateOutput` subcommand; its help text pointed at
      EIMP 3's workflow, which no longer exists
      (2026-08-12 09:58)
- [x] Help text must state both uses (§S.2): a test that things run, and the
      way to materialize results **for direct inspection without disturbing
      committed `output/`**
      (2026-08-12 09:58)
      Both stated, plus the sentence a user is most likely to assume wrong:
      "compares against nothing: a result differing from `output/` is the
      normal outcome of a change, not a failure here."
- [x] Run all tests — old and new — and make sure they all pass correctly
      (2026-08-12 10:21)
      With Phase 2's run: 412 passing.
- [x] Commit
      (2026-08-12 10:25)
      With Phase 2's commit.

---

## Phase 4 — Gates compare only against their predecessor

- [x] Read §S.4 of EIMP-01.md
      (2026-08-12 10:30)
- [x] Write the tests first: `output` reports
      `SectionDifference { Generated, Output }` on divergence; `checked` and
      `verified` invoke the evaluator **zero** times (assert with a
      call-counting `Evaluator`) and write nothing (assert on directory
      mtimes); `verified` no longer reports `output ↔ checked` problems; each
      gate still reports a signature failure on either side of its own pair
      (2026-08-12 10:34)
      Seven new tests, confirmed RED first. The two negatives are asserted
      with evidence rather than left to review: a counting `Evaluator` proves
      zero invocations, and stage-directory mtimes prove nothing is written.
      `verified_level_does_not_report_output_checked_problems` breaks
      output↔checked deliberately and asserts the verified gate stays green —
      the removed cumulative behavior, pinned.
- [x] Write the tests first: a legacy `output/` artifact carrying only
      `compiled`/`configured`/`stage:output` (no `stage:generated`) passes the
      `output` gate (§S.8)
      (2026-08-12 10:34)
      Hand-builds the legacy stamp chain rather than assuming one, and asserts
      the absence of `stage:generated` as a precondition.
- [x] Replace `escalation()` / `escalates_from()` with `compares_against()`
      and `generates()` per §S.4's API shape
      (2026-08-12 10:36)
      Both removed. `Ord` on `ValidationLevel` is kept — it still follows the
      chain, so `level >= Checked` stays meaningful even though the levels no
      longer imply one another.
- [x] Rewrite `check_integrity` to run, per level: O1, O2, orphans in the
      stages that level touches, and `stage_pair_problems` for that level's
      pair only — plus attestation (V6/V7) at `verified`
      (2026-08-12 10:37)
      The `for step in level.escalation()` loop is gone; the body is now a
      straight line: shape → orphans in `level.stage()` → the one pair →
      attestation if `Verified`.
- [x] Wire the `output` gate to run the generation phase first
      (2026-08-12 10:45)
      In the library this is `evaluate_all` + the level's integrity check. In
      the CLI it needed a decision, recorded in §S.4: **`einmo verify --level
      output` now requires `--command`** and refuses without it, because
      comparing a stale `generated/` produces a green gate that asserts
      nothing about the current tree. `--command` at a non-generating level is
      refused too. Both refusals are tested; the silent-green case is the
      dangerous one.
- [x] Confirm no new `Problem` variant is needed (§S.4) — divergence is
      `SectionDifference`, a runtime error is `ArtifactUnsound`
      (2026-08-12 10:38)
      Confirmed — but `Problem::level()` needed fixing. Its catch-all mapped a
      pair problem with `right: Output` to `ValidationLevel::Checked`, correct
      only while `Output` could never appear on the right. Both matches are now
      exhaustive over `Stage`, so a future stage cannot be silently misfiled.
- [x] **Found and fixed: a gate misnamed which SIDE was tampered.**
      (2026-08-12 10:42)
      `ComparisonResult::tampered` was `Vec<PathBuf>` and `stage_pair_problems`
      attributed every entry to the pair's **right** stage — so tampering the
      left-hand artifact was reported against the right-hand one. Harmless
      while both sides of a pair were always reviewed together; wrong once a
      gate's job is to name the link that broke. The information was never
      missing: `StagePairAgreement::Tampered { stages }` carries it, and
      `compare.rs` discarded it with a `{ .. }` wildcard. Now
      `Vec<TamperedEntry>`, with one `SignatureDoesNotVerify` per failing side.
- [x] **Recorded: a suite with no baseline is now red at the Output level.**
      (2026-08-12 10:44)
      Correct — the gate asserts "the code produces the committed baseline",
      and a never-promoted suite has none. But it is a real behavior change
      (evaluation used to BE the baseline), and it shifted what
      `TestResults::all_output_written_and_verified` means: it folds in
      `integrity.is_clean()`, so it now answers "the level's gate passes"
      rather than "every input evaluated". Documented on the method, with
      callers wanting the weaker claim pointed at
      `files.iter().all(...)`. Five pre-existing tests encoded the old
      assumption and were updated to promote, or to assert the weaker claim
      where that is what they meant.
- [x] Run all tests — old and new — and make sure they all pass correctly
      (2026-08-12 10:40)
      **419 tests, 0 failed** (`cargo test --workspace`) against Phase 2's
      412: +7 gate tests, +1 CLI refusal test, −1 (`levels_escalate_cumulatively`
      replaced by `levels_compare_against_their_immediate_predecessor_only`).
      `cargo fmt --check` and
      `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [x] Commit
      (2026-08-12 10:52)

---

## Phase 5 — Promotion and retraction

- [ ] Read §S.3 and §S.6 of EIMP-01.md
- [ ] Write the tests first: `is_legal_transition(Generated, Output)` is true;
      `(Generated, Checked)` and `(Generated, Verified)` are false; retract
      from `generated` is refused; retract from `output` cascades through
      `verified`
- [ ] Write the tests first: `promote generated to output` writes an `output/`
      artifact whose configured sections are byte-identical to its `generated/`
      source and whose stamps are the source's plus `stage:output`; a second
      signer co-signs rather than rewriting
- [ ] Add `(Generated, Output)` to `is_legal_transition`
- [ ] Invert the retraction rule in `EinmoCase::retract` and
      `EinmoSuite::retract`: refuse `generated`, allow `output` with a cascade
      through `checked` and `verified`. Update `RetractArgs`' doc comment,
      which currently says "`checked` or `verified`".
- [ ] Set the `promote generated to output` help text to the **weak** claim
      (§S.3): "ran without error, output is reasonable — not a semantic or
      stylistic review". Read it beside `promote output to checked` and
      confirm it reads as clearly weaker.
- [ ] Run all tests — old and new — and make sure they all pass correctly
- [ ] Commit

---

## Phase 6 — The review surface: server and TUI (no GUI)

`EIMP-1` built the review surface against **three** stages. A fourth changes
what it enumerates. **The dhtml frontend is backburnered** (`EIMP-1.md` §S.9,
`EIMP-1.plan.md` §Phase E) — the sprint's scope is library, tests, CLI
promotion, server, and TUI. The 4-pane page is therefore **knowingly left
stale** by this phase; that is a recorded decision, not an oversight.

- [ ] Read §S.0 and §S.1 of EIMP-01.md, and `EIMP-1.md` §S.2–§S.7
- [ ] Inventory every place the review surface enumerates stages —
      `src/review.rs` (`EinmoReview`, `ReviewMode`, worklist construction),
      `src/review_server.rs` (stage path params, body/diff endpoints),
      `src/case.rs` (`EinmoCase::stages`). List them in this plan before
      changing any, so the blast radius is written down rather than
      discovered.
- [ ] Decide and record: **does `generated/` appear in the review worklist?**
      This is EIMP-01.md's one Open Question. Arguments both ways: it is a
      full stage with signed artifacts and `einmo compare generated output`
      is exactly a review activity; but a reviewer promoting `output →
      checked` has no business in an uncommitted work file. Record the
      answer in EIMP-01.md §S.1 and remove it from §Open Questions.
- [ ] Write the tests first for whichever answer was chosen — the worklist
      contains (or excludes) `generated/`, and the server's stage endpoints
      accept (or `400` on) `generated`
- [ ] Implement in `src/review.rs` and `src/review_server.rs`
- [ ] TUI client (`scripts/einmo_review_client.sh`): confirm it still works
      against the updated server. If it hard-codes the three stage names,
      update it; if it reads them from the server, confirm that path.
- [ ] **Do not touch `src/dhtml/review.html`.** Leave the 4-pane page as it
      is. Add a one-line comment at its head naming the staleness and
      pointing at `EIMP-1.plan.md` §Phase E, so the next reader knows it is
      deliberate.
- [ ] Run all tests — old and new — and make sure they all pass correctly
- [ ] Commit

---

## Phase 7 — zweimomo: the integration testbed

`zweimomo` is where einmo is exercised end-to-end against real evaluators
(`BoaEvaluator` for JavaScript, `Pyo3Evaluator` for Python) over a real suite
at `zweimomo/suites/python/`. It is the only place the whole stage model runs
for real, so it is both the thing that must be updated and the best evidence
this EIMP works.

- [ ] Read `zweimomo/tests/suites.rs` in full and list every test that
      assumes the three-stage model in this plan before editing any
- [ ] **`eimp3_output_drift_comprehensive` (`zweimomo/tests/suites.rs:208`)**
      — this is EIMP 3's comprehensive test, and EIMP-01 removes the
      behavior it covers. **Rewrite it against the Output gate; do not
      delete it.** It encodes a real requirement — a changed evaluator must
      not silently redefine the baseline — which EIMP-01 keeps and relocates.
      Deleting a failing test instead of rewriting it is precisely the
      failure mode `EIMP-9` exists to catch.
  - [ ] Name the rewritten test for EIMP-01, and state in a comment what it
        inherited from EIMP 3 and what changed
  - [ ] It must still fail if the baseline is silently redefined
- [ ] `crash_crumb_survives_stack_overflow` — crumbs now land in
      `generated/`. Update the path it asserts on. **The test must still
      genuinely overflow the stack in a re-spawned child**; do not weaken it
      into a unit test.
- [ ] `python_suite_generates_and_verifies` and
      `javascript_tiers_generate_and_verify` — update `run_suite` / `run_tier`
      for the four-stage model: generate into `generated/`, then gate
- [ ] `zweimomo/src/evaluators.rs` — confirm the `Evaluator` impls need no
      change (EIMP-01 does not touch the trait); record that as verified
      rather than assumed
- [ ] Add a zweimomo integration test for the **new** surface: generate →
      `generated ≠ output` → Output gate red → `promote generated to output`
      → Output gate green, driven through the real Python evaluator
- [ ] Run all tests — old and new — and make sure they all pass correctly
- [ ] Commit

---

## Phase 8 — Verify the real suite is unchanged

- [ ] Run `einmo generate` over `zweimomo/suites/python`
- [ ] Compare `generated` against the committed `output`
      (`einmo compare generated output --root-cause`)
- [ ] **Report the result to the human in ONE statement**: how many artifacts
      differ, and whether any **configured section** differs. §S.8 says a
      section difference here is a bug this implementation introduced, not a
      diff to accept — fix the code, do not promote.
- [ ] Confirm all three gates are green against the untouched committed
      baseline: `--level output`, `--level checked`, `--level verified`
- [ ] Run all tests — old and new — and make sure they all pass correctly
- [ ] Commit

---

## Phase 9 — Documentation

- [ ] **Review and land the end-user "What Passing Means" section of
      `README.md`.** The section was **written up front**, before
      implementation, from §S.0 of EIMP-01.md — it is already in `README.md`
      carrying a status marker saying it describes a model that is specified
      but not yet implemented. This task verifies it against what was actually
      built and removes the marker. **The section is a claim to end users; do
      not remove the marker until every line of it is true.**
  - [ ] Re-read the section beside §S.0 of EIMP-01.md and beside the
        implementation. Every pass/fail condition in §S.0 must appear in the
        README, and the README must claim nothing §S.0 does not.
  - [ ] Verify each named signature against the code: `compiled`,
        `configured`, `stage:generated`, `stage:output`, `stage:checked`,
        `stage:verified`. If implementation renamed or added one, fix the
        README table.
  - [ ] Verify each named compared section against `compare.rs`: `INPUT`,
        `OUTPUT` / `OUTPUT[i]`, `DIFF` on referencing cases, `COMMENTS` only
        under the stricter configuration. Fix any drift.
  - [ ] Verify the "deliberately not compared" list: the `STAMPS` section and
        the metadata header (suite path, producing commit, einmo binary hash,
        run timestamp). Confirm by experiment that two runs minutes apart
        still match — the README states this as a promise to the reader.
  - [ ] Verify every command in the section actually runs as written:
        `einmo generate`, `einmo verify --level {output,checked,verified}`,
        `einmo compare generated output --root-cause`, `einmo show`,
        `einmo body`, `einmo promote generated to output`
  - [ ] Verify the red-gate remedy table names the real remedies, and that
        the retraction paragraph matches the implemented cascade
  - [ ] Confirm the audience guard still holds: no `Problem` variant names, no
        O1/C2/V5 codes, no Rust type names, no EIMP numbers in the prose
  - [ ] Have a reader who has not read EIMP-01 answer, from the README alone:
        "what exactly must be true for `--level output` to pass?" If they
        cannot, the section is not done.
  - [ ] **Remove the status marker** at the head of the section — this is the
        last sub-task in this block, and removing it asserts the section is
        now true of shipped einmo
- [ ] Update the rest of `README.md` for the new model — the parts NOT covered
      by the section above, which still describe the three-stage world:
  - [ ] "The Three Stages" → four stages; add the `generated/` row (written by
        `einmo generate`, gitignored, never committed) and retitle the section
  - [ ] The `flagged/` paragraph — it says the sink is nested inside "each of
        the three stages"; there are now four
  - [ ] "CLI" subcommand table — `generate` (with `evaluate` as its alias),
        `promote generated to output`, and the removal of `regenerate-output`
  - [ ] "Quick Start" — the loop is now generate → inspect → promote → gate
  - [ ] "Catastrophe Crumb Defense" — crumbs land in `generated/` now
  - [ ] "Configuration Precedence" / `einmo.toml` `[suite]` — the `generated`
        stage-directory name is configurable like the other three
  - [ ] "Appendix: Migrating an insta test to einmo" — its directory tree and
        worked commands assume three stages
  - [ ] Grep the whole file for `regenerate-output`, `evaluate`, and "three
        stages" and fix every remaining hit
- [ ] Update `AGENTS.md`: how an agent materializes results for inspection
      without disturbing `output/`, and the distinction between the weak
      `generated → output` claim and the `output → checked` review
- [ ] Update `rust_instructions.md` where it states the three-stage contract —
      it becomes four; say plainly which stage is the work file
- [ ] Update `docs/eimp/EIMP-3.md`: set `status: superseded by EIMP-01` and add
      a forward pointer. **Do not rewrite its history** — add the reference.
- [ ] Update `docs/eimp/INDEX.md` with the EIMP-01 row
- [ ] Update the `eimp-use-maintain` and `repo-context` skills if either names
      the stage set or the `evaluate` / `regenerate-output` verbs
- [ ] Run all tests — old and new — and make sure they all pass correctly
- [ ] Commit

---

## Phase 10 — Comprehensive test and completion

- [ ] Write and verify the EIMP-01 comprehensive test — one test walking the
      whole chain (§Test Plan): generate → inspect → promote to output → gate
      green → mutate the evaluator → gate red naming the section → promote →
      green again → promote to checked → retract output → checked and verified
      cascade away
- [ ] All tests pass: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`
- [ ] Report ALL accumulated doubts to the human in ONE statement — or record
      "no doubts". Blocking doubts stop here.
- [ ] STOP! ASK HUMAN to review before marking complete. Present the
      `einmo compare generated output` output from Phase 6 alongside.
- [ ] Update `EIMP-01.md` frontmatter `status: complete`
- [ ] Update `docs/eimp/INDEX.md` status for EIMP-01 and EIMP-3
- [ ] Commit

---

## Last Updated

**Date**: 2026-08-11 (2)
**Updated By**: Claude Code (Opus 5)
**Changes**: Added two phases that the initial plan missed entirely.
**Phase 6 — the review surface**: `EIMP-1` built `EinmoReview`, the server,
and the TUI against three stages, and a fourth changes what they enumerate;
this phase also resolves EIMP-01's one Open Question (does `generated/` appear
in the review worklist). Scoped to **server and TUI only — the dhtml frontend
is backburnered** per `EIMP-1.md` §S.9, so the 4-pane page is knowingly left
stale and says so in a comment. **Phase 7 — zweimomo**: the integration
testbed, where `eimp3_output_drift_comprehensive` must be **rewritten against
the Output gate rather than deleted** (it encodes a requirement EIMP-01 keeps
and relocates; deleting a failing test is the failure mode `EIMP-9` exists to
catch), the crash-crumb stack-overflow test repointed at `generated/` without
being weakened, and a new end-to-end test added for the generate → red gate →
promote → green loop through the real Python evaluator. Old Phases 6–8
renumbered to 8–10.

**Date**: 2026-08-11
**Updated By**: Claude Code / claude-opus-5
**Changes**: Initial plan. Ordered so the stage enum lands first (Phase 1),
the runner retargets before any gate changes (Phase 2–3), and the real
`zweimomo/suites/python` baseline is proven unchanged (Phase 6) before any
documentation claims the new model. The end-user "What Passing Means" section
of `README.md` was **written up front** (before implementation) from §S.0, and
ships behind a status marker saying it describes a specified-but-unimplemented
model; Phase 7 therefore *reviews* it against what was built, verifies every
named signature, section, exclusion, and command, and removes the marker as
its last step. A second Phase 7 block lists the parts of `README.md` the new
section does not cover and that still describe the three-stage world.
