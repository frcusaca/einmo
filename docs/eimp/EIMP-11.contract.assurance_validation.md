# EIMP-11 Contract Catalogue: Assurance Validation

This supplement expands EIMP 11's numbered contracts. The specification in
`EIMP-11.md` governs if the two documents disagree. The contract numbers are
used by the user stories so a reader can see which proposed guarantee makes
each real-world step work.

## What Is Being Guaranteed

Given independently controlled assurance authority and uncompromised execution,
a successful EIMP 11 validation record is intended to guarantee:

> The exact required inventory identified by this record was completely run by
> this exact verifier against this exact subject and tested artifact, under the
> named execution policy, and every required result passed its reviewed
> expectation.

The record does not guarantee that the inventory is sufficient, that every
important behavior was imagined, that the software has no defects, or that a
test's assertion is semantically strong. Those remain matters of specification,
test design, independent review, mutation/meta-testing, and policy.

## Authorities and Artifacts

| Item | Ordinary author | Authority required for assurance | Evidence identity |
|---|---|---|---|
| Subject code | Project developer or agent | Subject repository review policy | Repository, commit, tree, dependencies, tested artifact |
| Colocated tests | Project developer or agent | Subject repository review policy | Included in the subject identity |
| Validation tests and expectations | Validation author | Validation repository review policy | Validation repository commit/tree |
| Required-test inventory | Inventory authority | Separately secured inventory policy | Authenticated inventory digest and version |
| Official execution | Validation runner | Assurance execution policy | Runner, harness, environment/profile, run ID |
| Successful record | Assurance signer | Independently controlled signing policy | Signature over the complete validation claim |

The same person may hold more than one role in a small project, but the
capabilities MUST remain separable. An implementation-writing agent's ordinary
credentials MUST NOT silently authorize inventory reduction or assurance
signing.

## Contract 1 — Protected Inventory

### 1.1 — Stable required-test identity

Every protected suite and case has a stable identifier that is not inferred
only from a Rust function name, path, display string, or discovery order.

This makes the following set comparison meaningful:

```text
required ↔ discovered ↔ selected ↔ started ↔ completed
```

A rename requires an explicit inventory transition. It cannot silently turn a
required case into an absent case.

### 1.2 — Separate authority

The authoritative inventory lives in a separately secured location. A readable
mirror may exist in the validation repository for convenience, but it is not
authoritative and its digest must match the secured version.

The subject-writing agent may read the inventory. Secrecy is not required.
What matters is that the agent cannot update or approve it with the same
authority used to change the subject.

### 1.3 — Authenticated immutable versions

Every official run resolves one authenticated, immutable inventory version.
The resulting record stores its content digest. Mutable labels such as
`current` may help selection, but they are resolved before execution and are
never stored as the evidence identity.

### 1.4 — Explicit update and activation

Adding, removing, renaming, replacing, or exempting a requirement publishes a
new inventory version through an independently authorized operation. The old
version remains available to interpret historical records.

The activation event defines when policy begins requiring the new version.
Whether an inventory may have a staged activation is an open EIMP 11 question;
whatever mechanism is selected must not hide a period in which new behavior is
known but unprotected.

### 1.5 — Fail closed

Validation fails when the inventory is missing, malformed, unauthenticated,
ambiguous, or incompatible with the selected validation-repository revision.
It MUST NOT substitute an empty inventory, an older cached inventory, or a
best-effort discovery result and then report success.

## Contract 2 — Validation Execution

### 2.1 — Exact subject selection

The user selects an immutable subject commit or supplied artifact. A GitHub
branch may help a user choose a SHA, but the run resolves and records the SHA;
it does not later follow the branch. If the object is already available
locally, network access is unnecessary.

### 2.2 — Clean versioned inputs

Official validation refuses tracked changes, untracked files, modified ignored
inputs that affect execution, altered submodules, or other ambiguity in the
subject and validation checkouts. It checks before execution and again before
signing. There is no dirty-tree override that still creates assurance evidence.

An authoring run is distinct: it may be used while developing validation tests,
but it cannot be signed as an official success.

### 2.3 — Reproducible resolution

The runner records dependency locks, submodules, features, toolchain,
environment policy, and supplied artifacts when they can affect behavior. The
binary actually invoked is hashed after it is built or obtained.

This contract identifies the execution; it does not by itself guarantee that
all dependencies or toolchains will remain downloadable forever.

### 2.4 — Exact verifier identity

Before execution, the run fixes the validation-repository revision, evaluator
identity, einmo identity, execution policy, environment/profile, and protected-
inventory digest. A successful result cannot be assembled from cases produced
by different verifier revisions or runs.

### 2.5 — Complete lifecycle accounting

