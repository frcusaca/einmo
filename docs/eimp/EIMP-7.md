---
eimp: 7
title: Structured JSONL logging, and retiring the crash crumb
author: Claude Code (Opus 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-30
supersedes: []
begun: [ ]
---

# EIMP-7: Structured JSONL logging, and retiring the crash crumb

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

**No plan file yet, deliberately.** This EIMP lands after `EIMP-1`, and its
shape depends on what `EIMP-1`'s journal actually turns out to be. Writing a
plan now would be planning against a substrate that does not exist. The plan
is written when this EIMP is picked up.

## Abstract

Generalize `EIMP-1`'s review-session journal into einmo's **general
structured logging substrate** — append-only JSONL, keyed by `EinmoId`,
with verbosity levels — extend it to cover the *test-run* path (not only
the review-session path it was born in), and then **retire the crash crumb
entirely** in favor of it. The crash crumb today writes a placeholder
`.einmo` file into `output/` before every evaluation so that a harness crash
leaves evidence behind; that evidence is real, but it is purchased by
polluting the output tree with a file that is not a test result. An
unmatched `case_start` journal entry carries strictly more information, is
keyed by the same identifier as everything else, and leaves `output/`
containing only actual outputs.

## Motivation

**The crumb works, and its cost is structural.** `write_crash_crumb`
(`src/einmo_suite.rs`) writes a signed `.einmo` into `output/` with
`status: output-error` and a `status_detail` beginning `"TEST IN PROGRESS"`,
before the evaluator runs. If the harness dies mid-evaluation — a stack
overflow, a SIGKILL, a panic that escapes the catch — that file survives and
tells whoever finds it what happened. `zweimomo`'s
`crash_crumb_survives_stack_overflow` proves it survives a genuine stack
overflow, so this is a mechanism that demonstrably does its job.

The cost is that `output/` now contains files that are **not outputs**, and
every consumer of `output/` has to know that:

- **`EIMP-3`'s content/key decision table had to special-case it.** When
  `write_output` compares freshly-evaluated content against the existing
  `output/` baseline, a crumb sitting at that path is not a baseline — it is
  scaffolding this same run wrote moments ago, whose `OUTPUT` section is
  always empty. Without a special case, *every* case would be reported as
  drifted. The fix in `einmo_suite.rs` reads
  `existing.filter(|e| !e.metadata().status_detail.starts_with("TEST IN PROGRESS"))`
  — a string-prefix test on a metadata field, load-bearing for correctness.
- **`check_catastrophe_crumb` is a second consumer** of the same string
  prefix, with its own ignore/refuse/rerun policy and its own config knobs
  (`--ignore-catastrophe-crumbs`, `--rerun-catastrophes`).
- **The prefix is stringly-typed and duplicated.** `"TEST IN PROGRESS"`
  appears at multiple sites in `einmo_suite.rs` (plus tests). Nothing
  structurally prevents a real test whose output legitimately begins with
  that text from being mistaken for a crumb.

**A journal entry costs none of that.** `EIMP-1` §S.6 already specifies a
journal keyed by `EinmoId` with verbosity levels, whose finest level records
each case as it is read in and verified. A case that logs `case_start` and
never logs `case_end` identifies the in-flight case at crash time — the same
question the crumb answers, with more precision (it can carry the phase, the
evaluator, the elapsed time) and without writing anything into `output/`.

**Why this is not part of `EIMP-1`.** `EIMP-1`'s journal lives in the
*review-session* layer; the crumb lives in the *test-run* layer
(`EinmoSuite::evaluate`), which has no review session and must work when no
server, no session, and no reviewer exist. Extending the journal downward
into the test runner is real work with its own design questions (where does
a suite-run journal live? what happens when the suite is read-only?), and
retiring the crumb invalidates a set of existing tests. Folding that into
`EIMP-1` would have widened an already-large EIMP; `EIMP-1` therefore builds
a journal *capable* of the role, and this EIMP completes the migration.

## Specification

**Deliberately thin — this EIMP is scoped, not designed.** Its substrate is
`EIMP-1`'s journal, which does not exist yet; specifying handler shapes and
file formats against it now would be specifying against a guess. What is
fixed here is the scope and the constraints.

### S.1 Scope

1. **Generalize the journal beyond the review session.** It must be usable
   from `EinmoSuite`'s test-run path, where there is no session, no
   reviewer, and possibly no writable suite directory.
2. **Verbosity levels, configurable** — carrying `EIMP-1` §S.6's
   terse/normal/fine levels down into the test-run path, where `fine` means
   one record per `EinmoId` as each case is read, evaluated, and verified.
3. **`EinmoId` keying throughout.** Every case-scoped record carries the
   `EinmoId` — the same identifier the review session, the server, the CLI,
   and the corpus already use. One identifier end to end, no translation
   layer that could disagree.
4. **Crash detection via unmatched records**, replacing the crumb's
   placeholder-file mechanism.
5. **Retire the crash crumb**: remove `write_crash_crumb`,
   `check_catastrophe_crumb`, `is_catastrophe_crumb`, the `"TEST IN
   PROGRESS"` prefix and every consumer of it (including `EIMP-3`'s filter
   in `write_output`), and the associated config knobs — or consciously keep
   whichever of those still earn their place, recording why.

### S.2 Constraints

- **Do not lose the capability.** The crumb's actual guarantee is that
  evidence survives a *hard* crash — SIGKILL, stack overflow, an aborted
  process. A journal only preserves that guarantee if its writes reach disk
  before the evaluation begins. Buffered, flushed-at-exit logging would
  silently lose exactly the case it exists to report. Flush-before-evaluate
  is a correctness requirement, and `zweimomo`'s stack-overflow test (or its
  successor) must still pass.
