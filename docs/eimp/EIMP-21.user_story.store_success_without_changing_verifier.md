# EIMP-21 User Story: Store Success Without Changing Its Verifier Identity

## Story

An official validation of `S17 + V5 + I5` passes. Einmo must sign a successful
record and store it in the validation repository, which is intended to contain
good results. But adding a file to a Git repository changes its commit SHA.

The result must not claim it was produced by a verifier revision that exists
only after the result itself was written.

## The Circular Form That Cannot Work

Suppose `V5` is the clean validation commit containing tests and reviewed
expectations. A naive workflow would be:

1. run tests from `V5`;
2. create `results/R17.einmo`;
3. commit it, producing commit `E6`; and
4. put `E6` in `R17` as the validation-repository revision.

Step 4 changes the bytes of `R17`, which changes the tree and therefore changes
`E6`. Updating the embedded SHA repeats the cycle. More importantly, the tests
that ran came from `V5`, not from the later commit that stores their result.

The signed claim must identify verifier content independently of the act that
stores the claim. **Contract relied upon: 3.7.**

## Required Successful Workflow

### 1. Freeze verifier content before execution

Einmo identifies the exact test, evaluator, fixture, expectation, and policy
content as verifier identity `V5`. The subject and verifier inputs are clean
and immutable through the run. **Contracts relied upon: 2.2, 2.4.**

### 2. Complete and sign the result

After all required cases reconcile and pass, einmo creates `R17`. The record
binds `S17`, verifier content `V5`, inventory `I5`, the tested artifact, and the
execution context. No future evidence-storage commit is part of the claim
about what executed. **Contracts relied upon: 3.3, 3.4, 3.7.**

### 3. Append the immutable signed record

The signed `R17` is appended to the validation repository's good-evidence
storage. The storage operation may receive its own commit or ledger identity,
but it cannot alter `R17` or redefine `V5`. **Contracts relied upon: 3.5,
3.7.**

### 4. Use the record later

A later verifier revision `V6` may coexist with `R17`. A policy asking whether
`S17` passed `V5 + I5` can find and verify `R17`. A policy requiring `V6` cannot
silently reuse it. **Contracts relied upon: 3.6, 3.8, 4.5.**

## Candidate Storage Shapes

These are alternatives still under design, not simultaneous requirements.

### A. Separate evidence branch in the same Git repository

The validation-code branch holds `V5`; an append-oriented evidence branch
stores `R17`, which names `V5`.

Advantages:

- satisfies the user's model of one validation repository holding good results;
- preserves ordinary Git transport and review;
- verifier commits and evidence commits are unambiguous.

Costs:

- tooling must fetch and manage two histories;
- branch force-push protections become part of the trust model;
- concurrent result appends need conflict handling.

### B. One branch with a scoped verifier-tree digest

Tests and evidence share a branch, but the successful record identifies a
canonical digest of verifier-owned paths that excludes the result ledger.

Advantages:

- simple checkout and browsing model;
- appending evidence does not change the scoped verifier digest.

Costs:

- path classification becomes security-critical;
- a file affecting execution must never be misclassified as evidence-only;
- Git commit identity alone no longer identifies the verifier.

### C. Content-addressed evidence ledger attached to the repository

The repository identifies an external or internal object store containing
immutable successful records. Each record names `V5`; the ledger gives the
record its own digest and optional append proof.

Advantages:

- clean separation of verifier source and evidence;
- natural concurrent append and content addressing;
- no self-referential Git commit.

Costs:

- adds a storage abstraction and retention policy;
- offline use requires the selected evidence objects to be locally available;
- may feel like a second repository unless integrated carefully.

### D. A separate results repository

This removes the circularity cleanly but introduces a fourth protected
location—the subject repository, validation repository, protected inventory,
and result repository. It conflicts with the desired simple statement that the
validation repository stores good results unless “validation repository” is
defined as a logical system rather than one Git repository.

## Human View

The UI should display both identities:

```text
Verifier content that ran:       V5
Successful validation record:    sha256:<R17 digest>
Evidence-storage append:         E6
```

This lets a reviewer distinguish “which tests ran?” from “where was their
signed result stored?” **Contracts relied upon: 5.1, 5.3, 5.7.**

## Contract Trace

| Story event | Contracts |
|---|---|
| Freeze the clean verifier content | 2.2, 2.4 |
| Sign only after complete success | 3.3, 3.4 |
| Keep storage identity out of the verifier claim | 3.7 |
| Store only the successful immutable record | 3.5 |
| Select the result later under explicit policy | 3.6, 3.8 |
| Prevent reuse for a different verifier | 4.5 |
| Explain verifier versus storage identities | 5.1, 5.3 |

## Guarantees and Limits

Contract 3.7 requires non-circular identity but does not yet choose one storage
shape. Any implementation must also address concurrent appends, history
rewrites, garbage collection, access control, and offline availability.

Git immutability alone is not authority. A force-pushed branch or deleted
remote record can erase availability even when a retained local signature
remains verifiable.

## Acceptance Scenarios Derived from This Story

- A record names verifier content that existed before its run began.
- Appending a record cannot alter the signed record or verifier identity.
- Two successful concurrent runs can be stored without either changing the
  other's verifier claim.
- A verifier-affecting file cannot be placed in an evidence-only excluded path.
- A later verifier revision cannot claim an older record under its own identity.
- Removing an evidence-storage commit does not make a forged record valid,
  though it may harm availability.

## Questions Exposed by This Story

- Which candidate storage shape best preserves the mental model of a validation
  repository while keeping identities simple?
- Does good-evidence storage need append-only proofs, or are signed records and
  protected Git history sufficient initially?
- How are concurrent successful runs appended without a privileged merge bot
  becoming an undeclared assurance authority?
- Which paths affect verifier behavior and therefore must be included in its
  content identity?
- Should the evidence-storage append itself be signed separately from the
  successful validation record?

## Last Updated

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Renumbered this document from EIMP 11 to EIMP 21 and updated its internal references because EIMP 11 collided with another proposal.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Created the successful-record storage story, exposed the circular
Git identity problem, specified the required non-circular lifecycle, compared
four candidate storage shapes, and derived human-view and acceptance
requirements without prematurely selecting an implementation.
