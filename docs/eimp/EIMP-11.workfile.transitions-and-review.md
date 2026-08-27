# EIMP 11 Workfile — Transitions and Confirmed Review Execution

> **Design workfile, not normative specification.** This file presents the
> evidence and first-pass choices behind EIMP 11 §S.3 and §S.5.4. The accepted
> rules belong in `EIMP-11.md`; unresolved human choices remain explicit here.

## Dependency position

Gate A can be answered immediately from current behavior. The confirmed-review
decision also depends on the transaction vocabulary and conflict rules in
§S.4–S.6, but not on a filesystem representation. Storage prototypes must not
choose these semantics accidentally.

## Decision A — `verified → checked` (accepted with an ordering amendment)

### Maintainer response and disposition

The maintainer accepted removal of `verified → checked`, with the amendment
that it land only after the complete-stamp/multi-signature work. A human who
verifies a case has necessarily also checked it, while an agent may already
have contributed a checked signature; the transition change must therefore be
tested against multiple checked and verified signers rather than assuming one
signer at either stage.

EIMP 11 implements that dependency by completing §S.2 and its permutation and
duplicate-stamp tests before changing the backward edge. The exhaustive
transition tests then add adjacent lifecycle cases with multiple checked
signatures and a later human verified signature. This ordering amendment does
not retain `verified → checked`: withdrawal remains `retract verified`, and
additional checked signatures are accumulated through the ordinary
`output → checked` co-sign path before verification.

### Accepted rule

Remove `verified → checked` and make `retract verified`
the only operation that withdraws verified attestation, because EIMP 11 §S.3
defines an adjacent forward claim chain and the current operation is not a true
demotion.

### Evidence

- `src/transitions.rs:84-97` calls the edge legal.
- `src/case.rs:305-379` writes the destination but never removes the verified
  source. When bodies match it appends a checked stamp; otherwise it can copy a
  verified chain backward and append a later checked stamp.
- `src/review.rs:330-367,1172-1181` cannot plan this edge: promotion to checked
  selects output as its source.
- `src/case.rs:424-460` already expresses the coherent withdrawal operation:
  retracting verified removes `verified/` and preserves `checked/`.
- EIMP 1 §S.3 describes only `output → checked` and `checked → verified` review
  promotions, with retraction as the backward operation.

### Alternatives and consequences

1. Keep it unchanged: cheapest compatibility path, but preserves disagreement
   between its name, mutation, stamp order, and review API.
2. Redefine it as retract-verified: coherent effect, misleading API; the exact
   operation already exists.
3. Add a true historical restore: potentially valuable, but it needs explicit
   revision selection, conflicts, and renewed review. That is a possible later
   EIMP, not a reason to retain this edge.

### Einmo Library User Experience changes

The method signatures need not change, but their accepted input domain does.
Calls through `EinmoSuite::promote(Stage::Verified, Stage::Checked, …)`—and any
internal `EinmoCase::promote` call with the same pair—will return
`EinmoError::IllegalTransition` before reading or writing an artifact. The
crate-private `is_legal_transition` table loses the pair. Users that meant
“withdraw the human attestation but keep reviewed content” must call
`EinmoSuite::retract(Stage::Verified, …)` instead; its existing result semantics
and `checked/` preservation remain unchanged.

There is no transparent migration for a caller that meant “copy a historical
verified envelope back into checked.” That intent is intentionally unsupported:
the caller must restore a selected revision through a future explicit API and
submit it to review again. Removing the edge does not delete existing
`verified/` or `checked/` files and does not rewrite existing signatures.

Direct `output → verified` removal has the same API shape consequence:
`EinmoSuite::promote(Stage::Output, Stage::Verified, …)` becomes an immediate
illegal-transition error. Callers must perform and authorize two distinct
operations, `output → checked` and then `checked → verified`.

### Einmo Integration changes

The following CLI invocations stop succeeding and exit nonzero with an illegal
transition diagnostic, before filesystem mutation:

