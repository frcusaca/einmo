# EIMP-21 Traceability: Contracts to User Stories

## Purpose

This supplement audits whether EIMP 21's contracts are grounded in realistic
use. It does **not** report implementation status. “Grounded” means a story
demonstrates why the contract is needed and what a user expects to observe.

The EIMP specification governs if this matrix disagrees with it.

## Story Key

| Key | Story |
|---|---|
| D1 | [Day 1 — first baseline](EIMP-21.user_story.day_1_first_baseline.md) |
| D2 | [Day 2 — routine regression and failure](EIMP-21.user_story.day_2_regression_and_failure.md) |
| W2 | [Week 2 — verify, then expand coverage](EIMP-21.user_story.week_2_expand_coverage.md) |
| RC | [Removed commit and patched subject tests](EIMP-21.user_story.detected_many_broken_cases.md) |
| IGN | [Ignored case, suite, or validation job](EIMP-21.user_story.detected_ignored_suite.md) |
| SEQ | [Hierarchical and sequential failure cascade](EIMP-21.user_story.sequential_failure_cascade.md) |
| INV | [Unavailable or stale protected inventory](EIMP-21.user_story.inventory_unavailable_or_stale.md) |
| MUT | [Checkout or artifact mutated during execution](EIMP-21.user_story.checkout_mutated_during_run.md) |
| STORE | [Store success without changing verifier identity](EIMP-21.user_story.store_success_without_changing_verifier.md) |
| LEG | [Legitimate intentional behavior change](EIMP-21.user_story.intentional_behavior_change.md) |
| Y2 | [Year 2 — long-term evidence evolution](EIMP-21.user_story.year_2_evidence_evolution.md) |

## Reading the Status Column

- **Grounded:** at least one story exercises the normal and consequential
  behavior clearly enough to retain the contract.
- **Partially grounded:** the story establishes the guarantee but exposes a
  material variant needing another story or design decision.
- **Design gap:** the story depends on the guarantee, but the EIMP has not yet
  selected a mechanism precise enough to plan implementation.

## Contract 1 — Protected Inventory

| Contract | Stories | Status and exposed work |
|---|---|---|
| 1.1 Stable required-test identity | D1, W2, RC, IGN, SEQ, Y2 | Grounded at case level. Suite-level identity and aliases across legitimate renames still need schema design. |
| 1.2 Separate authority | D1, RC, IGN, Y2 | Grounded. Concrete secured storage and small-team role separation remain design gaps. |
| 1.3 Authenticated immutable versions | D1, D2, W2, RC, SEQ, INV, Y2 | Grounded. Canonical encoding and authentication mechanism are open. |
| 1.4 Explicit update and activation | D1, W2, RC, IGN, SEQ, LEG, Y2 | Grounded. LEG exercises a deliberate versioned policy transition and future activation. |
| 1.5 Fail closed | D1, W2, SEQ, INV | Grounded for unavailable, stale, and corrupt inventory. Disaster recovery remains open. |
| 1.6 Selection resists rollback | INV, Y2 | Grounded for offline exact use and authentic-but-stale substitution. Selection-state mechanism remains open. |
| 1.7 Requirement meaning is versioned | LEG, Y2 | Grounded by an intentional semantic change that must preserve old-record meaning. Identifier/version schema remains open. |
| 1.8 Activation does not depend on passing | W2, LEG | Grounded by a new confirmed case that may initially fail. Transitional policy remains open. |

## Contract 2 — Validation Execution

