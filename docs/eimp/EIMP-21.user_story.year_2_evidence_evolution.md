# EIMP-21 User Story: Year 2 — Evidence Through Long-Term Change

## Story

Two years after adopting extra-repository verification, the project has many
subject releases, verifier revisions, inventory versions, platform profiles,
successful validation records, and rotated credentials. The team needs old
records to retain precise meaning without allowing them to satisfy requirements
that did not exist when they were created.

## Historical Investigation

An auditor investigates release `S31`, shipped eighteen months ago. The
successful record identifies:

- exact subject commit, tree, dependencies, and tested binary;
- exact validation revision and evaluator;
- exact protected inventory and structured case graph;
- platform and execution policy;
- required and completed case identities;
- completion time, assurance signer, and result; and
- review rationale linked to consequential expectation or policy changes.

Later verifier and inventory changes did not mutate this claim. **Contracts
relied upon: 1.3, 1.4, 3.4, 3.6, 5.6, 6.8.**

The auditor can say “this exact claim was signed and remains cryptographically
valid.” That does not yet answer whether the signer is trusted under today's
policy or whether the old inventory was sufficient by current standards.

## Current Release Policy

The current release candidate is `S94`. Policy requires inventory `I22`,
verifier `V19`, Linux and Windows profiles, and an assurance key trusted for
the current period.

An authentic old record for `S31 + V7 + I8` is rejected as inapplicable. It is
not deleted or called fraudulent; it simply proves a different historical
fact. **Contracts relied upon: 3.6, 4.5, 5.7.**

If `S94` has a Linux success but no Windows success, the release view displays
partial evidence rather than aggregating the available result into “verified.”
The Windows inventory remains required for that profile. **Contracts relied
upon: 1.1, 2.5, 2.6, 5.3, 5.7, 6.5.**

## Retiring and Replacing Tests

One old protocol is no longer supported. The corresponding cases should not be
deleted from history.

1. A proposal explains why the protocol and its assurance obligations are
   being retired. **Contracts relied upon: 5.4–5.6.**
2. Validation revision `V20` removes or archives the obsolete implementation
   through ordinary review.
3. The inventory authority publishes `I23`, explicitly removing or replacing
   the stable requirements and recording activation. **Contracts relied upon:
   1.1–1.4, 6.8.**
4. Old records continue naming `I22`; new policy requires `I23`. Neither
   version rewrites the other. **Contracts relied upon: 3.6.**

An agent cannot make the retirement happen merely by deleting a file or adding
`#[ignore]`. Until `I23` is authorized, the missing `I22` case prevents
success. **Contracts relied upon: 1.2, 2.5, 2.6, 4.1.**

## Evolving Test Structure

The project splits a single `storage` capability into `local-storage` and
`replicated-storage`, and several linear tests become multi-step sequences.
This creates a new structured inventory rather than changing how old case IDs
are interpreted.

The verifier validates parent/child and prerequisite edges, rejects cycles or
missing references, and reports blocked descendants separately from passes.
Historical records still resolve their old hierarchy. **Contracts relied upon:
6.1–6.8.**

## Key Rotation and Revocation

The assurance signing key rotates annually. Each successful record identifies
the signer and completion time. Trust policy can therefore answer separately:

1. Is the record's signature valid?
2. Was that key authorized at the time?
3. Has later compromise or revocation changed how this historical record
   should be treated?
4. Does the otherwise valid record satisfy current release policy?

The exact revocation and trusted-time mechanism remains open. Whatever is
chosen must not require old records to be rewritten. This extends **contracts
3.4, 3.6, and 5.6.**

## Repository or Hosting Migration

The subject project moves from one GitHub organization to another. Canonical
repository identity rules must distinguish a legitimate migration from an
unrelated repository containing the same commit bytes. New records use the new
resolved identity and migration policy; old records retain the old identity.
**Contracts relied upon: 2.1, 3.4, 3.6, 4.5.**

A hosted provider is convenient but not foundational. If all necessary Git
objects, inventories, verifier revisions, keys, and artifacts are available
locally, verification of old records and new offline runs should remain
possible.

## Archival Failure

The team tries to reproduce a two-year-old result but discovers that its exact
compiler or dependency artifact is no longer available. The old record still
proves the identity of what was tested; its digests reveal whether a candidate
replacement is identical. It does not conjure missing materials.

Einmo reports the distinction:

```text
historical record signature: valid
historical claim:             interpretable
required materials:          incomplete
reproduction:                unavailable
current policy acceptance:   separately evaluated
```

Identity and archival availability are separate guarantees. **Contracts
relied upon: 2.3, 3.4, 3.6, 5.3.** A future archival contract may belong in
EIMP 21 or a follow-on EIMP.

## What Year 2 Establishes

The long-term goal is not for an old green result to remain sufficient forever.
It is for every result to retain a precise, reviewable meaning while current
policy can demand newer and stronger evidence.

## Contract Trace

| Story event | Contracts |
|---|---|
| Interpret an old record exactly | 1.3, 3.4, 3.6 |
| Reject authentic but stale evidence for a current release | 3.6, 4.5, 5.7 |
| Keep platform/profile evidence separate | 2.4–2.6, 5.7, 6.5 |
| Retire a requirement without rewriting history | 1.4, 5.4–5.6, 6.8 |
| Prevent deletion or `#[ignore]` from retiring a test | 1.2, 2.6, 4.1 |
| Evolve hierarchy and sequences explicitly | 6.1–6.8 |
| Evaluate rotated or revoked keys historically | 3.4, 3.6, 5.6 |
| Distinguish evidence identity from archival availability | 2.3, 3.4, 5.3 |

## Questions Exposed by This Story

- What canonical repository identity survives legitimate hosting migrations?
- What trusted-time and revocation model applies to old assurance signatures?
- Are successful validation records permanent, or may retention policy remove
  them after exporting an authenticated archive?
- Which materials must be archived to promise reproducibility rather than only
  evidence verification?
- How does current policy select among several inventory versions and platform
  profiles without relying on a mutable label inside the signed record?
- Should requirement retirement demand a stronger human rationale or approval
  threshold than requirement addition?

## Last Updated

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Renumbered this document from EIMP 11 to EIMP 21 and updated its internal references because EIMP 11 collided with another proposal.

**Date**: 2026-09-04  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Created the Year 2 EIMP 21 user story, grounding historical
interpretation, present policy, test retirement, structural evolution, key
rotation and revocation, repository migration, platform evidence, and archival
limits in the numbered contracts.
