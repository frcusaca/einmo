# EIMP-11 Phase 0 — Baseline and finding traceability

This workfile records the before-state for EIMP 11 on `jia` at commit
`d5ad583`. It reconciles the critical-finding audit, prior-EIMP ownership,
and the public surface before Phase 1 changes behavior.

## Baseline

The focused existing tests passed. The planned
`verified_level_rejects_empty_passphrase_key` test did not yet exist and is
retained as a Phase 1B test-first target. The complete `just` gate passed all
424 nextest tests, formatting, and workspace clippy with warnings denied;
workspace doctests also passed. The nextest run identifier was
`769d69f4-9c95-4073-8965-80a7089f458f`.

## Critical finding audit

| Finding | Current `jia` evidence | State | Test-first regression names |
|---|---|---|---|
| Invalid promotion destination becomes absence | `EinmoCase::promote` in `src/case.rs` reads the destination and applies `verify_bytes(&bytes).ok()`. A malformed or signature-invalid artifact therefore reaches the fresh-write branch. | Confirmed; not fixed | `promote_refuses_malformed_destination`, `promote_refuses_tampered_destination` |
| Invalid flagged destination becomes absence | `EinmoCase::flag` in `src/case.rs` applies the same collapse before advisory accumulation. Invalid bytes are overwritten by a new flagged artifact. | Confirmed; not fixed | `flag_refuses_malformed_flagged_destination`, `flag_refuses_tampered_flagged_destination` |
| Invalid note destination is overwritten | `promote_flag_to_note` in `src/transitions.rs` verifies the flagged source but writes the notes destination directly without observing an existing artifact. | Confirmed; not fixed | `promote_flag_to_note_refuses_malformed_destination`, `promote_flag_to_note_refuses_tampered_destination` |
| Verified policy examines one stamp | `EinmoTestRunner::attestation_problems` in `src/einmo_suite.rs` uses `find` for `stage:verified`, then judges only that stamp. It also treats zero verified stamps as already covered elsewhere. | Confirmed; not fixed | `verified_level_rejects_empty_passphrase_key`, `verified_attestation_human_then_computer_fails`, `verified_attestation_computer_then_human_fails`, `verified_attestation_expected_reviewer_among_cosigners_passes`, `verified_attestation_missing_expected_reviewer_fails` |
| Forward shortcut remains legal | `is_legal_transition` in `src/transitions.rs` permits `output -> verified`; `EinmoReview::source_stage_for_promote` independently selects `output` as a verified source candidate. CLI help advertises the shortcut. | Confirmed; not fixed | `forward_transition_matrix_is_exact`, `output_to_verified_is_refused_by_library_cli_and_review` |
| Backward edge remains legal | `is_legal_transition` permits `verified -> checked`, and CLI help advertises it even though review planning does not express it. | Confirmed; Gate A resolved removal, implementation pending after Phase 1B | `legal_transition_matrix_is_exact`, `verified_to_checked_matches_recorded_semantics` |

The `.ok()` reads of an existing `generated/` artifact in
`EinmoTestRunner::evaluate`, `evaluate_all`, and the parallel evaluator are a
different boundary: `generated/` is an uncommitted work file that generation
itself replaces. Phase 1A will retain them only with an explicit optional-cache
classification. The silent configuration reads in `parse_einmo_toml` are not
artifact reads and remain prohibited default-on-error behavior owned by
§S.9/Phase 3A.

## Prior-EIMP ownership ledger

