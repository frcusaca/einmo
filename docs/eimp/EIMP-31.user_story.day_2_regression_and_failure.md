# EIMP-31 User Story: Day 2 — Routine Regression and Failure

## Story

On Day 2, the application merges a small fix as commit `S2`. No externally
visible behavior was intended to change. Mira wants to know whether the new
commit still satisfies yesterday's protected obligations before anyone edits
the validation suite.

## Passing Path

### 1. Select the new commit

Mira specifies `S2`. Einmo fetches that exact SHA from GitHub if the object is
not available locally. It does not store or sign a floating branch name as the
subject. **Contracts relied upon: 2.1.**

### 2. Reuse yesterday's requirements without rewriting them

The validation repository remains at `V1`, and the secured inventory remains
`I1`. Their immutable identities mean the question is stable: does new subject
`S2` still satisfy exactly what `S1` satisfied yesterday? **Contracts relied
upon: 1.2–1.4, 2.4.**

### 3. Execute the complete inventory

Einmo verifies clean subject and validation checkouts, resolves the tested
artifact and environment, and reconciles every required case through
discovery, selection, start, and completion. **Contracts relied upon:
2.2–2.6.**

### 4. Store the new success

All cases pass. Einmo creates `R2`, bound to `S2 + V1 + I1` and the new tested
artifact. `R1` remains evidence about `S1`; it is not overwritten or relabeled.
Appending `R2` does not redefine verifier content `V1`. **Contracts relied
upon: 3.3–3.7.**

## Failing Path

Suppose one protected case produces a different result at `S2`.

1. Einmo returns a failing validation result and leaves the fresh output in
   `generated/` for inspection. Existing reviewed stages remain untouched.
   **Contracts relied upon: 3.1, 3.2.**
2. No assurance signer is invoked, and no `R2` is added to the validation
   repository. The absence of a record for `S2` means it has not passed this
   validation; it does not mean the run never happened or that the failure was
   converted to success. **Contracts relied upon: 3.2, 3.5.**
3. Mira gives the discrepancy to the application developer. The developer
   produces corrected commit `S3` rather than changing `S2` in place.
4. Validation resolves `S3` and runs again. Yesterday's `R1` cannot be replayed
   for `S3`, and the unsigned output from the `S2` failure cannot be presented
   as a success. **Contracts relied upon: 2.1, 3.4, 3.6, 4.5.**
5. If `S3` passes, einmo stores a new successful record bound to `S3`.
   **Contracts relied upon: 3.3–3.5.**

## Infrastructure-Failure Variant

Suppose the evaluator crashes or the runner times out before the last case.
This is different diagnostic information from an assertion mismatch, but it is
the same assurance outcome: the required inventory did not complete, so there
is no successful record. Einmo reports the typed failure and preserves useful
generated output. **Contracts relied upon: 2.5, 2.6, 3.2.**

The future result schema should distinguish at least test failure,
infrastructure failure, timeout, and incomplete execution without allowing any
of them to satisfy a success gate.

## What Day 2 Establishes

The validation repository accumulates successful subject-bound facts:

```text
R1: S1 passed V1 + I1
R2: S2 passed V1 + I1       # only if the Day 2 run succeeded
```

It does not accumulate signed failed-run facts. Failed generated output is a
diagnostic aid and follows its own retention policy.

## Contract Trace

| Story event | Contract |
|---|---|
| Fetch and identify the new SHA | 2.1 |
| Hold verifier and requirements constant | 1.3, 2.4 |
| Reject local ambiguity | 2.2, 2.3 |
| Prove all required tests actually completed | 2.5, 2.6 |
| Preserve fresh failure output without changing baselines | 3.1 |
| Do not sign or store a failed run | 3.2, 3.5 |
| Bind a passing result to the new commit | 3.3, 3.4 |
| Append success without changing verifier identity | 3.7 |
| Prevent yesterday's success from satisfying today's SHA | 3.6, 4.5 |

## Questions Exposed by This Story

- How should the CLI distinguish test failure, infrastructure failure, and an
  invalid validation setup while making all three non-successful?
- Does the next run replace, archive, or coexist with the prior unsigned
  `generated/` diagnostics?
- How does a release gate report “no current successful record” without
  implying that a failed record should exist in the validation repository?

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
**Changes**: Created the Day 2 EIMP 21 user story, covering an unchanged
inventory against a new subject SHA, both passing and failing paths,
infrastructure interruption, diagnostics carrying no assurance claim, retry with a new commit,
replay prevention, and non-circular result storage.
