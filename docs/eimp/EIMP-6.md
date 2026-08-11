---
eimp: 6
title: Add structured JSONL logging to the test-run path
author: Claude Code (Opus 5) <noreply@anthropic.com>
status: Brewing
type: Standards
created: 2026-07-30
supersedes: []
begun: [ ]
---

# EIMP-6: Add structured JSONL logging to the test-run path

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

**No plan file yet, deliberately.** This EIMP lands after `EIMP-1`, and its
shape depends on what `EIMP-1`'s journal actually turns out to be. Writing a
plan now would be planning against a substrate that does not exist. The plan
is written when this EIMP is picked up.

## Abstract

Generalize `EIMP-1`'s review-session journal into einmo's **general structured
logging substrate** — append-only JSONL, keyed by `EinmoId`, with verbosity
levels — and extend it to cover the *test-run* path, not only the
review-session path it was born in. A suite run should be able to say what it
did, case by case, in a machine-readable form keyed by the same identifier the
review session, the server, the CLI, and the corpus already use.

**The crash crumb stays.** An earlier draft of this EIMP proposed retiring it
in favor of unmatched journal records; that is now an explicitly rejected
alternative (§Rejected Alternatives A). The crumb is a proven mechanism for
surviving a hard crash, and its principal cost — polluting a committed
`output/` tree with files that are not outputs — is removed by `EIMP-01`,
which moves crumb creation into the uncommitted `generated/` stage.

## Motivation

### A suite run cannot currently report what it did

`EIMP-1` §S.6 specifies an append-only JSONL journal keyed by `EinmoId`, with
terse/normal/fine verbosity levels, recording each case as it is read in and
verified. That journal lives in the **review-session** layer. The test-run
layer — `EinmoSuite`'s evaluation path — has nothing equivalent. It produces a
`TestResults` aggregate at the end and prints a human-readable summary, and
that is all.

This is a gap for three consumers:

- **CI.** A gate that fails wants to say which cases failed and why in a form
  a machine can consume, not a form a person greps.
- **The review session.** It already speaks `EinmoId`-keyed JSONL. A test run
  that spoke the same language would let a reviewer follow one case from
  evaluation through review to attestation in a single stream, rather than
  two formats joined by hand.
- **Long or flaky runs.** A run that takes minutes gives no incremental
  signal. A `fine`-level journal records each case as it completes.

Building a second, differently-shaped logging mechanism for the test-run path
would be the wrong answer — hence generalizing the one `EIMP-1` builds rather
than inventing a sibling.

### Why the crash crumb is no longer the thing to remove

`write_crash_crumb` (`src/einmo_suite.rs`) writes a signed `.einmo` with
`status: output-error` and a `status_detail` beginning `"TEST IN PROGRESS"`
before the evaluator runs. If the harness dies mid-evaluation — SIGKILL, stack
overflow, a panic that escapes the catch — that file survives and says what
happened. `zweimomo`'s `crash_crumb_survives_stack_overflow` re-spawns the
test binary as a child and genuinely overflows the stack, proving the
mechanism does its job.

The original case for retiring it was structural: the crumb put files that are
**not outputs** into `output/`, so every consumer of `output/` had to know
that. **`EIMP-01` dissolves that argument.** Under it, evaluation writes only
the gitignored `generated/` stage, and crumbs land there — a directory that is
by definition not a committed result tree and that `einmo generate` rebuilds
every run. Nothing a reviewer commits is polluted, and nothing a person browses
is confusing.

What survives of the old critique is smaller and does not justify removal:

- The `"TEST IN PROGRESS"` prefix remains stringly-typed and read at more than
  one site, including `write_output`'s byte-identical no-op path, which must
  still not mistake a crumb this run just wrote for a prior baseline.
- `check_catastrophe_crumb` remains a second consumer with its own
  ignore/refuse/rerun policy and config knobs.

Both are worth tidying on their own merits — a typed marker instead of a
string prefix, one place that recognizes a crumb — and neither requires
deleting a mechanism that demonstrably survives a SIGKILL. **A journal entry
and a crumb answer the same question with different durability guarantees, and
keeping the stronger one is not redundancy worth paying to remove.**

### Why this is not part of `EIMP-1`

