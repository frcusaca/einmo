# EIMP-21 User Story: A Structured Sequence Exposes One Root Failure

## Story

A commerce system has hundreds of validation cases. They are organized
hierarchically by capability, and several pricing cases form a sequence of
increasing changes:

```text
commerce/
└── checkout/
    ├── inventory/
    ├── payment/
    └── pricing/
        ├── ordinary_order.foo
        ├── ordinary_order++discount.foo
        └── ordinary_order++discount++tax.foo
```

An agent changes monetary rounding. The tax case becomes wrong, and many
downstream scenarios produce discrepancies. The human needs to understand
whether this is one causal defect, many independent defects, or an intended
policy change.

## Existing Einmo Meaning

Einmo already preserves the nested `input/` hierarchy in every stage through
the input-relative `EinmoId`. It also resolves a `++` dependent case to its
reference in the same directory, orders the reference first, and can record a
deterministic `DIFF` between their independently evaluated outputs.

That is a **comparison sequence**. It does not necessarily mean the cases share
one mutable process or that `ordinary_order++discount.foo` consumes runtime
state produced by `ordinary_order.foo`. EIMP 21 must retain this current
behavior and name it accurately. **Contracts relied upon: 6.1, 6.2.**

## Official Validation

### 1. Resolve the protected structure

The secured inventory authenticates both the required leaf IDs and their
hierarchical/reference relationships. Removing the `pricing/` group, moving a
case to an unprotected path, or redirecting the tax case to an easier reference
changes the inventory and cannot happen implicitly. **Contracts relied upon:
1.1–1.4, 6.1–6.3, 6.8.**

### 2. Evaluate references before dependents

The runner evaluates the ordinary order, then the discounted order, then the
discounted-and-taxed order. Each remains a required leaf with its own lifecycle
and result. A successful parent summary cannot substitute for those leaf
outcomes. **Contracts relied upon: 2.5, 2.6, 6.2, 6.5.**

### 3. Detect the rounding discrepancy

The ordinary and discount cases match reviewed expectations. The tax case
differs. Other tax-sensitive scenarios elsewhere in the pricing hierarchy also
fail.

The run remains unsigned and no successful record is stored. **Contracts
relied upon: 3.1, 3.2, 3.5.**

### 4. Present structure without inventing certainty

The discrepancy packet says:

```text
commerce / checkout / pricing                   FAILED
├── ordinary order                              passed
├── add discount                                passed
├── add tax                                     failed: rounding discrepancy
└── related tax-sensitive cases
    ├── 11 independently failed
    └── 8 depend on failed prerequisite         blocked/incomplete
```

The report prioritizes the earliest shared discrepancy and provides all leaf
evidence. It may say the failures are structurally related; it must not assert
without evidence that one code line is certainly their sole cause. **Contracts
relied upon: 5.1, 5.3, 6.4–6.6.**

### 5. Relate the failure to development

The change briefing shows that monetary rounding code changed since the last
relevant successful subject. It links that component to the protected pricing
requirements. This focuses review without treating a change correlation as a
proof of causation. **Contracts relied upon: 5.2, 6.7.**

## Human Decision

The reviewer considers the requirement behind tax rounding.

- If the implementation is wrong, the agent prepares a new subject commit and
  the same verifier/inventory runs again.
- If the tax policy intentionally changed, the reviewer revises the relevant
  validation expectations through authoring and promotion, records rationale,
  commits a new verifier revision, and activates a new inventory version if
  structure or requirements changed.
- If the evidence is ambiguous, the reviewer stops with no assurance record.

No path automatically promotes all tax-related generated output merely because
many cases changed together. **Contracts relied upon: 4.3, 5.4–5.8.**

## Stateful-Sequence Variant

Another suite tests a true workflow:

```text
create order → authorize payment → capture payment → refund payment
```

Here `capture` cannot execute meaningfully if `authorize` failed. This is an
**execution-prerequisite sequence**, not merely a reference used to compute a
`DIFF`.

If authorization fails:

- `authorize` records the failing result;
- `capture` and `refund` record typed `blocked` outcomes naming the failed
  prerequisite;
- independent inventory and pricing branches may continue for diagnostic value;
- blocked cases are not counted as passed or completed assurance obligations;
  and
- the whole validation remains unsuccessful.

**Contracts relied upon: 2.5, 2.6, 6.2, 6.4–6.6.**

This variant may require new evaluator or runner semantics. The existing `++`
reference mechanism must not be silently redefined as shared state. Whether
stateful workflows belong in EIMP 21's first implementation remains open.

## Contract Trace

| Story event | Contracts |
|---|---|
| Preserve nested capability organization | 6.1 |
| Preserve and authenticate `++` reference chains | 1.3, 6.2, 6.8 |
| Reject missing references, cycles, or ambiguous structure | 1.5, 6.3 |
| Keep every required leaf outcome | 2.5, 2.6, 6.5 |
| Mark true prerequisite descendants blocked, not passed | 6.4 |
| Group related failures while exposing raw leaves | 5.3, 6.5, 6.6 |
| Link changed component to protected capability | 5.2, 6.7 |
| Leave the failed run unsigned | 3.2, 3.5 |
| Require deliberate human resolution | 5.4–5.8 |

## Guarantees and Limits

Structure improves explanation and completeness. It does not prove causation,
semantic coverage, or test quality. A hierarchy authored dishonestly can
mislead; therefore its exact version is protected and reviewed. An agent-
generated root-cause suggestion is supporting analysis, never the signed
validation fact.

## Acceptance Scenarios Derived from This Story

- Nested input paths resolve to stable, authenticated hierarchical identities.
- A moved leaf creates an inventory difference rather than disappearing.
- A `++` chain runs in deterministic reference order and retains current DIFF
  semantics.
- A missing reference or sequence cycle prevents validation.
- A group cannot pass while a required descendant failed or was blocked.
- A root-cause summary retains links to every leaf result.
- Independent branches may continue after one sequence becomes blocked.
- Stateful prerequisite semantics, if implemented, remain distinct from
  comparison-reference semantics.

## Questions Exposed by This Story

- Does EIMP 21 initially support only existing comparison sequences, or also
  true stateful execution prerequisites?
- If a sequence is already unable to pass, which independent cases should
  continue to maximize diagnostic value?
- Is hierarchy entirely derived from `EinmoId` paths, or may inventory metadata
  define additional semantic groupings?
- How is a root-cause ranking explained and verified without allowing an
  automated summary to hide secondary failures?
- Can one case belong to several requirement/component views while retaining
  one stable execution identity?

## Last Updated

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Renumbered this document from EIMP 11 to EIMP 21 and updated its internal references because EIMP 11 collided with another proposal.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Created the structured-sequence story, grounding existing nested
`EinmoId` paths and `++` comparison chains in a pricing failure, distinguishing
them from true stateful prerequisites, and deriving reporting, completeness,
human-decision, and acceptance requirements.
