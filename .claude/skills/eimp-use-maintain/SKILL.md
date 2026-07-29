---
name: eimp-use-maintain
description: "MUST USE when FINDING, EXECUTING, UPDATING, BACKBURNERING, or CANCELLING existing EIMPs (Einmo Improvement Process). Covers: listing/finding EIMPs in chronological order (little-endian ls|rev|sort -V|rev, eimp_check.py list/get_last/check), the EIMP-0 pinned meta-document, the two-file system (must read both spec and plan before executing), status lifecycle (Draft→Brewing→Final→Implementing→complete), plan execution flow (begin→work-on-main→commit-regularly→mark-complete), checkbox lifecycle (completing with timestamp, backburnering with [x] backburnered, cancelling with [x] Canceled + [-] per-item), sub-task execution patterns (parent not checked until children done), comprehensive test verification via cargo test, human communication protocol (PTAL reminder with EIMP number), and safety invariants. Gives exact copy-pasteable commands with <NUMBER> and <SHORT_DESCRIPTION> placeholders. Triggers: 'find eimp', 'list eimp', 'execute eimp', 'eimp status', 'eimp execution', 'check eimp checkbox', 'backburner eimp', 'cancel eimp', 'deprecate eimp', 'eimp begun', 'eimp progress', 'resume eimp'."
---

# EIMP — Using and Maintaining

This skill covers **finding, executing, updating, backburnering, and cancelling** existing EIMPs. For creating a new EIMP spec or writing a plan, use the `eimp-write-plan` skill.

> **Authoritative source**: `eimp.md` at the repository root. When this skill and `eimp.md` appear to disagree, `eimp.md` wins. Read `eimp.md` before executing or maintaining any EIMP.
>
> EIMP is adapted from the Foolish project's FOOP process (see `docs/eimp/EIMP-0.md` §References). Einmo is a small, single-maintainer repository, so this skill has no worktree/merge-to-trunk stage — EIMP plans execute directly on `main`.

---

## The Two Files of an EIMP (Read Both)

Every EIMP is expressed as (up to) two separate files:

| File | Purpose |
|------|---------|
| `EIMP-<NUMBER>.md` | **Specification** — the *what* and *why*. |
| `EIMP-<NUMBER>.plan.md` | **Plan** — the *how* and *in-what-order* (lowercase `.plan.md`). |

**Executing an EIMP requires reading BOTH files.** The plan assumes the context of the specification; do not act on `EIMP-<NUMBER>.plan.md` without first reading `EIMP-<NUMBER>.md`. The plan is meant to be executed sequentially from top to bottom.

---

## EIMP Numbering — Little-Endian (for finding)

EIMP numbering is **little-endian**: the filename digits ARE the identifier, but they sort in reverse. Chronological order (oldest → newest):

```
EIMP-1, EIMP-2, ... EIMP-9, EIMP-01, EIMP-11, EIMP-21, EIMP-31, EIMP-41, EIMP-51, EIMP-61, ...
```

EIMP-9 is the one **before** EIMP-01. The `eimp:` frontmatter field is a separate sort key (digits reversed) — do NOT use it as the identifier in prose.

**`EIMP-0` is pinned outside this sequence** — it is the process meta-document, not a numbered spec/plan.

| Context | Form | Example |
|---------|------|---------|
| Filename, code, formal citation | `EIMP-<NUMBER>` (dash) | `EIMP-01.md` |
| Prose / sentences | `EIMP <NUMBER>` (space) | "EIMP 01 and EIMP 11 are pre-teen EIMPs." |

---

## Task: Find and List EIMPs

### List all EIMPs in chronological order

**Always** use this command to establish ordering. Do not `ls` naively — little-endian breaks alphabetical sort.

```bash
ls docs/eimp | rev | sort -V | rev
```

Or use the helper script (gives identifiers + sort keys, EIMP-0 listed first):

```bash
python3 docs/eimp/scripts/eimp_check.py list
```

### Find the most recent numbered EIMP

```bash
python3 docs/eimp/scripts/eimp_check.py get_last
```

Output: `EIMP-<LAST_NUMBER>\tEIMP-<LAST_NUMBER>.md\t(sort key <N>)`

