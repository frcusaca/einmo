# EIMP-21 User Story: An Evaluator Mutates and Restores Its Inputs

## Story

An official validation begins with clean subject and validation checkouts. A
compromised evaluator changes a subject configuration file immediately before
one difficult case, runs against the easier configuration, and restores the
original bytes before the post-run cleanliness check.

Both Git checks report clean. A design based only on “clean before and clean
after” could sign a run that did not consistently exercise the named subject.

## Why Pre/Post Checks Are Not Sufficient

Assume the file begins and ends with digest `A`:

```text
preflight:       config digest A, checkout clean
case 1:          reads A
attacker writes: config digest B
case 2:          reads B
attacker writes: config digest A
postflight:      config digest A, checkout clean
```

The run contains mixed inputs even though both observations agree. The same
attack can target validation code, an expectation, a dependency, the inventory,
or the binary invoked between cases.

## Required Execution Model

### 1. Resolve and verify clean inputs

Einmo resolves exact subject, validation, inventory, dependency, policy, and
artifact identities. Both versioned checkouts must begin clean. **Contracts
relied upon: 2.1–2.4.**

### 2. Seal execution inputs

Before the first case, the runner exposes resolved inputs through a read-only or
otherwise immutable snapshot. The evaluator cannot write subject source,
validation source, reviewed expectations, protected inventory, dependencies,
policy, or the tested binary. **Contract relied upon: 2.9.**

Implementation mechanisms could include read-only mounts, immutable content-
addressed copies, filesystem permissions combined with process isolation, or a
platform-specific sandbox. The contract is the immutability outcome, not one
particular operating-system primitive.

### 3. Confine permitted writes

The run still needs writable locations:

- `generated/` for fresh einmo work artifacts;
- an isolated build-output directory, when building is part of validation;
- explicit evaluator scratch space; and
- diagnostic logs or crash evidence.

These locations are declared outputs and cannot be loaded later as verifier or
subject inputs unless a new identified phase explicitly promotes them.
**Contracts relied upon: 2.3, 2.9, 3.1.**

### 4. Reject attempted mutation

The malicious write fails or is detected as an execution-policy violation. The
run stops, leaves useful diagnostics, emits an attention event, and creates no
assurance record. It does not continue and label the remaining cases passed.
**Contracts relied upon: 2.6, 3.2, 5.3, 5.9.**

### 5. Recheck identities before signing

After all cases, the runner still repeats cleanliness and artifact-identity
checks before invoking the assurance signer. This catches mistakes outside the
sealed execution view and makes the evidence easier to audit, but it is not the
only mutation defense. **Contracts relied upon: 2.2, 2.3, 3.3.**

## Ordinary Build Variant

A normal compiler writes thousands of files. That is not an attack, but it
should not make the source checkout ambiguous. The official runner directs
build products into a declared isolated output directory, then hashes and
seals the selected executable before test invocation.

The successful record binds both the immutable source identity and the exact
tested artifact identity. **Contracts relied upon: 2.3, 2.9, 3.4.**

## Generated-Output Variant

Current einmo deliberately writes `generated/` during evaluation. Contract 2.9
does not prohibit that behavior. The boundary is:

```text
sealed inputs:       subject, verifier, reviewed stages, inventory, policy,
                     dependencies, tested artifact
writable outputs:    generated results, declared scratch, diagnostics
```

A later case must not read a prior generated result as an undeclared input. If
a sequential workflow legitimately consumes earlier output, that relationship
and material must be explicit under contracts 2.3 and 6.2.

## Human View

The discrepancy packet reports:

```text
validation outcome: no successful record
failure class:      execution-input mutation attempted
target:             subject/config/example.toml
phase/case:         before payments/refund
write outcome:      denied by sealed-input policy
```

The user sees an integrity/setup failure, not a misleading behavioral mismatch.
Raw sandbox diagnostics remain available. **Contracts relied upon: 5.1, 5.3,
5.9.**

## Contract Trace

| Story event | Contracts |
|---|---|
| Begin from clean exact identities | 2.1–2.4 |
| Prevent transient mutation and restoration | 2.9 |
| Isolate build products and hash tested artifact | 2.3, 2.9, 3.4 |
| Permit only declared generated/scratch outputs | 2.9, 3.1 |
| Treat attempted write as validation failure | 2.6, 3.2 |
| Alert human with the real failure class | 5.3, 5.9 |
| Recheck before assurance signing | 2.2, 2.3, 3.3 |

## Guarantees and Limits

Filesystem read-only permissions alone may be insufficient if the evaluator
can remount storage, control the kernel, replace a remote dependency, or alter
another process. The assurance strength depends on the runner's isolation and
threat model. EIMP 21 must state which attackers are in scope.

The contract prevents inputs from changing during a run; it does not prove the
initial inputs were correct or that a trusted compiler produced the tested
binary.

## Acceptance Scenarios Derived from This Story

- A write to subject or validation source during execution is denied or
  detected and prevents signing.
- A write followed by byte restoration cannot evade the boundary.
- Reviewed stage artifacts and protected inventory are read-only inputs.
- Build products are confined outside source/verifier inputs.
- The tested artifact cannot be replaced between cases.
- `generated/` and declared scratch remain writable without weakening input
  identity.
- An undeclared case-to-case generated-output dependency is rejected.
- Post-run clean checks still occur before success signing.

## Questions Exposed by This Story

- What threat level must the first implementation resist: accidental writes,
  ordinary unprivileged processes, containers, or a hostile local administrator?
- Which platforms can enforce the chosen sealed-input contract consistently?
- Is building inside the validation run required, optional, or a distinct
  provenance phase?
- How are network inputs prevented from changing between cases?
- Which legitimate sequential tests may consume prior outputs, and how are
  those dependencies declared and hashed?

## Last Updated

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Renumbered this document from EIMP 11 to EIMP 21 and updated its internal references because EIMP 11 collided with another proposal.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Created the during-run mutation story, demonstrated why clean
pre/post checks miss transient mutation, added the sealed-input execution model,
preserved writable generated and scratch outputs, and derived isolation,
diagnostic, and acceptance requirements.
