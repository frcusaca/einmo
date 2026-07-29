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
| [EIMP-1](EIMP-1.md) | EinmoReview — a thread-safe review-session object; thin bash, server, and dhtml frontends | Draft | 2026-07-19 | Atlas (ported by Claude Code (Sonnet 5)) |

---

## Last Updated

**Date**: 2026-07-29
**Updated By**: Claude Code (Sonnet 5)
**Changes**: Created the EIMP index. Seeded with `EIMP-0` (the process
meta-document) and `EIMP-1` (`EinmoReview`, retroactively ported from
`FOOP-25` in the `foolish-rust` workspace).
