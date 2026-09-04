# EIMP-11 User Story: Week 2 — Verify Old Obligations, Then Expand Coverage

## Story

Several days have passed since the last independent validation. The project
has accumulated features and fixes. The verifier opens the validation
repository, selects the project's new SHA from GitHub, and runs the existing
protected suites before writing tests for the new code.

This story defines the ordinary two-pass rhythm of extra-repository
verification:

1. establish that previously protected behavior still works; then
2. instrument, review, protect, and validate the new behavior.

## Starting State

The most recent applicable successful record is:

```text
R8
  subject:              S8
  validation revision: V4
  protected inventory: I4
  profile:              release-linux
```

The project has since advanced to `S17`. The validation repository and secured
inventory have not yet been changed. **Contracts relied upon: 1.3, 2.4, 3.4,
3.6.**

## Pass One — Check Existing Obligations

### 1. Select the exact subject SHA

The verifier selects `S17`. Einmo fetches the exact Git object from GitHub if
necessary and prepares or checks an isolated clean subject checkout. It fixes
the clean validation revision `V4` and authenticated inventory `I4` before
execution. **Contracts relied upon: 1.3, 1.5, 2.1–2.4.**

### 2. Present the development briefing

Einmo relates `S17` to prior subject `S8` and summarizes:

- whether `S17` descends from `S8`;
- commits and major components changed;
- dependency and toolchain changes;
- protected capabilities likely affected; and
- known new components or features without protected-case mappings.

The briefing helps the human understand what happened without asserting that
an automated summary is complete or correct. **Contracts relied upon: 5.2,
5.7, 6.6, 6.7.**

### 3. Execute the old protected inventory unchanged

Einmo runs every case required by `I4`. Hierarchy groups results by capability;
sequence edges put prerequisites before dependent checks. Required,
discovered, selected, started, and completed identifiers must reconcile.
**Contracts relied upon: 2.5, 2.6, 6.1–6.5.**

An `#[ignore]` marker, filter, missing prerequisite, cycle, or zero-test runner
cannot create a successful result. **Contracts relied upon: 4.1, 4.2,
6.3, 6.4.**

### 4. Interpret the first pass honestly

The suite passes. Einmo signs and stores `R17-I4`, bound to `S17 + V4 + I4`
and the exact tested artifact and environment profile. Appending the result
does not redefine `V4`. **Contracts relied upon: 3.3–3.5, 3.7.**

The human has learned:

> The past few days' work did not break the behavior already protected by
> inventory `I4`.

The human has **not** learned:

> All new behavior introduced between `S8` and `S17` is now protected.

The UI must display both facts. **Contracts relied upon: 3.6, 5.7.**

If pass one fails, einmo instead leaves generated output, presents a structured
discrepancy packet, and creates no assurance record. The verifier investigates
or sends the subject back for repair before expanding coverage. **Contracts
relied upon: 3.1, 3.2, 5.1–5.4, 6.4–6.6.**

## Pass Two — Add Coverage for New Work

### 5. Identify new assurance obligations

The verifier studies the change briefing, subject documentation, relevant
code, specifications, and risks. New cases are mapped into the existing
hierarchy or into new capabilities. Multi-step behavior receives explicit
sequence/reference relationships rather than depending on discovery order.
**Contracts relied upon: 5.1, 5.2, 6.1, 6.2, 6.7.**

Automated tools or agents may propose cases, but their suggestions do not
become protected requirements or approved expectations automatically.
**Contracts relied upon: 5.5.**

### 6. Author and inspect tests

The verifier edits the validation repository and uses ordinary einmo authoring
workflow:

1. add input cases and evaluator behavior;
2. generate fresh outputs;
3. inspect generated versus reviewed stages;
4. decide whether the observed behavior is correct;
5. promote through the established stages using the required authority; and
6. record rationale for consequential new expectations.

The validation checkout is dirty during authoring, so these runs cannot create
official assurance evidence. Existing signed stages are never rewritten merely
because generation differed. **Contracts relied upon: 2.7, 3.1, 4.3,
5.4–5.6, 5.8.**

### 7. Commit and review validation revision `V5`

After review, the new cases, structure, adapters, and expectations are
committed as `V5`. The official run will later require this exact clean
revision. **Contracts relied upon: 2.2, 2.4, 5.5.**

### 8. Activate inventory `I5`

The separate inventory authority authenticates and activates `I5`, containing
all prior requirements plus the newly reviewed stable identifiers and
structure. Renames, removed cases, and changed sequence edges are explicit.
`I4` remains immutable history. **Contracts relied upon: 1.1–1.4, 6.8.**

The system must display the transition state honestly. Once policy requires
`I5`, `R17-I4` remains authentic historical evidence but is insufficient for
that current policy. **Contracts relied upon: 3.6, 5.7.**

### 9. Run the expanded tuple

Einmo validates exact tuple `S17 + V5 + I5`. If every old and new requirement
completes and passes, einmo signs and stores stronger record `R17-I5`.
**Contracts relied upon: 2.1–2.6, 3.3–3.5, 6.1–6.5.**

If a new case exposes a defect, generated output remains available, no `I5`
success record exists, and the human chooses whether to repair `S17` through a
new subject commit or revise the proposed validation through review. **Contracts
relied upon: 3.2, 5.3–5.8.**

## Timeline

```text
S8 passed V4 + I4
        │
        ├── several days of subject development ──▶ S17
        │                                             │
        │                              pass one: V4 + I4
        │                                             │
        │                                  ┌──────────┴──────────┐
        │                                  │                     │
        │                                fail                  pass
        │                                  │                     │
        │                     diagnostics; no assurance    R17-I4 signed
        │                                                        │
        │                                             author/review V5
        │                                             activate I5
        │                                                        │
        │                              pass two: S17 + V5 + I5
        │                                                        │
        │                                  ┌─────────────────────┴──────┐
        │                                  │                            │
        │                                fail                         pass
        │                                  │                            │
        └──────────────────── diagnostics; no assurance            R17-I5 signed
```

## Contract Trace

| Story event | Contracts |
|---|---|
| Fetch exact new SHA | 2.1 |
| Compare development with last relevant success | 5.2, 5.7 |
| Hold old requirements fixed for the first pass | 1.3, 2.4 |
| Execute every old obligation | 2.5, 2.6, 6.1–6.5 |
| Describe old-coverage success narrowly | 3.6, 5.7 |
| Author new tests without producing assurance | 2.7, 3.1, 5.8 |
| Review new expectations and rationale | 4.3, 5.4–5.6 |
| Activate expanded structured inventory | 1.1–1.4, 6.8 |
| Reject old record under new policy | 3.6, 5.7 |
| Sign only the complete expanded pass | 3.3–3.5 |

## Questions Exposed by This Story

- When should `I5` become active: when its verifier is reviewed, immediately
  before its first official run, or only after a first pass?
- How does the system display “known new code lacks protected coverage” without
  claiming perfect automatic code-to-requirement mapping?
- Should pass one always precede verifier edits, or may a user explicitly skip
  it while accepting the loss of a clean regression comparison?
- How are test author, expectation reviewer, inventory activator, and assurance
  signer separated in small teams?
- Can the same subject have several simultaneous successful records for
  different platforms and profiles?
- Where are good results appended so they do not recursively change the
  verifier content identity?

## Last Updated

**Date**: 2026-09-04  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Created the central Week 2 EIMP 11 user story. Defined the
two-pass workflow of validating old obligations first, then authoring,
reviewing, protecting, and validating new structured coverage, with explicit
contract references, honest intermediate claims, and non-circular success
storage.
