# EIMP 11 Workfile — Crate and Document Boundaries

> **Design workfile, not normative specification.** This file supplies the
> Gate E and Gate F presentations behind EIMP 11 §§S.17 and S.19. Measurement
> tables and the requirement-migration ledger grow here; accepted results are
> summarized in EIMP 11 and the EIMPs that retain ownership.

## Dependency position

Crate measurements and document inventory may run independently of persistence
prototypes. Gate E precedes manifest/public-package changes. The document
migration ledger may be prepared before Gate E, but dispositions wait for Gate
F and current docs must describe the boundary actually selected.

## Decision E — published crate boundary

### What the crate boundary establishes

This is a subsystem-ownership decision. The proposed steady state is that
`einmo` is the reusable domain/storage/format crate and
`einmo-review-server` is an application package built on it. Dependency
direction, public API ownership, test placement, and release order all follow
that sentence. The split succeeds only if a library consumer can understand and
build core without inheriting an HTTP server, async runtime, frontend, or
server-specific policy.

### Proposed steady state — dependency choice through release

1. A snapshot-testing application depends on `einmo` and receives stages,
   envelopes, signatures, transactional storage, suites, and the core CLI
   without server runtime dependencies. The standardized distribution includes
   `cargo-einmo`, so normal case management and review can be reached through
   `cargo einmo` without assembling an unpublished workspace.
2. A review deployment additionally depends on or installs
   `einmo-review-server`. That package imports core and owns review sessions,
   HTTP DTOs/handlers, transport/authentication, TUI/server binaries, and
   frontend assets.
3. No dependency points from core to server. A cross-boundary domain concept
   belongs in core only when non-server consumers genuinely need it.
4. Core tests prove domain contracts; server tests prove review/transport;
   packaged-consumer tests prove public boundaries; `zweimomo` exercises the
   integrated workspace without becoming published.
5. Release automation tests/packages core first, then server against the exact
   compatible core version. Changelogs identify package ownership and whether a
   coordinated release is required.
6. Dependency-policy CI rejects accidental server/runtime dependencies in core.
   Measurements may refine corpus-signer placement but may not blur ownership.

Library users will notice a smaller stable dependency and API surface. Server
integrators will notice a second package and explicit imports. Developers will
notice enforceable ownership, test placement, and release sequencing instead of
a one-crate convention maintained by memory.

### First pass

First pass, propose to ratify and refine EIMP 4's two-crate decision—core
`einmo` plus `einmo-review-server`, with dependency direction from server to
core—because EIMP 4 §S.1 assigns the modules and its Rejected Alternative A
already rejects a review-server feature. EIMP 11 §S.17 should measure this
boundary, not reopen feature-versus-split absent materially contrary evidence.

Current `Cargo.toml:70-92` makes `axum`, `tokio`, `futures-util`, and
`tokio-stream` unconditional, while `src/lib.rs:32-33,61-68` exports review and
server APIs from the core package. Keep unpublished `zweimomo` outside both
published crates. Treat corpus-signing placement as the remaining measured
choice.

