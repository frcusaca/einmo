---
name: eimp-write-plan
description: "MUST USE when CREATING or PLANNING an EIMP (Einmo Improvement Process) — writing the specification file (EIMP-#.md) and the plan file (EIMP-#.plan.md). Covers: what an EIMP is, the two-file system (spec + plan), little-endian numbering (EIMP-1→EIMP-9→EIMP-01→EIMP-11→EIMP-21...), the EIMP-0 pinned meta-document, the eimp_check.py helper script (check/get_last/gen_next/list), naming convention (dash in filenames, space in prose), the spec template (frontmatter fields eimp/title/status/begun, body sections Abstract/Motivation/Specification/Test Plan/Rejected Alternatives/Open Questions/References), the D-prefix sort-key rule, plan construction rules (ordered/concrete/trackable, sub-tasks, variable expansion), checkbox format (timestamp on next indented line), sub-task splitting, comprehensive test placement, and the minimal plan skeleton. Gives exact copy-pasteable commands with <NUMBER> and <SHORT_DESCRIPTION> placeholders. Triggers: 'create eimp', 'new eimp', 'write eimp', 'eimp spec', 'eimp plan', 'eimp template', 'eimp numbering', 'eimp frontmatter', 'eimp_check gen_next', 'plan an eimp', 'eimp comprehensive test'."
---

# EIMP — Writing and Planning

This skill covers **creating** EIMPs: writing the specification and constructing the plan. For finding, executing, backburnering, or cancelling existing EIMPs, use the `eimp-use-maintain` skill.

> **Authoritative source**: `eimp.md` at the repository root. When this skill and `eimp.md` appear to disagree, `eimp.md` wins. Read `eimp.md` before creating or editing any EIMP.
>
> EIMP is adapted from the Foolish project's FOOP process (see `docs/eimp/EIMP-0.md` §References). Einmo is a small, single-maintainer repository, so this skill drops FOOP's worktree/multi-branch mechanics — EIMP plans execute directly on `jia`.

---

## What an EIMP Is

An EIMP (Einmo Improvement Process) is einmo's equivalent of Python's PEP or Rust's RFC. It proposes, discusses, and tracks changes to einmo's design and implementation.

An EIMP progresses through statuses: `Draft` → `Brewing` (ready for maintainer review) → `Final` (accepted) → `Implementing` (active coding) → complete.

---

## The Two Files of an EIMP

Every EIMP is expressed as (up to) two separate files that share the same `EIMP-<NUMBER>` stem:

| File | Purpose | Answers |
|------|---------|---------|
| `EIMP-#.md` | **Specification** — the proposal, motivation, design, semantics, discussion. | *What* and *why* |
| `EIMP-#.plan.md` | **Plan** — a checkboxed, sequentially-executed breakdown of the work. (Note the lowercase `.plan.md` extension.) | *How* and *in-what-order* |

**Executing an EIMP requires reading BOTH files.** The plan assumes the context of the specification; do not act on `EIMP-#.plan.md` without first reading `EIMP-#.md`. The plan is meant to be executed sequentially from top to bottom.

---

## EIMP Numbering — Little-Endian (Critical)

EIMP numbering is **little-endian**: the filename digits ARE the identifier, but they sort in reverse. Chronological order (oldest → newest):

```
EIMP-1, EIMP-2, EIMP-3, ... EIMP-9, EIMP-01, EIMP-11, EIMP-21, EIMP-31, EIMP-41, EIMP-51, EIMP-61, ...
```

- EIMP-9 is the one **before** EIMP-01.
- The digits in `EIMP-01` are `0` then `1` — read as "ten" when reversed, so its sort key is `10`.
- EIMP-21 → sort key 12. EIMP-51 → sort key 15.

| Filename | Identifier | Sort key (frontmatter only) |
|----------|------------|-----------------------------|
| `EIMP-9.md` | EIMP-9 | 9 |
| `EIMP-01.md` | EIMP-01 | 10 |
| `EIMP-21.md` | EIMP-21 | 12 |
| `EIMP-51.md` | EIMP-51 | 15 |

The **filename digits ARE the identifier**. The `eimp:` frontmatter field is a separate numeric sort key (the digits reversed). Do NOT use the sort-key value as the identifier in prose.

**`EIMP-0` is a pinned exception.** It is the process meta-document (this document's own authority, `eimp.md`'s companion) and sorts first by convention, *outside* the 1-indexed sequence above. `eimp_check.py` excludes it from the consecutive-numbering check. Never assign `EIMP-0` to a real spec/plan — it is reserved.

---

## Naming Convention

| Context | Form | Example |
|---------|------|---------|
| Filename, code, formal citation | `EIMP-<NUMBER>` (dash) | `EIMP-01.md` |
| Prose / sentences | `EIMP <NUMBER>` (space) | "EIMP 01 and EIMP 11 are pre-teen EIMPs." |

The space form in prose reduces digit-reversal errors: writing "EIMP 01" makes it harder to accidentally type "EIMP 10".

---

## File Locations

