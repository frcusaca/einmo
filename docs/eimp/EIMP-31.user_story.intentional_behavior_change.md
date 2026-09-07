# EIMP-31 User Story: A Legitimate Behavior Change Fails the Old Expectation

## Story

A payment system previously rounded a statutory fee down. A new regulation
requires half-even rounding beginning on a particular date. The implementation
agent correctly changes the code, but the independently reviewed validation
suite still protects the old rule.

The old suite fails. This is not evidence that the implementation change is
wrong; it is evidence that code and previously approved behavior no longer
agree. Human judgment must determine which should change.

## Starting State

Successful record `R50` binds:

```text
subject:              S50
validation revision: V12
protected inventory: I12
requirement:          fees/statutory_rounding@old-rule
```

The old requirement and expected outputs remain historical facts. They cannot
be edited in place to make it appear that `R50` validated the future rule.
**Contracts relied upon: 1.3, 1.4, 1.7, 3.4, 3.6.**

## First Validation of the New Subject

### 1. Resolve the changed subject

The new implementation is committed as `S51`. Einmo runs clean `S51` against
the still-active verifier `V12` and inventory `I12`. **Contracts relied upon:
2.1–2.4.**

### 2. Report the disagreement

The statutory-rounding cases execute and differ from their reviewed expected
results. Einmo leaves generated output, emits a typed discrepancy, and creates
no assurance record for `S51 + V12 + I12`. **Contracts relied upon: 2.5,
2.6, 3.1, 3.2, 5.9.**

The human-facing packet includes:

- the old requirement text and definition identity;
- the old expected and newly generated rounding results;
- the code and specification changes since `S50`;
- affected downstream fee calculations in the structured hierarchy; and
- explicit repair-subject and revise-requirement paths.

It does not label the generated output correct merely because a regulation was
mentioned in the commit message. **Contracts relied upon: 5.1–5.4, 6.1,
6.6, 6.7.**

## Human Review Establishes the New Rule

The reviewer verifies the regulatory source, effective date, scope, rounding
examples, and applicability to the product. The implementation agent may
prepare a summary and proposed cases, but the proposal is not the approval.
**Contracts relied upon: 5.5, 5.6.**

If review instead finds that the code misread the rule, the reviewer chooses
“repair subject,” preserves `V12 + I12`, and asks for a new implementation
commit. The EIMP does not force a test change whenever code and expectation
disagree.

Assume the review confirms the intended behavior changed.

## Revise the Validation Through Existing Einmo Procedure

### 1. Author new cases and expectations

The validation author adds boundary examples and updates affected expected
results. Authoring runs write `generated/`; the reviewer inspects differences
against the old baseline. **Contracts relied upon: 2.7, 3.1, 5.8.**

### 2. Use staged promotion rather than direct editing

The existing einmo process remains meaningful:

```text
generated → output → checked → verified
```

- `generated → output`: the new observations are consciously accepted as a
  reasonable proposed baseline.
- `output → checked`: the results are reviewed against the new regulation and
  justified statement by statement.
- `checked → verified`: a human attests under the existing human-authority
  rules.

Directly editing `.approved` or signed stage artifacts cannot substitute for
these decisions. **Contracts relied upon: 4.3, 5.4–5.6.**

### 3. Commit a new verifier definition

The reviewed tests, expectations, requirement link, and rationale become clean
validation revision `V13`. The new definition has a distinct digest or version
even if a stable logical requirement name is retained. **Contracts relied
upon: 1.7, 2.2, 2.4.**

### 4. Activate the new requirement

The inventory authority publishes and activates `I13`, which names the new
definition and effective policy. It does not wait to see whether `S51` passes
before deciding that the independently confirmed obligation exists.
**Contracts relied upon: 1.4, 1.8.**

If `S51` now fails a newly added boundary case, current validation remains red
until the subject is corrected. `I13` is not quietly rolled back to `I12`.
**Contracts relied upon: 1.6, 1.8, 3.2.**

### 5. Validate and store the new claim

Einmo officially validates clean tuple `S51 + V13 + I13`. Only complete success
produces a signed result. The new record does not alter what `R50` meant under
the old rule. **Contracts relied upon: 2.5, 2.6, 3.3–3.7.**

## Effective-Date Variant

The regulation takes effect next month. The inventory system may need an
authenticated future activation or two explicit profiles:

```text
current-law profile     → I12 until effective instant
future-law readiness    → I13 available for preflight validation
```

The UI must show which policy each success satisfies. A future-readiness pass
cannot be presented as current-law assurance, and an old-rule pass cannot
satisfy policy after the effective transition. **Contracts relied upon: 1.4,
1.6–1.8, 3.6, 3.8, 5.7.**

## Contract Trace

| Story event | Contracts |
|---|---|
| Preserve the old rule's historical meaning | 1.3, 1.7, 3.6 |
| Old expectation correctly fails new subject | 2.5, 2.6, 3.2 |
| Present specification and development context | 5.1–5.3, 6.6, 6.7 |
| Human chooses repair or deliberate requirement revision | 5.4–5.6 |
| Existing staged review remains authoritative | 2.7, 3.1, 4.3 |
| New semantic meaning receives a new definition | 1.7 |
| Activate obligation independently of subject success | 1.4, 1.8 |
| Refuse rollback to the easier old inventory | 1.6 |
| New passing tuple creates a distinct record | 3.3–3.7 |

## Guarantees and Limits

Einmo can prove which rule, test definition, expectation, and subject were
used. It cannot determine whether the new regulation was interpreted correctly
or whether the reviewer behaved responsibly. Structured context, source links,
separate authority, and review rationale facilitate that judgment without
pretending to replace it.

## Acceptance Scenarios Derived from This Story

- A code change that intentionally changes output still fails the old active
  expectation.
- The failure path offers no automatic acceptance or assurance signature.
- An expectation change follows existing generate/review/promote procedures.
- A requirement meaning change receives a new authenticated definition.
- Historical records retain their old definition and applicability.
- A newly active requirement remains active when its first official run fails.
- Future activation and readiness profiles cannot be confused with current
  policy.
- Agent-authored change explanation remains distinct from human approval.

## Questions Exposed by This Story

- Does a breaking semantic change require a new stable identifier, or an old
  identifier plus a new definition digest/version?
- How is trusted effective time established for scheduled inventory activation?
- Which stage and signature carry human rationale for a specification-derived
  expectation change?
- Can one subject legitimately hold simultaneous success records for current
  and future policy profiles?
- Does EIMP 31 need an explicit exception/transitional-result model, or should
  that be a later proposal?

## Last Updated

**Date**: 2026-09-07
**Updated By**: OpenAI Codex (GPT-6)
**Changes**: Renumbered this proposal family from EIMP 21 to EIMP 31 at the
maintainer's request; updated active references and retained earlier history.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Renumbered this document from EIMP 11 to EIMP 21 and updated its internal references because EIMP 11 collided with another proposal.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Created the legitimate behavior-change story, grounding an
intentional specification transition in existing einmo stages, human review,
versioned requirement meaning, independent activation, rollback resistance,
effective-date profiles, and distinct historical validation claims.
