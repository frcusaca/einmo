# EIMP 11 Workfile — Transaction Semantics

> **Design workfile, not normative specification.** This file expands EIMP 11
> §§S.4–S.7. Normative operation boundaries and conflict rules remain in the
> specification.

## Dependency position

This workfile precedes persistence prototypes. It can be developed in parallel
with the durability workfile after mutation-path inventory. Gate B approves
review and generation policy; only then should the storage API and executable
reference model be frozen.

## Maintainer discussion — history is not transaction state

The maintainer did not approve this packet as a whole and questioned whether
immutable revisions and transaction receipts would create a second history
system beside the external Git-style repository. The preferred minimal model
is one signed provenance link: the destination attests to its immediate source
stage and source hash, including the source signer/signature material. Longer
history remains the external repository's responsibility.

The next iteration therefore separates three concepts that the first pass left
too easy to conflate:

- **artifact provenance** is the signed immediate predecessor link carried by
  the envelope: source stage, source hash, and source signer/signature material;
- **transaction state** is opaque concurrency and crash-recovery metadata for
  the current operation, not a user-visible version archive;
- **repository history** is external and remains the mechanism for retaining
  older completed suite states.

Revision identifiers, dependency observations, and receipts may be retained
only as long as required for conflict detection, idempotent retry, recovery,
and safe garbage collection. EIMP 11 must not promise browsing or restoring
historical revisions. Persistence prototypes must measure the minimum retained
transaction metadata and show that unreachable revisions are collected after
recovery. A future historical-restore facility requires its own EIMP.

## First-pass operation descriptor

First pass, propose that every logical operation carry the following fields,
because §S.4 distinguishes snapshot, isolation, commit, and durability while
§S.5 identifies boundaries larger than one file:

| Field | Purpose |
|---|---|
| opaque base revision | Names the current snapshot reviewed by the caller; it is not a historical-revision API |
| artifact observations | Strong versions of every source, destination, or basis read |
| membership observations | Version of any enumeration or input set used by the decision |
| intended changes | Explicit writes and removes; no hidden mutation loop |
| required capabilities | Snapshot, single/multi-case atomicity, recovery, durability |
| stable transaction id | Crash-safe retry and `AlreadyApplied` recognition |

The §S.5 boundaries become:

| Operation | Dependency/visibility boundary |
|---|---|
| promote/co-sign | one case, source plus destination, one destination update |
| flag | origin plus nested flagged destination, write and remove together |
| retract | one case and every downstream artifact removed by the cascade |
| confirmed review | every action in the confirmed plan |
| batch promotion | selected cases plus promised section/corpus signature |
| generation publication | discovered input set plus the entire new current generated view |
| read-only suite operation | one declared snapshot or `SnapshotChanged` |

The present implementation demonstrates why per-file atomicity is insufficient:
`src/case.rs:305` separates promotion reads from writes;
`src/case.rs:391` writes flagged then removes origin; `src/case.rs:435` loops
retract removals; `src/suite.rs:140,214,244` mutate suites case by case.

## First-pass conflicts and idempotence

First pass, propose optimistic serializable validation over the declared
dependency set rather than conflicting on every unrelated suite revision,
because §S.6 permits disjoint work but prohibits stale reviewed intent.

- changed source/basis or live decision identity conflicts the entire operation;
- write/write and delete/modify at the same location conflict;
- invalid current bytes are an invalid-artifact error, never merge material;
- membership drift conflicts only operations that depended on enumeration;
- disjoint changes merge only when dependency sets do not intersect;
- a known committed transaction id returns `AlreadyApplied`, even after later
  unrelated commits;
- equal proposed destination bytes alone are insufficient to bless a stale
  review when its other decision dependencies no longer hold.

### Co-sign exception

First pass, retain one semantic merge: if the current destination has the same
verified body and a valid chain with new independent stamps, re-read that
current chain and append only the new signer. Never text-merge envelopes or
rebuild from a stale planned destination. If the signer is already present over
the same body, return idempotent success. This follows §S.5.1, §S.6, and EIMP 1
§§S.4a–S.5; current equal-body append behavior is visible at
`src/case.rs:353-363` but lacks compare-and-commit protection.