| What | Path |
|------|------|
| EIMP specs & plans | `docs/eimp/EIMP-<NUMBER>.md` and `docs/eimp/EIMP-<NUMBER>.plan.md` |
| Index | `docs/eimp/INDEX.md` (canonical list, sorted by number) |
| Template | `docs/eimp/EIMP-template.md` |
| Helper script | `docs/eimp/scripts/eimp_check.py` |
| Meta-EIMP (defines the process itself) | `docs/eimp/EIMP-0.md` |

---

## The Numbering Helper Script

Use `docs/eimp/scripts/eimp_check.py` to manage EIMP numbering. Run it before creating a new EIMP and periodically to catch drift.

```bash
python3 docs/eimp/scripts/eimp_check.py check     # verify consecutive numbering (EIMP-0 excluded)
python3 docs/eimp/scripts/eimp_check.py get_last  # most recent numbered EIMP
python3 docs/eimp/scripts/eimp_check.py gen_next  # filename for next EIMP
python3 docs/eimp/scripts/eimp_check.py list      # all EIMPs in chronological order (EIMP-0 first)
```

**When creating a new EIMP, ALWAYS run `gen_next` first** to get the correct filename and identifier. The script handles the little-endian encoding for you.

---

## Task: Create a New EIMP Specification

### Step 1 — Get the next EIMP number

```bash
python3 docs/eimp/scripts/eimp_check.py gen_next
```

Output: `EIMP-<NUMBER>\tEIMP-<NUMBER>.md\t(sort key <N>)`

Remember the `<NUMBER>` — you will substitute it everywhere below.

### Step 2 — Copy the template

```bash
cp docs/eimp/EIMP-template.md docs/eimp/EIMP-<NUMBER>.md
```

### Step 3 — Fill in the frontmatter

Edit `docs/eimp/EIMP-<NUMBER>.md`. The frontmatter must be:

```yaml
---
eimp: D<NUMBER>
title: <SHORT TITLE — one line, no trailing period>
author: <Name> <email@example.com>
status: Draft
type: Standards
created: <YYYY-MM-DD>
supersedes: []
begun: [ ]
---
```

**Frontmatter field rules:**

| Field | Rule |
|-------|------|
| `eimp` | The sort key. Two accepted forms (see "The `eimp:` field" below). |
| `title` | Short title, one line, no trailing period. |
| `status` | Start as `Draft`. Lifecycle: `Draft` → `Brewing` → `Final` → `Implementing` → complete. |
| `type` | Typically `Standards`. |
| `created` | Date in `YYYY-MM-DD` format. |
| `supersedes` | List of EIMP identifiers this one replaces. Empty list `[]` if none. |
| `begun` | `[ ]` (not yet started). Changed to `[x]` when work begins (see `eimp-use-maintain` skill). |

**The `eimp:` field — two accepted forms:**

1. **`D` prefix (big-endian decimal):** `eimp: D<NUMBER>` — the `D` means "the filename digits reversed as a big-endian decimal." So `eimp: D42` = sort key 42 = file `EIMP-24.md`.
2. **Direct value:** `eimp: <NUMBER>` (no `D`) — the literal sort-key value directly.

In all cases, the **filename** (`EIMP-<NUMBER>.md`) is the ultimate identifier and the right numbering. The `eimp:` field is only a sort key for tooling.

### Step 4 — Fill in the body

The body follows this structure (from the template):

```markdown
# EIMP-<NUMBER>: <TITLE>

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root — **read it before creating or editing an EIMP.**

## Abstract

One paragraph. What does this EIMP propose? Read this and you should know
whether to read the rest.

## Motivation

Why does this matter? What's the problem being solved? What does the world
look like today, and what does it look like after this EIMP is implemented?

## Specification

The design itself. Be precise. If it adds or changes a public API, give the
Rust signatures. If it changes behavior, give the before/after.

Use code blocks for anything formal.

## Test Plan

How is this verified?

- New unit tests in `<file>` covering ...
- New integration tests at ...
- Existing tests that need updating ...

If a feature can't be cleanly tested, say so explicitly and explain why.

## Rejected Alternatives

At least one alternative MUST be listed, even if it's just "do nothing" with
an explanation of why doing nothing is worse.

### A. <Alternative name>
Description and reason for rejection.

### B. <Alternative name>
Description and reason for rejection.

## Open Questions

Things still to decide. List them as bullets. As they're resolved, edit the
EIMP body and remove from this section. When this section is empty and the
EIMP is `Implementing`, the design is frozen.

- ?

## References

- Prior EIMPs: ...
- External docs: ...
- Code locations: ...
```

Note: EIMP deliberately has **no "FIR Impact" / "UBC Step Impact" / `phase` field** — those are Foolish-VM-specific concepts with no einmo analogue. Do not add always-"None" boilerplate sections; the template above is the full set.

---

## Task: Create the Plan File

An EIMP is not actionable without a plan. Write `docs/eimp/EIMP-<NUMBER>.plan.md` (lowercase `.plan.md`). **Read `EIMP-<NUMBER>.md` first** — the plan is derived from the specification; the spec exists before the plan, so you can name a concrete `short_description` and decompose the spec into ordered tasks.

