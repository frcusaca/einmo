# EIMP-11.plan — Reliability hardening and transactional persistence

Execute this plan sequentially on `jia`. Read
[`EIMP-11.md`](EIMP-11.md) before beginning. Every sub-section's test list is
live: add each new test name as it is written, run the subset frequently, and
run the full `just` gate when that sub-section completes.

Test names in an “Establish relevant tests” checkbox are stable target names.
Names already present in the tree are the old subset to run immediately. A name
not yet present is created by that sub-section's following test-first task,
added to the runnable subset as soon as it lands, and must fail for the intended
reason before implementation begins. Do not interpret a not-yet-present target
name as a passing test or silently substitute a different test.

Items marked **HUMAN DECISION REQUIRED** are hard stops. The implementing
agent prepares evidence and a recommendation, then asks the maintainer and
does not implement the affected semantic/design choice until the maintainer
answers. The request must follow `eimp.md`'s human-communication protocol and
state that it comes from EIMP 11 and that work is on `jia`.

## Human review queue

Review these items in order. Checking an item records approval of the linked
first-pass recommendation unless the checkbox is annotated with an amendment.
An amendment must also be copied into the corresponding EIMP 11 section before
implementation crosses that gate. For each item, review all standardized
descriptors in its workfile: **Einmo Library User Experience changes**,
**Einmo Integration changes**, **Einmo Development changes**, and
**Migration**. Approval
therefore covers both the policy and its stated API, CLI/test/CI, documentation,
contributor-workflow consequences, and ordered compatibility work.

- [x] **[HUMAN REVIEW 1 — RESOLVED] Transition graph:** review Decision A in
  [`EIMP-11.workfile.transitions-and-review.md`](EIMP-11.workfile.transitions-and-review.md).
  Decide whether to remove `verified → checked`, use `retract verified` to
  withdraw attestation, and reserve historical restoration for separate design.
      (2026-08-27 14:17)
  Accepted with ordering amendment: complete Phase 1B's complete-stamp and
  multi-signature semantics first, then remove the backward edge and test
  multiple checked signers followed by a human verified signer.
- [x] **[HUMAN REVIEW 2 — RESOLVED] Confirmed review boundary:** review
  Decision B.1 in the transitions/review workfile. Decide whether confirmed
  batch plans are suite-wide all-or-nothing in EIMP 11, with partial execution
  deferred and `execute_one` retained as the smaller explicit boundary.
      (2026-08-27 14:17)
  Accepted as proposed.
- [x] **[HUMAN REVIEW 3 — MERGED INTO EIMP-A] Transaction semantics:** review
  [`EIMP-11.workfile.transaction-semantics.md`](EIMP-11.workfile.transaction-semantics.md)
  in this order: operation descriptors and boundaries; dependency observations;
  conflict/idempotence rules; same-body co-sign merge; incomplete-generation
  retention; reference-model deliverables. Decide whether the newest incomplete
  generation is retained for seven days by default with configurable bounds.
  External Git owns completed history, signed envelopes carry their immediate
  source-stage/hash/signature link, and transaction revisions are recovery/
  concurrency metadata rather than a browsable archive. EIMP-A must place the
  still-open incomplete-generation-retention choice in the stage-by-stage claim
  and ownership context before asking it again.
      (2026-08-27 15:02)
