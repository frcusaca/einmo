---
eimp: D11
title: Reliability hardening, transactional persistence, and documentation reset
author: OpenAI Codex (GPT-5) <noreply@openai.com>
status: Draft
type: Standards
created: 2026-08-26
supersedes: []
begun: [ ]
---

# EIMP-11: Reliability hardening, transactional persistence, and documentation reset

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root — **read it before creating or editing an EIMP.**

## Abstract

EIMP 11 turns the current code-review findings into one priority-ordered
hardening project. It closes three critical integrity gaps, defines which
einmo operations must be atomic and what conflict means, replaces the
storage API with coherent snapshot and transactional commit semantics,
makes the filesystem backend honest about and eventually capable of those
semantics, makes configuration and parsing fail closed, repairs CLI and
runtime reliability, restores enforceable engineering gates, and replaces
stale documentation with a concise account of einmo's current model and why
it exists. Work proceeds from critical integrity defects through lower-risk
maintainability improvements; no lower tier begins while a higher tier is
known broken.

## Motivation

The current implementation has strong local building blocks: signed
envelopes, raw-byte verification, typed stages and case identifiers, a
storage abstraction, an uncommitted generation stage, and extensive tests.
The review at commit `9c8589a` nevertheless found places where the behavior
does not uphold its own stated invariants:

- promotion silently treats a corrupt destination as absent and overwrites
  it;
- verified-attestation policy inspects only the first verified stamp;
- the transition table allows `output → verified`, bypassing the checked
  claim while the verified gate still judges `checked ↔ verified`;
- configuration failures are swallowed and replaced by defaults;
- filesystem writes overwrite committed signed state in place;
- a trailing CLI option can be consumed as a filename, turning a requested
  failing comparison into exit status zero;
- the default parallel runner ignores the suite duration limit;
- the documented development gates are not fully enforced by a working CI
  contract;
- parser strictness, lock-poison behavior, passphrase messaging, public
  metadata invariants, module size, dependency surface, and documentation
  all need tightening.

The persistence issue is larger than changing `std::fs::write` to
temporary-file-plus-rename. A rename can make one file replacement atomic,
but an einmo action often has a larger logical boundary:

- promotion reads a source and a possible destination, then writes a new
  destination;
- flagging writes a flagged artifact and removes its origin;
- retracting output may remove output, checked, and verified together;
- one review execution applies a confirmed set of decisions;
- one batch promotion can modify many cases and a corpus signature;
- generation owns a complete `generated/` view corresponding to one input
  discovery/evaluation pass.

If those operations partially apply, observers can see claims that never
formed a coherent state. If two writers start from the same suite view, a
last-writer-wins overwrite can discard signatures, reintroduce retracted
artifacts, or attest to content different from the content reviewed.

EIMP 11 therefore specifies atomicity at the domain-operation level first,
then requires the persistence API and its implementations to model that
contract. The filesystem implementation must not imply guarantees it does
not provide. Until it implements a recoverable suite transaction protocol,
it must expose its capability and refuse operations whose promised boundary
is stronger than its actual behavior.

The documentation also needs a reset. The repository contains valuable
historical EIMPs, but public and contributor-facing descriptions disagree
about the number of stages, whether `flagged` is a stage, legal promotions,
test commands, and what `zweimomo` requires. The replacement documentation
specified here records the current state, its invariants, and why einmo is
shaped this way. It does not reproduce project-process history. Old documents
that claim obsolete behavior are marked superseded/deprecated or moved into
an explicitly historical role after their still-live requirements have been
migrated.

## Impact Overview

### Einmo Library User Experience changes

The public storage model changes from independent path operations to immutable
suite snapshots, explicit change sets, typed conflicts, commit outcomes,
capabilities, and recovery. Existing promotion signatures may remain stable,
but `output → verified` and `verified → checked` become
`IllegalTransition`; callers use adjacent promotions and retraction instead.
Confirmed review execution changes from best-effort partial progress to one
atomic outcome. Metadata becomes validated/private, configuration becomes
fallible, parsing becomes strict, and compression-score claims are narrowed to
the metric's actual heuristic meaning. Crate imports may move according to EIMP
4's core/server split.

### Einmo Integration changes

CLI scripts lose the two non-adjacent/backward promotion pairs and must handle
typed conflicts, strict configuration, corrected option parsing, and atomic
review/batch behavior. Direct filesystem mutation becomes unsupported when the
transactional backend is authoritative; legacy stage paths may become checkout
or export views. HTTP/TUI review flows, journals, JSON/report consumers,
`zweimomo`, signed fixtures, current documentation, packaging, and release
automation change accordingly. CI gains exhaustive transition-surface tests,
storage contract/concurrency/fault/recovery tests, platform durability jobs,
strict parser/config tests, documentation consistency, package-boundary checks,
dependency policy, and comprehensive/mutation coverage.

### Einmo Development changes

Developers must become familiar with domain transaction boundaries, immutable
snapshots, declared artifact and membership dependencies, optimistic conflicts,
transaction receipts, same-body co-sign merging, durability levels, recovery
state machines, and the distinction between authoritative revisions and exported
stage paths. Every mutating feature must declare its operation descriptor before
implementation and use the shared transaction path rather than a write loop.
Affected test subsets are established per plan subsection; unfamiliar backend
protocol steps receive deterministic fault injection, cross-backend reference
tests, platform support evidence, and recovery tests. Transition facts,
documentation facts, security claims, crate ownership, and historical-document
dispositions gain explicit authorities and consistency checks.

