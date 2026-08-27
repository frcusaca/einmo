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

# EIMP-<NUMBER>: <TITLE>
EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root — **read it before creating or editing an EIMP.** The one
template-specific note: the `eimp:` front-matter field may either match the
filename digits directly:
```markdown
eimp: <NUMBER>
```
or give the big-endian decimal value, preceded by `D` (so `eimp: D42` is the
same as `eimp: 24`, i.e. the file `EIMP-24.md`):
```markdown
eimp: D<NUMBER>
```
In all cases, the `EIMP-<NUMBER>.md` file name is ultimately the right
numbering.

## Abstract

One paragraph. What does this EIMP propose? Read this and you should know
whether to read the rest.

## Motivation

Why does this matter? What's the problem being solved? What does the world
look like today, and what does it look like after this EIMP is implemented?

## Impact Overview

Give the document-wide consequences through the three impact aspects and the
mandatory Migration section. Be concrete; write “No change” with a reason when
an impact aspect genuinely does not apply rather than leaving it blank.

### Einmo Library User Experience changes

What public Rust APIs, accepted inputs, errors, compatibility guarantees, and
observable library semantics change? What must an embedding application migrate?

### Einmo Integration changes

What CLI commands/exit behavior, configuration, file formats, evaluator/test
semantics, HTTP/TUI/external-tool contracts, documentation, packaging, tests,
fixtures, and CI gates change or must be added?

### Einmo Development changes

What must einmo developers know before coding or reviewing this change? Name
internal architecture/ownership, affected and new tests/fixtures/CI gates,
invariants, new or unusual machinery and terminology to learn, contributor
workflow, extension/planning rules, diagnostics, maintenance, and release work.

### Migration

None.

Keep “None” when no migration is required. For any incompatible library, CLI,
configuration, format, integration, fixture, deployment, or contributor-workflow
change, replace it with one ordered procedure naming affected versions and
artifacts, prerequisites, backup, exact commands/code edits, validation,
rollback, and compatibility-bridge removal.

## Specification

The design itself. Be precise. If a feature has an API, give the Rust
signatures. If it changes behavior, give the before/after.

For each significant sub-section, include or link to the applicable impact and
migration descriptions when they are not already obvious from the overview. A reviewer should
be able to identify the affected library surface, integrations/tests/CI/docs,
internal development obligations, and any migration before approving that
sub-section. Subsection migration steps must also be consolidated in Migration.

If this EIMP introduces a subsystem, abstraction, vocabulary, or significant
refactor, include one section titled `Proposed steady state — ...` that explains
the whole lifecycle from caller/operator intent through API, internal authority
and processing, visible success, failure/conflict, recovery/rollback, CI proof,
and developer maintenance. Define new terms there. Do not repeat established
vocabulary in every subsection; cite its authoritative definition.

Use code blocks for anything formal:

```rust
// example: a new public type and its constructor.
pub struct Foo {
    pub(crate) bar: String,
}
```

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

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Added the start-to-finish Proposed steady state requirement for new
subsystems, abstractions, vocabulary, and significant refactors, while allowing
later sections to cite established definitions.