- **Retirement is all-or-nothing per mechanism.** Leaving the crumb in place
  *and* adding journal entries is the worst outcome: two mechanisms, two
  consumers, twice the special-casing. Either the crumb goes or this EIMP
  does not land.
- **A read-only or absent journal must not fail a test run.** Logging is
  observability; it must never become a new way for a suite to fail.
  (Contrast the crumb, which fails loudly by design when it cannot be
  written — decide deliberately whether that behavior transfers.)
- **`EIMP-4`'s crate boundary holds.** If the journal lives in
  `einmo-review-server` after the split but the test runner lives in core
  `einmo`, the substrate must move to core, or be split. Resolve this
  explicitly — it is the first real design question this EIMP faces.

### S.3 Crumb work is frozen as of 2026-07-30

Pending this EIMP, **no further investment goes into the crash crumb**: no
new features, no new config knobs, no new consumers of the `"TEST IN
PROGRESS"` prefix. Existing behavior and existing tests stay as they are —
the crumb keeps working until this EIMP retires it. This is recorded so that
future work does not deepen a mechanism already scheduled for removal.

## Test Plan

- **The hard-crash guarantee, preserved.** `zweimomo`'s
  `crash_crumb_survives_stack_overflow` — which re-spawns the test binary as
  a child and genuinely overflows the stack — must have a successor
  asserting the *journal* identifies the in-flight case after the same
  crash. This is the load-bearing test; if it cannot be made to pass, the
  crumb should not be retired.
- **Unmatched-record detection**: a run killed mid-case leaves a
  `case_start` with no `case_end`, and tooling identifies the case from it.
- **Verbosity levels**: each level emits what it promises and no more;
  `fine` records every case, `terse` does not.
- **Logging failure is not test failure**: a read-only or unwritable journal
  destination degrades without failing the suite (per §S.2, if that is the
  resolved policy).
- **Existing tests migrated, not deleted.** einmo's `catastrophe_crumb_*`
  tests and `zweimomo`'s crumb test encode real requirements. Each must be
  rewritten against the journal or consciously retired with a recorded
  reason — a test count that drops silently is capability lost silently.
