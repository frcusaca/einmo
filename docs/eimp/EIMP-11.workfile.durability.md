# EIMP 11 Workfile — Durability Contract

> **Design workfile, not normative specification.** This file supports EIMP 11
> §§S.4 and S.8 and Gate B.2. It will eventually hold platform measurements and
> fault evidence; the promised durability floor remains normative in EIMP 11.

## Dependency position

Durability criteria can be designed in parallel with transaction semantics,
after current mutation paths are inventoried. Gate B.2 freezes the promise
before filesystem prototypes are compared. A prototype that cannot meet the
promise is not a viable architecture even if it benchmarks well.

## Maintainer scope correction — operational coherence, not integrity

Einmo's integrity boundary is the approval-level signature chain and the final
suite-wide signature over the included tests, results, and affirmative gate
outcomes. Durability does not create or strengthen that cryptographic claim. It
answers only whether one logical storage operation that reported success is
recovered coherently after interruption.

Git history, CI acceptance policy, release decisions, and GitHub status display
may consume a signed einmo result and may receive dedicated integrations, but
their retention, policy, and publication are outside this contract. Crash
crumbs are diagnostic artifacts, not approval signatures or provenance roots.
The earlier packet's broader ecosystem language is superseded by this boundary.

## What this workfile introduces

EIMP 11 introduces a durability contract as a new communication and coding
abstraction. Until now, a successful filesystem call has been allowed to stand
in for the much larger claim “einmo committed the reviewed suite state.” Those
are not the same claim. This contract gives library callers, operators, CI, and
backend developers one vocabulary for asking and answering: **what failures may
happen after `commit` reports success without losing or mixing the committed
state?**

The abstraction has four parts:

- **requested durability** — the level the caller requires for this commit;
- **supported durability** — levels the backend can honestly provide in its
  current operating-system/filesystem environment;
- **achieved durability** — the level attached to a successful commit result;
- **recovery obligation** — what einmo must do before it serves another
  snapshot after interruption.

This vocabulary is intentionally visible at the storage boundary. It prevents
documentation from promising more than code implements, prevents a backend from
silently weakening a caller requirement, lets CI test named guarantees, and
gives developers explicit acceptance criteria for synchronization and recovery.

### Durability levels

- **Process-crash durability:** the einmo process may terminate at any
  instruction boundary while the OS and storage stack continue running. After
  restart, recovery exposes old or new coherent state, never a mixture.
- **OS-crash durability:** the machine may lose the OS's volatile filesystem
  state. Successful commit includes the documented file and directory
  synchronization sequence needed to recover the committed revision after boot.
- **Power-loss durability:** power may disappear after successful return. Einmo
  promises survival only on named platform/filesystem/device combinations whose
  flush, ordering, and atomic-replacement behavior supports and passes the
  contract. It is a capability, not a portable slogan.

These names describe guarantees after successful return. A fault before return
may leave the caller with a failure or uncertain outcome, but recovery still may
not expose a mixed revision.

## Proposed steady state — request, commit, crash, recovery

Here is the proposed durability subsystem from start to finish:

1. A caller begins from a coherent suite snapshot and requests a named
   `DurabilityLevel`, directly or through a documented default.
2. The backend reports environment-specific support. If the requested level is
   unavailable, commit returns `UnsupportedDurability` before mutation; there is
   no silent downgrade.
3. The backend validates the base/dependencies, serializes and verifies the
   complete change set, and stages it outside authoritative visibility.
4. It executes the ordered writes, file syncs, transaction-record sync,
   prepare, publication, and directory sync required by the requested level.
5. `commit` returns success only after crossing that named success boundary and
   reports the new revision plus achieved durability.
6. New readers see the coherent new revision; readers holding an older immutable
   snapshot may finish without observing intermediate state.
7. After interruption, recovery runs before any new snapshot is served. It uses
   durable transaction metadata and the authoritative publication state to
   finalize published work or discard unreachable staging deterministically.
8. CI proves the promise through deterministic failure points, subprocess
   termination/restart, and platform-specific jobs. A platform enters the
   support table only after that evidence exists.

Library users will notice typed capability and commit results and explicit
unsupported-level errors. Operators will notice a support table and an honest
durability/performance choice. Developers will notice one centralized protocol,
named fault points, recovery-before-read, and a requirement to re-prove the
contract whenever synchronization ordering changes.

## Decision B.2 — successful commit

### First pass requiring a narrowed follow-up