```text
einmo promote verified to checked <work-dir> [files...]
einmo promote verified:checked <work-dir> [files...]
einmo promote verified..checked <work-dir> [files...]
einmo promote output to verified <work-dir> [files...]
einmo promote output:verified <work-dir> [files...]
einmo promote output..verified <work-dir> [files...]
```

The corresponding `cargo einmo …` wrapper forms change identically. Withdrawal
becomes `einmo retract <work-dir> verified`. Direct verification becomes two
commands with independently appropriate credentials:

```text
einmo promote output to checked <work-dir> [files...]
einmo promote checked to verified <work-dir> [files...]
```

CLI help in `src/cli.rs:94-103`, the command table at `README.md:743`, and the
week-2 explanation at `zweimomo/suites/javascript/week.2/README.week.2.md:86-97`
must list only adjacent forward transitions. Review planning loses its fallback
from checked to output when targeting verified (`src/review.rs:1172-1181`): an
output-only case must first receive/execute a checked decision and cannot be
silently verified from output. HTTP/TUI requests targeting verified must expose
“checked source missing” rather than plan a shortcut.

Integration tests and CI must add:

- an exhaustive `Stage × Stage` matrix exercised through library, plain CLI,
  cargo wrapper, review planning, and server DTO/handler surfaces;
- subprocess tests asserting all six obsolete CLI spellings fail nonzero and
  leave source/destination bytes unchanged;
- a two-step happy path showing distinct checked and verified stamps/keys;
- `retract verified` coverage proving verified disappears while checked bytes
  and stamps remain unchanged;
- a review-plan test proving output-only content cannot produce a verified
  action, plus an adjacent two-action lifecycle test where applicable;
- documentation consistency tests deriving legal pairs from the authoritative
  graph so help and README cannot drift.

Existing tests or fixtures that assert the two removed pairs are legal must be
rewritten as refusal tests. Signed fixtures are regenerated through the CLI in
scratch; they are never hand-edited. Existing envelopes with lifecycle-inverted
chains remain parseable unless strict stamp-grammar work separately rejects
them; removal prevents new creation but is not an implicit file-format break.

### Einmo Development changes

There must be one authoritative transition representation consumed or tested by
`src/case.rs`, `src/suite.rs`, `src/cli.rs`, `src/review.rs`, review-server DTOs,
and documentation generation. Developers adding a transition must document the
claim it creates, credential policy, source/destination behavior, retraction
relationship, and exhaustive matrix change. “Convenient shortcut” is not
sufficient when it synthesizes an omitted review claim.

The transition correction is deliberately split: removing `output → verified`
is executable before Gate A; removing `verified → checked` waits for the human
answer. A future restore feature must be planned as revision selection plus a
new review transaction, not reintroduced as a backward `promote` edge.

### Migration

1. Replace library calls promoting verified to checked with retract-verified
   when the intent is attestation withdrawal.
2. Replace direct output-to-verified calls and CLI invocations with separate
   output-to-checked and checked-to-verified operations using the proper signer
   at each claim boundary.
3. Update automation that treats the removed pairs as legal to expect
   `IllegalTransition` and nonzero CLI exit without mutation.
4. Regenerate affected signed fixtures through the CLI; do not hand-edit them.
5. Validate the adjacent two-step workflow and retract-verified preservation.
6. Rollback, if required before release, is restoration of the old transition
   table and help/tests together; no stored artifact conversion is required.

## Settled first pass — `output → verified`

First pass, propose to remove the shortcut without compatibility mode, because
EIMP 11 §S.3 says the checked claim cannot be synthesized and EIMP 1's review
model applies adjacent lifecycle transitions. Current code permits and plans
the bypass (`src/transitions.rs:84-97`, `src/review.rs:1172-1181`) and current
help advertises it (`src/cli.rs:94-103`, `README.md:743`). Automatically
expanding it into two promotions is rejected: those promotions make different
claims and can have different signer policy.

No human decision is known unless compatibility with this invalid shortcut is
declared a new requirement.