### Migration

Migration is required and is staged so unsupported atomicity fails before
mutation during transition:

1. Remove callers and automation using `output → verified` or
   `verified → checked`; replace them with adjacent promotions or
   retract-verified and regenerate signed fixtures only through the CLI.
2. Introduce snapshot/change-set/typed-outcome APIs alongside legacy storage;
   migrate read-only, single-case, then batch/review/generation consumers and
   remove legacy mutations only after repository-wide validation.
3. After Gate C, verify and back up the old directory layout, migrate it through
   the selected minimal representation, validate every envelope/manifest,
   publish coherently, and retain a documented rollback boundary and
   compatibility window. Generate checkout/export views only if the chosen
   representation actually requires them.
4. Migrate review plans, journals, CLI/JSON consumers, and integrations to
   atomic outcomes and versioned records before removing legacy report fields.
5. Correct compression-score documentation so it makes no entropy, identity,
   uniqueness, or humanity claim; retain its current API/tooling unless EIMP-E
   later approves a replacement. Existing signed envelopes require no rewrite.
6. Apply EIMP 4's package migration in publish order, with temporary re-exports
   only when measured compatibility requires them.
7. Move live documentation requirements through the migration ledger before
   changing EIMP status, banners, links, or deleting any historical file.

Exact validation and rollback steps are maintained in the linked EIMP 11
workfiles and must be finalized at their human gates before incompatible work.

## Specification

### S.0 — Scope, priority, and completion rule

The implementation plan is normative about order:

1. **Critical integrity:** tampered destinations, complete verified-stamp
   policy, and the legal transition graph.
2. **Transactional integrity:** atomic boundaries, coherent reads, conflict
   rules, persistence API, and filesystem behavior.
3. **Fail-closed inputs and gate behavior:** configuration, CLI parsing,
   duration limits, and automated quality gates.
4. **Security and type hardening:** strict envelope parsing, lock-poison
   handling, passphrase semantics, metadata invariants, and dependency
   policy.
5. **Maintainability and documentation:** module decomposition and a
   current-state documentation replacement/deprecation pass.

A tier is complete only when its focused tests and the complete repository
gate pass. A lower tier may be researched while a higher tier is being
implemented, but lower-tier behavior does not land ahead of a known
higher-tier defect.

EIMP 11 does not automatically supersede EIMP 8 or EIMP 9 on creation. Its
documentation phase inventories their unresolved requirements. An older
EIMP is marked superseded only after every still-live requirement has either
been migrated into EIMP 11, assigned to another active EIMP, or explicitly
rejected with a reason. EIMP 9 remains authoritative for the test-tooling
contract until that audit is complete.

### S.1 — Tampered destinations are never absence

Every artifact read has one of three relevant states:

```rust
enum ReadState<T> {
    Absent,
    Verified(T),
    Invalid(ArtifactInvalidity),
}
```

`Invalid` includes malformed envelopes and failed signatures. It is not
convertible to `Absent` with `.ok()`, `unwrap_or_default`, or equivalent
logic.

Promotion, co-signing, flagging into an existing sink, note promotion, and
transaction conflict resolution must return an error when an existing
source or destination is invalid. They do not overwrite, delete, merge, or
quarantine it automatically. Explicit recovery is a separate operation and
is outside this EIMP unless implementation discovers that a minimal recovery
verb is required to test the refusal contract.

Diagnostics name the case, artifact location, and whether parsing or
signature verification failed without exposing secret material.

### S.2 — Verified attestation considers the complete stamp set

For each verified artifact, the verified gate evaluates all stamps whose key
is `stage:verified`.

The policy is:

- at least one verified stamp must exist;
- zero verified stamps may use the well-known computer key;
- when `reviewer_key_prefix` is configured, at least one verified stamp must
  match it;
- unexpected human co-signers do not by themselves invalidate an artifact,
  but their identities remain visible in reports;
- duplicate identical stamps do not change the verdict.

This policy is order-independent. Human→computer and computer→human chains
produce the same failure. Diagnostics identify every computer-key verified
stamp and state when the expected reviewer is absent.

### S.3 — One authoritative transition graph

The ordinary forward promotion graph is exactly:

```text
generated → output → checked → verified
```

`output → verified` is illegal. It cannot create the checked review claim
that the verified gate relies upon.

`verified → checked` is removed and retraction is the only backward operation,
because the current implementation leaves verified in
place and can append a checked stamp after a verified chain; the review planner
cannot express the edge, while `retract verified` already withdraws attestation
and preserves checked. See
[`EIMP-11.workfile.transitions-and-review.md`](EIMP-11.workfile.transitions-and-review.md)
for code evidence, alternatives, and the recorded Gate A disposition.

This accepted change lands only after §S.2's complete-stamp and
multi-signature semantics are implemented and tested. A human verifier may
also have supplied a checked signature while an agent supplied another; the
adjacent `output → checked → verified` lifecycle must preserve and evaluate
that valid multiplicity before the obsolete backward edge is removed.

