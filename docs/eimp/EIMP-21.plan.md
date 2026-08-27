# EIMP-21.plan — Paired Python and JavaScript Project Euler answer suites

Execute this plan sequentially on `jia`. Read [`EIMP-21.md`](EIMP-21.md)
before beginning. Each problem is a complete vertical slice: its Python and
JavaScript implementations, four named tests, generated inspection, and
reviewed baselines land together. Never put an answer literal in Rust tests and
never edit a signed `.einmo` artifact by hand.

## Phase 0 — Begin and establish the contract

- [ ] Establish relevant tests for this phase. Use [these instructions](../../README.md#running-specific-tests) to run: `js_arithmetic_smoke`, `python_arithmetic_smoke`, `javascript_tiers_generate_and_verify`, `python_suite_generates_and_verifies`.
- [ ] Run the complete repository gates before substantive work; fix any existing failure first.
- [ ] Commit `EIMP-21.md` and `EIMP-21.plan.md`; set `begun: [x]` and `status: Implementing` only when execution actually starts.
- [ ] Record the Project Euler copyright, direct-link, spoiler, and responsible-use rules from EIMP 21 §S.0 and §S.6 in `zweimomo/suites/euler/README.md`.
- [ ] Confirm Impact Overview and Migration remain accurate before implementation; this EIMP has no existing-suite migration.
- [ ] Commit: `EIMP-21 Phase 0: begin paired Euler corpus`.

## Phase 1 — Module-style paired evaluators (§S.2–S.3)

- [ ] Establish relevant tests for this phase. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_python_calls_solve`, `euler_javascript_calls_solve`, `euler_python_rejects_missing_solve`, `euler_javascript_rejects_missing_solve`, `euler_python_rejects_non_string`, `euler_javascript_rejects_non_string`.
- [ ] Write Python evaluator tests first: normal multi-line module, exact string return, missing/non-callable `solve`, raised exception, arguments rejected, and non-string rejected.
- [ ] Write JavaScript evaluator tests first with the same cases and error classifications.
- [ ] Implement crate-private `EulerPyo3Evaluator` using a fresh namespace and an explicit zero-argument `solve()` call; return exactly one output string.
- [ ] Implement crate-private `EulerBoaEvaluator` using a fresh context and an explicit zero-argument `solve()` call; return exactly one output string.
- [ ] Add problem/language context to errors without leaking or supplying an expected answer.
- [ ] Confirm existing `Pyo3Evaluator` and `BoaEvaluator` expression behavior/tests are unchanged.
- [ ] Run the full `just` gate and commit: `EIMP-21 Phase 1: module-style paired evaluators`.

## Phase 2 — Manifest and scratch-generation harness (§S.1, §S.4)

- [ ] Establish relevant tests for this phase. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_manifest_is_ordered_and_paired`, `euler_manifest_rejects_answer_field`, `euler_fixture_tree_matches_manifest`, `euler_languages_match_for_every_manifest_problem`, `euler_suites_write_returned_answers`.
- [ ] Write manifest parser/validation tests first: unique ascending contiguous IDs, canonical filenames and links, both source paths, no answer field, and no unlisted sources.
- [ ] Add `zweimomo/suites/euler/manifest.toml`, paired language directories, suite configuration, spoiler/attribution README, and empty generated-stage ignore rules.
- [ ] Write a helper that executes one problem twice in fresh runtimes, requires one non-empty canonical string, and compares the two runs for determinism.
- [ ] Write a paired helper that compares Python and JavaScript result strings without accepting an expected answer argument.
- [ ] Write the scratch einmo harness: copy suites, generate with the language adapter, extract the generated OUTPUT string, and compare generated↔output plus output↔checked.
- [ ] Add failure diagnostics naming problem id, language, contract, and stage link without printing a hidden expected answer before review.
- [ ] Run the full `just` gate and commit: `EIMP-21 Phase 2: manifest-driven answer harness`.

## Phase 3 — Problem 001 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_001_python_returns_string`, `euler_problem_001_javascript_returns_string`, `euler_problem_001_languages_agree`, `euler_problem_001_einmo_writes_answer`.
- [ ] Add Problem 001 manifest record and failing paired/source/baseline completeness tests.
- [ ] Implement Python Problem 001 `solve() -> str` without an answer literal.
- [ ] Independently implement JavaScript Problem 001 `solve() -> string`.
- [ ] Run the four focused tests; independently validate the computed answer against the official problem.
- [ ] Generate both scratch suites, inspect with einmo `compare`/`show`/`body`, and create repository output/checked baselines only through CLI promotion.
- [ ] Run aggregate Euler tests and the full `just` gate; commit: `EIMP-21 Problem 001: paired solutions`.

## Phase 4 — Problem 002 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_002_python_returns_string`, `euler_problem_002_javascript_returns_string`, `euler_problem_002_languages_agree`, `euler_problem_002_einmo_writes_answer`.
- [ ] Add Problem 002 manifest record and failing completeness tests.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 002: paired solutions`.

## Phase 5 — Problem 003 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_003_python_returns_string`, `euler_problem_003_javascript_returns_string`, `euler_problem_003_languages_agree`, `euler_problem_003_einmo_writes_answer`.
- [ ] Add Problem 003 manifest record and failing completeness tests.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 003: paired solutions`.

## Phase 6 — Problem 004 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_004_python_returns_string`, `euler_problem_004_javascript_returns_string`, `euler_problem_004_languages_agree`, `euler_problem_004_einmo_writes_answer`.
- [ ] Add Problem 004 manifest record and failing completeness tests.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 004: paired solutions`.

## Phase 7 — Problem 005 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_005_python_returns_string`, `euler_problem_005_javascript_returns_string`, `euler_problem_005_languages_agree`, `euler_problem_005_einmo_writes_answer`.
- [ ] Add Problem 005 manifest record and failing completeness tests.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 005: paired solutions`.

## Phase 8 — Problem 006 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_006_python_returns_string`, `euler_problem_006_javascript_returns_string`, `euler_problem_006_languages_agree`, `euler_problem_006_einmo_writes_answer`.
- [ ] Add Problem 006 manifest record and failing completeness tests.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 006: paired solutions`.

## Phase 9 — Problem 007 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_007_python_returns_string`, `euler_problem_007_javascript_returns_string`, `euler_problem_007_languages_agree`, `euler_problem_007_einmo_writes_answer`.
- [ ] Add Problem 007 manifest record and failing completeness tests.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 007: paired solutions`.

## Phase 10 — Problem 008 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_008_python_returns_string`, `euler_problem_008_javascript_returns_string`, `euler_problem_008_languages_agree`, `euler_problem_008_einmo_writes_answer`.
- [ ] Add Problem 008 manifest record and any attributed/licensed problem data required; write failing completeness tests first.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 008: paired solutions`.

## Phase 11 — Problem 009 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_009_python_returns_string`, `euler_problem_009_javascript_returns_string`, `euler_problem_009_languages_agree`, `euler_problem_009_einmo_writes_answer`.
- [ ] Add Problem 009 manifest record and failing completeness tests.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 009: paired solutions`.

## Phase 12 — Problem 010 paired slice

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `euler_problem_010_python_returns_string`, `euler_problem_010_javascript_returns_string`, `euler_problem_010_languages_agree`, `euler_problem_010_einmo_writes_answer`.
- [ ] Add Problem 010 manifest record and failing completeness tests.
- [ ] Implement independent Python and JavaScript `solve()` functions returning strings without answer literals.
- [ ] Run focused tests and independently validate the answer; record an explicit runtime budget for both implementations.
- [ ] Generate, inspect, and promote output/checked baselines only through einmo CLI.
- [ ] Run aggregate Euler tests and full `just`; commit: `EIMP-21 Problem 010: paired solutions`.

## Phase 13 — Comprehensive verification and closure

- [ ] Establish relevant tests for this phase. Use [these instructions](../../README.md#running-specific-tests) to run: `eimp21_project_euler_001_through_010_comprehensive`, `euler_manifest_is_ordered_and_paired`, `euler_languages_match_for_every_manifest_problem`, `euler_python_suite_generates_and_matches`, `euler_javascript_suite_generates_and_matches`.
- [ ] Write and verify `eimp21_project_euler_001_through_010_comprehensive`: fresh runtime per solution; exactly one string; deterministic rerun; language equality; scratch generated artifact contains that string; generated↔output and output↔checked both agree for all twenty implementations.
- [ ] Prove manifest problems are exactly 001–010 and every record has paired source/output/checked files with direct attribution links.
- [ ] Confirm no Rust test or manifest field contains an expected answer and no implementation reads a baseline, performs network access, or merely returns the known answer literal.
- [ ] Run formatting, workspace clippy with warnings denied, workspace tests/nextest, doctests, documentation links, dependency policy, and applicable mutation tests; record machine-readable results.
- [ ] Review the Impact Overview and confirm Migration is still None for pre-existing users and suites.
- [ ] Add a follow-up-EIMP note explaining how a later finite problem range reuses the manifest and four-test vertical slice without extending EIMP 21.
- [ ] Update `EIMP-21.md` to `status: complete` only when Problems 001–010 and all gates are complete.
- [ ] Update `docs/eimp/INDEX.md` and affected Markdown Last Updated sections.
- [ ] Commit: `EIMP-21 complete: paired Project Euler Problems 001 through 010`.

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Created the ordered implementation plan for paired Python and
JavaScript Project Euler Problems 001–010, including module evaluators,
manifest-driven tests, scratch einmo answer writing, reviewed baselines, and
comprehensive closure.
