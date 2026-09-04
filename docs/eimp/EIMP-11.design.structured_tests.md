# EIMP-11 Design Supplement: Structured Tests

## Purpose

EIMP 11 needs more than a flat inventory of case identifiers. Humans understand
systems through parts, capabilities, scenarios, and change over time. Einmo's
inputs already have two useful kinds of structure:

1. **Hierarchy:** the `input/` directory tree may be arbitrarily deep, and all
   stage directories mirror it through stable `EinmoId` paths.
2. **Sequence/dependency:** dependent input names form reference chains using
   the configured separator (currently commonly `++`); einmo orders references
   before dependents and records deterministic differences from the reference
   result.

EIMP 11 preserves those mechanisms and makes their assurance meaning explicit.
It may add authenticated structural metadata, but it must not flatten or
replace the existing directory and dependent-case model.

## Why Structure Matters for Assurance

Suppose an agent removes a commit that implemented one foundational behavior.
Forty external cases may fail. A flat report presents forty unrelated red
lines. A structured report can say:

```text
authentication
└── session renewal                         FAILED
    ├── initial token                       passed
    ├── renewal before expiry               failed
    ├── renewal after clock adjustment      blocked/failed
    └── revoked-session rejection           failed
```

The human can see the affected capability, the earliest failing step, and the
downstream consequences. Every leaf result remains available; grouping does
not hide failures.

Structure also lets einmo express a coverage gap honestly. If a new
`authentication/passkeys/` capability appears in the subject but the protected
inventory has no corresponding requirement mapping, “all existing tests pass”
can be displayed alongside “new capability has no protected coverage.”

## Existing Semantics That Remain

- An input-relative path is a stable stage-independent `EinmoId`.
- Nested input paths are mirrored beneath `generated/`, `output/`, `checked/`,
  and `verified/`.
- A malformed or ambiguous input tree is a failure.
- Existing dependent-case naming resolves a reference in the same directory.
- References run before their dependents.
- A dependent case may compare its output with the reference output through
  the existing `DIFF` and metadata behavior.
- Body content remains evaluator-defined opaque text.

Directory nesting currently provides organization; it does not automatically
mean that a parent directory is an executable case. Likewise, current
dependent ordering and reference differences do not automatically create a
shared mutable, stateful test session. EIMP 11 must not claim stronger sequence
semantics than the evaluator actually implements.

## Proposed Structural Model

For assurance purposes, the protected inventory should describe a graph with
two separately meaningful relations:

```text
contains(parent, child)       # hierarchy
precedes(prerequisite, case)  # sequence/dependency
```

The simplest form remains a tree of groups containing leaf cases, plus optional
directed edges between cases. The graph must be deterministic and acyclic for
an official validation profile.

A possible conceptual example is:

```text
system: authentication
  capability: session-renewal
    scenario: ordinary-renewal
      case: issue-token
      case: renew-token
        after: issue-token
      case: use-renewed-token
        after: renew-token
```

This is conceptual design, not a committed file format.

## Contract 6 — Structured Tests

### 6.1 — Hierarchical identity is preserved

Every case belongs to a deterministic hierarchy. Existing nested `EinmoId`
paths are the initial representation. The protected inventory authenticates
the hierarchy as well as the leaf identifiers, so moving a case between
capabilities is an explicit inventory change.

### 6.2 — Sequence relationships are explicit

Any prerequisite, reference, or ordering edge that affects execution or
interpretation is part of the authenticated inventory. Existing `++`
dependent-case chains map into this model rather than being discarded.

The final schema must distinguish at least:

- a **reference/comparison edge**, where one independently evaluated case is
  the baseline for another case's `DIFF`; and
- an **execution prerequisite**, where a later case cannot meaningfully run
  unless an earlier step completed successfully.

The two are not assumed to be equivalent.

### 6.3 — Structure is deterministic and validatable

