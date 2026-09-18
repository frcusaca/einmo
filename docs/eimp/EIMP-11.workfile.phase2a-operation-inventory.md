# EIMP 11 — Phase 2A operation inventory

## Scope and accepted decisions

@agent: This is evidence for the transaction handoff, audited against `jia`
at `8008457`. It does not implement transactions or select a filesystem
representation. The newly merged extra-repository assurance EIMP is separate.

@human: The earlier transaction Gate A approval selected option 3 and its
proposed bounds.

@agent: The accepted contract is preserved in commit `f74feb6` on
`eimp-41-transaction-semantics`: delete incomplete generations by default;
explicit retention permits at most one private, read-only diagnostic per suite
for seven days, excluded from snapshots and gates, with startup pruning and
backend refusal when unsupported. The old branch numbers are historical
identifiers here: `jia` now assigns EIMP 31 to extra-repository assurance.
The charter and child handoffs must be reconciled before importing their files.
No repeated human approval of retention or ownership is needed.

## Mutation boundaries

| Entry point | Reads and dependencies | Writes/removes today | Partial failure today | Required boundary |
|---|---|---|---|---|
| `EinmoCase::promote` (`src/case.rs`) | Verified source; absent or verified destination; body equality; existing destination signers | Replaces destination with promoted or co-signed bytes | No version check between reads and write; a concurrent signer or changed source can be missed | Source and destination observations plus one destination write; semantic co-sign must append to the current verified chain |
| `EinmoCase::flag` | Source envelope and existing flagged destination/advisory | Writes flagged destination, then removes source | A failed removal leaves both locations; concurrent advisory accumulation can be lost | One write-and-remove commit with both locations observed |
| `EinmoCase::retract` | Presence at every stage in the cascade | Removes highest stage first, one location at a time | Later failure leaves a partial cascade; presence checks race with writers | Entire cascade, including observed absence, in one commit |
| `promote_flag_to_note` (`src/transitions.rs`) | Selected flagged membership, advisory source, existing note validity | Writes each signed note directly; retains flagged source | Earlier notes survive a later error; source/destination can change after inspection | Declare note publication scope explicitly; source remains a read dependency, not a removal |
| `EinmoSuite::promote` (`src/suite.rs`), CLI `cmd_promote` | Selected case membership, each source/destination; verified corpus bytes for score policy | Calls case promotion sequentially | Case N failure leaves earlier promotions applied | Selected batch in one commit; include corpus signature only when that operation promises one |
| `EinmoSuite::flag` / `retract`, CLI wrappers | Selected membership and each operation's dependencies | Sequential case mutations | Earlier cases survive a later error | Preserve each declared flag/cascade boundary; freeze batch scope in the operation descriptor |
| `EinmoReview::execute`, HTTP execute, review-server console | Live decision basis, selected actions, source presence, stage keys; verified corpus for score policy | Groups promotions, then runs retract/flag actions; clears executed and skipped decisions afterward | Action errors become skipped entries while earlier actions remain applied; local mutex is not cross-process isolation | Confirmed plan is suite-wide all-or-nothing, with decision identity and basis dependencies; clear decisions only after a commit receipt |
| `execute_one`, `flag_now`, `retract_now` | One decision or immediate action and its artifact dependencies | Delegates to review/case mutation paths and updates session bookkeeping | Immediate execution does not establish storage atomicity | Explicit smaller transaction using the same machinery |
| `evaluate_all` (`src/einmo_suite.rs`), generate/output-gate entry points | Input membership/bytes, dependency ordering, configuration, prior generated artifacts and crumbs | Prunes generated files, writes crumbs, then writes results individually | Failed/crashed run can leave pruned, old, and new artifacts together | Evaluate in a private run; publish successful generated view once; incomplete-run handling follows accepted option 3 |
| `evaluate` / `evaluate_inline` | Input or supplied text, configuration, existing generated result where applicable | Writes crumb followed by result | Crumb/result write is separate from successful run publication | Explicit single-case generation operation; do not accidentally claim suite publication |
| `update_corpus_signature` / `CorpusSigner::sign` / `sign_via_storage` | Stage membership and ordered artifact bytes; existing section signature | Writes `.section.sig` directly through filesystem | Membership/bytes may change during digesting; signature write has no shared artifact commit | Observe one snapshot; signature metadata needs a transactional address alongside case artifacts |

All rows currently lack durable transaction identity and commit receipts.
`AlreadySigned` is an artifact-level no-op, not proof that a whole earlier
transaction committed. Read-only compare, integrity, worklist, and manifest
construction must also move to declared snapshots.

## Storage implementation check

`EinmoDirectory::write` creates parent directories and calls `std::fs::write`;
`remove` calls `std::fs::remove_file`, treating absence as success. Neither
operation has a transaction revision, dependency validation, or recovery
record. `InMemoryStorage` locks its map separately for each read, write,
remove, and enumeration: individual map access is synchronized, but a domain
operation spanning several accesses is not atomic. The reference transaction
model must therefore strengthen both backends' contracts, not only the disk
implementation.

## Resumption baseline — 2026-09-15

The tree remains at `8008457`; the earlier baseline process is no longer
available, so its completion cannot be asserted. A fresh `just` run passed
formatting and stopped during clippy dependency resolution because
`index.crates.io` could not resolve. An offline retry failed because `fips205`
is missing from the active registry index. A search of preserved caches under
`/persist`, `/yolo`, and `/tmp` found no replacement package/cache entry.

The exact nextest 0.9.88 executable remains available at
`/yolo/target/release/cargo-nextest`. No package was installed. No clippy or
test success is recorded for this resumption, and no implementation checkbox
is completed. Restore registry access or the locked dependency cache, rerun
`just`, and run workspace doctests separately before transaction changes.

## Remaining evidence before Phase 2A completion

Update 2026-09-18: the maintainer's external build restored the dependencies.
The subsequent offline run completed all 445 tests: 438 passed and seven
socket-binding tests failed under sandbox restrictions. The unrelated
decision-replacement fixture regression was repaired and passes. See the
[Phase 1C verification record](EIMP-11.workfile.phase1c-transition-graph.md#regression-repair-verification--2026-09-16)
for the run ID and failures. Workspace doctest checks also succeeded; the
unrestricted full baseline remains pending.

- Confirm the complete mutation call graph, including storage implementations
  and every generation entry point, against this inventory.
- Resolve note and multi-case flag/retract selection scope in operation
  descriptors without broadening the accepted review/batch contract.
- Reconcile the accepted charter and child plans with the current numbering.
- Populate platform/crash-floor evidence and validate retention cleanup,
  privacy, and disk-growth requirements; filesystem architecture remains open.
- Record a completed baseline and focused test results before marking any
  Phase 2A implementation or handoff checkbox complete.

## Last Updated

**Date**: 2026-09-18
**Updated By**: OpenAI Codex (GPT-6)
**Changes**: Recorded restored dependency availability and linked the actual
completed test run and outstanding sandbox verification limitation.

**Date**: 2026-09-15
**Updated By**: OpenAI Codex (GPT-6)
**Changes**: Audited storage mutation primitives and recorded the reproducible
dependency-cache/network blocker without claiming a completed baseline.

**Date**: 2026-09-07
**Updated By**: OpenAI Codex (GPT-6)
**Changes**: Began the Phase 2A operation inventory from the merged tree,
preserved the accepted retention decision by commit reference, and identified
remaining handoff and transaction-boundary evidence.
