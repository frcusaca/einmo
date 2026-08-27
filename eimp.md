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
> workflow, so EIMP plans execute directly against `jia` with regular
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

## Three-Axis Impact Description

Every EIMP specification includes a document-wide **Impact Overview near the
front**, after Motivation and before the detailed Specification. It explains
the proposal through three named impact aspects plus a mandatory Migration
section:

1. **Einmo Library User Experience changes** — public Rust API additions,
   removals, signature or type changes, accepted-input and error behavior,
   compatibility/migration effects, and observable semantics for applications
   embedding einmo.
2. **Einmo Integration changes** — CLI commands and exit behavior,
   configuration and file-format effects, test/evaluator behavior, HTTP/TUI or
   external-tool contracts, documentation updates, deployment/packaging
   consequences, and tests or CI gates that must be added or changed.
3. **Einmo Development changes** — all notices developers of einmo need before
   coding or reviewing the change: internal architecture and ownership,
   affected and newly required tests/fixtures/CI gates, invariants future code
   must preserve, new or unusual mechanisms and terminology contributors must
   learn, contributor workflow, planning and extension rules,
   debugging/observability needs, and maintenance/release consequences.
4. **Migration** — a uniform, ordered migration procedure for incompatible
   changes: affected versions/artifacts/callers, prerequisites, backup, exact
   commands or code edits, validation, rollback, and removal date for any
   compatibility bridge.

The overview is concrete, not an always-“None” form. It describes the largest
document-wide consequences and says **No change** with a short reason when an
axis genuinely does not apply. “No public Rust signature changes; the existing
method now returns `IllegalTransition` for two formerly accepted pairs” is
useful, while a bare “None” is not.

Migration is the deliberate exception: it is **always present** and is expected
to say **None** for most EIMPs. If any proposed change is incompatible for a
library caller, CLI/configuration user, stored artifact, external integration,
test fixture, deployment, or contributor workflow, replace None with the full
ordered migration procedure. Do not scatter required migration steps across
other sections without collecting them here.

Each significant Specification sub-section should also be explainable through
the same impact aspects. Include explicit paragraphs or subheadings
inside the sub-section when its consequences are not already obvious from the
document-wide overview. At minimum, name:

- the exact public/library behavior before and after;
- commands, integrations, documentation, tests, fixtures, and CI jobs that stop
  working, change meaning, or must be added;
- the architectural rule, affected tests, unfamiliar concepts, and contributor
  obligations created for future work;
- any subsection-specific migration steps, also consolidated in Migration.

An EIMP plan preserves this traceability. Each implementation sub-section either
links to the applicable impact description or includes tasks covering all
affected axes and Migration. Human-decision tasks present the proposed choice
together with its consequences; they do not ask a human to approve an abstract
policy without showing which APIs, commands, tests, documents, and development
rules it changes.

This is einmo-specific impact analysis, not the Foolish-VM “FIR Impact” or “UBC
Step Impact” fields. Do not add those unrelated fields or duplicate boilerplate.

When an EIMP proposes a new subsystem, abstraction, vocabulary, or significant
refactor, it must state the complete proposed steady state at least once. Give
the reader the “way it will work from start to finish”: caller or operator
intent, API entry, internal processing and authority, externally visible
success, failure/conflict behavior, recovery or rollback, tests/CI evidence, and
ongoing developer maintenance. Define new terms before using them. Later
subsections may use established vocabulary without repeating the narrative;
small changes using already-established standards may simply cite them.

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
- Each implementation sub-section links to or expands the specification's
  Library User Experience, Integration, Development, and Migration impacts.
  Tasks cover
  affected APIs; commands/configuration/formats/docs/tests/CI; and internal
  architecture/developer notices. Incompatible changes include ordered
  migration and rollback tasks. A human gate presents every aspect before
  requesting a decision.
- Every sub-section (and every phase that is not subdivided) STARTS with the
  "Establish relevant tests" checkbox naming that sub-section's test subset
  (see "Sub-Section Test Subsets" below).

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
`jia`-trunk-plus-worktrees layout), **EIMP work happens directly on `jia`**
— there is no separate worktree/branch-per-EIMP stage. Good progress should
be committed regularly, as logical units complete. Upon completion, or at
request of the user, the EIMP's checkboxes are all checked off and its
status updated to reflect the completed work.

When asking a human questions, always remind them: "Above message comes
from EIMP-<NUMBER> working to ...brief description...; changes are on
`jia`. PTAL"

### Sub-Section Test Subsets (frequent-run discipline)

Every sub-section of the plan — and every phase that is not subdivided — **starts** with one
checkbox that establishes the SMALL set of tests relevant to that sub-section: the old unit
tests its work must not break, plus the new tests written for it. The checkbox names the
tests and links to the central test-running documentation; the planner fills in REAL test
names (expand every placeholder, same rule as other plan variables):

```markdown
- [ ] Establish relevant tests for this sub-section. Use [these instructions](../../README.md#running-specific-tests) to run: <test_name_1>, <test_name_2>, <module>::<test_a>.
```

The list is alive: as the sub-section writes new tests, each one is added to its checkbox's
list.

**During development** the implementer runs this subset frequently — after each feature
increment and each time a new test lands — and analyzes the results before moving on. **When
the sub-section is complete**, ALL tests run (`just` — the full gate) — do not wait for the
phase boundary if the sub-section ends earlier.

**Run tests through subagents whenever the environment provides them.** Parallel subagent test
runs are the agent equivalent of a human opening several terminals: launch the test subset as
a separate subagent task, keep implementing, and collect the results. Do not serialize long
test runs behind typing when a subagent could be running them.

The command forms live ONLY in `README.md` §"Running specific tests" — the plan names TESTS,
the central document owns the COMMANDS. When the test tooling evolves, only that README
section changes; existing plans keep working because they reference tests by name.

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

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Required every new subsystem, abstraction, vocabulary, or major
refactor to include one forward-facing start-to-finish proposed steady-state
narrative; later sections may rely on established vocabulary without repetition.

**Date**: 2026-08-13
**Updated By**: Sisyphus (mimo-v2.5-pro)
**Changes**: Added **§"Sub-Section Test Subsets (frequent-run discipline)"** under Plan Files:
every plan sub-section (and every undivided phase) STARTS with an "Establish relevant tests"
checkbox naming the sub-section's small test subset — old unit tests the work must not break,
plus new tests as they are written — and linking to the central test-running reference
(`README.md` §"Running specific tests"). The subset runs frequently during development; when
the sub-section completes, ALL tests run. Implementers run tests through subagents in parallel
where available (the agent equivalent of several terminals). Plans name TESTS; the command
forms live only in the README section, so tooling evolution touches one place. Added the
matching bullet to "Constructing the Plan"; both EIMP skills updated to match.

**Date**: 2026-07-31
**Updated By**: Sisyphus (mimo-v2.5-pro)
**Changes**: Updated all references from `main` to `jia` (primary branch
name correction per maintainer). Added post-EIMP follow-up items to
EIMP-1.plan.md (worktree best practices EIMP, test-suite performance).