Alternatives are one feature-variable crate (simpler versioning, larger feature
matrix and feature-unification surprises), three or more crates (stronger
isolation, premature publication coordination), or the unconditional monolith
(mechanically simple, fails EIMP 4's goal).

### Question for the maintainer

> Do measurements reveal a reason to amend EIMP 4's two-crate boundary, or
> should EIMP 11 ratify it and limit the remaining design choice to
> corpus-signing placement?

### Evidence table to complete

- dependency trees and prohibited server/runtime dependencies in core;
- clean build time, package/binary size, audit/license surface;
- EIMP 4 five-symbol packaged-core consumer;
- review-server end-to-end and both packages' publish dry runs;
- public API inventory and `zweimomo` workspace integration;
- resolution of EIMP 4's `0.0.6` text versus the index's `0.0.7` narrative.

### Impact descriptors — crate boundary

#### Einmo Library User Experience changes

Core consumers stop compiling or receiving review-server APIs and HTTP/runtime
dependencies. Imports currently re-exported from root `einmo` move to the
`einmo-review-server` package; core domain imports remain stable where EIMP 4's
public-surface contract allows. Corpus-signing imports depend on the recorded
placement decision.

#### Einmo Integration changes

Applications using review/server APIs add the second package and update import
paths; release automation publishes core before server. CI adds core dependency
denials, packaged-consumer smoke tests, server end-to-end tests, both publish dry
runs, API checks, and `zweimomo` workspace integration. Feature-combination CI
is not added unless evidence overturns the split.

#### Einmo Development changes

Module ownership, dependency direction, version coordination, changelog/release
order, and cross-crate test placement become explicit. New server dependencies
cannot leak into core. EIMP 4 remains the crate-split authority; EIMP 11 records
measurements and proposes amendments rather than competing architecture.

Developers must learn which package owns each responsibility, which tests prove
the boundary, how Cargo feature unification differs from package isolation, and
why a convenient core-to-server dependency is prohibited. Code review treats
package placement and dependency-tree changes as architecture, not manifest
housekeeping.

#### Migration

1. Publish or locally stage the core package first, then the review-server
   package depending on the same compatible core version.
2. Move review/server imports and binaries to `einmo-review-server`; preserve
   core imports named by EIMP 4's compatibility contract.
3. Update manifests, lockfile, release automation, docs, examples, and CI.
4. Validate packaged-core consumer, server end-to-end, both publish dry runs,
   public API checks, and `zweimomo` integration.
5. Keep a temporary re-export only if measured downstream compatibility
   requires it, with a stated removal release; rollback before publication is a
   manifest/module move reversal, while post-publication rollback requires a
   compatible corrective release rather than deleting versions.

## Decision F — old-document disposition

### What the documentation boundary establishes

This is a documentation-ownership system. The proposed steady state gives each
kind of truth one obvious current home while retaining EIMPs as design evidence:
README teaches users, `DEVELOPING.md` teaches current contributor operations,
`docs/architecture.md` states the current model and guarantees, source/API docs
describe callable surfaces, and EIMPs own accepted decisions and active plans.
Historical documents may explain how a decision arose, but are never silently
treated as current operating instructions.

### Proposed steady state — new fact through later supersession

1. A fact is classified as user workflow, developer operation,
   architecture/invariant, API contract, or design decision and written in the
   corresponding authority.
2. Repeated mechanical facts—stage lists, transition pairs, command forms—are
   generated from or consistency-tested against code rather than copied.
3. When design changes, a requirement ledger maps every live statement and
   unchecked task to its new owner before deprecation.
4. Current documents are updated first. Historical documents then receive a
   prominent status/banner and forward link while stable anchors and unique
   rationale are preserved.
5. CI checks links, examples, frontmatter/index agreement, and authoritative
   mechanical facts. A document is not current merely because another document
   links to it.
6. Deletion is last and exceptional: only material with no unique rationale,
   live requirement, or stable citation is removed.

Users will notice shorter, non-conflicting guidance. Integrations will notice
stable current links and executable examples. Developers will have an explicit
answer to “where does this fact belong?” plus a ledger and checks preventing
historical prose from silently regaining authority.

### First pass

First pass, propose the following, because §S.19 requires traceable migration
before status changes and the current plans retain live requirements:

- EIMP 01: do not deprecate; finish its remaining human review/closure. It is
  the current four-stage and adjacent-gate design on which EIMP 11 relies.
- EIMP 9: keep active/paused until mutation testing, workspace-complete test
  reporting, bootstrap, and comprehensive verification are implemented or
  individually migrated.
- EIMP 8: narrow finding by finding. Supersede only if every P0–P41 item is
  implemented, rejected with rationale, or assigned to a live owner.
- Historical EIMPs: retain a non-current banner and forward link when unique
  rationale or stable citations remain; remove only when neither remains.

### Question for the maintainer

> Approve completing EIMP 01 normally, retaining EIMP 9 until its test-tooling
> contract lands, and narrowing EIMP 8 finding by finding, with supersession
> allowed only after a complete migration ledger?

## Requirement-migration ledger

Populate one row for every EIMP 8 P-item and every unchecked EIMP 9/EIMP 01
requirement:

| Source/section/task | Current-state requirement | Implementation evidence | Destination owner | Disposition | Exact closure condition |
|---|---|---|---|---|---|
| _pending inventory_ | | | | | |

Current documentation states what is true now and why its boundary exists.
Chronology, agent activity, extraction narrative, and obsolete command variants
stay out unless necessary for compatibility. Stage lists, transitions, and CLI
forms should be generated or consistency-tested from authoritative code rather
than copied among manuals.

Gate F receives the completed ledger plus link/frontmatter/index checks,
documented-stage/transition consistency tests, executable command examples, and
proof that active docs do not normatively point to superseded behavior.

### Impact descriptors — document disposition

#### Einmo Library User Experience changes

No API changes arise merely from disposition. Users gain one current README and
architecture reference instead of conflicting historical instructions;
historical EIMPs remain available with clear non-current banners and forward
links when they retain rationale.

#### Einmo Integration changes

Automation and contributor links move to current command/test references. CI
checks links, EIMP index/frontmatter agreement, documented stages/transitions,
and executable examples. No file is deleted or status-changed until every
stable citation and live requirement has a recorded destination.

#### Einmo Development changes

The migration ledger becomes the unit of work: every old requirement has one
owner, evidence pointer, disposition, and closure condition. Current manuals
state present behavior and its rationale; EIMPs retain design authority and
history. Developers must update authoritative/generated facts rather than copy
transition or stage lists into another untested prose location.

Developers must learn the authority map and requirement-ledger workflow. Any
change to stages, transitions, commands, persistence guarantees, package
boundaries, or test commands identifies its authoritative source and consistency
tests during review. Cleanup is complete only when the old requirement has an
owner and closure condition, not when prose was deleted.

#### Migration

1. Populate the requirement ledger before changing status or banners.
2. Move live operating instructions to README, `DEVELOPING.md`, architecture
   docs, or their owning active EIMP and update inbound links.
3. Add non-current banners and forward links while preserving stable anchors
   where practical.
4. Run link, index/frontmatter, documented-fact, and executable-example checks.
5. Remove a historical file only after proving it has no unique rationale or
   stable citation; rollback restores the file/link from version control, but
   status changes still require the ledger to be corrected consistently.

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Incorporated Review 7's accepted amendment that the standardized
distribution support normal case management and review through `cargo einmo`.


**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Added forward-facing subsystem narratives for package ownership and
documentation authority, each walking from consumer use through tests, release
or migration, and ongoing developer maintenance.
