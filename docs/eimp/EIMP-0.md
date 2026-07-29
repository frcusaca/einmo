---
eimp: 0
title: EIMP Purpose, Process, and Format
author: Claude Code (Sonnet 5) <noreply@anthropic.com>
status: Final
type: Process
created: 2026-07-29
supersedes: []
---

# EIMP-0: EIMP Purpose, Process, and Format

## Abstract

This document defines the **Einmo Improvement Process** (EIMP), the
mechanism for proposing, discussing, and tracking changes to einmo's design
and implementation. EIMP is to einmo what PEP is to Python, JEP to OpenJDK,
and SIP to Scala — and, directly, what FOOP is to the Foolish language: EIMP
is FOOP's process adapted for a standalone, single-maintainer Rust crate.
This document — the meta-document, pinned at `EIMP-0` — defines the process
itself.

## Motivation

Einmo began life inside the `foolish-rust` workspace, where design decisions
were tracked as FOOPs (`FOOP-92` specified einmo itself; `FOOP-25` specified
the not-yet-implemented `EinmoReview` session object). When einmo was
extracted into its own repository, that design history did not automatically
come with it — a standalone crate needs its own place to record "why," not
just "what," or the same problem FOOP-1 identified for Foolish recurs here:

- Decisions get re-litigated whenever someone (including future-self) looks
  at the code.
- The "why" of design choices erodes; only the "what" survives in the source.
- New contributors (human or agent) have no entry point for proposing
  changes.
- Design work already done elsewhere (like `FOOP-25`'s `EinmoReview`
  specification) has no home in the new repository and risks being lost or
  silently re-derived from scratch.

An EIMP is a single document that captures one design decision: the
motivation, the chosen design, the alternatives rejected, and the test plan.
Decisions live as immutable historical artifacts.

## Inspirations

| Process | Maturity | Key takeaway for EIMP |
|---------|---------|------------------------|
| **FOOP** (Foolish) | ~90 documents, one project | Direct ancestor: little-endian numbering, two-file spec/plan layout, checkbox lifecycle — all inherited near-verbatim. |
| **PEP** (Python) | 1000s, 25+ years | Numbered, immutable; metadata header; status state machine; one canonical index. |
| **JEP** (OpenJDK) | ~500, vendor-driven | Strong technical bar; explicit "candidate → completed" gates. |
| **RFC** (IETF) | 9000+, 50+ years | Long-form; alternatives and prior art encouraged. |

EIMP borrows from all of these but stays intentionally **lightweight** — like
FOOP, einmo currently has a small number of maintainers, and process
overhead must not exceed implementation effort.

## Specification

### 1. EIMP Numbering

EIMPs are numbered with **little-endian decimal**: `EIMP-1`, `EIMP-2`,
`EIMP-9`, `EIMP-01`, `EIMP-11`, `EIMP-21`. Numbers are assigned sequentially
in order of submission (not order of acceptance). Numbers are never reused.

**Collation rule**: EIMP-`abcd` sorts by numerical value `dcba`. That is,
read the digits in reverse and sort by the resulting number. Sort the
directory with:

```bash
ls docs/eimp | rev | sort -V | rev
```

**EIMP-0 is pinned, not part of the 1-indexed sequence.** This document is
`EIMP-0` — the process meta-document, the einmo analogue of FOOP-1. It sorts
first by convention (its number is literally `0`), and
`docs/eimp/scripts/eimp_check.py` excludes it from the consecutive-numbering
check that governs `EIMP-1`, `EIMP-2`, .... Real specs and plans (features,
API changes, design decisions) start at `EIMP-1`.

Why little-endian? Because einmo inherited the convention directly from
FOOP, where it was chosen to dovetail with Foolish's preference for
non-conventional notations, and to produce natural time-correlated batching
when the directory is sorted. Einmo keeps the convention for continuity with
its origin and because the mechanical benefit (batching, one shell-line
sort) applies here too.

### 2. EIMP Document Format

Each EIMP is a single Markdown file: `EIMP-N.md` where N is the EIMP number.

Files live in `docs/eimp/`.

The file MUST begin with a YAML front matter block:

