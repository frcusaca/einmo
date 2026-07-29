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

## Specification

The design itself. Be precise. If a feature has an API, give the Rust
signatures. If it changes behavior, give the before/after.

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
