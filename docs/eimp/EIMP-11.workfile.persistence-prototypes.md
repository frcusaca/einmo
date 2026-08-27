# EIMP 11 Workfile — Filesystem Persistence Prototypes

> **Design workfile, not normative specification.** This is the Gate C
> comparison record for EIMP 11 §S.8. It begins with a reasoned first pass and
> will receive prototype measurements and fault results before selection.

## Dependency position

Prototype implementation starts only after Gate B freezes transaction and
durability semantics and after the persistence API contract is stable enough
that every prototype runs the same tests. Prototype lanes may then run in
parallel and must join before Gate C. Production backend work waits for Gate C.

## Maintainer scope correction — simplify before prototyping

The immutable-revision/`CURRENT` design below is retained as one candidate, not
the accepted target. Its first pass expanded too far into history, backup,
checkout/export, and external ecosystem policy. The revised comparison asks for
the smallest representation that makes one EIMP-B operation old-or-new and
recoverable under the selected EIMP-C crash floor.

The prototype contract excludes historical revision browsing, Git repository
management, CI decisions, release policy, and status publication. It may retain
only the transaction metadata needed for dependency validation, idempotent
retry, recovery, active readers, and safe garbage collection. Ordinary stage
paths remain a product constraint to measure, not something declared
non-authoritative before evidence. Backup/restore and explicit export tooling
are deferred unless the selected representation makes them necessary for safe
migration.

## Original candidate vocabulary retained for comparison

This workfile proposes a persistence subsystem, not merely safer calls to the
existing directory writer. It gives the transaction and durability contracts a
physical representation with one answer to each question: Which revision is
authoritative? How does a reader pin it? Where does uncommitted work live? What
single event publishes it? How does recovery classify interrupted work? Which
files may be inspected, exported, backed up, or garbage-collected?

The vocabulary used below is:

- **revision:** immutable suite state described by a manifest and artifact
  versions;
- **`CURRENT`:** the single authoritative reference naming the published
  revision;
- **staging:** unreachable bytes prepared for a possible commit;
- **prepared transaction:** a durable record whose complete change set is
  staged, but whose publication/finalization may require recovery;
- **checkout/export view:** familiar stage paths materialized for inspection,
  Git, or another tool, but not accepted as authoritative mutation;
- **reachable:** referenced by `CURRENT` or another retained snapshot/receipt,
  and therefore ineligible for garbage collection.

## Original candidate steady state — open, mutate, publish, recover, maintain

Here is the proposed revision-directory subsystem from start to finish:

1. Opening `EinmoDirectory` reads and validates `CURRENT` once, opens the named
   immutable manifest, and returns a snapshot whose reads resolve through that
   manifest. It does not repeatedly enumerate mutable stage paths.
2. Read-only APIs and commands consume the pinned snapshot. An export command
   may materialize familiar stage paths and labels the revision represented.
3. A mutation creates one change set against the snapshot. A suite-local writer
   lock serializes publication while optimistic dependency validation detects
   stale decisions and permits safe disjoint work under §S.6.
4. The writer creates same-filesystem staging, writes the new immutable
   manifest/artifacts, verifies every envelope, and records transaction id plus
   requested durability.
5. After synchronization and prepare, one atomic replacement of `CURRENT`
   publishes the entire revision. No loop of legacy-path renames defines commit.
6. Commit returns under the durability contract. Checkout refresh and cleanup
   may follow because they are not authoritative visibility.
7. Recovery reads `CURRENT` and prepared records before serving snapshots.
   Published work is finalized; unreachable work is discarded or retained as
   explicitly diagnostic generation. Recovery never guesses from a partial
   checkout.
8. Backup pins one reachable revision and required metadata. Restore validates
   it and changes authority through the same publication discipline.
9. Garbage collection removes only revisions proven unreachable after recovery,
   retention, active-snapshot, receipt, and backup rules are satisfied.

Library users will notice revision-aware snapshots and inspection. Integrations
will notice explicit checkout/export and stale-view diagnostics. Developers will
notice a versioned on-disk protocol, publication state machine, recovery table,
migration tooling, and garbage-collection proof that must evolve together.

## Decision C — authoritative representation (first pass withdrawn pending simplification)

### Original first pass retained for comparison

First pass, propose immutable revision directories—or content-addressed blobs
with immutable manifests—published by one atomically replaced `CURRENT`
pointer, because §S.4 requires coherent snapshots, §S.5 has multi-file
boundaries, §S.6 already models opaque revisions, and §S.7 asks storage to open
a snapshot before it reads. One pointer is a comprehensible visibility event.

Staging and a prepared record live on the same filesystem. A writer locks,
validates dependencies, stages and verifies bytes, synchronizes to the requested
durability, marks prepared, replaces `CURRENT`, synchronizes its parent when
required, then finalizes or garbage-collects lazily. Recovery uses `CURRENT` as
the visibility boundary: published prepared work rolls forward; unpublished
prepared work is discarded unless the recorded protocol proves completion is
required. Readers never traverse staging.