```yaml
---
eimp: <number>                        # integer, no zero padding (0 for the meta-doc)
title: <short title>                  # one line
author: <name> <email>                # at least one author
status: <status>                      # see status state machine below
type: <Standards|Process|Informational>
created: YYYY-MM-DD
supersedes: [<eimp>, ...]             # list of EIMPs this replaces (often empty)
superseded_by: <eimp>                 # if status is Superseded, which EIMP replaces this
begun: [ ]                            # [x] once implementation work has commenced
implementation: <commit-sha or PR>    # added when status reaches Implementing+
---
```

Body sections (use `##` headings, in this order):

1. **Abstract** — one paragraph, what this EIMP proposes
2. **Motivation** — why this matters; the problem being solved
3. **Specification** — the design itself, in detail
4. **Test Plan** — how this is verified; new unit/integration tests
5. **Rejected Alternatives** — at least one; designs considered and not chosen
6. **Open Questions** — known unknowns; what's left to decide
7. **References** — links to prior EIMPs, external docs, prior art

An EIMP without **Motivation** and **Rejected Alternatives** is incomplete.
The rejected-alternatives section is the single most valuable historical
artifact an EIMP produces; a future maintainer who only reads "what we
chose" will eventually re-propose the rejected idea.

Note the omission relative to FOOP: einmo has no FIR, no UBC evaluator, and
no `phase` targeting — so EIMP drops FOOP's "FIR Impact," "UBC Step Impact,"
and `phase` frontmatter field entirely, rather than carrying them forward as
always-"None" boilerplate.

### 3. EIMP Types

| Type | Purpose |
|------|---------|
| **Standards** | Adds, removes, or changes einmo's design or public API |
| **Process** | Changes how the project itself operates (EIMP-0 is one) |
| **Informational** | Documents a decision or convention without normative force |

### 4. Status State Machine

```
Draft → Brewing → Final → Implementing → complete
```

| Status | Meaning |
|--------|---------|
| **Draft** | Authored, not yet submitted for review. Editable freely. |
| **Brewing** | Submitted; being actively designed. May still change substantially. |
| **Final** | Accepted. The design is frozen. Ready for implementation planning. |
| **Implementing** | Active coding. The plan is being executed. Open Questions section should be empty (design frozen). |
| **complete** | All work done, tests green, merged to `main`. |

Additional terminal statuses, used sparingly:

| Status | Meaning |
|--------|---------|
| **Withdrawn** | Author retracted before acceptance. |
| **Rejected** | Maintainer declined. The EIMP stays in the index as a historical record. |
| **Superseded** | A later EIMP replaces this one. The superseder's number goes in `superseded_by`. |

Once an EIMP is `complete`, `Rejected`, or `Superseded`, the file is
effectively immutable — corrections happen via a new EIMP that supersedes
it, not by editing history.

### 5. Approval

Einmo currently has a small maintainer group. All EIMP transitions from
`Brewing → Final` and `Final → Implementing` require maintainer approval.
When the project grows, this section will be superseded by a new EIMP
defining a broader review process.

### 6. The Index

`docs/eimp/INDEX.md` is the single canonical list of EIMPs (EIMP-0 plus
every numbered EIMP). It MUST be kept in sync with the actual files. It is
regenerated by listing all `EIMP-*.md` files using the little-endian
collation rule (§1).

The index has columns: number, title, status, created, author.

### 7. Lifecycle Workflow

1. **Author creates** `EIMP-N.md` with status `Draft`. The next available N
   is found via `eimp_check.py gen_next`.
2. **Author submits** by changing status to `Brewing` and committing the
   file plus an `INDEX.md` update.
3. **Discussion happens** in commit messages or directly on the EIMP file
   via further commits. The EIMP body evolves.
4. **Maintainer accepts** by changing status to `Final`, then `Implementing`
   once work begins (`begun: [x]`), recording the commit(s) that carry it
   out.
5. **Implementation lands**; status changes to `complete`.
6. **Or the maintainer rejects**; status changes to `Rejected`. A
   `## Rejection` section is appended explaining why.

### 8. Retroactive EIMPs

Decisions made before this process existed, or made elsewhere (e.g. as a
FOOP in `foolish-rust`) and ported into einmo's own repository, may be
backfilled as EIMPs to preserve the record. Retroactive EIMPs:

