# EIMP — Einmo Improvement Process (Full Reference)

> **Read every line of this file before you read or write any EIMP.**
> `AGENTS.md` carries only a short summary of the common, every-day EIMP
> operations and points here for everything else. This document is the
> authoritative description of the EIMP process, its philosophy, the
> numbering system, the file layout, plan construction, and the checkbox
> lifecycle. When `AGENTS.md` and this file appear to disagree about EIMP
> mechanics, this file is the more detailed source — reconcile in favor of
> the explicit rules written here.
>
> EIMP is adapted from the Foolish project's FOOP (Foolish Optimization
> Process); see `docs/eimp/EIMP-0.md` §References for lineage. The numbering
> scheme, two-file layout, and checkbox lifecycle are unchanged. The
> worktree/multi-branch mechanics are simplified — einmo is a small,
> single-maintainer repository without Foolish's `jia`-trunk-plus-worktrees
> workflow, so EIMP plans execute directly against `main` with regular
> commits (see "Plan execution" below).

---

## What an EIMP Is

EIMP documents are einmo's equivalent of Python's PEP or Rust's RFC. They
propose, discuss, and track changes to einmo's design and implementation.

- **Location**: `docs/eimp/EIMP-###.md`
- **Index**: `docs/eimp/INDEX.md` (canonical list, sorted by number)
- **Template**: `docs/eimp/EIMP-template.md`
- **Meta-EIMP**: [EIMP-0](docs/eimp/EIMP-0.md) defines the process itself
  and is pinned outside the normal 1-indexed numbering sequence (see
  "EIMP Numbering is Little Endian" below).

An EIMP progresses through statuses: `Draft` → `Brewing` (ready for
maintainer review) → `Final` (accepted) → `Implementing` (active coding) →
complete.

---

## The Two Files of an EIMP

Every EIMP is expressed as (up to) two separate files that share the same
`EIMP-<NUMBER>` stem:

- **`EIMP-#.md`** — the **specification** and related information: the
  proposal, motivation, design, semantics, and discussion. This is the
  *what* and the *why*.
- **`EIMP-#.plan.md`** — the **plan**: a checkboxed, sequentially-executed
  breakdown of the work needed to implement the specification. This is the
  *how* and the *in-what-order*. (Note the lowercase `.plan.md` extension.)

**Executing an EIMP requires reading BOTH files.** The plan assumes the
context of the specification; do not act on `EIMP-#.plan.md` without first
reading `EIMP-#.md`. The plan is meant to be executed sequentially from top
to bottom.

---

## EIMP Numbering is Little Endian

EIMP-1 is before EIMP-2, EIMP-9 is the one before EIMP-01, and so on and so
forth. To list the directory in order of oldest to newest, use this command:

```bash
ls docs/eimp|rev|sort -V|rev
```

*always* use this command to list the EIMPs to establish ordering.

**EIMP-0 is a special case.** It defines the process itself (the einmo
analogue of a meta-document) and is pinned at `0`, outside the normal
1-indexed little-endian sequence. `docs/eimp/scripts/eimp_check.py`
excludes it from the consecutive-numbering check, the same way it excludes
`EIMP-template.md` and `INDEX.md`. Numbering for real specs/plans starts at
`EIMP-1`.

---

## EIMP Naming Convention (Critical)

The identifier `EIMP-01` uniquely identifies an improvement proposal. In
free text, use "EIMP 01" (no dash, space instead). This convention reduces
the risk of digit reversal: writing "EIMP 01" in prose makes it harder to
accidentally type "EIMP 10". In sentences, use the space form: "EIMP's 01,
11, 21 are the only pre-teen eimps we will implement." Reserve the dash form
`EIMP-01` for filenames, code references, and formal citations only.

The **filename digits ARE the identifier**. The `eimp:` frontmatter field is
a separate numeric sort key, equal to the digits reversed. Do NOT use the
sort-key value as the identifier in prose. Examples:

| Filename     | Identifier (use this) | Sort key (frontmatter only) |
|--------------|-----------------------|-----------------------------|
| `EIMP-9.md`  | EIMP-9                | 9                           |
| `EIMP-01.md` | EIMP-01               | 10                          |
| `EIMP-21.md` | EIMP-21               | 12                          |
| `EIMP-51.md` | EIMP-51               | 15                          |

---

## EIMP Numbering Helper Script

Use `docs/eimp/scripts/eimp_check.py` to manage EIMP numbering. Run it
before creating a new EIMP and periodically to catch drift:

```bash
python3 docs/eimp/scripts/eimp_check.py check     # verify consecutive numbering
python3 docs/eimp/scripts/eimp_check.py get_last  # most recent EIMP
python3 docs/eimp/scripts/eimp_check.py gen_next  # filename for next EIMP
python3 docs/eimp/scripts/eimp_check.py list      # all EIMPs in chronological order
```

When creating a new EIMP, **always** run `gen_next` first to get the correct
filename and identifier. The script handles the little-endian encoding for
you, and excludes `EIMP-0` from the consecutive-sequence check.

---

## Plan Files for EIMP Implementation

When implementing an EIMP, write a detailed plan to
`docs/eimp/EIMP-###.plan.md` (lowercase extension). The plan breaks the EIMP
into concrete, trackable tasks using checkboxes. The plan file should have a
level of detail so as for coding to be immediately commenceable. If research
was done on the web, through prior einmo design docs, or experimentation
performed to establish a correct usage pattern, those should be clearly
documented in the EIMP file; the plan steps shall, where needed, contain
section or sub-section header pointers into the EIMP file — a large todo
with sub-tasks may have several "read such-and-such section of the EIMP" as
first few checkboxes.