One table/function is the authoritative source for:

- library validation;
- CLI help and parsing;
- review planning;
- server DTO validation;
- documentation;
- exhaustive transition tests.

No surface maintains an independent list of legal pairs.

### S.4 — Atomicity vocabulary

EIMP 11 uses the following terms precisely:

- **Coherent snapshot:** enumeration, artifact bytes, and version tokens are
  observed as one logical suite revision. A snapshot never combines an old
  case list with new bytes without reporting that it is weak/incoherent.
- **Atomic commit:** every change in a declared change set becomes visible
  together, or none does.
- **Isolation:** a commit validates that every artifact and suite fact on
  which its decisions depended still has the expected version.
- **Durability:** after successful commit, the backend's documented crash
  boundary preserves the new state.
- **Conflict:** current state differs from an expected version in a way that
  cannot be proven idempotent or disjoint.
- **Recovery:** after interruption, the backend deterministically completes
  or rolls back an incomplete commit before serving a new coherent snapshot.

Atomicity is about a logical operation, not merely an individual file write.
It preserves coherent application of signed approval claims; it is not itself
an approval, integrity proof, provenance root, or historical record.

### S.5 — Required operation boundaries

The following are normative transaction boundaries.

#### S.5.1 — Single-case promotion/co-sign

The read of source, read of destination, verification, content decision,
stamp append, and destination update are one transaction. If source or
destination changes after the snapshot, the commit conflicts. A matching
destination with the same final bytes is idempotent; a matching body with a
new independent signer may be merged only by re-reading the current verified
destination and appending the new stamp to that current chain.

#### S.5.2 — Flag

Writing `<stage>/flagged/<case>` and removing `<stage>/<case>` are one
transaction. Observers never see both as the committed result and never see
neither as the committed result. Re-flag advisory accumulation reads and
validates the existing flagged destination within the same transaction.

#### S.5.3 — Retract cascade

All artifacts removed by one retract cascade are one transaction. Retracting
output cannot leave checked or verified visible after output disappears.

#### S.5.4 — Confirmed review plan

A confirmed `ExecutionPlan` is one logical transaction. Every
decision's basis fingerprint, live decision identity, source, and destination
is validated against the plan's snapshot before any result becomes visible.
If any action conflicts or fails, none of the plan is committed.

Suite-wide all-or-nothing is the only EIMP 11 batch mode; any explicitly
previewed partial mode is deferred, because the confirmed
plan is a promise about exactly what will execute; `execute_one` already offers
a smaller chosen boundary. See the review workfile's Gate B.1 packet.

#### S.5.5 — Batch promotion and corpus signatures

A batch promotion selected by files/filter is one transaction, including its
corpus-signature update when the command promises the signature represents
the resulting batch. Failure in case N does not leave cases 1..N-1 promoted.

#### S.5.6 — Generation publication

Evaluation itself may run incrementally and may leave signed crash crumbs.
Publication of a successful generation run as the current `generated/` view
is atomic at suite scope: readers see the prior complete generation or the
new complete generation. A failed/crashed run remains identifiable as an
incomplete run and is not confused with the last successful generated view.

This requirement may be implemented with revisioned generations plus an
atomic current-manifest pointer rather than renaming an entire directory.

#### S.5.7 — Read-only operations

`compare`, integrity gates, list, review worklist construction, and corpus
manifest construction consume one declared snapshot. They either return a
result for that snapshot or report `SnapshotChanged`; they do not silently
mix revisions.

### S.6 — Version and conflict model

Every snapshot exposes an opaque `SuiteRevision` plus a version for each
artifact read. Filesystem implementations may derive versions from strong
content hashes and an internal revision manifest; metadata timestamps alone
are insufficient.

These revisions are concurrency and recovery tokens, not a second historical
repository. The authoritative artifact-level provenance is one signed link to
the immediate source envelope: source stage and hash together with its
signer/signature material. Completed long-term history remains in the external
Git-style repository. Transaction metadata and unreachable physical revisions
are retained only for conflict detection, retry, recovery, and safe garbage
collection; EIMP 11 exposes no revision-browsing or historical-restore API.

A transaction records:

```rust
pub struct SuiteSnapshot { /* opaque revision + verified observations */ }
pub struct ChangeSet { /* reads depended upon + writes/removes requested */ }
pub enum CommitOutcome {
    Committed { revision: SuiteRevision },
    AlreadyApplied { revision: SuiteRevision },
    Conflict(ConflictReport),
}
```

Conflict resolution follows git-like three-way reasoning over base,
current, and proposed state, but it never merges opaque signed bytes by text:

| Base/current/proposed relation | Result |
|---|---|
| current equals base | apply proposed |
| current already equals proposed | idempotent success |
| changes touch disjoint artifact locations and neither depends on the other | merge allowed |
| same destination changed to different body/content | conflict |
| delete versus modify of the same artifact | conflict |
| source/basis changed after review | conflict |
| destination gained valid independent stamps over identical body | re-read and semantic co-sign merge may be attempted |
| any current artifact is malformed or fails verification | hard invalid-artifact error, never a merge |
| suite membership/input set changed and the operation depended on enumeration | conflict |