### Original compatibility recommendation retained for comparison

First pass, propose that revision storage be authoritative and familiar
`generated/output/checked/verified` paths become read-only checkout/export
views, because updating many legacy paths cannot be one visibility event for
external readers. Direct edits to exported paths are unsupported. This is the
largest compatibility decision: if individually Git-trackable legacy paths are
a hard requirement, prototype scoring must weight it explicitly rather than
pretending both properties coexist.

The human choice is therefore not merely which internal implementation looks
cleanest; it is which representation remains authoritative. The first pass
chooses coherent revisions as authority and ordinary files as derived views. If
legacy Git-trackable paths must remain authoritative, that prototype must prove
how every einmo reader obtains old-or-new visibility while external processes
observe and modify those paths. Otherwise it does not satisfy the transaction
contract already specified.

## Alternatives

### Revision directories plus `CURRENT` — recommended first pass

Pros: direct coherent snapshots, simplest recovery reasoning and audit trail,
cheap immutable readers, fault matrix maps to one publication point. Cons: disk
amplification/GC, migration, and changed Git/directory inspection workflow.

### WAL over materialized stage paths

Pros: preserves the visible layout. Cons: a sequence of path replacements is
not jointly visible; coherent readers must overlay the WAL/manifest or lock and
retry, external readers see mixtures, and rollback/roll-forward state is more
complex. It tends to recreate pointer indirection less clearly.

### Embedded transactional database

Pros: mature ACID, concurrency, and recovery. Cons: harms ordinary signed-file
inspection and Git workflows, adds dependency/migration/export boundaries, and
weakens the product's directory-based identity. Its durability still depends on
documented sync mode and filesystem behavior.

## Question for the maintainer

> **@human:** After the narrowed prototypes demonstrate old-or-new recovery,
> which smallest representation should EIMP-C implement? State explicitly
> whether ordinary stage paths remain authoritative. Do not select immutable
> revisions/`CURRENT`, derived checkout paths, WAL, or a database merely from
> this original first-pass recommendation.

## Comparison record to complete

Each lane must provide the same evidence:

- exact on-disk layout, snapshot algorithm, commit state machine, and recovery
  table;
- 1/100/1000-case latency, disk amplification, recovery time, and GC cost;
- deterministic fault injection after every prepare/publish/finalize step;
- two-writer conflict and reader old-or-new tests;
- durability-platform results from the durability workfile;
- migration and rollback examples; add Git diff, export, or backup examples
  only when a surviving candidate changes those workflows;
- dependency/publication effects;
- weighted conclusion in correctness, auditability, portability, performance
  order.

## Impact descriptors

### Einmo Library User Experience changes

The storage trait remains snapshot/transaction based regardless of selected
representation. Directory-backed users may no longer treat mutable legacy stage
paths as the authoritative store. Inspection/export APIs must make the
authoritative revision and checkout freshness visible; backup and restore must
operate on a coherent revision rather than copy paths during mutation.

### Einmo Integration changes

Shell scripts, Git workflows, editors, and external tools that read or write
`generated/output/checked/verified` directly are affected most. Under the first
pass, reads from exported paths are convenience views and direct writes are
unsupported; integrations use einmo commands/APIs for mutation and an explicit
export/checkout refresh for conventional files. CI must test migration,
round-trip export, stale-checkout diagnostics, backup/restore, recovery, GC,
two-writer conflict, and every injected publication failure.

### Einmo Development changes

All prototypes implement the identical contract and durability matrix; a
prototype cannot improve its score by weakening semantics. Production work may
begin only after Gate C records authority, legacy-path, migration, and recovery
choices. On-disk format changes require versioning, forward/backward migration,
inspection tooling, and an abandoned-transaction/GC safety proof.

Developers must learn the distinction between authority and materialization,
publication and finalization, prepared and committed, reachable and merely
present, and recovery versus garbage collection. Required tests include layout
version migration, manifest validation, stale-checkout detection, reader pinning
during publication, every protocol fault point, recovery idempotence,
backup/restore, and GC retention of every live reference.

### Migration

The final ordered procedure depends on Gate C and must be completed here before
implementation. At minimum it will identify supported old layouts, stop
writers, verify every existing envelope, take a recoverable backup, migrate to
the selected representation, exercise normal CLI/library reads, document the
rollback boundary, and prove failure never destroys the old layout. Manifest
import, atomic pointer selection, and checkout/export generation apply only if
the selected minimal representation uses them.

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Updated candidate dependencies after EIMP-A became the integrity
and decomposition charter: EIMP-B owns operation semantics and EIMP-C owns the
eventual filesystem representation.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Withdrew the broad `CURRENT`/checkout recommendation as a decision,
narrowed prototypes to operational old-or-new recovery, and excluded history,
Git/CI/status policy, and unnecessary backup/export scope from first-backend
selection.


**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Defined the proposed persistence subsystem vocabulary and added a
start-to-finish operating model covering snapshot open, staging, atomic
publication, recovery, export, backup, and garbage collection.