## Decision B.1 — confirmed review transaction (accepted)

### Accepted rule

The displayed and confirmed `ExecutionPlan` is one
suite-wide, all-or-nothing transaction and that EIMP 11 implement no partial
mode, because §S.4 and §S.5.4 define a logical atomic boundary and EIMP 1 says
the preview is exactly what will execute. `execute_one` remains the explicit
way to select a one-action boundary before confirmation.

### Current mismatch

`ExecutionPlan` lacks a suite revision, decision versions, and basis
observations (`src/review.rs:330-367`). Execution preflights keys but then
mutates cases incrementally; errors become skipped actions while other actions
remain committed, after which executed and skipped decisions are cleared and a
mixed result is journaled (`src/review.rs:887-1090`). A confirmed preview can
therefore land only partly.

### Proposed contract

1. `plan()` captures a suite revision and stable plan/transaction id.
2. Each action records the live decision identity and every basis/source,
   destination, and enumeration observation on which it depends.
3. All keys, artifacts, serialization, and dependencies are preflighted.
4. Any drift, invalid artifact, missing source/key, or conflict aborts before
   visibility and leaves pending decisions intact.
5. One commit makes all mutations visible; a receipt makes retry return
   `AlreadyApplied` after a crash between commit and decision cleanup.
6. The journal records plan id, base revision, attempted set, and the single
   commit/conflict outcome.
7. `execute_one()` constructs a one-action plan over the same machinery.

### Alternative

An explicitly previewed future “apply independently” operation could partition
actions into per-case transactions. It must describe those boundaries before a
new confirmation; it must not be a boolean that silently weakens normal
`execute`. Implementing it now multiplies retry, journal, report, and API states
before the transactional core exists.

### Einmo Library User Experience changes

`ExecutionPlan` gains opaque plan/transaction identity, suite revision, decision
identity/version, and dependency observations. `execute()` changes from a
best-effort report that may contain both executed and skipped mutations to one
commit outcome: committed/already-applied or a typed pre-visibility failure or
conflict. Pending decisions are cleared only after committed/idempotent success.
`execute_one()` remains, but becomes a one-action plan over identical semantics.

### Einmo Integration changes

CLI, TUI, and HTTP confirmation screens must display the atomic boundary and
report “nothing applied” on any conflict. Automation that currently treats
`ExecutionReport.skipped` as acceptable partial progress must refresh/re-plan
instead. Journals gain plan id, base revision, attempted set, and one outcome.
CI needs stale-basis, changed-decision, missing-key/source, crash-after-commit,
idempotent-retry, and all-actions-or-none visibility tests across library,
server, and CLI confirmation paths.

### Einmo Development changes

Review code stops mutating inside action loops. Planning, preflight, change-set
construction, commit, journal receipt, and decision cleanup become explicit
phases with fault points between them. New review actions must declare their
read dependencies and writes and must run through the same transaction path;
they cannot add an ad hoc “skip and continue” branch.

### Migration

1. Extend stored/in-memory plans with revision, identity, and dependency data;
   reject or explicitly re-plan legacy plans that lack it.
2. Update integrations that consume mixed executed/skipped reports to handle a
   single commit/conflict outcome and refresh/re-plan on failure.
3. Update journal readers for plan id, base revision, attempted set, and atomic
   outcome before removing legacy fields.
4. Validate one-action compatibility through `execute_one`, crash-safe retry,
   and all-or-none integration tests.
5. Roll back only before emitting new durable plan/journal records, or retain a
   versioned reader for those records during the compatibility window.

## Last Updated

**Date**: 2026-08-27  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Recorded Reviews 1 and 2. Removal of `verified → checked` is
accepted after complete-stamp/multi-signature semantics and tests; confirmed
batch review is accepted as suite-wide all-or-nothing with `execute_one` as the
explicit smaller boundary.


**Date**: 2026-08-27  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Added ordered Migration procedures for removed transition edges
and the confirmed-plan contract, including automation, fixtures, journals,
validation, and rollback considerations.