Conflict reports name affected case/location and classify the conflict. They
do not silently choose “ours” or “theirs.” The caller must refresh/re-plan or
explicitly abandon its change.

**First pass:** use optimistic serializable validation over declared artifact
and membership dependencies, not blanket rejection after any suite revision
change. Stable transaction receipts establish retry idempotence; same-body
co-sign accumulation is the sole semantic merge. See
[`EIMP-11.workfile.transaction-semantics.md`](EIMP-11.workfile.transaction-semantics.md).

### S.7 — Persistence API

`EinmoStorage` is replaced or extended so coherent reads and transactions are
part of the type-level contract rather than conventions at call sites. Exact
names may change during implementation, but the API must express this shape:

```rust
pub trait EinmoStorage {
    type Snapshot<'a>: EinmoSnapshot
    where
        Self: 'a;

    fn capabilities(&self) -> StorageCapabilities;
    fn snapshot(&self) -> Result<Self::Snapshot<'_>, EinmoError>;
    fn commit(
        &self,
        base: &Self::Snapshot<'_>,
        changes: ChangeSet,
    ) -> Result<CommitOutcome, EinmoError>;
    fn recover(&self) -> Result<RecoveryReport, EinmoError>;
}

pub trait EinmoSnapshot {
    fn revision(&self) -> &SuiteRevision;
    fn read(&self, id: &EinmoId, at: ArtifactLocation)
        -> Result<ArtifactObservation, EinmoError>;
    fn list_ids(&self, at: ArtifactLocation)
        -> Result<&[EinmoId], EinmoError>;
}

pub struct StorageCapabilities {
    pub coherent_snapshots: bool,
    pub atomic_single_case: bool,
    pub atomic_multi_case: bool,
    pub crash_recovery: bool,
}
```

Requirements:

- a caller cannot mutate storage through a snapshot;
- change sets contain explicit writes/removes and their expected versions;
- source/basis observations used for a decision are registered dependencies;
- commits validate dependencies before visibility;
- batch callers build one change set, not a loop of independently committed
  case writes;
- backends that lack a required capability return a typed
  `UnsupportedAtomicity` error before mutation;
- tests exercise the same storage contract against the in-memory and
  filesystem implementations.

### S.8 — Filesystem backend: honest limitations and target design

The current filesystem backend provides independent path reads and direct
per-file overwrites. It does **not** guarantee:

- coherent suite enumeration plus reads;
- atomic flag move;
- atomic retract cascade;
- all-or-nothing batch/review execution;
- conflict detection across writers;
- crash recovery of a multi-file change;
- durable publication of a complete generation.

This limitation is documented on `EinmoDirectory`, in public developer
documentation, and in capabilities returned by the transitional API. During
migration, operations requiring unavailable guarantees fail before mutation;
they do not retain a misleading all-or-nothing name while applying a loop of
file writes.

The target filesystem implementation uses a suite-local transaction area on
the same filesystem as stage data, a write-ahead transaction record, staged
files, and an atomically replaced revision/manifest pointer. At minimum:

1. acquire a suite writer lock with atomic creation/OS locking;
2. read the current revision and validate the base/dependency versions;
3. serialize and verify every proposed signed artifact before staging;
4. write staged content in the suite-local transaction directory;
5. flush staged files and the transaction record according to the durability
   contract;
6. mark the transaction prepared;
7. publish one new revision pointer/manifest atomically;
8. finalize materialized stage paths or expose them through the revision
   manifest;
9. on startup/read, recover a prepared transaction deterministically;
10. garbage-collect unreachable staged revisions only after recovery proves
    they are not current.

The detailed representation is decided by a prototype before production
migration. The prototype compares at least:

- revision directories plus an atomic current pointer;
- write-ahead log plus rollback/roll-forward;
- a transactional embedded database storing artifact blobs and manifests.

The selected design must state platform assumptions. A single POSIX rename
does not make an arbitrary series of renames atomic, directory fsync behavior
matters for durability, and advisory locks alone do not protect readers from
partially visible state.

The first backend's minimum crash floor remains a narrowed human choice between
process-crash and OS-crash recovery. Power-loss claims may be omitted unless a
named deployment requires and can verify them. Unsupported requested levels
fail before mutation. See
[`EIMP-11.workfile.durability.md`](EIMP-11.workfile.durability.md).

The earlier preference for immutable revisions/manifests and an atomic
`CURRENT` pointer is now one prototype candidate, not the selected target.
Prototype the smallest representation that supplies EIMP-B's old-or-new
operation boundary and the chosen EIMP-C crash floor; state from evidence
whether ordinary stage paths remain authoritative. Do not add history browsing,
Git management, CI policy, status publication, or backup/export features merely
to complete the persistence design. See
[`EIMP-11.workfile.persistence-prototypes.md`](EIMP-11.workfile.persistence-prototypes.md).

### S.9 — Strict, fallible configuration

Configuration loading returns `Result`; it never silently replaces malformed
or unreadable configuration with defaults. Typed serde structures reject
unknown fields and wrong types. Validation rejects negative/out-of-range
depth, duration, and parallel values before conversion.