### Plan Construction Rules

Build the plan so that:

1. **Tasks are listed in the order they must be executed** (top to bottom).
2. **Each task is concrete and trackable** on its own.
3. **Tasks that prove larger than expected split into indented sub-tasks** (see "Sub-Tasks" below).
4. **All RHS variables should be expanded and literally placed** into the plan file as the plan is being created — einmo plans are simple enough that this usually just means concrete file paths, not `${WORKTREE_*}`-style placeholders (there is no worktree stage; see "No Worktree Stage" below).
5. If the spec has research/experimentation (web search, historic docs, prototyping), those should be **clearly documented in the EIMP file**, and the plan steps shall, where needed, contain **section or sub-section header pointers** into the EIMP file. A large todo with sub-tasks may have several "read section X of EIMP-<NUMBER>.md" as its first few checkboxes.
6. **Sanity-check sub-tasks** may be installed where ambiguity exists — e.g. "[ ] sub-agent please consult with primary agent or human regarding the current approach to..." These can be installed or removed by the planning agent as specification, clarification, design, and planning progresses.
7. **Commit regularly as work proceeds** — do not batch all work into a single commit at the end.

### No Worktree Stage

FOOP (Foolish's process, which EIMP is adapted from) isolates each feature's work in a dedicated git worktree/branch, merged back to a trunk branch (`jia`) on completion. **EIMP does not do this.** Einmo is a small, single-maintainer repository; EIMP plans execute **directly on `jia`**, with regular commits marking progress. Do not add worktree-creation, worktree-cleanup, or merge-to-`jia`-style checkboxes to an EIMP plan — they describe a workflow this repository does not use. If einmo's contributor base grows enough to need isolation, a new Process EIMP should introduce it explicitly (see `EIMP-0` Rejected Alternative D).

### Checkbox Format

When an item is checked off, **always place a timestamp (to the minute) on the next line with indent into the bulleted list**:

```markdown
- [ ] Task not yet done
- [x] Task completed                    ← bad (no timestamp)
- [x] Task completed                    ← good
      (2026-05-06 13:11)                ← timestamped properly
```

This gives both agents and humans a clear view of how work is progressing over time.

### Sub-Tasks

If a task proves larger than expected and splits into multiple sub-tasks, indent them under the parent. Use completed sub-tasks to justify why the split occurred:

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

### Comprehensive EIMP Test

Every EIMP has the right — and the obligation — to generate a **comprehensive test** (or test suite) that thoroughly exercises the new feature interacting with existing features, using einmo's own `cargo test` infrastructure.

| Attribute | Value |
|-----------|-------|
| **Purpose** | Coverage of high-value feature combinations and edge cases that per-module unit tests may not reach. |
| **Process** | Write the tests FIRST (per project rules), implement against them, then run `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` to confirm everything is green. |
| **Placement in plan** | A checkbox task "Write and verify the EIMP-<NUMBER> comprehensive test(s)" should appear in the plan, after all implementation phases and before the final "mark complete" step. |

### Minimal Plan Skeleton

```markdown
# EIMP-<NUMBER>.plan — <SHORT_DESCRIPTION>

- [ ] Begin work: commit EIMP-<NUMBER>.md and EIMP-<NUMBER>.plan.md, check `begun: [x]` in frontmatter
      (YYYY-MM-DD HH:MM)
- [ ] (read §<SECTION> of EIMP-<NUMBER>.md)
- [ ] Write the tests first
- [ ] <implementation task 1>
- [ ] <implementation task 2>
- [ ] Write and verify the EIMP-<NUMBER> comprehensive test(s)
- [ ] All tests pass: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] Update EIMP-<NUMBER>.md frontmatter `status: complete`
- [ ] Update `docs/eimp/INDEX.md`
```

---

## Quick Reference — All Creation Commands

```bash
# ── Numbering ──
python3 docs/eimp/scripts/eimp_check.py gen_next   # get next EIMP number
python3 docs/eimp/scripts/eimp_check.py check      # verify no gaps

# ── Create spec ──
cp docs/eimp/EIMP-template.md docs/eimp/EIMP-<NUMBER>.md
$EDITOR docs/eimp/EIMP-<NUMBER>.md                  # fill frontmatter + body

# ── Create plan ──
$EDITOR docs/eimp/EIMP-<NUMBER>.plan.md             # write from spec, expand all variables
```

---

## Safety Invariants

1. **Read `eimp.md` before creating or editing any EIMP.** This skill is a cookbook; `eimp.md` is the authority.
2. **Always run `gen_next` before creating a new EIMP.** Never guess the next number. Never assign `EIMP-0` — it is reserved for the process meta-document.
3. **At least one Rejected Alternative must be listed** in the spec, even if it's "do nothing."
4. **No worktree/branch checkboxes.** EIMP plans execute directly on `jia`.
5. **Never start substantive work when tests are broken.** Fix first.
6. **Never commit from inside this skill** unless the user explicitly asks.