| Contract | Stories | Status and exposed work |
|---|---|---|
| 2.1 Exact subject selection | D1, D2, W2, RC, Y2 | Grounded for Git SHA. Supplied binaries and legitimate repository migration need more design. |
| 2.2 Clean versioned inputs | D1, D2, W2, RC, MUT | Grounded for normal, authoring, and post-run transitions. “Ignored but material” files need precise classification. |
| 2.3 Reproducible resolution | D1, D2, W2, RC, MUT, Y2 | Partially grounded. MUT covers isolated builds and tested artifacts; the identity-bearing environment field set remains open. |
| 2.4 Exact verifier identity | D1, D2, W2, RC, Y2 | Grounded. Compatibility between verifier and inventory needs schema design. |
| 2.5 Complete lifecycle accounting | D1, D2, W2, RC, IGN, SEQ, Y2 | Strongly grounded. The adapter/report protocol carrying stable identifiers is open. |
| 2.6 Omission is failure | D1, D2, W2, RC, IGN, SEQ, Y2 | Strongly grounded. Typed exemption semantics, if any, remain unresolved. |
| 2.7 Authoring is not assurance | D1, W2, RC, LEG | Grounded, including existing staged expectation review. CLI/API separation between authoring and official validation is open. |
| 2.8 Invocation is independently accountable | IGN | Grounded by the exact failure that motivated it. Scheduler and consuming-gate ownership are design gaps. |
| 2.9 Execution inputs are immutable | MUT | Grounded by transient write/restore, build output, artifact replacement, and permitted generated-output variants. Isolation threat level and platform mechanism remain open. |

## Contract 3 — Results and Assurance

| Contract | Stories | Status and exposed work |
|---|---|---|
| 3.1 Generated output is work material | D1, D2, W2, RC, SEQ | Grounded in existing einmo procedure. The diagnostic retention layout remains open. |
| 3.2 Failure receives no assurance signature | D1, D2, W2, RC, IGN, SEQ | Strongly grounded. Mechanical `.einmo` stage stamps are explicitly not assurance signatures. |
| 3.3 Only complete success is signed | D1, D2, W2, RC, IGN | Grounded. The exact atomic transition into signing needs design. |
| 3.4 Signature binds the whole claim | D1, D2, W2, RC, INV, STORE, Y2 | Grounded. Native record schema and canonical encoding are open. |
| 3.5 Validation repository stores good evidence | D1, D2, W2, RC, SEQ, STORE | Grounded as product semantics. STORE compares physical ledger layouts. |
| 3.6 Applicability is exact | D2, W2, RC, IGN, INV, STORE, Y2 | Strongly grounded by new SHA, stale inventory, verifier changes, and long-term policy. |
| 3.7 Result storage is non-circular | D1, D2, W2, STORE | Behavior is grounded; STORE compares four mechanisms without selecting one. This remains a design decision. |
| 3.8 Evidence selection is deterministic | W2, RC, IGN, INV, STORE, Y2 | Strongly grounded. Exact policy selection rules remain open. |

## Contract 4 — Security Acceptance Tests

| Contract | Stories | Status and exposed work |
|---|---|---|
| 4.1 Ignore and filter detection | W2, RC, IGN, Y2 | Strongly grounded across inner case, suite, and whole-job boundaries. |
| 4.2 Zero-test detection | W2, IGN | Grounded; needs adapter and end-to-end acceptance fixtures. |
| 4.3 Baseline tampering detection | D1, W2, RC, SEQ, LEG | Grounded in existing signed-stage and promotion semantics plus new verifier identity. |
| 4.4 Dirty-checkout rejection | W2, RC, MUT | Grounded together with execution-time immutability; implementation acceptance tests remain. |
| 4.5 Replay prevention | D2, RC, INV, STORE, Y2 | Grounded across subject, inventory, verifier, and policy time; each identity dimension still needs its own implementation test. |
| 4.6 Semantic weakening boundary | D1, W2, RC | Grounded as an honest non-guarantee. Mutation/meta-test integration remains design work. |

## Contract 5 — Human Understanding and Responsible Action