The runner rejects duplicate identities, missing parents, missing referenced
cases, cycles, ambiguous ordering, and structure that disagrees with the
selected validation revision. It does not silently repair the graph or drop an
edge.

### 6.4 — Blocked is not passed

When a prerequisite fails, a dependent case may receive a typed `blocked`
outcome instead of executing. It remains incomplete for assurance purposes and
prevents a successful validation record. The report identifies the earliest
causal failure and every blocked descendant.

### 6.5 — Rollups preserve leaf truth

A group or sequence summary may report aggregate state, but it cannot replace
or contradict leaf results. A group is successful only when every required
descendant satisfies its lifecycle and result obligations. Unexpected or
unmapped leaves remain visible.

### 6.6 — Structure supports human explanation

Discrepancy views use hierarchy and sequence to show:

- the affected system area and protected behavior;
- the earliest failing or missing prerequisite;
- failures likely downstream of the same cause;
- unaffected sibling capabilities; and
- the exact leaf evidence behind every summary.

This implements EIMP 11 contracts 5.1 and 5.3 without relying on a language
model to invent the system organization at report time.

### 6.7 — Structure supports coverage accounting

Inventory nodes may link to specification clauses, subject components, risks,
or feature identifiers. A development-delta view can then report areas changed
since the prior validated SHA that have no mapped protected requirement.

The mapping is evidence supplied and reviewed by people or tooling; it is not
proof that coverage is semantically complete.

### 6.8 — Structural evolution is explicit

Adding a capability, moving a case, changing a prerequisite, or splitting a
sequence creates a new protected-inventory version under contract 1.4.
Historical records continue to reference the old graph. A new structure cannot
retroactively change the meaning of an old validation record.

## Execution and Reporting Example

Assume the required cases are:

```text
accounts/create
accounts/login
accounts/session/issue
accounts/session/renew       after accounts/session/issue
accounts/session/use         after accounts/session/renew
```

If `accounts/session/renew` fails:

| Case | Execution outcome | Assurance outcome |
|---|---|---|
| `accounts/create` | completed/pass | satisfied |
| `accounts/login` | completed/pass | satisfied |
| `accounts/session/issue` | completed/pass | satisfied |
| `accounts/session/renew` | completed/fail | not satisfied |
| `accounts/session/use` | blocked by `renew` | not satisfied |

The run fails and remains unsigned under contracts 2.5, 2.6, 3.2, and 6.4.
The human-facing summary may lead with the `renew` root cause, but the record of
the blocked `use` case remains visible under 5.3 and 6.5.

## Relationship to the Protected Inventory

The separately secured inventory must authenticate enough structure to prevent
these attacks:

- move a required case into an unprotected directory;
- remove a parent group so descendants disappear from selection;
- delete or redirect a prerequisite edge;
- reorder a stateful scenario into an easier sequence;
- introduce a cycle that causes cases to be skipped;
- report only the first passing branch of a hierarchy; or
- collapse many required leaves into one reassuring group count.

Whether the inventory stores the full graph or authenticates a canonical graph
produced from the validation repository remains an open schema decision. In
either design, the assurance signer must bind the resolved graph digest.

## Questions for the User Stories

- Which real suites use directory hierarchy only for organization, and which
  directories represent meaningful system capabilities?
- Are current `++` chains sufficient for comparison sequences, or do real
  workflows require state shared between steps?
- When one step fails, should safe independent descendants still run for
  diagnostic value even though the validation is already unable to succeed?
- How are subject components or specification clauses mapped to inventory
  nodes without creating burdensome duplicate metadata?
- What report best distinguishes one root-cause failure plus forty blocked
  descendants from forty independent regressions?

## Last Updated

**Date**: 2026-09-04  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Created the structured-test design supplement, preserving einmo's
existing hierarchical input paths and dependent-case ordering while proposing
contracts 6.1–6.8 for authenticated hierarchy, explicit sequence relations,
typed blocked outcomes, truthful rollups, human explanation, coverage mapping,
and structural evolution.