Errors include the source path and field. Signing, reviewer-key, collation,
generation, verification, CLI, and server entry points propagate those
errors. “No file exists” may select defaults; “a file exists but is invalid”
is an error.

### S.10 — CLI option and exit-code integrity

File-list positionals do not swallow later options. Options may appear in
normal clap positions; `--` is required only when a literal filename begins
with `-`. Every gate-like option has an integration test asserting both its
diagnostic and process exit code with options before and after positionals.

In particular, a divergent `compare --require-match` always exits nonzero.

### S.11 — Duration limits in parallel execution

Parallel execution honors `suite_duration_limit` by stopping the scheduling
of new cases after a shared deadline. The contract distinguishes:

- cooperative suite deadline: already-running evaluators may finish;
- per-case elapsed limit: classifies an over-limit completed evaluation;
- hard subprocess termination: not provided unless explicitly implemented.

If this contract cannot be implemented in the current evaluator trait, the
configuration combination is rejected instead of ignored.

### S.12 — Automated engineering gates

The repository gains one working, documented automation path that enforces:

- formatting;
- workspace clippy with warnings denied;
- workspace nextest without misleading fail-fast JUnit totals;
- doctests;
- MSRV and supported-platform checks;
- dependency advisory/license/source policy;
- scoped mutation testing for changed security/gate modules.

EIMP 9 remains the detailed authority until its requirements are migrated or
completed. EIMP 11 does not duplicate its historical narrative; it ensures
the resulting contract is enforced before this hardening project completes.

### S.13 — Strict envelope and stamp parsing

Parsing rejects unknown and duplicate header/metadata fields, malformed
metadata lines, unsupported encodings, empty separators, duplicate section
names, a non-final or repeated `STAMPS` declaration, empty/structurally
invalid chains, invalid timestamps where timestamps are required, and
trailing material not explicitly defined as advisory data.

Wire DTOs absorb textual quirks; validated domain types expose only states
that can be serialized and verified. Existing signed fixtures are checked
for compatibility before strictness lands. Any deliberately accepted legacy
form receives an explicit versioned compatibility rule, not a general
leniency exception.

### S.14 — Poisoning and service failure semantics

Production review/server code does not panic on poisoned locks. Each shared
state type documents whether poison means:

- recover the inner value because the guarded operation is transactional;
- discard/rebuild a cache;
- mark the session failed and return a typed service error;
- shut down the server cleanly.

Handlers return `ApiError`; `Drop` remains non-panicking. Tests deliberately
poison each lock and assert the documented outcome.

### S.15 — Human-attestation and passphrase messaging

The compression-based passphrase score is not proof of entropy, identity, or
human presence and cannot substitute for an expected reviewer public key.

The score and current API/tooling are retained by maintainer decision. It is
described only as a corpus-relative compression heuristic, never guessing cost,
entropy, key possession, identity, uniqueness, or proof of human participation.
Any second password-quality measure or change to blocking behavior is deferred
to EIMP-E and requires its own threat model and migration. See
[`EIMP-11.workfile.attestation-score.md`](EIMP-11.workfile.attestation-score.md)
for evidence, alternatives, and the recorded disposition.

Documentation states that cryptographic identity comes from possession of
the expected private key and verification against the configured public-key
prefix. The stock compiled key and empty computer key are public labels, not
trust anchors.

### S.16 — Validated metadata and section construction

`Metadata` fields become private. Construction validates status, timestamps,
provenance, references, and section declarations. `EinmoFile` derives its
declared ordered section list from actual sections; callers cannot provide a
contradictory list.

Where a wire representation needs permissive strings, conversion into domain
types is fallible. Public access remains read-only through methods.

### S.17 — Dependency and crate surface

The project records and enforces dependency policy for a crypto-touching
crate.

**First pass:** ratify and refine EIMP 4's existing two-crate decision—core
`einmo` and `einmo-review-server`—unless measurements reveal materially contrary
evidence. Do not reopen feature-versus-split as though EIMP 4 had not selected
a direction. Keep `zweimomo` unpublished and decide corpus-signing placement
from measured coupling. See
[`EIMP-11.workfile.crate-and-document-boundaries.md`](EIMP-11.workfile.crate-and-document-boundaries.md).

This work coordinates with EIMP 4 rather than independently performing a
conflicting crate split. The phase measures the selected package boundary and
updates EIMP 4; it reopens the decision only if recorded evidence contradicts
EIMP 4's assumptions.

### S.18 — Module decomposition

Large modules are split by current responsibility without changing the
curated `lib.rs` API merely for aesthetics. Candidate boundaries are:

- `runner/{evaluation,parallel,crash_crumb,integrity}`;
- `review/{decision,plan,execution,cache}`;
- `review_http/{dto,handlers,auth,serve}`;
- `wire/{format,metadata,stamp}` if strict parsing makes that boundary
  clearer.

The decomposition follows behavior and ownership, not line-count targets.
No generic `utils` module is introduced.

### S.19 — Replacement documentation and deprecation

The final documentation set has concise, non-overlapping roles:

- `README.md`: user model, four stages, three adjacent gates, normal CLI
  workflow, and safe result inspection;
- `DEVELOPING.md` (new): current developer setup, test/gate commands,
  architecture map, persistence guarantees/limitations, and release checks;