- [x] **[HUMAN REVIEW 3A — FIRST CHILD SELECTED] EIMP decomposition:** review
  [`EIMP-11.md` §S.20](EIMP-11.md#s20--suggested-extraction-eimps).
  The maintainer selected EIMP-A as the first child: a plain-language,
  stage-by-stage integrity and provenance standard explaining exactly what
  signatures guarantee and what they do not. EIMP-A also proposes the complete
  disposition of EIMP-B through EIMP-G and overlapping existing EIMPs before
  deeper work is split out. Current documentation adopts the accepted standard
  after that EIMP completes.
      (2026-08-27 15:02)
- [x] **[HUMAN REVIEWS 4–5 — MERGED INTO EIMP-A] Durability and persistence:**
  The durability and persistence workfiles are inputs to EIMP-A's integrated
  claim/ownership analysis. They are not independent human gates. EIMP-A must
  explain operational coherence versus signed integrity, show exactly which
  questions belong to later EIMP-B/EIMP-C, and present them in context before
  requesting another decision.
      (2026-08-27 15:02)
- [x] **[HUMAN REVIEW 6 — RESOLVED/DEFERRED] Attestation score:** review
  [`EIMP-11.workfile.attestation-score.md`](EIMP-11.workfile.attestation-score.md).
  The maintainer chose to retain the compression score. It must not be described
  as entropy, identity, or proof of human participation. A second password
  quality score/interface may be proposed in EIMP-E, but is not urgent EIMP 11
  work; do not remove the current API or dependencies under Gate D.
      (2026-08-27 14:38)
- [x] **[HUMAN REVIEW 7 — DIRECTION ACCEPTED; MEASUREMENTS CONFIRM] Crate
  boundary:** review Decision E in
  [`EIMP-11.workfile.crate-and-document-boundaries.md`](EIMP-11.workfile.crate-and-document-boundaries.md).
  Decide whether to ratify EIMP 4's core/server split and where corpus signing
  belongs; amend the split only if measurements contradict its assumptions.
  Review the dependency→ownership→testing→release steady-state model.
  Accepted with the requirement that the standardized distribution support
  managing and reviewing cases through `cargo einmo`.
      (2026-08-27 14:38)
- [x] **[HUMAN REVIEW 8 — DIRECTION ACCEPTED; FINAL DISPOSITIONS WAIT FOR LEDGER]
  Document disposition:** review Decision F in the crate/document workfile.
  Decide whether the proposed direction is correct: complete EIMP 01 normally,
  retain EIMP 9 until its tooling contract lands, and narrow EIMP 8 finding by
  finding. Final status/banner/removal approval waits for the populated
  requirement-migration ledger. Review the fact→authority→migration→historical
  record lifecycle as the continuing documentation standard.
  Accepted as the direction; individual status/banner/removal actions remain
  blocked on the completed ledger at Gate F.
      (2026-08-27 14:38)

## Execution tranches and stop points

The plan remains sequential, but its work is partitioned into contiguous
tranches. An agent may execute one tranche straight through without asking a
question. It stops at the following human gate and does not enter the next
tranche until every decision listed at that gate is recorded in EIMP 11.

| Tranche | Status now | Contiguous work | Stops before |
|---|---|---|---|
| **A — critical fixes** | **Executable immediately; Reviews 1–2 resolved** | Phase 0; Phase 1A; Phase 1B; Phase 1C's forward-graph correction and evidence | Recorded Gate A disposition |
| **B — atomicity evidence** | Waiting on accepted EIMP-A ownership map after Phase 1C | Finish Phase 1C; then execute or hand off Phase 2A exactly as EIMP-A assigns it | EIMP-A contextualizes generation retention, durability, and persistence before another decision |
| **C — persistence API and prototypes** | Waiting on EIMP-A and its approved EIMP-B/EIMP-C disposition | Execute or hand off Phase 2B–2D under one recorded owner per task | Architecture selection in the owner assigned by EIMP-A |
| **D — backend and general hardening** | Waiting on Gate C; Review 6 resolved | Finish Phase 2D; Phase 2E; all of Phase 3; Phase 4A; Phase 4B; Phase 4C documentation correction | Recorded Gate D disposition |
| **E — type and dependency evidence** | Executable after Tranche D; Gate D resolved | Finish Phase 4C; Phase 4D; Phase 4E measurements | Gate E: ratify or amend EIMP 4; place corpus signing |
| **F — maintainability and documentation preparation** | Waiting on Gate E | Finish Phase 4E; Phase 5A; Phase 5B; Phase 5C inventory, migration ledger, and proposed dispositions | Gate F: approve old-document dispositions |
| **G — deprecation and closure** | Waiting on Gate F | Finish Phase 5C and Phase 6 | No further known human gate |

“Waiting” here means only that the tranche may not be implemented yet. The
preceding tranche deliberately includes all research, prototypes,
measurements, and recommendations needed for the human to answer the next
gate without sending the implementer back for more routine discovery.

### Task markers and dependency discipline

- **`[SEQ]`** — run after the preceding numbered task in that sub-section;
  it changes the same contract or depends on the preceding artifact.
- **`[PAR lane-name]`** — may run concurrently with other `PAR` lanes after
  the sub-section's test-establishment and test-first tasks are complete.
- **`[JOIN]`** — do not start until every named parallel lane immediately
  above it has completed and its artifacts have been reviewed together.
- **`[HUMAN]`** — hard stop described by `HUMAN DECISION REQUIRED`.

Parallel lanes may inspect and test independently, but they must not edit the
same source or Markdown file concurrently. When two lanes touch one file,
execute them sequentially or assign one lane ownership of the shared edit.
No Cargo commands run concurrently because this repository shares target
state in some environments. A phase-level commit follows each join so a
partially integrated set of lanes never becomes the new baseline.

### Design workfile dependency map

The workfiles are EIMP 11 design evidence, not separately executable EIMPs.
They expose boundaries that may become sub-EIMPs only after dependencies and
accepted semantics are stable.

| Workfile | May proceed after | Must join before | Possible later extraction signal |
|---|---|---|---|
| transitions and review | current-code audit | resolved Decisions A and B.1 before API freeze | separately designed historical restore or partial-review UX |
| transaction semantics | mutation inventory and §S.4 vocabulary | Gate B and API freeze | backend-independent transaction types become independently releasable work |
| durability | mutation inventory and §S.4 vocabulary | Gates B.2 and C | platform enablement needs a distinct verification lifecycle |
| persistence prototypes | Gate B plus stable contract tests | Gate C | selected backend/migration can execute independently |
| attestation score | complete-stamp integrity correction | Gates D and E | a replacement password-advice facility gains a distinct security policy |
| crate and document boundaries | dependency/document inventory | Gates E and F | crate split or EIMP 8 migration gains its own acceptance boundary |

Transaction semantics and durability may run in parallel. Persistence
prototypes depend on both and compare against one shared contract. Crate and
document evidence is independent until current architecture documentation must
describe selected persistence and package boundaries. Do not create a sub-EIMP
merely for file size; extract only work with its own normative decision,
testable completion condition, and execution sequence that leaves EIMP 11
unambiguous.

### Extraction execution protocol (§S.20)

- [ ] Create EIMP-A first, before Phase 2A implementation. Run
  `eimp_check.py gen_next` at creation time and use `eimp-write-plan`; keep the
  design-local alias beside its assigned real EIMP identifier.
- [ ] EIMP-A starts with the plain-language stage/signature claim matrix and
  suite-wide signature statement, then proposes the disposition of EIMP-B
  through EIMP-G and all overlapping existing EIMPs. It explicitly includes or
  excludes every current EIMP 11 section, workfile, finding, test, and human
  question; Reviews 4 and 5 are contextual inputs, not separate gates.
- [ ] Do not create EIMP-B through EIMP-G until EIMP-A's integrity statement and
  ownership proposal are accepted. For each subsequently approved extraction,
  run `eimp_check.py gen_next`, create its specification and plan with
  `eimp-write-plan`, and copy the complete accepted contract, impacts,
  migration, tests, dependencies, and completion conditions assigned by EIMP-A.
- [ ] In the same documentation change, replace moved EIMP 11 implementation
  tasks with timestamped handoff checkboxes naming the new EIMP and exact moved
  sections/test responsibilities; never leave duplicate active ownership.
- [ ] Re-run `eimp_check.py check`, update `docs/eimp/INDEX.md`, and verify every
  EIMP 8/EIMP 9/EIMP 01/EIMP 4 overlap has exactly one active implementation
  owner.
- [ ] If EIMP-A merges, defers, or rejects a later lettered candidate, record
  that disposition and leave or reassign its EIMP 11 tasks exactly as stated;
  unused aliases create no files, statuses, or reserved numbers.

## Tranche A — Critical fixes (executable immediately)

## Phase 0 — Begin and re-baseline

- [ ] Establish relevant tests for this phase. Use [these instructions](../../README.md#running-specific-tests) to run: `metadata_whitespace_tamper_detected`, `promote_refuses_tampered_source`, `verified_level_rejects_empty_passphrase_key`, `generated_promotes_only_into_output`, `eimp01_output_gate_goes_red_on_divergence_and_green_after_promotion`.
- [ ] **[SEQ 0.1]** Re-run the complete repository gates before substantive work; resolve any pre-existing failure first and record environment-only blockers without substituting `cargo test` for the required nextest contract.
  - [ ] Record toolchain, clippy, nextest, Python-development-library, and target-directory state.
  - [ ] Confirm fmt, workspace clippy, workspace nextest, and doctest outcomes separately.
  - [ ] Preserve machine-readable failure output before repairing a real baseline failure.
- [ ] **[SEQ 0.2]** Begin work: commit `EIMP-11.md`, `EIMP-11.plan.md`, and the index update; set `begun: [x]` and `status: Implementing` only when implementation actually starts.
- [ ] **[PAR finding-audit]** Re-audit EIMP 11's P0/P1/P2 findings against current `jia`; record changed locations, already-fixed findings, and new regression-test names without weakening invariants.
- [ ] **[PAR prior-eimp-audit]** Map EIMP 11 sections to EIMP 8 accepted unresolved findings and EIMP 9 tasks; include owner, status, overlap, and migration constraint.
- [ ] **[PAR public-surface-audit]** Capture current public API, CLI help/exit codes, stage graph, config behavior, and filesystem capability as a before-state artifact.
- [ ] **[JOIN 0.3]** Reconcile the three audits; ensure every planned change has one owner and no accepted EIMP 8/EIMP 9 task disappears.
- [ ] **[SEQ 0.4]** Commit: `EIMP-11 Phase 0: baseline and finding traceability`.

## Phase 1 — Critical integrity defects

### Phase 1A — Refuse invalid destinations (§S.1)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `promote_refuses_tampered_source`, `agreement_tampered_is_never_folded_into_differ`, `promote_refuses_tampered_destination`, `flag_refuses_tampered_flagged_destination`, `promote_flag_to_note_refuses_tampered_destination`.
- [ ] **[SEQ 1A.1]** Write failing destination-preservation tests first.
  - [ ] Malformed destination at each ordinary stage.
  - [ ] Signature-invalid destination at each ordinary stage.
  - [ ] Invalid existing nested flagged artifact during re-flag.
  - [ ] Invalid existing note destination during flag-to-note promotion.
  - [ ] Assert error class/path and byte-identical destination preservation.
- [ ] **[SEQ 1A.2]** Introduce one internal read-state type distinguishing absent, verified, malformed, and signature-invalid observations; keep path/context with invalid states.
- [ ] **[PAR promote-path]** Replace promotion/co-sign destination `.ok()` collapse; test absent, matching, co-signable, differing, malformed, and tampered branches.
- [ ] **[PAR flag-note-path]** Replace flagged/note destination `.ok()` collapse; test advisory accumulation and invalid-destination refusal.
- [ ] **[PAR fail-open-audit]** Inventory every `verify_bytes(...).ok()`, `EinmoFile::from_file(...).ok()`, and default-on-error use; classify as fail-safe optional cache, explicit diagnostic probe, or prohibited state collapse.
- [ ] **[JOIN 1A.3]** Route prohibited paths through the shared read-state/error mapping; ensure CLI, review, and server preserve the typed failure.
- [ ] **[SEQ 1A.4]** Run focused tests, full `just`, scoped mutants for invalid/absent branch swaps, then commit: `EIMP-11 Phase 1A: refuse invalid destinations`.

### Phase 1B — Evaluate every verified stamp (§S.2)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `verified_level_rejects_empty_passphrase_key`, `verified_attestation_human_then_computer_fails`, `verified_attestation_computer_then_human_fails`, `verified_attestation_expected_reviewer_among_cosigners_passes`, `verified_attestation_missing_expected_reviewer_fails`.
- [ ] **[SEQ 1B.1]** Write order/permutation tests first.
  - [ ] Human only; expected human among multiple humans; expected human absent.
  - [ ] Human→computer and computer→human.
  - [ ] Duplicate identical verified stamps.
  - [ ] Zero verified stamps in a verified artifact.
  - [ ] Unexpected human co-signer is reported but does not independently fail.
- [ ] **[SEQ 1B.2]** Introduce an internal attestation-policy result containing all verified signer observations, computer-key offenders, expected-reviewer match, and final verdict.
- [ ] **[PAR gate-policy]** Replace first-match gate logic with order-independent all-stamps evaluation.
- [ ] **[PAR reporting]** Add stable prose and JSON diagnostics for missing expected reviewer and every computer-key offender; redact secret inputs.
- [ ] **[PAR review-surface]** Make review/server summaries consume the same policy result instead of re-deriving signer meaning.
- [ ] **[JOIN 1B.3]** Confirm all surfaces agree on every permutation and duplicated stamp case.
- [ ] **[SEQ 1B.4]** Mutation-test `any`/`all`, first/last, empty-set, and computer-key branches; run full `just`; commit: `EIMP-11 Phase 1B: verify the complete attestation set`.

### Phase 1C — Enforce one transition graph (§S.3)

- [ ] Establish relevant tests for the autonomous forward-graph sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `generated_promotes_only_into_output`, `forward_transition_matrix_is_exact`, `output_to_verified_is_refused_by_library_cli_and_review`, `retract_output_cascades_through_checked_and_verified`.
- [ ] **[SEQ 1C.1]** Write forward-only transition tests first: the three adjacent forward edges are legal and every forward skip is illegal; cover the accepted `verified → checked` removal separately in 1C.4 after the resolved-decision marker.
- [ ] **[PAR core-graph]** Remove `output → verified` from the authoritative library graph without changing the backward edge.
- [ ] **[PAR surface-graph]** Add tests proving CLI parser/help, review planning, and server requests reject `output → verified`.
- [ ] **[PAR fixture-audit]** Locate tests/docs/fixtures relying on the shortcut; classify each required migration without editing signed artifacts yet.
- [ ] **[JOIN 1C.2]** Make all forward surfaces consume or test against the authoritative representation.
- [ ] **[SEQ 1C.3]** Run full `just` and commit the forward-graph correction before applying the recorded backward-edge decision.

## Resolved Gate A — Backward transition semantics

- [x] **HUMAN DECISION RECORDED:**
  [`EIMP-11.workfile.transitions-and-review.md`](EIMP-11.workfile.transitions-and-review.md)
  Decision A removes `verified → checked` because it is not a demotion, can
  invert stamp lifecycle order, is absent from review planning, and duplicates
  `retract verified`. The maintainer accepted it with the requirement that
  complete-stamp/multi-signature work and tests land first.
      (2026-08-27 14:17)

## Tranche B — Atomicity evidence (after Gate A)

### Phase 1C completion — Apply the backward-edge decision

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `legal_transition_matrix_is_exact`, `verified_to_checked_matches_recorded_semantics`, `retract_output_cascades_through_checked_and_verified`.
- [ ] **[SEQ 1C.4]** After Phase 1B is complete, expand the forward-only matrix into exhaustive `Stage × Stage` expectations incorporating resolved Gate A, including multiple checked signers followed by a human verified signer.
- [ ] **[PAR core-backward]** Implement Gate A in the authoritative graph and retraction/transition APIs.
- [ ] **[PAR surface-backward]** Implement Gate A in CLI help/parser, review planning, and server DTO behavior.
- [ ] **[PAR fixture-migration]** Update affected EIMP 01 tests; regenerate/promote signed fixtures only through CLI in scratch after review.
- [ ] **[JOIN 1C.5]** Run the exhaustive matrix at every surface and confirm no independent legal-pair list remains.
- [ ] **[SEQ 1C.6]** Run full `just`; commit: `EIMP-11 Phase 1C: make the stage graph authoritative`.

## Phase 2 — Specify and prototype atomic persistence

### Phase 2A — Freeze operation boundaries and conflicts (§S.4–S.6)

- [ ] Establish relevant tests for the pre-decision evidence sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `promote_cosigns_when_destination_content_matches`, `retract_output_cascades_through_checked_and_verified`, `eimp01_generate_promote_comprehensive`, `comprehensive_multi_reviewer_end_to_end`.
- [ ] **[SEQ 2A.1]** Inventory every call path that mutates artifacts; assign each to exactly one §S.5 transaction boundary and record current partial-failure behavior.
  - [ ] Promotion and co-sign: library, CLI, review execution, HTTP execution.
  - [ ] Flag and note promotion: origin removal, destination creation, advisory accumulation.
  - [ ] Retract: output/checked/verified cascades and absent members.
  - [ ] Batch promotion: filters/file lists, per-case failure, corpus signature update.
  - [ ] Generation: prune, crash crumbs, per-case writes, successful-view publication.
- [ ] **[PAR operation-table]** Convert §S.5 into a table of read dependencies, writes/removes, visibility boundary, idempotent result, and conflict classes for each operation.
- [ ] **[PAR review-brief]** Validate and extend the first-pass all-or-nothing
  packet in the transitions/review workfile with newly discovered API, UX,
  retry, audit-journal, or recovery evidence; do not restart from an empty
  comparison.
- [ ] **[PAR durability-brief]** Populate
  [`EIMP-11.workfile.durability.md`](EIMP-11.workfile.durability.md) with the
  platform matrix and exact return/sync boundary needed to validate its
  narrowed process-crash-versus-OS-crash choice; omit power-loss scope unless a
  named first deployment requires it.
- [ ] **[PAR generation-brief]** Validate the transaction-semantics workfile's
  seven-day bounded incomplete-run proposal against catastrophe crumbs, disk
  growth, privacy, cleanup, and diagnostic requirements.
- [ ] **[JOIN 2A.2]** Cross-check the four artifacts for contradictions; ensure every proposed human choice has a concrete default, rejected alternatives, and consequences for the persistence API.
- [ ] **[SEQ 2A.3]** Commit the inventory, operation table, and three decision briefs without implementing undecided semantics.

## Gate B — Transaction policy routing through EIMP-A

- [x] **[HUMAN B.1] HUMAN DECISION RECORDED:** confirmed review execution is
  suite-wide all-or-nothing; partial execution is deferred and `execute_one`
  remains the explicit smaller boundary.
      (2026-08-27 14:17)
- [x] **[HUMAN B.2–B.3] ROUTING DECISION RECORDED:** EIMP-A presents crash
  durability and incomplete-generation retention in the context of the exact
  stage/signature claims, operational non-goals, and proposed EIMP-B/EIMP-C
  ownership. No direct isolated question is asked from these workfiles.
      (2026-08-27 15:02)
- [ ] Stop: do not enter Tranche C until EIMP-A records accepted dispositions
  for B.2–B.3 and assigns every Phase 2 task to EIMP 11 or one numbered child.

## Tranche C — Persistence API and prototypes (after Gate B)

### Phase 2A completion — Encode transaction-policy decisions

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `transaction_disjoint_changes_merge`, `transaction_same_destination_diverges`, `transaction_delete_modify_conflicts`, `transaction_membership_drift_conflicts`, `promote_cosigns_when_destination_content_matches`.
- [ ] **[SEQ 2A.4]** Write the backend-independent deterministic reference-state-machine tests first for every base/current/proposed row in §S.6.
- [ ] **[PAR conflict-model]** Define typed conflicts for same-location divergence, delete/modify, dependency drift, membership drift, invalid current artifacts, and unsupported atomicity.
- [ ] **[PAR cosign-model]** Specify semantic co-sign merging: verify current body/chain, prove identical attested body, append only the new signer to current state, and conflict if either body or dependency moved.
- [ ] **[PAR generation-model]** Specify generation publication and incomplete-run retention according to B.3; distinguish last successful revision from an incomplete diagnostic run.
- [ ] **[PAR review-model]** Encode B.1's review-plan boundary and retry/reporting semantics into the model.
- [ ] **[PAR durability-model]** Encode B.2's success boundary and required recovery outcome without choosing the filesystem representation yet.
- [ ] **[JOIN 2A.5]** Run the reference model against all conflict/idempotence tests; make the operation table, types, and model terminology identical.
- [ ] **[SEQ 2A.6]** Run the full `just` gate and commit: `EIMP-11 Phase 2A: freeze transaction and conflict semantics`.

### Phase 2B — Replace the persistence API (§S.7)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `einmo_directory_and_in_memory_storage_agree_on_the_contract`, `snapshot_reads_one_revision`, `commit_validates_all_dependencies_before_write`, `unsupported_atomicity_fails_before_mutation`, `transaction_already_applied_is_idempotent`.
- [ ] Write the new transactional storage contract suite first against a reference in-memory implementation.
- [ ] Introduce opaque `SuiteRevision`, artifact versions/observations, immutable snapshot access, explicit change sets, typed conflict reports, commit outcomes, recovery reports, and storage capabilities.
- [ ] Make invalid artifact observations distinct from absence throughout the API.
- [ ] Refactor `EinmoCase` to plan changes from a snapshot instead of directly mutating storage.
- [ ] Refactor `EinmoSuite` batch operations to construct one change set instead of committing inside a per-case loop.
- [ ] Refactor compare, integrity, review worklist, and corpus-manifest construction to consume one snapshot and report revision change instead of mixing reads.
- [ ] Preserve the in-memory backend as the executable reference semantics; test every contract case against it.
- [ ] Run the full `just` gate when Phase 2B completes.
- [ ] Commit: `EIMP-11 Phase 2B: transactional persistence API`.

### Phase 2C — Document the current filesystem limitation (§S.8)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `filesystem_capabilities_report_current_guarantees`, `unsupported_atomicity_fails_before_mutation`, `einmo_directory_write_creates_missing_parent_directories`.
- [ ] Before strengthening it, make `EinmoDirectory::capabilities` accurately report every guarantee the existing backend lacks.
- [ ] Add prominent API documentation to `EinmoDirectory`, transaction entry points, and temporary `DEVELOPING.md` persistence notes stating that direct path storage does not provide coherent multi-file transactions.
- [ ] Make review-plan, batch, cascade, flag-move, and generation-publication entry points refuse before mutation when the backend cannot provide their required boundary.
- [ ] Ensure single-file replacement uses same-directory temporary write, verification, flush policy, and rename, while explicitly documenting that this alone is not suite atomicity.
- [ ] Run the full `just` gate when Phase 2C completes.
- [ ] Commit: `EIMP-11 Phase 2C: make filesystem guarantees honest`.

### Phase 2D — Prototype the filesystem transaction protocol (§S.8)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `filesystem_transaction_old_or_new_never_mixed`, `filesystem_recovery_rolls_forward_prepared_commit`, `filesystem_recovery_discards_unprepared_commit`, `two_writers_from_one_revision_conflict`, `reader_never_observes_partial_retract`.
- [ ] Build the smallest throwaway prototypes needed to compare old-or-new
  commit and recovery while preserving ordinary stage paths as an explicit
  measured constraint. Include revision-directory/`CURRENT`, WAL/materialized
  paths, or an embedded database only when each remains a plausible minimal
  representation against the uniform evidence template in
  [`EIMP-11.workfile.persistence-prototypes.md`](EIMP-11.workfile.persistence-prototypes.md);
  do not wire a prototype into production before the comparison is recorded.
- [ ] Measure normal operation, 1/100/1000-case commits, disk amplification, recovery time, and platform constraints.
- [ ] Add deterministic fault injection after each prepare/publish/finalize step; assert recovery exposes exactly old or new state.

## Gate C — Filesystem transaction architecture

- [ ] **HUMAN DECISION REQUIRED:** present the completed persistence-prototypes
  workfile. Present only representations that satisfy the narrowed old-or-new
  contract and chosen crash floor. Ask which smallest representation to adopt
  and whether ordinary stage paths remain authoritative; do not privilege the
  withdrawn `CURRENT`/checkout first pass or add history/Git/CI/status scope.
- [ ] Stop: do not enter Tranche D until Gate C is answered.

## Tranche D — Backend and general hardening (after Gate C)

- [ ] Record the selected design and rejected prototypes against correctness, auditability, portability, and performance in that order; resolve the persistence-design Open Question.
- [ ] Update §S.8 from target shape to the chosen exact on-disk/recovery protocol before production implementation.
- [ ] Run the full `just` gate when Phase 2D completes.
- [ ] Commit: `EIMP-11 Phase 2D: select the filesystem transaction protocol`.

### Phase 2E — Implement filesystem transactions (§S.8)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `filesystem_transaction_old_or_new_never_mixed`, `filesystem_recovery_rolls_forward_prepared_commit`, `filesystem_recovery_discards_unprepared_commit`, `transactional_flag_never_duplicates_or_loses_artifact`, `transactional_retract_is_all_or_nothing`, `transactional_review_plan_is_all_or_nothing`, `transactional_generation_publishes_complete_revision`.
- [ ] Implement suite writer locking without check-then-write races; verify ownership/type/mode before reclaiming stale state.
- [ ] Implement staging, dependency validation, prepared record, atomic publication, durability operations, recovery, and safe garbage collection exactly as selected in Phase 2D.
- [ ] Run recovery before serving a snapshot; never expose prepared-but-unpublished or mixed materialized state.
- [ ] Upgrade filesystem capabilities only after every contract and fault-injection test passes.
- [ ] Migrate promote/co-sign, flag, retract, review execution, batch/corpus-signature update, and generation publication to atomic commits.
- [ ] Add multi-process integration tests, not only threads, for writer conflicts and reader visibility.
- [ ] Run scoped mutation tests on conflict checks and commit-state transitions.
- [ ] Run the full `just` gate when Phase 2E completes.
- [ ] Commit: `EIMP-11 Phase 2E: transactional filesystem backend`.

## Phase 3 — Fail-closed inputs and reliable gates

### Phase 3A — Strict configuration (§S.9)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `config_precedence`, `malformed_existing_config_is_error`, `unknown_config_field_is_error`, `wrong_typed_signing_key_is_error`, `negative_limits_are_rejected`, `missing_config_uses_defaults`.
- [ ] Write table-driven failure tests first for unreadable, malformed, unknown, duplicate, wrong-typed, negative, and overflowing values with exact path/field diagnostics.
- [ ] Replace `parse_einmo_toml`'s default-on-error behavior with typed serde parsing, unknown-field denial, validation, and `Result` propagation.
- [ ] Audit every `TestConfig::new`/config load entry point so no CLI, library, server, signing, or verification path bypasses the fallible load.
- [ ] Add a migration diagnostic for any previously accepted but invalid repository config; do not silently reinterpret it.
- [ ] Run the full `just` gate when Phase 3A completes.
- [ ] Commit: `EIMP-11 Phase 3A: fail closed on invalid configuration`.

### Phase 3B — CLI argument and exit-code integrity (§S.10)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `compare_require_match_exits_failure_before_positionals`, `compare_require_match_exits_failure_after_positionals`, `option_like_filename_requires_double_dash`, `promote_options_parse_before_and_after_files`, `verify_options_parse_before_and_after_files`.
- [ ] Write process-level clap/exit-code tests first for every subcommand with `[FILES]...`.
- [ ] Remove or narrow `trailing_var_arg`; preserve literal leading-dash filenames through explicit `--`.
- [ ] Verify JSON and prose modes return identical success/failure status for the same semantic result.
- [ ] Audit README examples for option placement and add a regression test derived from the exact previously failing command.
- [ ] Run the full `just` gate when Phase 3B completes.
- [ ] Commit: `EIMP-11 Phase 3B: make CLI gates impossible to neutralize by ordering`.

### Phase 3C — Parallel duration semantics (§S.11)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `suite_duration_limit_stops_serial_scheduling`, `suite_duration_limit_stops_parallel_scheduling`, `parallel_and_serial_agree`, `already_running_evaluations_finish_after_suite_deadline`.
- [ ] Write deterministic tests first with an injected clock and evaluator barriers; do not use wall-clock sleeps as the assertion mechanism.
- [ ] Implement a shared deadline checked before each parallel work claim, or reject the configuration combination if the current evaluator abstraction cannot meet §S.11.
- [ ] Make skipped/not-started cases explicit in `TestResults` and machine output.
- [ ] Remove the `KNOWN GAP` comment only after the behavior and limitation docs match tests.
- [ ] Run the full `just` gate when Phase 3C completes.
- [ ] Commit: `EIMP-11 Phase 3C: enforce suite deadlines in parallel mode`.

### Phase 3D — Automated repository gates (§S.12)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `javascript_tiers_generate_and_verify`, `python_suite_generates_and_verifies`, `eimp01_generate_promote_comprehensive`, `eimp01_output_gate_goes_red_on_divergence_and_green_after_promotion`.
- [ ] Reconcile with EIMP 9 task by task; migrate or complete its live requirements and record ownership in both plans before marking anything superseded.
- [ ] Fix workspace JUnit coverage/completeness, fail-fast reporting, `jia` mutation scoping, doctest execution, and required developer-tool installation/version checks.
- [ ] Add checked-in CI for fmt, workspace clippy, workspace nextest, doctests, MSRV/platform matrix, and dependency policy.
- [ ] Add scoped mutation jobs for `case`, `signature`/attestation, transaction conflict logic, parser, config, and CLI gate predicates.
- [ ] Make CI fail if the expected workspace test count/suites are absent; do not use a truncated JUnit file as green evidence.
- [ ] Run the full local equivalent of CI and archive machine-readable results.
- [ ] Commit: `EIMP-11 Phase 3D: enforce the engineering gates`.

## Phase 4 — Security and type hardening

### Phase 4A — Strict envelope/stamp grammar (§S.13)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `roundtrip_byte_exact_default_separator`, `metadata_whitespace_tamper_detected`, `parser_rejects_unknown_and_duplicate_fields`, `parser_rejects_empty_separator`, `parser_rejects_invalid_section_declaration`, `parser_rejects_structurally_invalid_stamp_chain`.
- [ ] Inventory every committed `.einmo` wire form before tightening; identify any required versioned compatibility rule.
- [ ] Write a table-driven invalid corpus first for every §S.13 rejection.
- [ ] Introduce wire DTO→validated-domain conversion; reject ambiguity before verification.
- [ ] Add property tests for serialize/parse/verify round trips and a fuzz target for arbitrary bytes, separators, metadata, section lists, and stamp JSON.
- [ ] Confirm raw-byte signature verification remains authoritative and no parse/serialize normalization is used as the signed message.
- [ ] Run scoped mutation tests on field/section/stamp-chain validation.
- [ ] Run the full `just` gate when Phase 4A completes.
- [ ] Commit: `EIMP-11 Phase 4A: strict envelope grammar`.

### Phase 4B — Lock poison and service behavior (§S.14)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `poisoned_verified_cache_rebuilds`, `poisoned_decision_book_returns_service_error`, `poisoned_exec_lock_fails_session_without_panicking`, `drop_never_panics_after_poison`.
- [ ] Classify each production mutex/rwlock by recover/rebuild/fail-session/shutdown behavior and write poison tests first.
- [ ] Replace production `.expect("... lock poisoned")` paths with the typed policy; handlers return `ApiError` and destructors remain infallible.
- [ ] Ensure journal poison cannot silently stop all later audit events; recover safely or emit one durable/session-visible failure.
- [ ] Run the full `just` gate when Phase 4B completes.
- [ ] Commit: `EIMP-11 Phase 4B: typed lock-poison behavior`.

### Phase 4C — Attestation messaging and typed passphrase errors (§S.15)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `test_calculate_passphrase_score`, `weak_nonempty_passphrase_is_not_reported_as_identity_proof`, `expected_reviewer_key_controls_verified_identity`, `computer_key_is_always_non_human`.
- [ ] Write messaging/API tests first that forbid claims of entropy, identity, or human presence from compression score alone.
- [ ] Inventory every score/error/report consumer and correct names or prose
  that imply entropy, identity, uniqueness, or human presence; retain the
  current API, calculation, blocking behavior, and dependencies in EIMP 11.

## Resolved Gate D — Compression-score UX

- [x] **HUMAN DECISION RECORDED:**
  [`EIMP-11.workfile.attestation-score.md`](EIMP-11.workfile.attestation-score.md).
  Retain the compression score, `Promoted::passphrase_score`, `einmo-tools`, and
  `zstd`; describe the metric only as a compression heuristic. A second
  password-quality measure is optional EIMP-E work and is not urgent.
      (2026-08-27 14:38)

## Tranche E — Type hardening and dependency evidence (Gate D resolved)

- [ ] Implement resolved Gate D only by correcting unsupported score claims;
  leave score calculation, blocking paths, report fields, `einmo-tools`, and
  `zstd` intact unless a later EIMP-E explicitly changes them.
- [ ] Update CLI/README/security docs to distinguish public stock keys, computer labels, expected reviewer keys, and passphrase quality.
- [ ] Run the full `just` gate when Phase 4C completes.
- [ ] Commit: `EIMP-11 Phase 4C: make attestation claims honest`.

### Phase 4D — Validated metadata (§S.16)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `metadata_roundtrip`, `metadata_constructor_rejects_invalid_timestamp`, `einmo_file_derives_section_declaration`, `metadata_cannot_declare_missing_or_duplicate_sections`.
- [ ] Write invalid-construction tests first, then make `Metadata` fields private and add checked constructors/accessors.
- [ ] Derive the serialized section declaration from actual ordered sections and make contradictory in-memory state unrepresentable.
- [ ] Add newtypes/validated types for timestamps, provenance, and references where they carry decisions.
- [ ] Migrate public consumers without exposing raw mutation escape hatches.
- [ ] Run the full `just` gate when Phase 4D completes.
- [ ] Commit: `EIMP-11 Phase 4D: validate metadata by construction`.

### Phase 4E — Dependency and published surface (§S.17)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `core_feature_builds_without_review_server`, `cli_feature_builds`, `review_server_feature_builds`, `post_quantum_corpus_feature_builds`, `javascript_tiers_generate_and_verify`, `python_suite_generates_and_verifies`.
- [ ] Measure dependency tree, clean build time, binary size, and audit surface for core-only, CLI, review-server, and corpus-signing use cases.

## Gate E — Published crate boundary

- [ ] **HUMAN DECISION REQUIRED:** present Decision E and the completed
  measurement table in
  [`EIMP-11.workfile.crate-and-document-boundaries.md`](EIMP-11.workfile.crate-and-document-boundaries.md).
  First pass proposes ratifying EIMP 4's core/server split rather than reopening
  feature-versus-split; ask whether measurements justify amending EIMP 4 and
  where corpus signing belongs before changing manifests or public layout.
- [ ] Stop: do not enter Tranche F until Gate E is answered.

## Tranche F — Maintainability and documentation preparation (after Gate E)

- [ ] Coordinate the chosen result with EIMP 4; update that specification/plan rather than creating a competing crate split.
- [ ] Add feature/build matrix tests for the accepted boundary and ensure published metadata accurately declares optional capabilities.
- [ ] Add automated advisory, license, duplicate-version, and source policy checks.
- [ ] Run the full `just` gate when Phase 4E completes.
- [ ] Commit: `EIMP-11 Phase 4E: reduce and police dependency surface`.

## Phase 5 — Maintainability and documentation reset

### Phase 5A — Decompose by responsibility (§S.18)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `parallel_and_serial_agree`, `crash_crumb_survives_stack_overflow`, `comprehensive_multi_reviewer_end_to_end`, `serve_uds_end_to_end`, `eimp01_generate_promote_comprehensive`.
- [ ] Freeze the public API with compile/doc tests before moving code.
- [ ] Extract runner evaluation/parallel/crash/integrity responsibilities in small commits; keep behavior tests green after each extraction.
- [ ] Extract review decision/plan/execution/cache responsibilities in small commits.
- [ ] Extract HTTP DTO/handler/auth/serve responsibilities in small commits.
- [ ] Consider wire submodules only if Phase 4A's validated-domain boundary benefits; do not split merely to reduce line counts.
- [ ] Remove obsolete duplicated helpers surfaced by extraction; do not introduce `utils`.
- [ ] Run the full `just` gate after each completed extraction sub-section.
- [ ] Commit at each stable module boundary with EIMP 11 attribution.

### Phase 5B — Replace current documentation (§S.19)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `documentation_stage_list_matches_stage_all`, `documentation_transition_list_matches_authoritative_graph`, `readme_cli_examples_parse`, `developer_gate_commands_exist`, `documentation_links_resolve`.
- [ ] Inventory `README.md`, `AGENTS.md`, `rust_instructions.md`, `zweimomo/README.md`, every EIMP/index entry, and public crate/module docs as current reference, active proposal, or historical record.
- [ ] Create `DEVELOPING.md` containing only current setup, architecture reading map, focused/full test workflow, persistence guarantees/limitations, and release gates plus concise “why” rationale.
- [ ] Create `docs/architecture.md` containing only the current stage/gate/signature/transaction/conflict/trust model and backend capability table plus concise “why” rationale.
- [ ] Reduce `README.md` to the current user model and workflows; remove conflicting developer/process/history material after it has a current home.
- [ ] Correct `src/lib.rs`, `src/stage.rs`, CLI help, and `zweimomo/README.md` to the four-stage/nested-flag model, authoritative transitions, nextest workflow, working-directory assumptions, and Python development-library requirement.
- [ ] Add generated/consistency documentation tests so stage lists, transitions, and command names cannot drift independently again.
- [ ] Apply the Markdown Last Updated protocol to every changed Markdown file.
- [ ] Run the full `just` gate when Phase 5B completes.
- [ ] Commit: `EIMP-11 Phase 5B: current-state documentation replacement`.

### Phase 5C — Deprecate or narrow old documents (§S.19)

- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: `documentation_links_resolve`, `eimp_index_matches_frontmatter`, `no_active_document_points_to_superseded_behavior`.
- [ ] Build a requirement-migration ledger for EIMP 8, EIMP 9, EIMP 01, and any other document that claims current behavior superseded by EIMP 11.
- [ ] Prepare a proposed disposition for each old document: remain active with narrowed scope; mark superseded with a forward link; retain as historical with a non-current banner; or remove only if it has no unique rationale or stable citations.

## Gate F — Old-document disposition

- [ ] **HUMAN DECISION REQUIRED:** present Decision F and the completed ledger
  in the crate/document boundary workfile. First pass proposes completing EIMP
  01 normally, retaining EIMP 9 until its tooling contract lands, and narrowing
  EIMP 8 P-item by P-item before possible supersession. Ask for approval before
  changing any old status, banner, or file.
- [ ] Stop: do not enter Tranche G until Gate F is answered.

## Tranche G — Deprecation and closure (after Gate F)

- [ ] Do not mark EIMP 8 superseded until all accepted unresolved P-items, including findings outside EIMP 11's original review scope, have explicit destinations or rejection rationales.
- [ ] Do not mark EIMP 9 superseded until the automated test-tooling contract is fully enforced and its remaining tasks have explicit destinations.
- [ ] Update frontmatter, plan cancellation/supersession markers, and `docs/eimp/INDEX.md` consistently with timestamped checkbox rules.
- [ ] Verify that active/current docs contain no normative link to a deprecated behavior description except an explicitly labeled historical reference.
- [ ] Run the full `just` gate when Phase 5C completes.
- [ ] Commit: `EIMP-11 Phase 5C: deprecate superseded documentation safely`.

## Phase 6 — EIMP 11 comprehensive verification and close

- [ ] Establish relevant tests for this phase. Use [these instructions](../../README.md#running-specific-tests) to run: `eimp11_transactional_hardening_comprehensive`, `eimp01_generate_promote_comprehensive`, `eimp01_output_gate_goes_red_on_divergence_and_green_after_promotion`, `comprehensive_multi_reviewer_end_to_end`, `crash_crumb_survives_stack_overflow`.
- [ ] Write and verify `eimp11_transactional_hardening_comprehensive` first against a scratch copy of a real `zweimomo` suite:
  - [ ] take one coherent suite snapshot and plan a multi-case review promotion;
  - [ ] concurrently add a valid co-signer to one unchanged destination and modify another destination's body;
  - [ ] assert semantic co-sign merge is possible but the divergent body conflicts and therefore the whole confirmed plan leaves the suite unchanged;
  - [ ] retry from a refreshed snapshot and inject a crash at the prepared transaction boundary;
  - [ ] recover and assert readers see exactly the old or new revision, never a mixed retract/promotion/generation state;
  - [ ] place a tampered destination in the refreshed suite and assert the transaction refuses it without changing its bytes;
  - [ ] produce human→computer and computer→human verified stamp orders and assert both fail identically;
  - [ ] run output, checked, and verified gates after recovery and assert each reports only its adjacent link;
  - [ ] invoke `compare --require-match` with the option before and after file positionals and assert identical nonzero status on divergence;
  - [ ] load malformed configuration and assert every CLI/library/server entry point fails closed.
- [ ] Run all property/fuzz seed corpora and every filesystem fault-injection point.
- [ ] Run formatting, workspace clippy with warnings denied, workspace nextest, doctests, MSRV/platform CI, dependency policy, coverage, and scoped mutation testing; record machine-readable results.
- [ ] Confirm every EIMP 11 Open Question is resolved or moved into a named follow-up EIMP with no implementation ambiguity left here.
- [ ] Confirm every EIMP 8/EIMP 9 requirement has an explicit active owner before changing their status.
- [ ] Update `EIMP-11.md` frontmatter to `status: complete` only after all required work is done.
- [ ] Update `docs/eimp/INDEX.md` status/current-scope text and all affected Markdown Last Updated sections.
- [ ] Commit: `EIMP-11 complete: transactional hardening and documentation reset`.

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded EIMP-A as the selected first child, merged Reviews 3–5
into its contextual integrity/ownership work, blocked later child creation and
Phase 2 allocation on its accepted map, and renumbered candidate references.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Added HUMAN REVIEW 3A and an executable extraction protocol for
lettered candidate EIMPs A–G; incorporated Reviews 4–8 by narrowing persistence
to operational coherence, retaining the compression score, adding the `cargo
einmo` distribution requirement, and recording conditional document approval.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded Reviews 1 and 2 with timestamps and executable ordering,
removed their obsolete hard stops, and narrowed Review 3/Gate B to the still-open
durability and incomplete-generation-retention choices while keeping transaction
history recovery-only.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Expanded HUMAN REVIEW 4 onward to require review of each proposed
subsystem's complete steady-state lifecycle and vocabulary, not only its local
policy choice and impact descriptors.