### Check numbering integrity

Run periodically to catch drift (gaps, duplicates, or a missing EIMP-0):

```bash
python3 docs/eimp/scripts/eimp_check.py check
```

### Find a specific EIMP's files

```bash
ls docs/eimp/EIMP-<NUMBER>*.md
# Shows: docs/eimp/EIMP-<NUMBER>.md  and  docs/eimp/EIMP-<NUMBER>.plan.md (if it exists)
```

### Find EIMPs by status

EIMPs do not have a built-in status filter in the helper script. To find EIMPs at a specific status, grep the frontmatter:

```bash
# Find all EIMPs in "Implementing" status:
grep -l '^status: Implementing' docs/eimp/EIMP-*.md

# Find all EIMPs in "Draft" status:
grep -l '^status: Draft' docs/eimp/EIMP-*.md

# Find all EIMPs that have begun (begun: [x]):
grep -l '^begun: \[x\]' docs/eimp/EIMP-*.md
```

### Find backburnered plans

Backburnered plans are excluded from normal "ready/pending/active" queries. They can **only** be found by explicitly searching for the backburner marker:

```bash
grep -l 'backburnered' docs/eimp/EIMP-*.plan.md
```

---

## EIMP Status Lifecycle

An EIMP progresses through statuses:

```
Draft → Brewing → Final → Implementing → complete
```

| Status | Meaning |
|--------|---------|
| `Draft` | Initial state. Being written, not yet ready for review. |
| `Brewing` | Ready for maintainer review. The spec is complete enough for discussion. |
| `Final` | Accepted. The design is frozen. Ready for implementation planning. |
| `Implementing` | Active coding. The plan is being executed. Open Questions section should be empty (design frozen). |
| `complete` | All work done, tests green, committed to `main`. |

To change status, edit the `status:` field in the EIMP's frontmatter:

```yaml
status: Implementing
```

The `begun:` field tracks whether work has started:

```yaml
begun: [ ]   # not yet started
begun: [x]   # work has begun
```

---

## Task: Execute an EIMP Plan

### Execution Flow (step by step)

1. **Read both files.** Read `EIMP-<NUMBER>.md` (the spec) first, then `EIMP-<NUMBER>.plan.md` (the plan). Do not act on the plan without the spec's context.

2. **Begin work**:
   - Check the `begun: [x]` box in the EIMP's frontmatter.
   - Commit the EIMP file stating that work has commenced on this EIMP.