| Contract | Stories | Status and exposed work |
|---|---|---|
| 5.1 Protected-system context | W2, RC, IGN, SEQ | Grounded through capability and requirement views. Metadata authorship and review are open. |
| 5.2 Development-delta context | W2, RC, SEQ | Grounded by ordinary advancement and removed history. Repository migration and merge-heavy histories need examples. |
| 5.3 Attention routing | W2, RC, IGN, SEQ, Y2 | Grounded. Root-cause prioritization must remain explainable and preserve all leaves. |
| 5.4 Explicit resolution choices | W2, RC, IGN, SEQ, LEG, Y2 | Strongly grounded by accidental and intentional behavior-change paths. |
| 5.5 Separate proposal and approval | W2, RC, LEG, Y2 | Grounded. Small-team authority configuration remains open. |
| 5.6 Review rationale | W2, RC, LEG, Y2 | Grounded. Required fields and which decisions demand rationale are open. |
| 5.7 Honest coverage | W2, RC, IGN, Y2 | Strongly grounded, especially by the first versus second Week 2 pass. |
| 5.8 Reproduction before authority | W2, RC | Grounded. Credential and local/offline UX need design. |
| 5.9 Actionable escalation | RC, IGN | Grounded for local failure and optional delivery. Acknowledgement policy and integration scope are open. |

## Contract 6 — Structured Tests

| Contract | Stories | Status and exposed work |
|---|---|---|
| 6.1 Hierarchical identity | W2, RC, IGN, SEQ, Y2 | Grounded in existing nested `EinmoId` paths. Multiple semantic views beyond the path tree are open. |
| 6.2 Explicit sequence relationships | W2, RC, SEQ, Y2 | Grounded for current comparison/reference chains; true stateful prerequisites may be later scope. |
| 6.3 Validatable structure | W2, SEQ, Y2 | Grounded for missing nodes and cycles. Canonical graph encoding is open. |
| 6.4 Blocked is not passed | W2, RC, SEQ | Grounded. Diagnostic continuation after a prerequisite failure needs policy. |
| 6.5 Truthful rollups | W2, RC, IGN, SEQ, Y2 | Strongly grounded by many-failure and multi-profile views. |
| 6.6 Structure supports explanation | W2, RC, SEQ | Grounded. Automatic root-cause claims must remain evidence-linked and qualified. |
| 6.7 Structure supports coverage | W2, RC, SEQ | Grounded conceptually. Code/component-to-requirement mapping is a major design gap. |
| 6.8 Explicit structural evolution | W2, RC, IGN, SEQ, Y2 | Grounded through additions, moves, sequence changes, and retirements. |

## Gaps Discovered by the Stories

The following issues were not obvious from the initial architecture and now
need normative decisions:

1. **Non-circular result storage:** the validation repository is both verifier
   source and the store of good results. Appending a result must not change the
   verifier identity that result names. This became contract 3.7.
2. **Prior-evidence selection:** “last relevant success” must be selected by
   visible deterministic policy, not convenience. This became contract 3.8.
3. **Invocation outside the ignored function:** an einmo library call cannot
   detect that it never ran. Independent suite invocation plus a consuming
   evidence gate became contract 2.8.
4. **Alert delivery levels:** emitting an event, delivering a notification, and
   receiving human acknowledgement are separate claims. This became contract
   5.9.
5. **Inventory activation and rollback:** Week 2 exposed tension between
   activating new requirements immediately and waiting for a first pass. LEG
   resolves the default: confirmed obligations activate independently of
   subject success, while transition policies remain explicit.
6. **Comparison versus stateful sequence:** current `++` chains order
   independently evaluated references; they must not silently acquire shared-
   state semantics.
7. **Identity versus archival:** a digest preserves meaning but cannot ensure
   old dependencies, toolchains, and binaries remain available.

## Next Stories Needed

To close the weakest rows before an implementation plan, add stories for:

- a legitimate test retirement or temporary exception in a small team;
- several competing prior successes across platforms and inventories;
- a real stateful sequence, if that is intended for initial EIMP 21 scope;
- a repository migration and assurance-key compromise; and
- an uncovered new component reaching a release decision.

## Last Updated

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Renumbered this document from EIMP 11 to EIMP 21 and updated its internal references because EIMP 11 collided with another proposal.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Created the consolidated EIMP 21 contract-to-story matrix, audited
every contract against eleven user stories, distinguished grounded behavior from
open mechanisms, and recorded design gaps discovered only by walking the
proposal through realistic timelines, storage, outage, and adversarial events.