- `docs/architecture.md` (new): current domain model, signature trust model,
  transaction boundaries, conflict rules, and filesystem backend guarantees;
- EIMPs: accepted design decisions and active plans, not duplicated user or
  developer manuals.

Replacement docs state the current system and why it has these boundaries.
They do not retell the extraction history, agent process, or chronological
development narrative unless that history is necessary to explain a current
compatibility constraint.

The documentation phase performs an inventory:

1. classify each existing document as current reference, active proposal, or
   historical record;
2. migrate every still-live requirement into the appropriate current doc or
   active EIMP;
3. add a prominent superseded/deprecated banner and forward link to obsolete
   files rather than silently deleting rationale;
4. update `docs/eimp/INDEX.md` statuses only after migration is traceable;
5. remove obsolete files only when no stable citation or unique rationale
   depends on them; otherwise retain them as historical records excluded from
   current instructions.

At minimum this audit covers EIMP 8, EIMP 9, EIMP 01, the root README,
`rust_instructions.md`, `AGENTS.md`, and `zweimomo/README.md`. EIMP 11 may
supersede EIMP 8 only if all of EIMP 8's accepted unresolved findings are
accounted for; it may not erase findings that were outside the 2026-08-25
review scope.

**First pass:** complete EIMP 01 normally, retain EIMP 9 until its test-tooling
contract is implemented or migrated, and narrow EIMP 8 through a P0–P41 ledger;
supersede it only after every finding has a disposition. The ledger schema and
exact Gate F question are in the crate/document boundary workfile.

### S.20 — Suggested extraction EIMPs

EIMP 11 is presently an umbrella design. The following lettered names are
**design-local aliases**, not assigned EIMP identifiers or filenames. If the
maintainer approves an extraction, `eimp_check.py gen_next` assigns its real
little-endian number when the specification is created. Chronological EIMP 12
is already represented by filename `EIMP-21`; if no intervening EIMP is
created, EIMP-A would therefore become chronological EIMP 13 with filename
`EIMP-31`, not `EIMP-12` or a literal `EIMP-A.md`.

Until a numbered specification and plan are created, every item below remains
owned by EIMP 11. Extraction is permitted only where the work has its own
normative decision, testable completion condition, and execution sequence. An
extracted EIMP copies the complete accepted contract it owns, while EIMP 11
retains a short dependency and acceptance summary. No finding may disappear
between plans, and no two active plans may claim implementation ownership.

#### EIMP-A — Integrity, provenance, and decomposition charter

Create this candidate first. Its primary deliverable is a short plain-language
statement for each einmo stage and signature answering: when someone encounters
this repository and verifies this artifact, what may they conclude, who made
that claim, what bytes and predecessor claim does the signature cover, and what
does einmo explicitly not guarantee? It includes concrete statements such as
how a project demonstrates human review with human-signed checked/verified
claims and what the final suite-wide signature establishes about the included
tests, results, and affirmative gate outcomes.

EIMP-A iterates from that simple statement into only enough exact format,
verification, failure, and integration detail to make the claim enforceable.
It distinguishes approval integrity, immediate-source provenance, optional
repository-commit provenance, operational atomicity/recovery, and external use
of signed results. Git history, CI policy, release decisions, and status display
remain consumers rather than hidden parts of the einmo guarantee.

EIMP-A is also the decomposition charter for this program. Before proposing
implementation, it reviews EIMP-B through EIMP-G and every overlapping existing
EIMP. For each it records: exact included claim/work; exact exclusion; source
EIMP sections/findings absorbed; dependencies; human choices still required;
and whether to create, merge into an existing EIMP, defer, or reject it. The
durability and persistence questions currently called Reviews 4 and 5 are
inputs to this contextual analysis, not independent gates.

Completion requires an accepted integrity/provenance statement, a stage-by-stage
claim matrix, a suite-signature statement, explicit non-goals, and a complete
ownership proposal for EIMP-B through EIMP-G plus EIMPs 1, 4, 8, 9, and 01.
After acceptance, current documentation is updated to use the same integrity
and attestation vocabulary before deeper child EIMPs are finalized.

#### EIMP-B — Transaction semantics and storage API

Extract §§S.4–S.7 and the transaction-semantics workfile into one EIMP defining
operation descriptors, coherent snapshots, dependency observations, conflict
and idempotence rules, transaction receipts, the reference state machine, and
the backend-independent storage API. It owns in-memory reference semantics and
cross-backend contract tests, but not a production filesystem representation.

EIMP-B protects coherent application of signed approval claims. Its revisions
and receipts are concurrency/recovery mechanisms, not integrity authorities,
long-term provenance, or a version-browsing service. It begins after EIMP 11's
critical fixes and recorded Decisions A and B.1. It may reach Final only after
the remaining generation-retention and durability inputs are deliberately
narrowed. Completion requires an executable reference model and transactional
API whose unsupported capabilities fail before mutation.

#### EIMP-C — Recoverable filesystem persistence

Extract §S.8, the durability workfile, and the persistence-prototype workfile
into a filesystem-backend EIMP. It owns the minimum platform contract,
prototype evidence, selected layout, migration, multi-process conflict tests,
fault injection, recovery, and garbage collection. It depends on EIMP-B's
frozen contract and cannot redefine domain transaction boundaries.