| EIMP 11 area | Existing owner and status | Overlap and migration constraint |
|---|---|---|
| §§S.1–S.3 urgent integrity | EIMP 11, Implementing | These are newly confirmed critical review findings. EIMP 11 owns the code and regression tests until an accepted, integrated handoff says otherwise. Unmerged child-EIMP branches are not authority on `jia`. |
| §S.12 automated gates | EIMP 9, begun but incomplete | EIMP 9 retains T1–T6 and T11–T12, including `jia` mutation scope, workspace CI, fail-at-end output, timeout headroom, toolchain policy, bootstrap consolidation, and mutant artifacts. EIMP 11 contributes only its scoped tests and evidence. |
| §S.14 lock poison/test isolation | EIMP 9 T9 for test-only environment locks; EIMP 8 P1/P39 record related analysis | EIMP 9 keeps the test-lock cleanup. EIMP 11 Phase 4B owns production/service poison semantics only; rejected EIMP 8 P1 is not revived. |
| Review/server security and lifecycle | EIMP 8 Draft, accepted findings unresolved | EIMP 8 retains P26 stored XSS, P27 stale-plan execution, P28 arbitrary suite selection, P29 replay duplication, P30 token exposure, P32 destructive socket handling, P34 lock race, P35 shared-temporary hardening, and the accepted robustness findings. Phase 1 must not opportunistically absorb or close them. |
| Review performance and robustness | EIMP 8 Draft, accepted findings unresolved | P2, P5, P7–P9, P12, P31, P33, P36, P41 and accepted low-severity findings remain EIMP 8 work unless a later timestamped ledger moves each requirement. |
| Rejected/adjudicated review findings | EIMP 8 | P0's blocker classification and P1/P16/P23 as stated remain rejected; EIMP 11 does not reintroduce them. |
| Published crate boundary | EIMP 4 | EIMP 11 §S.17 supplies evidence but does not take ownership of distribution and `cargo einmo` management. |
| Generation and adjacent-gate claims | EIMP 01 | EIMP 11 tightens enforcement without changing EIMP 01's ownership of the four-stage model and one-link gate semantics. |
| Review product surface | EIMP 1 | EIMP 11 supplies shared integrity policy; EIMP 1 remains owner of the review product. |

No EIMP 8 or EIMP 9 requirement is closed by this audit. Supersession requires
a later item-by-item, timestamped migration or rejection.

## Public before-state

### Library

`lib.rs` publicly exposes the envelope model and verification functions;
configuration and key resolution; `EinmoCase`, `EinmoSuite`, and their
promotion/flag/retract reports; the byte-oriented `EinmoStorage` trait and
filesystem `EinmoDirectory`; comparison, generation, integrity, corpus
signing, review, journal, server, and suite-lock types. In particular:

- `EinmoStorage` offers independent `read`, `write`, `remove`, and `list_ids`
  calls. It has no snapshot, transaction, version, conflict, or capability
  abstraction.
- `EinmoDirectory::write` creates parents and calls `std::fs::write`
  directly. It promises neither atomic replacement nor durability.
- `Metadata` remains publicly constructible with public fields, and envelope
  parsing/serialization expose the current validation boundary.

### CLI and exit codes

The public commands are `promote`, `flag`, `retract`, `compare`, `verify`,
`verify-signatures`, `confirm-signatures`, `show`, `list`, `body`,
`self-check`, and `generate` (`evaluate` alias). Clap parse/help failures use
exit code 2; operational errors and failed required predicates use exit code
1; successful operations use 0. Several commands accept trailing positional
files, and Phase 3B retains responsibility for option/positional ambiguity.

Promotion help currently advertises five legal pairs: the three adjacent
forward edges plus `output -> verified` and `verified -> checked`. Core
validation and review planning do not share one declarative representation.

### Configuration and filesystem capability

`parse_einmo_toml` returns a non-fallible configuration. Crate-wide read or
parse errors become defaults, while per-suite read or parse errors are
skipped. This is the Phase 3A before-state. The only shipped persistence
backend is `EinmoDirectory`; its independent operations can expose partial
multi-artifact mutations and cannot advertise stronger semantics.

## Reconciliation

Phase 1 ownership is therefore singular and executable:

1. Phase 1A owns invalid-existing-artifact refusal for promotion, flag
   accumulation, and note publication, while preserving the separate
   replaceable-`generated/` cache classification.
2. Phase 1B owns one order-independent policy over every verified stamp and
   the diagnostics consumed by all gate/review surfaces.
3. Phase 1C owns removal of both non-adjacent graph edges and consolidation
   of library, CLI, review, server, tests, and documentation onto the same
   authoritative graph, with Gate A applied only after Phase 1B.
4. EIMP 9 keeps tooling-contract work; EIMP 8 keeps every accepted review
   finding listed above. Later phases must use an explicit handoff rather
   than silently duplicating or dropping either backlog.

## Last Updated

**Date**: 2026-09-01
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded the EIMP-11 Phase 0 baseline, critical-finding re-audit,
prior-EIMP ownership ledger, public before-state, and reconciled Phase 1
ownership.
