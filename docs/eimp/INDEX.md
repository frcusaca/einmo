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
| [EIMP-2](EIMP-2.md) | einmo-review-server — a minimal HTTP prototype of the review/sign/promote/flag loop | complete | 2026-07-29 | Claude Code (Sonnet 5) |

---

## Last Updated

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