The plan sub-tasks can also be sanity check markers for the implementing
agent. For example, if it is clear that the EIMP and plan left some
ambiguity (perhaps at demand of a human saying "we can figure that out when
we get there.") In particular if a major coding decision needs to be made,
or if research and experimentation is expected. The sanity check instruction
subtask could say "[ ] sub-agent please consult with primary agent or human
regarding the current approach to..." During review of EIMP/plan, the
planning agent may install or remove these as it progresses with
specification, clarification, design and planning for the project.

### Constructing the Plan

The plan is derived from the already-written specification (`EIMP-#.md`).
Because the specification exists before the plan, you can name a concrete
`short_description` for the work and decompose the specification into an
ordered list of checkbox tasks. Build the plan so that:

- Tasks are listed in the order they must be executed (top to bottom).
- Each task is concrete and trackable on its own.
- Tasks that prove larger than expected split into indented sub-tasks (see
  "Sub-Tasks" below).
- All RHS variables should be expanded and literally placed into the plan
  file as the plan is being created.
- Once work begins on an EIMP, updates to the `docs/eimp/` folder track the
  same commits as the implementation — there is no separate worktree stage
  to gate them (see "Plan execution" below).

### Checkbox Format

Checkboxes in a plan file track progress. When an item is checked off,
**always place a timestamp (to the minute) on the next line with indent into
the bulleted list**:

```markdown
- [ ] Task not yet done
- [x] Task completed                    ← bad (no timestamp)
- [x] Task completed                    ← good it is
      (2026-05-06 13:11)                ← timestamped properly
```

This gives both agents and humans a clear view of how work is progressing
over time.

### Backburnering (Delay)

When a specification is considered VERY important but interfering with
current highest priorities, it is marked with `[x] backburnered`. To be
revived by removing the `[x] backburnered` marker. These plans are to be
excluded when agent or human asks for plans that are: ready, pending,
iterating, in progress, developing, active, etc. Backburnered plans can only
be found and addressed directly by using the words "backburnered plan(s)".

```markdown
- [x] backburnered
      (2026-05-06 14:00)
- [ ] Do this or system will break
- [ ] And fix that bug
- [ ] ...
```

### Cancelling (Deprecation)

Canceled features shall be marked as "not to be done" using the marker `[-]
don't do this`. An entirely deprecated plan shall have a `[x] canceled` box
at the top. The agent should first add the canceled check item, then mark
all todos with per-item cancellation `[-] each one`. The deprecation can
have elaboration regarding the reasons and context on the same line after
the initial `[x] Canceled.` text. Here is the example of a properly canceled
spec:

```markdown
- [x] Canceled. Optionally explain there's a new spec see EIMP-####
      (2026-05-06 14:00)
- [-] Do this or system will break
- [-] And fix that bug
- [-] ...
```

### Plan execution

Expect to execute each task one after another. Parent tasks should not be
checked off until children are complete. Once a project starts, the
`begun: [ ]` checkbox is checked in the EIMP's frontmatter, and the EIMP
file is committed stating that work has commenced on such and such EIMP.
Because einmo is a small, single-maintainer repository (unlike Foolish's
`jia`-trunk-plus-worktrees layout), **EIMP work happens directly on `main`**
— there is no separate worktree/branch-per-EIMP stage. Good progress should
be committed regularly, as logical units complete. Upon completion, or at
request of the user, the EIMP's checkboxes are all checked off and its
status updated to reflect the completed work.

When asking a human questions, always remind them: "Above message comes
from EIMP-<NUMBER> working to ...brief description...; changes are on
`main`. PTAL"

### Sub-Tasks

If a task proves larger than expected and splits into multiple sub-tasks,
indent them under the parent. Use completed sub-tasks to justify why the
split occurred:

```markdown
- [ ] Implement the thing # <-- this checkbox is the last to be checked after all sub-tasks are done.
  - [ ] Write the tests first
  - [x] Detected a design gap requiring additional work
        (2026-05-06 14:00)
  - [ ] Implement the missing piece
  - [x] Tests green
        (2026-05-06 14:31)
  - [ ] `cargo fmt` / `cargo clippy -D warnings` clean
```

---

## Comprehensive EIMP Tests

Every EIMP has the right — and the obligation — to generate a
**comprehensive test** (or test suite) that thoroughly exercises the new
feature interacting with existing features, using einmo's own Rust unit/
integration test infrastructure (`cargo test`) — there is no `.foo`
approval-test corpus in this repository the way there is in Foolish.

- **Purpose**: coverage of high-value feature combinations and edge cases
  that per-module unit tests may not reach.
- **Process**: the agent writes the tests FIRST (per project rules), then
  implements against them, then runs `cargo test` / `cargo clippy -D
  warnings` / `cargo fmt --check` to confirm everything is green.
- **Placement in plan**: a checkbox task "Write and verify the EIMP-<N>
  comprehensive test(s)" should appear in the plan, after all implementation
  phases and before the final "mark complete" step.

---

## Last Updated

**Date**: 2026-07-29
**Updated By**: Claude Code (Sonnet 5)
**Changes**: Created `eimp.md`, adapted from the Foolish project's `foop.md`
(FOOP → EIMP terminology, `docs/foop/` → `docs/eimp/`). Dropped the
worktree/multi-branch (`jia`-trunk) mechanics — einmo is a small,
single-maintainer repository, so EIMP plans execute directly against `main`
with regular commits instead of a per-EIMP worktree/branch lifecycle.
Dropped the `.foo`-approval-test-specific "Comprehensive FOOP Tests"
section in favor of a "Comprehensive EIMP Tests" section referencing
einmo's own `cargo test` infrastructure. EIMP-0 introduced as a pinned,
non-1-indexed meta-document (the einmo analogue of FOOP-1), excluded from
`eimp_check.py`'s consecutive-numbering check.