- **`EIMP-3`'s decision table still correct without the filter**: with
  crumbs gone, `write_output`'s `"TEST IN PROGRESS"` filter is removed, and
  the drift/no-op/co-sign/fresh cases must all still behave — verified by
  `EIMP-3`'s existing tests passing unmodified.
- Comprehensive test: run a suite where one case crashes hard, one drifts,
  one co-signs, and one is a clean no-op, and assert the journal tells the
  whole story from `EinmoId`-keyed records alone, with `output/` containing
  only real outputs.

## Rejected Alternatives

### A. Keep the crash crumb; add the journal alongside it

Let both mechanisms coexist — the crumb for the test-run path, the journal
for the review path. Rejected: this is the status quo plus more code. Two
overlapping mechanisms mean `output/` still needs its special case, the
`"TEST IN PROGRESS"` prefix is still load-bearing in `write_output`, and
every future reader has to learn both. §S.2 makes this explicit: either the
crumb goes or this EIMP does not land.

### B. Retire the crash crumb as part of EIMP-1

Fold the retirement into `EIMP-1`'s journal work. Rejected: `EIMP-1` is
already the largest EIMP in the sprint, and the crumb lives in a different
layer (the test runner) than `EIMP-1`'s journal (the review session).
Retirement also invalidates existing tests in two crates, which is exactly
the kind of change that should not be riding along inside a phase about
something else.

### C. Retire the crumb without a replacement

Simply delete it: the drift detection `EIMP-3` added arguably covers the
"something went wrong" case already. Rejected: it does not. Drift detection
reports that a *completed* evaluation disagreed with the baseline; the crumb
reports that an evaluation *never completed*. A hard crash produces no
drift, no failure, and — without the crumb — no evidence at all.

### D. Use an existing logging framework (`tracing`, `log`) instead

Adopt `tracing` and emit structured events rather than building on einmo's
own journal. Rejected as the primary mechanism, though worth revisiting for
the human-facing diagnostic layer: the crash-evidence role needs a
guaranteed flush-before-evaluate to a known location, which is a storage
guarantee rather than a logging-facade concern. It would also add a
substantial dependency to core `einmo` immediately after `EIMP-4` split the
crate specifically to keep its dependency tree lean.

## Open Questions

- **Where does a test-run journal live?** `EIMP-1` put the session journal
  in a scratch/state dir precisely so it does not travel with the corpus.
  The same reasoning suggests a scratch dir here — but crash evidence that
  vanishes with the scratch dir is weaker than the crumb it replaces, which
  sits in the repository where someone will actually find it. This tension
  is the central design question and is not resolved here.
- **Which crate owns the journal after `EIMP-4`'s split** (§S.2, last item).
- **Does the crumb's fail-loudly-if-unwritable behavior transfer?** §S.2
  proposes that logging must not fail a run; the crumb behaves oppositely.
  Both defensible; pick one deliberately.
- **How is an unmatched record surfaced?** A CLI verb (`einmo journal
  --incomplete`), a line in `einmo verify`'s output, or something the next
  run notices automatically — the crumb had the advantage of being found by
  simply looking at `output/`.

## References

- `EIMP-1` (`docs/eimp/EIMP-1.md`) §S.6 — the journal this EIMP generalizes;
  specified to be *capable* of the crumb's purpose without retiring it.
- `EIMP-3` (`docs/eimp/EIMP-3.md`) — its content/key decision table carries
  the `"TEST IN PROGRESS"` filter that this EIMP removes.
- `EIMP-4` (`docs/eimp/EIMP-4.md`) §S.1 — the crate split that determines
  where the journal substrate must live.
- Code: `src/einmo_suite.rs` (`write_crash_crumb`, `check_catastrophe_crumb`,
  `is_catastrophe_crumb`, `write_output`'s crumb filter, the
  `catastrophe_crumb_*` tests); `zweimomo/tests/suites.rs`
  (`crash_crumb_survives_stack_overflow`).