`EIMP-1`'s journal lives in the review-session layer; the test-run layer
(`EinmoSuite::evaluate`) has no review session and must work when no server, no
session, and no reviewer exist. Extending the journal downward is real work
with its own design questions — where does a suite-run journal live, what
happens when the suite directory is read-only — and folding it in would have
widened an already-large EIMP. `EIMP-1` builds a journal capable of the role;
this EIMP extends it into the test runner.

## Specification

**Deliberately thin — this EIMP is scoped, not designed.** Its substrate is
`EIMP-1`'s journal, which does not exist yet; specifying handler shapes and
file formats against it now would be specifying against a guess. What is fixed
here is the scope and the constraints.

### S.1 Scope

1. **Generalize the journal beyond the review session.** It must be usable
   from `EinmoSuite`'s test-run path, where there is no session, no reviewer,
   and possibly no writable suite directory.
2. **Verbosity levels, configurable** — carrying `EIMP-1` §S.6's
   terse/normal/fine levels down into the test-run path, where `fine` means
   one record per `EinmoId` as each case is read, evaluated, and verified.
3. **`EinmoId` keying throughout.** Every case-scoped record carries the
   `EinmoId` — the same identifier the review session, the server, the CLI,
   and the corpus already use. One identifier end to end, no translation layer
   that could disagree.
4. **The generation phase and the three gates all log.** A gate that compares
   two stages records what it compared and what it found, in the same stream
   and the same keying as a generation run (`EIMP-01` §S.0 names the four
   phases).

### S.2 Non-goals

**The crash crumb is out of scope and stays as it is.** No part of this EIMP
removes `write_crash_crumb`, `check_catastrophe_crumb`, `is_catastrophe_crumb`,
the `"TEST IN PROGRESS"` prefix, or their config knobs
(`--ignore-catastrophe-crumbs`, `--rerun-catastrophes`). The crumb's
hard-crash guarantee is a property this EIMP neither replaces nor weakens.

A journal record may *additionally* note an in-flight case, and that is
welcome — but it is observability, not crash evidence, and it carries no
obligation to reach disk before the evaluator runs.

Tidying the crumb's stringly-typed marker is likewise out of scope here; it is
worth its own small EIMP if anyone wants it.

### S.3 Constraints

- **A read-only or absent journal must not fail a test run.** Logging is
  observability; it must never become a new way for a suite to fail. This is
  the opposite of the crumb's fail-loudly-if-unwritable behavior, and the
  divergence is deliberate — the two mechanisms have different jobs, and only
  one of them is load-bearing for correctness.
- **No new heavy dependency in core `einmo`.** `EIMP-4` splits the crate
  specifically to keep the core dependency tree lean; a logging substrate that
  pulls a large framework into core defeats that.
- **`EIMP-4`'s crate boundary holds.** If the journal lives in
  `einmo-review-server` after the split but the test runner lives in core
  `einmo`, the substrate must move to core, or be split. Resolve this
  explicitly — it is the first real design question this EIMP faces.
- **Existing behavior unchanged.** Adding logging must not change what any
  phase does, what it writes, or whether it passes. Every existing test passes
  unmodified.

## Test Plan

- **Verbosity levels**: each level emits what it promises and no more; `fine`
  records every case, `terse` does not.
- **`EinmoId` keying**: a case can be followed from a generation run through a
  gate to a review decision by its id alone, across both journals.
- **Logging failure is not test failure**: a read-only or unwritable journal
  destination degrades without failing the suite (§S.3).
- **No behavior change**: the full existing suite passes unmodified, including
  einmo's `catastrophe_crumb_*` tests and `zweimomo`'s
  `crash_crumb_survives_stack_overflow` — the crumb is untouched by this EIMP
  and its tests must not need edits.
- **Machine-readability**: emitted JSONL parses, one object per line, and a
  consumer can reconstruct a run's per-case outcomes from it alone.
- Comprehensive test: run a suite in which one case errors, one diverges from
  the baseline, and one is a clean no-op, and assert the journal tells the
  whole story from `EinmoId`-keyed records alone.

## Rejected Alternatives

### A. Retire the crash crumb in favor of unmatched journal records

**This was this EIMP's original scope, and is now rejected.** The proposal was
that a case logging `case_start` and never `case_end` identifies the in-flight
case at crash time, answering the crumb's question with more precision and
without writing into the output tree.

