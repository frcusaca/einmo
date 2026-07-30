# EIMP Index

Canonical list of all Einmo Improvement Process documents.

EIMP numbers are little-endian: EIMP-`abcd` sorts by numerical value `dcba`.
`EIMP-0` is a pinned meta-document (the process itself) and sorts first by
convention, outside the 1-indexed sequence. Sort the numbered directory
entries with:

```bash
ls docs/eimp | rev | sort -V | rev
```

---

| EIMP | Title | Status | Created | Author |
|------|-------|--------|---------|--------|
| [EIMP-0](EIMP-0.md) | EIMP Purpose, Process, and Format | Final | 2026-07-29 | Claude Code (Sonnet 5) |
| [EIMP-1](EIMP-1.md) | EinmoReview — a thread-safe review-session object; thin bash, server, and dhtml frontends | Implementing | 2026-07-19 | Atlas (ported by Claude Code (Sonnet 5)) |
| [EIMP-2](EIMP-2.md) | einmo-review-server — a minimal HTTP prototype of the review/sign/promote/flag loop | complete | 2026-07-29 | Claude Code (Sonnet 5) |
| [EIMP-3](EIMP-3.md) | Output-stage drift fails the run; explicit regenerate; multi-signer output stamps | complete | 2026-07-30 | Claude Code (Sonnet 5) |
| [EIMP-4](EIMP-4.md) | Split einmo into core + einmo-review-server, publish both to crates.io at 0.0.6 | Draft | 2026-07-30 | Claude Code (Opus 5) |
| [EIMP-5](EIMP-5.md) | Parallelized corpus signing — parallel machinery over the Merkle-structured CorpusSigner | Draft | 2026-07-30 | Claude Code (Opus 5) |

---

## The jia-sprint (current)

**Goal**: a functioning einmo library and review system, ready for foolish to
depend on it as a normal crates.io dependency instead of the stale vendored
copy at `/yolo/src/einmo`.

The sprint's EIMPs, in execution order:

1. **`EIMP-1`** (Implementing) — finish the review loop: the remaining
   `EinmoReview` surface, `ReviewMode`, multi-signer promote, flag
   semantics, the journal, the TUI-owned private server, the dhtml
   frontend, `CorpusSigner` (single-threaded).
2. **maintainer performance-verifies the review loop** — an explicit STOP
   in `EIMP-1.plan.md`, and `EIMP-4`'s first gate.
3. **`EIMP-4`** (Draft) — split into `einmo` + `einmo-review-server`,
   publish both at `0.0.6`, repoint `foolish-ubca` and `/yolo/src/zweimomo`
   at the published crate, delete the vendored copy.

Explicitly **outside** the sprint: `EIMP-5` (parallel corpus signing — the
serial implementation ships in `EIMP-1` and is sufficient at current corpus
sizes), the Merkle-tree restructuring design, and the logging EIMP that
would retire the crash crumb. All three are recorded rather than dropped —
see `docs/todo/AIAGENT-einmo-repo.todo.md`.

---

## Last Updated

**Date**: 2026-07-30 (3)
**Updated By**: Claude Code (Opus 5)
**Changes**: Named the current sprint the **jia-sprint** (above) and added
the two EIMPs that scope it. `EIMP-4` specifies splitting the repository
into a lean core `einmo` and an `einmo-review-server` crate carrying
`EinmoReview`, the server, the TUI, and the dhtml frontend, then publishing
both at `0.0.6` — the split exists because `foolish-ubca` imports five
symbols but would otherwise inherit a whole HTTP stack. `EIMP-5` takes
`CorpusSigner`'s parallel machinery, deliberately split out so `EIMP-1` can
ship it single-threaded and core can stay runtime-free. `EIMP-1` was
re-baselined against reality: its Phase 0 drift survey is done, every item
`EIMP-2` already delivered is checked off with attribution, Phase D is
re-scoped (the reduction happened by replacement, not edit), and §S.7a now
specifies the TUI-owned private server (which implies an axum 0.7→0.8
upgrade that deletes `EIMP-2`'s hand-rolled UDS accept loop).

**Date**: 2026-07-30 (2)
**Updated By**: Claude Code (Sonnet 5)
**Changes**: `EIMP-3` reached `complete` — both phases plus the
comprehensive test implemented and verified against `zweimomo`'s real
`day.1` fixture (no-op rerun, second-signer co-sign, drift-fails-untouched,
`regenerate_output` replace, clean rerun). Also began `EIMP-1` (all six
Open Questions resolved, `status: Implementing`) — no implementation phases
of it are done yet.

**Date**: 2026-07-30
**Updated By**: Claude Code (Sonnet 5)
**Changes**: Added `EIMP-3` — output-stage drift now fails a suite run
instead of silently overwriting `output/`; a new explicit `einmo
regenerate-output` verb replaces drifted content deliberately; extends the
existing skip-if-unchanged fast path to multi-signer accumulation at
`output` (a second signer's matching content gets a stamp appended, not a
rewrite). Scoped as the core-test-run analogue of `EIMP-1`'s own
`checked`/`verified` multi-signer accumulation. Work begun.

**Date**: 2026-07-29 (3)
**Updated By**: Claude Code (Sonnet 5)
**Changes**: `EIMP-2` reached `complete` — all ten plan phases (A–J) plus
the comprehensive test implemented, tested, and verified end-to-end
against `zweimomo`'s real suite over a pty-driven `einmo_review_client.sh`
session. Frontmatter status updated; the "Resolved during scoping" record
removed from `EIMP-2.md`'s Open Questions per `EIMP-0`'s convention
(the plaintext-passphrase-transport "Still open" item remains, as intended).

**Date**: 2026-07-29 (2)
**Updated By**: Claude Code (Sonnet 5)
**Changes**: Added `EIMP-2` — a minimal HTTP-server prototype slice of
`EIMP-1`'s `EinmoReview` design (list/body/decide/execute over a unix-domain
socket, `experimental_reviewer.sh` rewired to call it instead of shelling
out to `einmo` directly), including a JavaScript-only (Boa) port of
`foolish-rust`'s `zweimomo` test crate to provide real test fixtures.

**Date**: 2026-07-29
**Updated By**: Claude Code (Sonnet 5)
**Changes**: Created the EIMP index. Seeded with `EIMP-0` (the process
meta-document) and `EIMP-1` (`EinmoReview`, retroactively ported from
`FOOP-25` in the `foolish-rust` workspace).
