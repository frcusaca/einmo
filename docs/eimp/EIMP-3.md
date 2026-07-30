---
eimp: 3
title: Output-stage drift fails the run; explicit regenerate; multi-signer output stamps
author: Claude Code (Sonnet 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-30
supersedes: []
begun: [x]
---

# EIMP-3: Output-stage drift fails the run; explicit regenerate; multi-signer output stamps

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

## Abstract

Changes `einmo_suite.rs`'s `write_output` (the function that runs a suite's
evaluator over each input and writes the `output/` stage) in two ways:
(1) when a freshly evaluated result's content differs from what is already
signed at `output/`, the run now **fails** that case (a red/non-zero result)
instead of silently overwriting the existing file with new content and a
fresh stamp chain; and (2) a new explicit CLI verb lets a human deliberately
replace drifted output with the freshly generated content, after which the
normal `EinmoReview` flow (list/view/promote — `EIMP-1`) takes over. Also
extends `write_output`'s existing skip-if-unchanged fast path (today: exact
match of a single fixed 3-stamp key list) to **multi-signer accumulation**
at the `output` stage specifically: a second signer (e.g. a second DC/build
machine) whose freshly computed content matches what's already there gets
their stamp *appended* to the existing file, not a wholesale rewrite —
mirroring the same content-then-key check `EIMP-1` gives `EinmoReview` for
`checked`/`verified`, but scoped here to `output` and to the core test-run
path, not the review session object.

## Motivation

Today, `write_output` (`src/einmo_suite.rs:1133`) already has a skip-if-
unchanged fast path: it compares the freshly evaluated file's INPUT/OUTPUT[*]
/PERSPECTIVE/DIFF/COMMENTS sections and its expected `compiled`/`configured`/
`stage:output` stamp *keys* (as one fixed, ordered 3-entry list) against
what's on disk; if both match exactly, it restores the original bytes
untouched (no rewrite, no timestamp churn) and returns early
(`einmo_suite.rs:1221-1260`). But when sections differ (a real change in what
the evaluator produced) OR the key list doesn't match verbatim (e.g. a
second signer's key is present, which today never matches the fixed
3-entry list), the function falls through to "Content or keys changed —
sign and write" (`einmo_suite.rs:1263`) and **unconditionally overwrites**
`output/` with the new content under a fresh stamp chain. There is currently
no failure signal, no gate, and no flag for this — a suite run that produces
different output than last time silently redefines the baseline every time
it runs.

This is wrong for two independent reasons:

- **Silent baseline drift.** `output/` is meant to be a stable artifact that
  `checked/`/`verified/` are compared against and promoted from
  (`compare.rs`, `transitions::promote`). If it silently redefines itself
  on every differing run, a nondeterministic evaluator, an accidental code
  change under test, or a flaky environment can quietly invalidate the
  "this matches what a human reviewed" claim without anyone noticing —
  there is no gate a CI pipeline can key off of. The fix: content drift at
  `output` is a **test failure**, exactly like a `Status::OutputError` or a
  failed `--require-match` today, not a quiet rewrite.