For each required stable identifier, the runner records discovery, selection,
start, and completion. It reconciles identifier sets, not only counts. Two
missing cases and two unexpected cases do not cancel each other out.

### 2.6 — Omission is failure

Ignored, filtered-out, missing, timed-out, crashed, aborted, or otherwise
incomplete required cases prevent a successful validation record. A test
process that exits zero after running zero required cases still fails
validation.

### 2.7 — Authoring is not assurance

Einmo remains usable to create tests, generate outputs, review changes, and
debug failures. Those development actions use the established einmo workflow.
Only a clean official validation over fixed identities may progress to an
assurance signature.

## Contract 3 — Results and Assurance

### 3.1 — Generated output is work material

Execution writes fresh output to `generated/` so it can be inspected and
compared without changing reviewed `output/`, `checked/`, or `verified/`
artifacts.

Existing generated `.einmo` artifacts may contain their normal mechanical
integrity/provenance stamps. Such a stamp does not sign the validation run and
does not turn a failed run into assurance evidence.

### 3.2 — Failure remains unsigned

If any required case fails or remains incomplete, the user receives a failing
terminal result and generated diagnostic output. The run receives no assurance
signature and produces no successful record in the validation repository.

### 3.3 — Only complete success is signed

The assurance signer is invoked only after:

1. every execution identity is fixed;
2. both versioned checkouts pass final cleanliness checks;
3. all required identifier sets reconcile;
4. every required case completes;
5. all required comparisons and policies pass; and
6. the result is serialized as one complete claim.

### 3.4 — The signature binds the whole claim

The successful record binds the canonical subject identity and commit, source
tree, dependency inputs, tested artifact, validation revision, inventory,
harness, policy, environment/profile, required/completed identifiers, unique
run ID, completion time, and successful terminal outcome.

### 3.5 — The validation repository stores good evidence

The validation repository stores successful validation records. It is not a
transparency log of all attempts. Failed generated output may be retained
locally or in a separately configured diagnostic store, but it is not committed
as a validation success.

### 3.6 — Applicability is exact

A record is evidence only for the tuple it names. It cannot be replayed for a
new subject, verifier, inventory, dependency graph, artifact, platform profile,
or policy.

An old record remains historically authentic after the inventory advances. A
current gate may nevertheless reject it as insufficient. Cryptographic
validity and present policy acceptance are distinct decisions.

## Contract 4 — Security Acceptance Tests

### 4.1 — Ignore and filter detection

Acceptance tests alter discovery and selection using `#[ignore]`, name filters,
features, conditional compilation, workspace membership, and equivalent
mechanisms. A required identifier missing from any lifecycle set blocks
success.

### 4.2 — Zero-test detection

Acceptance tests exercise runners that report process success while discovering
or selecting zero required tests. Validation still fails.

### 4.3 — Baseline tampering detection

Acceptance tests edit `.approved` and signed-stage artifacts directly, use
content from another validation revision, and bypass promotion. Verification
rejects the changed bytes, identity mismatch, or unauthorized transition.

### 4.4 — Dirty-checkout rejection

Acceptance tests cover tracked, untracked, ignored-but-material, and submodule
changes before execution, plus mutations introduced during execution. No such
run reaches assurance signing.

### 4.5 — Replay prevention

Acceptance tests substitute each bound identity independently: subject commit,
source tree, lockfile, submodule, tested binary, validation revision, inventory,
harness, policy, environment/profile, and run. Each mismatch is rejected.

### 4.6 — Semantic weakening has an explicit boundary

Einmo cannot reliably infer that arbitrary test code was weakened. It cannot,
in general, understand that a complicated predicate is equivalent to
`condition || true`.

The controls are therefore layered:

- assurance tests and expectations are outside the subject author's ordinary
  write authority;
- validation changes receive independent review;
- the inventory prevents a required identifier from silently disappearing;
- mutation and meta-tests can demonstrate that important tests fail when the
  protected behavior is broken; and
- the successful record identifies the exact reviewed verifier revision.

This limitation MUST be stated honestly in user-facing assurance claims.

## Contract 5 — Human Understanding and Responsible Action

### 5.1 — Protected-system context

Each important protected case should identify the behavior, requirement,
component, or risk it exists to check. A failure view should answer “what part
of the system is this about?” and “why was this case protected?” without
requiring the human to reconstruct intent from a test function alone.

### 5.2 — Development-delta context

The system should compare the selected subject with the last relevant
successfully validated subject. It should expose new and removed commits,
ancestry divergence, affected components, dependency changes, and verifier or
inventory changes. A history rewrite or cherry-pick that omits an older commit
must be conspicuous even when the final tree is presented as ordinary progress.

This is explanatory evidence, not a substitute for testing: a textual change
summary cannot determine whether behavior is correct.

### 5.3 — Attention routing