- Use `created:` set to the date the decision was actually made (best guess
  if unknown, or the origin FOOP's `created:` date), not the date the EIMP
  file was written.
- Note "Retroactive: ported from `FOOP-<N>` in the foolish-rust workspace,
  documenting a decision made on `<date>`" as the first line of the
  Abstract, when applicable.
- Skip directly to whatever status accurately reflects the ported work's
  actual state (e.g. `Draft` if unimplemented, `complete` if the
  implementation already exists).

`EIMP-1` (`EinmoReview` — a thread-safe review-session object) is the first
retroactive EIMP, ported from `FOOP-25` in the `foolish-rust` workspace.

### 9. What EIMPs Are NOT For

- **Bug fixes**: a one-line change doesn't need an EIMP. Just fix it.
- **Refactoring**: moving code around doesn't need an EIMP.
- **Test additions** that don't change semantics: just add the test.
- **Documentation typos**: just fix them.

Rule of thumb: if you can't articulate the decision in one sentence in the
abstract, it's probably not EIMP-worthy. Conversely, if there are at least
two reasonable options and choosing one over the other has lasting
consequences, write an EIMP.

## Test Plan

The EIMP process itself is "tested" by usage:

- `EIMP-1` (retroactive, ported from `FOOP-25`) demonstrates that the format
  can capture an existing decision cleanly.
- `docs/eimp/INDEX.md` exists and lists EIMP-0 plus every numbered EIMP in
  correct collation order.
- `eimp_check.py check` passes: EIMP-0 present, numbered EIMPs consecutive
  from 1.

## Rejected Alternatives

### A. No process at all

Just write code, leave decisions undocumented. **Rejected**: the exact
motivating case for this document — `FOOP-25`'s `EinmoReview` design already
existed and would have been lost (or silently re-derived, at real cost) had
it not been ported into a structured document in einmo's own repository.

### B. Reuse FOOP's numbering and files verbatim, no EIMP-0 special case

Simply continue FOOP numbers (e.g. the first einmo doc would be `FOOP-93`)
or start a plain `EIMP-1` sequence with no pinned meta-document. **Rejected**:
continuing FOOP numbers ties einmo's own process to a repository it no
longer lives in; a plain `EIMP-1` sequence would either force the process
document itself to compete for a slot in the real numbering (awkward — the
process doc isn't a design decision about einmo) or leave the process
undocumented. Pinning it at `EIMP-0`, excluded from the consecutive check,
solves both: it has a stable identity and never collides with real EIMP
numbers.

### C. Big-endian numbering (EIMP-001, EIMP-002, ..., EIMP010)

The conventional choice. **Rejected**: for continuity with FOOP, and because
the natural batching of "EIMPs from the same era" when correctly sorted is a
useful affordance inherited along with the rest of the process.

### D. Keep FOOP's worktree/`jia`-trunk mechanics verbatim

**Rejected**: that mechanism exists because Foolish is a larger, multi-
contributor project where isolating in-progress feature work in worktrees
matters. Einmo is a small, single-maintainer repository; EIMP plans execute
directly on `main` with regular commits (see `eimp.md` "Plan execution").
Should einmo's contributor base grow, a future Process EIMP can reintroduce
worktree isolation.

## Open Questions

None at time of submission. This EIMP defines the steady-state process; the
natural way to refine it is via subsequent Process EIMPs that supersede it
in part.

## References

- **FOOP-1** (`foolish-rust`) — "FOOP Purpose, Process, and Format," the
  direct ancestor of this document.
- **FOOP-25** (`foolish-rust`) — "EinmoReview — a thread-safe review-session
  object" — ported into this repository as `EIMP-1`.
- **FOOP-92** (`foolish-rust`, Complete) — einmo itself, before extraction
  into its own repository.
- [PEP 1 — PEP Purpose and Guidelines](https://peps.python.org/pep-0001/)
- [JEP 1 — JDK Enhancement-Proposal & Roadmap Process](https://openjdk.org/jeps/1)
- [IETF RFC Editor Style Guide](https://www.rfc-editor.org/styleguide/)