The original first pass proposed that every production transactional filesystem backend
recover correctly after process and OS crash, while advertising power-loss
durability only for platform/filesystem combinations whose file synchronization,
directory synchronization, and atomic pointer replacement prerequisites are
supported and tested. This follows §S.4's separation of durability/recovery and
§S.8's prepare, flush, publish, and recovery protocol.

The current `src/storage.rs` path backend uses direct `std::fs::write` and
removal without a transaction record, synchronization boundary, or recovery;
it cannot make any of these multi-file promises.

### Proposed API meaning

- A caller requests a `DurabilityLevel`; capabilities report supported levels.
- Unsupported levels fail before mutation; no silent downgrade is allowed.
- Successful return means all staged bytes and the manifest reached the
  documented boundary, pointer publication completed, and recovery exposes
  old or new state—never a mixture.
- Recovery runs before a coherent snapshot is served.
- The commit result records the achieved level and revision.

Exact names remain provisional until Gate B is accepted, but the model should
be recognizable in an API of this shape:

```rust
pub enum DurabilityLevel {
    ProcessCrash,
    OsCrash,
    PowerLoss,
}

pub struct CommitRequirements {
    pub durability: DurabilityLevel,
}

pub struct Committed {
    pub revision: SuiteRevision,
    pub achieved_durability: DurabilityLevel,
}
```

`StorageCapabilities` must express supported levels with enough environmental
context to explain refusal. A Boolean `durable: true` is insufficient because
it does not identify the covered failure boundary.

### Alternatives

| Promise | Benefit | Cost/risk |
|---|---|---|
| process crash only | cheap and portable | inadequate for normal machine crashes and signed reviewed state |
| OS crash floor | meaningful local guarantee | requires exact sync protocol and subprocess/fault tests |
| unconditional power loss | strongest wording | cannot be honestly universal where devices/controllers lie |
| capability-gated power loss | strong where verified | needs support matrix and refusal behavior |

### Questions for the maintainer

> **@human:** With signatures—not storage durability—as the integrity boundary,
> should the first EIMP-C backend promise process-crash recovery only, or also
> OS-crash recovery before `commit` may report success? Power-loss claims can be
> omitted from the first backend unless a named deployment requires them.

> Which operating systems and filesystem classes must the first production
> implementation support?

## Evidence to add before Gate C

- exact write/sync/prepare/pointer-swap/parent-sync return sequence;
- Linux and macOS support matrix, including filesystem assumptions;
- deterministic failure points before and after every durable step;
- subprocess-kill and remount/power-fault approximation results;
- achieved durability and recovery result for each prototype;
- performance measurements separated by requested durability level.

## Impact descriptors

### Einmo Library User Experience changes

Callers can request a durability level and inspect backend capabilities. A
backend that cannot meet the request returns a typed error before mutation;
successful commit reports the achieved level and durable revision. “Success” is
therefore stronger and potentially slower than today's direct write return.

### Einmo Integration changes

Deployments must declare supported OS/filesystem classes and may need an
operator-selected durability level. CI separates logical fault injection from
platform jobs, exercises subprocess termination and recovery-before-read, and
records unsupported combinations rather than treating them as silently weaker
success. Benchmarks must label the requested durability mode.

### Einmo Development changes

Filesystem code must centralize synchronization and publication ordering; no
new backend may claim durability from rename alone. Changes to the commit
protocol require updates to the fault matrix, support table, recovery proof,
and exact return boundary. Platform expansion is specification work, not just a
conditional-compilation patch.

Developers must become familiar with the distinction between atomic visibility
and durability, file versus directory synchronization, uncertain commit
outcomes, recovery-before-read, and the platform support matrix. Required tests
include deterministic protocol fault points, child-process kills,
restart/recovery assertions, old-or-new visibility checks, and platform-labeled
performance results. A change that merely “seems safer” does not extend the
guarantee until those tests and the support table say it does.

### Migration

None for persisted data at the policy-decision stage. When a production backend
adopts the contract, its persistence migration must be documented in the
persistence-prototypes workfile. Callers requesting durability will need to set
or accept an explicit level; existing callers use the documented default only
after its compatibility behavior is specified.

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Reassigned the eventual filesystem crash-floor choice to candidate
EIMP-C; EIMP-A must first present it in the integrity and ownership context.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Narrowed durability to operational old-or-new recovery, separated
it from signature-chain integrity and external Git/CI/status consumers, and
replaced the broad first-pass approval request with a smaller EIMP-C choice.


**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Reframed durability as a new caller/backend/CI abstraction, defined
its vocabulary and failure levels, and added an end-to-end proposed steady state
from requested guarantee through commit, interruption, and recovery.