## Decision B.3 — incomplete generation retention

The history discussion does not answer this separate diagnostic-retention
question. An incomplete generation was never a completed repository state and
is not part of the signed predecessor chain. Its proposed short retention is
solely for crash diagnosis, bounded cleanup, and recovery inspection.

### First pass

First pass, propose that failed generation never advance the generated-current
pointer and that the newest incomplete run be retained for seven days by
default, bounded by configurable count and age, because §S.5.6 requires a
complete prior-or-new view while `src/einmo_suite.rs:1146` deliberately creates
forensic crash crumbs.

The incomplete namespace is excluded from normal compare/list snapshots. It
records creation time, run status, and runner metadata with restrictive
permissions. Startup or the next run prunes expired, non-current,
non-transaction revisions; explicit inspect and prune commands make retention
visible. Immediate deletion loses diagnosis; indefinite retention risks disk
growth and disclosure of sensitive evaluator input/output.

### Question for the maintainer

> **@human:** Given that completed history remains in Git and transaction revisions are
> recovery-only, may EIMP 11 retain at most the newest *incomplete* generation
> for seven days by default, excluded from the current view, with configurable
> bounds and explicit inspect/prune? If not, should incomplete runs be removed
> immediately after recovery or retained only when explicitly requested?

## Reference-model deliverables

- deterministic base/current/proposed table for every §S.6 row;
- review-plan and generation state machines;
- stable transaction-receipt and retry examples;
- typed conflict taxonomy with case/location/dependency diagnostics;
- cross-backend contract tests defined independently of the test-only
  `InMemoryStorage` (`src/storage.rs`), so the fake is an implementation of the
  semantics rather than their accidental authority;
- input membership and bytes as observations even though inputs are unsigned.

## Impact descriptors

### Einmo Library User Experience changes

Storage consumers stop composing `list_ids`/`read`/`write` calls as unrelated
operations. They open an immutable snapshot, construct an explicit change set,
and receive `Committed`, `AlreadyApplied`, typed `Conflict`, invalid-artifact,
`SnapshotChanged`, or `UnsupportedAtomicity` outcomes. Batch methods guarantee
their documented operation boundary instead of returning after partial loops.
Opaque revisions and versions are comparison tokens, not paths or timestamps.

### Einmo Integration changes

Concurrent writers may now receive conflicts that require refresh/re-plan;
automation must not retry by blindly overwriting. Read-only commands either
report one revision or fail on change. Generation tooling gains separate current
and incomplete-run concepts plus inspect/prune behavior. CI adds the full
base/current/proposed matrix, two-writer tests, enumeration-drift tests,
same-body co-sign races, transaction-id retry, and old-or-new visibility tests.

### Einmo Development changes

Every new mutating feature must fill in the operation descriptor before code:
base, artifact dependencies, membership dependencies, changes, capability,
durability, and stable transaction id. The reference state machine is normative
over backend implementations; the in-memory fake must pass it but cannot define
semantics by accident. No call-site loop may claim batch atomicity.

### Migration

1. Introduce snapshot/change-set types alongside the old storage calls.
2. Adapt read-only consumers first, then single-case mutations, then batch and
   review/generation boundaries; capability checks refuse unsupported atomicity
   before each migrated operation mutates.
3. Update callers for typed conflict/idempotence outcomes and remove blind
   overwrite retries.
4. Migrate incomplete generated artifacts into the diagnostic namespace without
   changing the last successful current view.
5. Run the cross-backend reference contract after each boundary moves.
6. Remove legacy mutation methods only after repository-wide call-site search
   and compatibility tests prove no consumer remains; rollback uses the
   transitional API only while no new revision-only state has been committed.

## Last Updated

**Date**: 2026-08-27  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Incorporated Review 3 as a scope correction rather than approval:
external Git owns completed history, envelopes retain one signed predecessor
link, and transaction revisions are recovery/concurrency metadata only. Split
out the still-open incomplete-generation retention question.


**Date**: 2026-08-27  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Added an ordered transitional-storage migration covering consumer
sequencing, typed outcomes, incomplete generations, validation, legacy removal,
and rollback limits.