EIMP-C is deliberately narrower than the current first pass. It makes one
logical operation old-or-new across a process/OS crash; it does not turn crash
crumbs, transaction revisions, Git checkouts, CI decisions, or GitHub status
display into einmo integrity claims. Completed history remains external and
unreachable transaction revisions are collected after recovery.

#### EIMP-D — Fail-closed operational reliability

Extract §§S.9–S.11 and S.14 into one EIMP covering configuration, CLI argument
and exit-code integrity, suite duration behavior, and typed lock/service
failure. These features share one externally visible rule: invalid input or
runtime infrastructure failure produces an explicit typed/nonzero outcome and
never silently substitutes success, defaults, partial results, or a panic.

EIMP-D is independent of the filesystem representation once its tests use
storage capabilities rather than assume path mutation. Completion requires
matching library, CLI, JSON, and server behavior with deterministic tests for
every formerly swallowed or order-dependent failure.

#### EIMP-E — Envelope and attestation hardening

Extract §§S.13, S.15, and S.16 into one EIMP covering strict wire grammar,
validated metadata, typed passphrase-quality results, and accurate attestation
claims. EIMP 11 §S.2's urgent complete-stamp correction remains in EIMP 11 and
becomes EIMP-E's prerequisite; extraction must not delay it.

The compression score is retained. It may be described only as its actual
compression-based heuristic, never as entropy, identity, or proof of human
participation. A second password-quality measure is optional future design and
is not urgent EIMP 11 work. EIMP-E owns any later interface improvement, wire
compatibility inventory, DTO-to-validated-domain conversion, and fuzz/property
tests.

#### EIMP-F — Current-state documentation authority

Extract the documentation portion of §S.19 after EIMP-A through EIMP-E and the
existing EIMPs 4, 9, and 01 have stable outcomes to document. It owns the
user/developer/architecture document boundaries, consistency checks,
requirement-migration ledger, and old-document dispositions. It does not own
implementation requirements merely because it documents them.

Completion means one authoritative current home for every stage, transition,
persistence, trust, CLI, test-gate, and contributor-workflow fact, with
historical documents clearly scoped and no live requirement lost.

#### EIMP-G — Repository-source provenance (optional)

Consider a separate EIMP for a reproducible workflow that checks out or clones
a repository at an exact commit, generates from that tree, and includes the
repository identity and commit SHA in signed input provenance. This would
strengthen the claim that a verified result came from identified checked code.
It must specify dirty-tree, submodule, dependency, evaluator-binary, and
unavailable-remote behavior before claiming reproducibility.

EIMP-G does not make Git history, CI policy, release decisions, or status-badge
publication einmo responsibilities. Those systems consume a signed einmo
result. Create EIMP-G only after the desired signed claim and threat boundary
are explicit; it is useful but not required for EIMP 11's critical fixes.

Responsibility-based module decomposition is deliberately excluded from the
lettered candidate set. File size alone does not justify an EIMP. Phase 5A may
keep local refactors beside the behavior change that needs them or ask EIMP-A
to propose another EIMP only if the audit finds an independently testable
architecture boundary.

#### Existing EIMP ownership retained

- EIMP 9 remains the owner of §S.12's test-tooling contract; EIMP 11 contributes
  new gate cases and a finding ledger rather than a duplicate gates EIMP.
- EIMP 4 remains the owner of §S.17's published crate boundary, including a
  standardized distribution through which cases can be managed and reviewed
  with `cargo einmo`.
- EIMP 01 remains the owner of generation-stage and adjacent-gate semantics;
  EIMP-B/C own only atomic publication and persistence mechanics.
- EIMP 1 remains the owner of the review product surface; EIMP 11 and EIMP-A
  state the integrity claims, while EIMP-B owns the confirmed-transaction
  contract it consumes.
- EIMP 8 is narrowed only through the requirement-migration ledger; accepted
  findings move to exactly one retained or extracted owner before supersession.

#### Integrity boundary retained across all EIMPs

Einmo's integrity boundary is the approval-level signature chain and the final
suite-wide signature over the included tests, results, and affirmative gate
outcomes. Transactional storage preserves coherent application of those
claims; it does not replace them. Checking the signed result into Git, using it
for a CI decision, or displaying software status are supported consumers and
may receive purpose-built integrations, but their policy and publication are
outside einmo's responsibility.

If an extraction is approved, EIMP 11 replaces the corresponding implementation
checkboxes with timestamped handoffs naming the assigned EIMP and exact moved
sections/tests. Merely creating child specifications does not satisfy EIMP 11.

## Test Plan

Testing is staged with the implementation plan and written before each fix.
The essential coverage is:

- unit tests for invalid/tampered destination refusal and order-independent
  verified-stamp policy;
- an exhaustive legal-transition matrix shared by library/CLI/review tests;
- a reusable transactional storage contract suite run against in-memory and
  filesystem backends;
- deterministic conflict tests for disjoint merge, idempotence, divergent
  writes, delete/modify, co-sign merge, membership drift, and invalid current
  artifacts;
- crash/fault-injection tests at every filesystem transaction phase, proving
  recovery exposes either the old or new revision and never a mixed state;