3. **Work directly on `main`.** Unlike FOOP (Foolish's process, which EIMP is adapted from), there is **no worktree or per-EIMP branch** — einmo is a small, single-maintainer repository. Implementation, and any further edits to the EIMP spec or plan, all happen as regular commits on `main`.

4. **Commit regularly** as progress is made. Good progress should be committed frequently, as logical units complete — not batched into one commit at the end.

5. **Execute checkboxes top-to-bottom.** Each task is executed one after another. Parent tasks are not checked off until all children are complete.

6. **Upon completion** (or at request of user), check off the final checkboxes, update the EIMP's `status:` to `complete`, and update `docs/eimp/INDEX.md`.

### When asking the human questions

Always remind them of context:

> Above message comes from EIMP-<NUMBER> working to <brief description>; changes are on `main`. PTAL

---

## Task: Checkbox Lifecycle

### Completing a task (with timestamp)

When an item is checked off, **always place a timestamp (to the minute) on the next line with indent**:

```markdown
- [x] Task completed
      (2026-07-11 14:32)
```

**Wrong** (no timestamp):
```markdown
- [x] Task completed
```

**Parent tasks are not checked until all children are complete.** This is a hard rule — the parent checkbox is the last to be checked in a block of sub-tasks.

### Backburnering (Delaying)

When a specification is considered VERY important but interfering with current highest priorities, it is marked with `[x] backburnered`. To be revived by removing the `[x] backburnered` marker.

```markdown
- [x] backburnered
      (2026-07-11 14:00)
- [ ] Do this or system will break
- [ ] And fix that bug
- [ ] ...
```

**Exclusion rule:** These plans are to be **excluded** when an agent or human asks for plans that are: ready, pending, iterating, in progress, developing, active, etc. Backburnered plans can **only** be found and addressed directly by using the words "backburnered plan(s)".

**Reviving:** Remove the `[x] backburnered` marker (and its timestamp line) from the plan. The remaining tasks become active again.

### Cancelling (Deprecation)

Canceled features are marked as "not to be done." The procedure:

1. **First** add the canceled checkbox at the top of the plan.
2. **Then** mark **all** todo items with per-item cancellation `[-]`.
3. The deprecation can have elaboration regarding the reasons and context on the same line after the initial `[x] Canceled.` text.

```markdown
- [x] Canceled. Optionally explain — see EIMP-<NEW_NUMBER>
      (2026-07-11 14:00)
- [-] Do this or system will break
- [-] And fix that bug
- [-] ...
```

**Entirely deprecated plan:** Has a `[x] Canceled` box at the top, and every todo is marked `[-]`.

**Per-item cancellation:** Use `[-]` (not `[ ]` or `[x]`) for each cancelled task. This distinguishes "not done because cancelled" from "not done yet" (`[ ]`) and "done" (`[x]`).

**Cancelling because content moved elsewhere** (e.g. superseded by a differently-scoped EIMP, or migrated to a different document entirely): use the same `[x] Canceled.` + `[-]`-per-item pattern, with the explanation naming where the content now lives:

```markdown
- [x] Canceled. Superseded — see EIMP-<NUMBER> for the current design.
      (2026-07-11 14:00)
- [-] <original task 1>
- [-] <original task 2>
```

---

## Task: Comprehensive Test Verification

Every EIMP should have a comprehensive test (or test suite), written using einmo's own `cargo test` infrastructure (see `eimp-write-plan` skill). During execution, before marking the EIMP complete, verify it:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

All three must be clean before an EIMP's status moves to `complete`.

---

## Task: Resume an EIMP (after interruption)

If an EIMP was in progress and work was interrupted:

1. **Find the EIMP**: `python3 docs/eimp/scripts/eimp_check.py list` or look for `begun: [x]` in frontmatter.
2. **Read both files**: `EIMP-<NUMBER>.md` and `EIMP-<NUMBER>.plan.md`.
3. **Check the plan for completed checkboxes** — they have timestamps, so you can see where work stopped.
4. **Continue from the next unchecked checkbox**, directly on `main` (no worktree to recreate).

### Finding backburnered plans to revive

Backburnered plans are excluded from normal queries. To find them:

```bash
grep -l 'backburnered' docs/eimp/EIMP-*.plan.md
```

To revive: remove the `[x] backburnered` marker (and its timestamp line) from the plan.

---

## Quick Reference — All Execution Commands

```bash
# ── Finding ──
python3 docs/eimp/scripts/eimp_check.py list       # all EIMPs, chronological (EIMP-0 first)
python3 docs/eimp/scripts/eimp_check.py get_last   # most recent numbered EIMP
python3 docs/eimp/scripts/eimp_check.py check      # verify consecutive numbering
ls docs/eimp | rev | sort -V | rev                 # chronological ls
grep -l '^status: Implementing' docs/eimp/EIMP-*.md  # find by status
grep -l 'backburnered' docs/eimp/EIMP-*.plan.md      # find backburnered

# ── Comprehensive test ──
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

---

## Safety Invariants

1. **Read `eimp.md` before executing or maintaining any EIMP.** This skill is a cookbook; `eimp.md` is the authority.
2. **Read BOTH files (spec + plan) before acting.** The plan assumes the spec's context.
3. **Work happens directly on `main`.** No worktree, no per-EIMP branch — commit regularly.
4. **Execute checkboxes top-to-bottom.** Parent tasks are not checked until all children are complete.
5. **Every checkbox completion gets a timestamp** on the next indented line (to the minute).
6. **Backburnered plans are excluded** from "ready/pending/active" queries. Only found by explicitly saying "backburnered."
7. **Cancelled plans** have `[x] Canceled` at top + `[-]` on every todo item.
8. **Never start substantive work when tests are broken.** Fix first.
9. **Never commit from inside this skill** unless the user explicitly asks.