- **Single-signer-only fast path blocks the "second DC" use case.** The
  existing key-list comparison (`einmo_suite.rs:1223-1241`) requires an
  *exact* 3-entry match, so a file already carrying an additional signer's
  stamp (or being independently re-run by a second signer whose key isn't
  the first-listed one) never takes the skip-write path — it falls into
  the unconditional-overwrite branch above, destroying the first signer's
  stamp on every subsequent differently-keyed run. `docs/todo/AIAGENT-einmo-repo.todo.md`
  already flags this as a known gap ("multiple stamps at the same stage ...
  is not confirmed to exist yet — needs a design pass, likely its own
  EIMP"); this is that EIMP, scoped to `output`.

After this EIMP: a suite run that reproduces the exact content already
signed at `output` either no-ops (if the running signer's key is already
present) or appends that signer's stamp in place (if not) — never touching
unrelated bytes or timestamps. A run that produces genuinely different
content **fails** that case instead of overwriting silently. A human who
wants to accept the new content explicitly runs the new regenerate verb,
which performs the same content-then-key check but treats "differs" as
"replace" instead of "fail" — after which the file is a normal `output/`
candidate again, and `EinmoReview` (`EIMP-1`) drives review/promotion from
there exactly as it would for any other candidate.

## Specification

### Content/key decision table (`output` stage only)

For a given input, after evaluating it fresh and assembling the candidate
`EinmoFile` (unsigned) exactly as `write_output` does today
(`einmo_suite.rs:1133-1217`), compare against the existing `output/` file
(`EinmoFile::from_file`, which verify-on-inspects; a corrupt/tampered
existing file is treated as absent — this part is unchanged, `einmo_suite.rs:1080`):

| Existing file | Content sections match? | Running signer's key already among existing `stage:output` stamps? | Outcome |
|---|---|---|---|
| absent (or corrupt) | n/a | n/a | write fresh, sign, done (unchanged from today) |
| present | no | n/a | **fail** this case (new `Status` — see below); `output/` is left untouched |
| present | yes | yes | no-op: restore original bytes untouched, no rewrite (unchanged from today's fast path, generalized past the fixed 3-key list — see below) |
| present | yes | no | **append** this signer's `stage:output` stamp to the *existing* file in place; every prior stamp (including other signers') is preserved byte-for-byte; only the new STAMPS line is added |

"Content sections match" means the same section-by-section comparison
`write_output` already performs (`einmo_suite.rs:1242-1247`): same section
count, same names, same bodies, in order. STAMPS is never part of this
comparison (signature.rs's stamp chain is deliberately excluded from content
identity, `compare.rs`'s existing `required_sections` convention).

"Running signer's key already among existing stamps" replaces today's exact
3-entry key-list equality (`einmo_suite.rs:1223-1241`) with: the existing
file has *at least one* `stage:output` stamp whose pubkey equals the
`stage:output` key this run would derive (`Stamps::stamped_by`, already
used elsewhere for prefix search — `signature.rs:449` — reused here for an
exact pubkey match, not a prefix). The `compiled`/`configured` certification
stamps are unaffected by multiplicity; they exist once, from the first
signer, and are not re-added by subsequent signers appending an `output`
stamp (`Stamp::key()` validation already permits `stage:<name>` without a
uniqueness constraint, `signature.rs:373-385` — appending a second
`stage:output` stamp under a different pubkey is a legal wire form today,
just never produced by any code path yet).

### New `Status` variant: drift

```rust
// status.rs (or wherever `Status` lives today)
pub enum Status {
    Normal,
    InputError,
    OutputError,
    /// NEW: freshly evaluated content differs from the signed `output/`
    /// baseline already on disk. The run did not overwrite anything; the
    /// existing `output/` file is untouched. Distinct from `OutputError`
    /// (an evaluator failure) — this is an evaluator *success* whose result
    /// disagrees with the prior baseline.
    Drifted,
}
```

A suite run reports `Drifted` cases as failures in its summary and exit
code, the same way `OutputError`/`InputError` already are today — this
EIMP does not change how the harness aggregates pass/fail, only adds one
more case that counts as non-passing.

### New CLI verb: explicit regenerate

```
einmo regenerate-output <suite> [--filter <glob>] [--files <path>...]
```

Re-runs evaluation for the matching inputs exactly as a normal suite run
would, but for any case in the `Drifted` outcome, **replaces** `output/`
with the freshly evaluated content and a fresh stamp chain (today's
existing unconditional-overwrite behavior, preserved here under an
explicit, deliberately-named verb instead of being every run's silent
default). Cases that would no-op or append-a-stamp behave identically to a
normal run — this verb only changes what happens to the `Drifted` case.
Requires the same `output`-stage signing key a normal run already uses (no
new key material, no passphrase prompt beyond what `einmo run` already
needs) — this is a deliberate-intent gate on the *verb choice*, not a
signing gate.

**Where review picks up.** After `regenerate-output` replaces a case's
`output/` file, that file is a brand-new, freshly-signed `output/`
candidate — indistinguishable from any other `output/` artifact. From here
the existing/`EIMP-1` review flow (`EinmoReview::items`/`body`/`decide`/
`execute`) takes over exactly as it would for a case whose `output/` never
drifted: list it, view its INPUT/OUTPUT sections, promote it to `checked`/
`verified` in the ordinary way. This EIMP does not add any review-specific
handling for a just-regenerated case — it is a normal `output/`-stage
candidate the moment `regenerate-output` finishes writing it.

### Scope boundary vs. `EIMP-1`

This EIMP touches `write_output` (`einmo_suite.rs`) — the core suite-run
path that predates and is independent of `EinmoReview`. `EIMP-1`'s own
multi-signer accumulation (its §S.5, extended per its Open Questions
resolution to cover the content-then-key check for `checked`/`verified`
promotion inside `EinmoReview::execute`) is a separate, analogous change to
a separate code path (`transitions::promote`, called from
`EinmoReview::execute`). The two EIMPs implement the *same shape* of
decision table independently, once for `output` (this EIMP, the test-run
path) and once for `checked`/`verified` (`EIMP-1`, the review path) — no
code sharing is mandated by this EIMP, but implementers should look for a
natural common helper (e.g. a `content_and_signer_state` function taking an
existing file, a candidate file, and a signer pubkey) if one falls out
naturally; forcing one prematurely is not a goal.

## Test Plan

- Unit — `write_output` drift-vs-append-vs-noop-vs-fresh, in
  `src/einmo_suite.rs`'s existing test module:
  - Existing `output/` absent → fresh write (regression test for unchanged
    behavior).
  - Existing `output/` present, content matches, same signer → byte-for-byte
    untouched, same timestamp (regression test for the existing fast path,
    re-expressed against the new comparison).
  - Existing `output/` present, content matches, **different** signer →
    existing stamps preserved, new stamp appended, content sections
    untouched, timestamp of the file changes (a write occurred) but no
    section body changes.
  - Existing `output/` present, content **differs** → `Status::Drifted`,
    existing `output/` file byte-for-byte untouched on disk.
  - Existing `output/` present but tampered/corrupt → treated as absent,
    fresh write (regression test — `einmo_suite.rs:1080`'s `.ok()` already
    does this; pin it explicitly against the new code path).
- Unit — `Stamps::stamped_by`-style exact-pubkey lookup for `stage:output`
  (new or reused helper), including the two-signers-present case.
- Integration — `einmo regenerate-output`: a suite with one drifted case;
  confirm normal run reports it as `Drifted` and leaves `output/` untouched;
  confirm `regenerate-output` replaces it, re-verifies, and that a
  subsequent normal run now reports it clean (content matches, same
  signer).
- Comprehensive test: a suite fixture (reuse `zweimomo`'s ported suites)
  exercising, in one run: a case that no-ops (rerun, unchanged), a case
  signed by a second signer (co-sign, stamps accumulate), a case that
  drifts (fails, file untouched), then `regenerate-output` on the drifted
  case, then a normal review pass (`EIMP-1`'s `EinmoReview`, once
  implemented) promoting the regenerated case through to `checked`.

## Rejected Alternatives

### A. Keep silent overwrite, add an opt-out flag instead

Add `--fail-on-drift` as an opt-in flag to the normal run command, leaving
today's silent-overwrite the default. Rejected: the whole point is that
silent baseline drift is the dangerous behavior — defaulting to it means
every existing and future CI invocation stays exposed unless someone
remembers to opt in. Fail-by-default with an explicit opt-in *replace* verb
(this EIMP's design) makes the safe behavior the path of least resistance.

### B. Do nothing; treat this as acceptable because `checked`/`verified` are the real gates

`output/` isn't itself a promoted/reviewed artifact, so one could argue
drift there doesn't matter — only drift at `checked`/`verified` (caught by
`compare.rs`'s existing `--require-match`) is consequential. Rejected: by
the time a human runs `compare`, the *evidence* of what actually changed
(the previous `output/` baseline) has already been overwritten and is gone
— comparison happens against whatever the latest run produced, not against
what was there before. Failing fast at `output` preserves the prior
baseline for inspection instead of destroying it as a side effect of
finding out something changed.

### C. Reject the second signer's run outright instead of appending a stamp

When a second signer's key doesn't match the first and content otherwise
matches, refuse the run (analogous to failing on content drift) rather than
appending a stamp. Rejected: this is not drift — the content is identical,
only the *signer* differs, which is exactly the deliberate "second DC
independently confirms this" use case the todo item asks for
(`docs/todo/AIAGENT-einmo-repo.todo.md`). Refusing it would make
independent cross-verification impossible without deleting and re-running
from scratch under one shared key, defeating the point of having distinct
per-signer stamps at all.

## Open Questions

- None — resolved at begun-time: no interactive confirmation gate for
  `regenerate-output` beyond the verb's own explicit name/narrow scope; no
  new passphrase gate beyond a normal run's existing `output`-stage signing
  behavior.

## References

- `EIMP-1` (`docs/eimp/EIMP-1.md`) — `EinmoReview`'s own, separate
  multi-signer accumulation for `checked`/`verified` (§S.4/§S.5, extended
  per its Open Questions resolution); this EIMP is its `output`-stage,
  core-test-run analogue.
- `docs/todo/AIAGENT-einmo-repo.todo.md` — the "multiple-signature" use
  case this EIMP designs and implements (applies to `output` here;
  `checked`/`verified` is `EIMP-1`'s side of the same idea).
- Code: `src/einmo_suite.rs` (`write_output`, `evaluate_raw_parallel`),
  `src/signature.rs` (`Stamps`, `Stamp`, `stamped_by`), `src/transitions.rs`
  (the analogous, already-existing `promote` for comparison).