Rejected for two reasons. First, `EIMP-01` removes the cost that motivated it:
crumbs now land in the gitignored `generated/` stage, so no committed tree is
polluted and no reviewer browses them. Second, the replacement is strictly
weaker where it matters. The crumb's guarantee is that evidence survives a
*hard* crash — SIGKILL, stack overflow, an aborted process. A journal
preserves that only if its writes reach disk before every evaluation begins;
buffered, flushed-at-exit logging silently loses exactly the case it exists to
report. Paying for a flush-before-evaluate journal in order to delete a
working flush-before-evaluate crumb is motion, not progress.

### B. Keep the crumb and add the journal alongside it

**This is what this EIMP now does.** Recorded here as the accepted option
because the earlier draft rejected it, and the reversal should be visible
rather than silent. The earlier objection — two overlapping mechanisms means
`output/` needs a special case and the `"TEST IN PROGRESS"` prefix stays
load-bearing — is now much cheaper than it was: the special case is confined to
`write_output`'s no-op path, and the tree it protects is uncommitted.

### C. Retire the crumb without a replacement

Simply delete it: `EIMP-01`'s Output gate arguably covers "something went
wrong" already. Rejected: it does not. The gate reports that a *completed*
generation disagreed with the baseline; the crumb reports that an evaluation
*never completed*. A hard crash produces no divergence, no completed run, and
— without the crumb — no evidence at all.

### D. Use an existing logging framework (`tracing`, `log`) instead

Adopt `tracing` and emit structured events rather than building on einmo's own
journal. Rejected as the primary mechanism, though worth revisiting for the
human-facing diagnostic layer: it would add a substantial dependency to core
`einmo` immediately after `EIMP-4` split the crate specifically to keep its
dependency tree lean, and it would give einmo two `EinmoId`-keyed record
streams with different shapes.

## Open Questions

- **Where does a test-run journal live?** `EIMP-1` put the session journal in a
  scratch/state dir precisely so it does not travel with the corpus. The same
  reasoning suggests a scratch dir here. Note this is now a pure observability
  question rather than a crash-evidence one — the crumb keeps that job — which
  makes a scratch dir a much easier answer than it was.
- **Which crate owns the journal after `EIMP-4`'s split** (§S.3).
- **Does a gate log, or only generation?** §S.1 item 4 says all four phases.
  Confirm that a non-writing gate emitting a journal record does not violate
  `EIMP-01` §S.0's "it writes nothing" guarantee — a journal in a scratch dir
  is not a suite write, but the guarantee's wording may need a footnote.

## References

- `EIMP-1` (`docs/eimp/EIMP-1.md`) §S.6 — the journal this EIMP generalizes.
- `EIMP-01` (`docs/eimp/EIMP-01.md`) — moves crumb creation into the
  uncommitted `generated/` stage, removing the output-tree-pollution cost that
  motivated this EIMP's original retirement proposal. Its §S.0 names the four
  phases this EIMP's records must cover.
- `EIMP-4` (`docs/eimp/EIMP-4.md`) §S.1 — the crate split that determines
  where the journal substrate must live.
- Code: `src/journal.rs`; `src/einmo_suite.rs` (`write_crash_crumb`,
  `check_catastrophe_crumb`, `is_catastrophe_crumb`, `write_output`'s crumb
  filter, the `catastrophe_crumb_*` tests); `zweimomo/tests/suites.rs`
  (`crash_crumb_survives_stack_overflow`).

## Last Updated

**Date**: 2026-08-11
**Updated By**: Claude Code (Opus 5)
**Changes**: Rescoped to **add** structured JSONL logging to the test-run path
only; **the crash crumb is no longer retired** and is now an explicit non-goal
(§S.2), with the retirement moved to Rejected Alternative A. The reversal has
two causes: `EIMP-01` moves crumb creation into the uncommitted `generated/`
stage, dissolving the output-tree-pollution argument that motivated
retirement; and a journal is a strictly weaker crash-evidence mechanism unless
it also flushes before every evaluation, at which point deleting the crumb
buys nothing. Title and abstract updated to match, the "retirement is
all-or-nothing" constraint removed, the crumb-work freeze (§S.3 of the prior
draft) removed as moot, the test plan rewritten around no-behavior-change, and
`status` moved `Draft` → `Brewing`.
