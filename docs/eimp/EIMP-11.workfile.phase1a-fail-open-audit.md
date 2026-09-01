# EIMP-11 Phase 1A — Fail-open audit

This workfile classifies the error-discarding reads required by EIMP 11
§S.1 after the Phase 1A implementation.

## Artifact verification reads

| Location | Classification | Disposition |
|---|---|---|
| `EinmoCase::promote` destination | Prohibited state collapse | Replaced with `ReadState`; malformed and signature-invalid destinations return their typed error with case/stage context and remain byte-identical. |
| `EinmoCase::flag` nested destination | Prohibited state collapse | Replaced with `ReadState`; invalid flagged artifacts are neither replaced nor merged, and the source remains in place. |
| `promote_flag_to_note` destination | Prohibited unchecked overwrite | It now observes the destination through `ReadState` before writing and refuses invalid existing notes with their path. |
| `EinmoTestRunner::evaluate` existing `generated/` read | Fail-safe optional cache | Retained. `generated/` is the generation-owned, gitignored work artifact. An invalid prior work file contributes no reusable signature state and the current generation replaces it. It is not an accepted stage destination. |
| Serial `EinmoTestRunner::evaluate_all` existing `generated/` read | Fail-safe optional cache | Retained for the same generation-owned boundary. |
| Parallel `EinmoTestRunner::evaluate_all` existing `generated/` read | Fail-safe optional cache | Retained for the same generation-owned boundary. |

There are no remaining `verify_bytes(...).ok()` calls. The three remaining
`EinmoFile::from_file(...).ok()` calls are exactly the replaceable
`generated/` cache observations above.

## Default-on-error inventory

| Location | Classification | Owner |
|---|---|---|
| `parse_einmo_toml` crate-wide read/parse and per-suite read/parse | Prohibited state collapse, unrelated to artifact mutation | EIMP 11 §S.9 / Phase 3A. Configuration must become fallible; Phase 1A does not conceal or duplicate that work. |
| `TestConfig::new` absent optional signing values and ignored-crumb list | Intentional semantic defaults after configuration parsing | Retained. These defaults apply to absent fields, not parse/read failures; Phase 3A must preserve that distinction. |
| `EinmoTestRunner::write_generated` absent evaluator detail | Intentional optional-value default | Retained. `None` means there is no status detail. |
| Review-server SSE JSON serialization | Fail-open diagnostic serialization | Retained for Phase 4/service review. The current serializable event shape should be infallible, but an encoding failure becoming empty event data deserves typed handling rather than being folded into Phase 1A. |

Other `.ok()` uses are not envelope verification/default-on-error sites in
the §S.1 audit. They cover optional environment values, diagnostic Git
metadata, fallible probes, journal filtering, or conversion helpers. Their
own EIMP 8/EIMP 9/late-EIMP-11 owners remain unchanged.

## Mutation evidence

The scoped `cargo-mutants` run used the six destination tests and examined
the new classifier/error mapper plus the notes read guard. Both viable
NotFound comparison mutations were caught. Classifier replacement mutants
were unviable because `ReadState` deliberately has no default/collection
conversion that could manufacture absence. One mutation changing every notes
read error into absence survived but is observationally equivalent at this
API boundary: the subsequent write to the same unreadable target returns the
same typed I/O error without changing it. Two pre-existing configuration
merge mutants were also included by the tool but are outside Phase 1A and
remain owned by the configuration work.

The test-first run identifier after implementation was
`01700ad2-5f73-4f89-bff0-bffeea9caf53`: all eight focused old/new tests
passed.

## Last Updated

**Date**: 2026-09-01
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Classified every artifact verification `.ok()` and
default-on-error site required by Phase 1A, recorded ownership, and captured
focused mutation evidence.
