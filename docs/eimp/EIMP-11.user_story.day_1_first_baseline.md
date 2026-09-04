# EIMP-11 User Story: Day 1 — Establish the First Baseline

## Story

Mira maintains an application repository and wants assurance tests whose
requirements cannot be silently reduced by the implementation-writing agent.
She creates a separate validation repository. On Day 1 there is no prior
successful validation record and no protected inventory, so the first task is
to establish a reviewed starting point without pretending that generated
behavior is already correct.

## Walkthrough

### 1. Select the first subject

Mira selects application commit `S1`. The verifier resolves the exact commit,
its tree, dependency lock, submodules, and eventually the binary actually
tested. A branch name such as `main` may help Mira find `S1`, but the branch is
not the subject identity. **Contracts relied upon: 2.1, 2.3.**

### 2. Author the initial external tests

Mira implements conformance and regression cases in the validation repository.
Each intended requirement receives a stable identifier. She runs the tests
repeatedly, inspects `generated/`, and uses existing einmo comparison and
promotion procedures to establish reviewed expectations.

These are authoring runs. They help develop the suite but create no assurance
record. Generated behavior is not accepted merely because the application
produced it. **Contracts relied upon: 1.1, 2.7, 3.1.**

### 3. Review and commit the verifier

The tests, evaluator adapters, fixtures, and expectations are reviewed and
committed as validation revision `V1`. Direct edits to `.approved` or signed
stage files do not substitute for promotion and review. **Contracts relied
upon: 2.4, 4.3, 4.6.**

### 4. Publish the first protected inventory

An independently authorized operation publishes inventory `I1`. It names the
stable identifiers that `V1` must discover and execute. The secured inventory
has an authenticated digest and recorded activation. It is not merely an
editable list beside the test source. **Contracts relied upon: 1.1–1.4.**

### 5. Perform the first official validation

The runner fixes the tuple `S1 + V1 + I1`, confirms both checkouts are clean,
builds or resolves the tested artifact, and executes the required cases. It
reconciles the required, discovered, selected, started, and completed
identifier sets. **Contracts relied upon: 1.5, 2.1–2.6.**

### 6. Record success or diagnose failure

If every case completes and passes, einmo creates successful validation record
`R1`. Its assurance signature binds `S1`, `V1`, `I1`, the tested artifact, and
the execution context. **Contracts relied upon: 3.3–3.5.**

If anything fails, Mira receives generated diagnostic output instead. The run
has no assurance signature and the validation repository gains no successful
record. She corrects the subject, tests, or expectations through their proper
review paths, commits new identities, and begins another official run.
**Contracts relied upon: 3.1, 3.2, 4.3.**

## What Day 1 Establishes

The first record makes one precise claim:

> Inventory `I1`, implemented and reviewed at verifier revision `V1`, passed
> against exact subject `S1` and the named tested artifact.

It does not claim that `I1` is complete, that `S1` has no bugs, or that every
future subject revision is valid.

## Contract Trace

| Story event | Contract |
|---|---|
| Stable case names are assigned | 1.1 |
| Inventory cannot be rewritten by the subject agent | 1.2 |
| First inventory is authenticated and activated | 1.3, 1.4 |
| Missing inventory cannot turn into an empty green run | 1.5 |
| Exact application commit and binary are selected | 2.1, 2.3 |
| Official inputs are clean and fixed | 2.2, 2.4 |
| All required cases are accounted for | 2.5, 2.6 |
| Test development remains possible without producing assurance | 2.7, 3.1 |
| Failure produces diagnostics only | 3.2 |
| Complete success produces the first record | 3.3–3.5 |
| Direct baseline editing is not acceptance | 4.3 |

## Questions Exposed by This Story

- Who may approve and activate `I1` in a one-maintainer project?
- How does the first reviewer distinguish a correct expected result from a
  snapshot that merely records current behavior?
- Must the inventory identify one compatible validation commit or a permitted
  range?
- Can the official run consume a prebuilt artifact, and what provenance must
  accompany it?

## Last Updated

**Date**: 2026-09-04  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Created the Day 1 EIMP 11 user story, grounding initial test
authoring, reviewed expectations, protected-inventory publication, clean
official execution, and the first successful validation record in contracts
1.1–4.6.