- concurrency tests with two writers starting from one snapshot;
- strict configuration and envelope parser tables plus fuzz/property tests;
- CLI process tests for option order and exit status;
- parallel deadline tests with an injected clock/evaluator barrier;
- poisoned-lock service tests;
- documentation checks for the stage list, legal transitions, commands, and
  cross-document links;
- a comprehensive EIMP 11 integration test that combines concurrent review,
  conflict, crash recovery, tamper refusal, and post-recovery gate checks on a
  scratch `zweimomo` suite.

The complete formatting, workspace clippy, nextest, doctest, dependency,
coverage, and scoped-mutation gates run before completion.

## Rejected Alternatives

### A. Treat every file rename as sufficient atomicity

Rejected. A flag is a write plus delete, a retract is a cascade, and a review
plan or batch promotion spans cases. Individually atomic replacements can
still expose a logically impossible suite.

### B. Hold one process mutex around existing filesystem writes

Rejected. It does not protect against other processes, does not provide
crash recovery, does not give readers a coherent revision, and does not make
partial writes durable.

### C. Last writer wins

Rejected. It can discard co-signatures and execute stale review intent.
Conflicts must be explicit; semantic co-sign merging is the narrow exception.

### D. Silently preserve weak filesystem behavior behind the same API

Rejected. An API that implies atomic commit while looping over direct file
writes is more dangerous than an explicit unsupported-capability error.

### E. Fix only the three critical defects

Rejected. Those defects are urgent, but fail-open configuration, CLI exit
status, persistence semantics, and unenforced gates can recreate equally
serious failures.

### F. Rewrite or delete all historical EIMPs

Rejected. Historical rationale and stable citations remain useful. Obsolete
documents are deprecated with forward links after live requirements migrate;
deletion is reserved for documents with no remaining unique value.

### G. Keep extending the existing README and instruction files

Rejected. Their roles currently overlap and contradict one another. A small
current-state user guide, developer guide, and architecture reference are
easier to verify and maintain.

## Open Questions and First-Pass Answers

These are confirmation points, not invitations to begin design from an empty
page. Each gate presents the linked packet and asks the maintainer to accept or
amend its concrete first pass.

- **Resolved A:** remove `verified → checked`; use retraction. Apply only after
  complete-stamp/multi-signature work and test multiple checked signers followed
  by a human verified signer.
- **Resolved B.1:** confirmed plans are suite-wide all-or-nothing; defer partial
  execution and retain `execute_one` as the explicit smaller boundary.
- **Routed through EIMP-A — B.2/B.3/C:** present crash durability,
  incomplete-generation retention, and minimal old-or-new persistence together
  with the stage/signature claims and proposed EIMP-B/EIMP-C ownership. Do not
  ask these as isolated gates before their relationship to einmo's integrity
  boundary is explicit.
- **Resolved D:** retain the compression score and current tooling, narrow its
  claims to a heuristic, and defer any second quality measure to EIMP-E.
- **Accepted E direction:** retain EIMP 4's two-crate boundary and standardized
  `cargo einmo` distribution unless measurements contradict it; Gate E still
  records corpus-signer placement and any evidence-based amendment.
- **Accepted F direction:** finish EIMP 01, retain EIMP 9, and narrow EIMP 8 by
  complete finding ledger before possible supersession; Gate F still approves
  individual document dispositions after the ledger exists.

## References

- Prior EIMPs: EIMP 7 (storage abstraction), EIMP 8 (earlier code-review
  findings), EIMP 9 (test-tooling contract), EIMP 01 (generation stage and
  adjacent gates), EIMP 4 (crate split).
- Review artifact: `~/how_to_understand_and_develope_einmo.md`, created from
  the `jia` tree at `9c8589a` on 2026-08-25/26.
- Code locations: `src/case.rs`, `src/storage.rs`, `src/suite.rs`,
  `src/einmo_suite.rs`, `src/config.rs`, `src/format.rs`, `src/signature.rs`,
  `src/cli.rs`, `src/review.rs`, `src/review_server.rs`, `justfile`.
- External concepts: optimistic concurrency control, write-ahead logging,
  atomic rename, three-way merge, crash consistency, and transactional
  snapshot isolation.

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Incorporated the maintainer's decomposition response: EIMP-A is
now the first child and owns the plain-language stage/signature integrity and
provenance standard plus the contextual disposition of EIMP-B through EIMP-G
and overlapping existing EIMPs. Renamed later candidates and routed durability
and persistence questions through EIMP-A rather than isolated gates.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Added §S.20 with lettered candidate EIMPs A–G, explicit future
little-endian numbering, retained ownership for existing EIMPs, a narrower
transaction/persistence scope, optional repository-SHA provenance, and the
signature-chain/suite-signature integrity boundary.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Incorporated human responses 1–3: accepted removal of
`verified → checked` after multi-signature work, accepted all-or-nothing
confirmed review, and constrained transaction revisions to recovery/concurrency
rather than long-term history while leaving incomplete-run retention open.


**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Added the mandatory document-wide Impact Overview covering library
users, integrations/CI, developer notifications/tests/unfamiliar concepts, and
the consolidated ordered Migration for EIMP 11's incompatible changes.