Einmo should turn a large failed run into a concise discrepancy packet:

- which protected requirements failed or did not execute;
- last known passing subject and current subject;
- expected and generated results;
- relevant subject and verifier changes;
- whether the failure is behavioral, completeness, provenance, or
  infrastructure related; and
- links or commands for inspecting the full raw evidence.

Prioritization must not discard inconvenient failures. The complete evidence
remains reachable even when a summary focuses human attention.

### 5.4 — Explicit resolution choices

After a discrepancy, the interface distinguishes at least:

1. **Repair the subject:** preserve the protected expectation and ask for a new
   implementation commit.
2. **Revise the verifier:** deliberately change tests or expectations through
   ordinary authoring, review, and clean-commit procedures.
3. **Change protected policy:** publish a new inventory or authorized exception
   through the separately secured authority.
4. **Stop:** retain diagnostics and create no assurance result.

No choice automatically promotes the generated output or edits an approved
baseline merely to obtain green status.

### 5.5 — Separate proposal and approval

The record distinguishes who or what proposed a code, verifier, expectation,
or inventory change from who approved it. An agent may prepare a patch and a
human-readable explanation, but its proposal does not become independent
approval through the same credential or automated step.

### 5.6 — Review rationale

Consequential approvals should record what the reviewer believed changed, the
requirements considered, the evidence inspected, and the scope of the
decision. The rationale is bound to exact subject, verifier, inventory, and
artifact identities so it cannot be transplanted to a different change.

A recorded rationale supports accountability and later understanding. It does
not prove the human was diligent or correct.

### 5.7 — Honest coverage

The system distinguishes:

- old protected obligations passed against the new subject;
- new behavior has been identified but is not yet protected;
- new tests exist only in authoring state;
- a new inventory is active but has no successful result; and
- the expanded inventory has passed.

This prevents a green regression run from being presented as comprehensive
coverage of code introduced since the prior validation.

### 5.8 — Reproduction before authority

Humans and agents may inspect the subject, run the verifier, and read generated
output without credentials for expectation promotion, inventory activation,
or assurance signing. The system should make investigation easy while keeping
judgment-changing actions explicit and separately authorized.

## Contract 6 — Structured Tests

The protected test set is not flat. EIMP 11 preserves einmo's existing
arbitrary-depth input hierarchy and dependent-case ordering, then makes the
resolved structure part of assurance:

- **6.1** preserves hierarchical identity;
- **6.2** makes reference and execution-sequence relationships explicit;
- **6.3** rejects duplicate, incomplete, cyclic, or ambiguous structure;
- **6.4** treats a dependency-blocked case as incomplete, never passed;
- **6.5** requires group rollups to preserve every leaf truth;
- **6.6** uses structure to explain affected capabilities and causal failures;
- **6.7** maps structure to requirements, components, risks, and coverage
  gaps; and
- **6.8** makes every structural change a new protected-inventory version.

The detailed semantics and their relationship to current `EinmoId` paths and
`++` dependent chains live in
[the structured-test supplement](EIMP-11.design.structured_tests.md).

## Contract Interaction Summary

| Event | Contracts that decide the outcome |
|---|---|
| Subject branch advances | 2.1, 2.3, 3.4, 3.6 |
| Agent patches colocated tests | 1.2, 2.4, 4.6 |
| Agent removes a prior commit and patches colocated tests | 1.2, 2.1, 2.4, 5.2, 5.3 |
| Required test gains `#[ignore]` | 1.1, 2.5, 2.6, 4.1 |
| A prerequisite fails and descendants cannot run | 2.5, 2.6, 6.2, 6.4–6.6 |
| Runner discovers zero tests | 2.5, 2.6, 4.2 |
| `.approved` is edited directly | 2.2, 4.3 |
| Run fails | 3.1, 3.2, 3.5 |
| Human decides how to resolve a discrepancy | 5.1, 5.3–5.6, 5.8 |
| Run passes completely | 3.3, 3.4, 3.5 |
| Inventory gains new cases | 1.3, 1.4, 3.6 |
| A case moves or a sequence changes | 1.4, 6.1–6.3, 6.8 |
| Old result is offered for a new SHA | 3.4, 3.6, 4.5 |
| Verifier key or policy changes | 3.4, 3.6 |

## Last Updated

**Date**: 2026-09-04  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Created the detailed catalogue for EIMP 11 contracts 1.1–6.8,
including authority boundaries, exact guarantees, interaction with existing
einmo stages and stamps, acceptance-test implications, and the explicit limit
on recognizing semantically weakened assertions. Added human-understanding,
development-delta, attention-routing, responsible-choice, attribution,
rationale, honest-coverage, and reproduction-without-authority contracts.
Added the structured-test contract index and links to its detailed supplement.
