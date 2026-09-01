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
| [EIMP-3](EIMP-3.md) | Output-stage drift fails the run; explicit regenerate; multi-signer output stamps | superseded by EIMP-01 | 2026-07-30 | Claude Code (Sonnet 5) |
| [EIMP-4](EIMP-4.md) | Split einmo into core + einmo-review-server, publish both to crates.io at 0.0.6 | Draft | 2026-07-30 | Claude Code (Opus 5) |
| [EIMP-5](EIMP-5.md) | Merkle-tree corpus signing — faster to compute, cheaper to update | Draft | 2026-07-30 | Claude Code (Opus 5) |
| [EIMP-6](EIMP-6.md) | Add structured JSONL logging to the test-run path | Brewing | 2026-07-30 | Claude Code (Opus 5) |
| [EIMP-7](EIMP-7.md) | EinmoCase / EinmoSuite / EinmoDirectory — unify case access behind an EinmoStorage trait | complete | 2026-07-31 | Claude Code (Sonnet 5) |
| [EIMP-8](EIMP-8.md) | Code-review findings — einmo library, review server, and zweimomo | Draft | 2026-07-31 | opencode (z-ai/glm-5.2); Claude Code (Opus 5) |
| [EIMP-9](EIMP-9.md) | The test-tooling contract — one reliable way to run einmo's tests and read the results | Implementing (paused) | 2026-08-01 | Claude Code (Opus 5) |
| [EIMP-01](EIMP-01.md) | A separate generation phase writing an uncommitted `generated/` stage, and validation levels that compare only against their predecessor | Implementing | 2026-08-11 | Claude Code (Opus 5) |
| [EIMP-11](EIMP-11.md) | Reliability hardening, transactional persistence, and documentation reset | Implementing | 2026-08-26 | OpenAI Codex (GPT-5) |
| [EIMP-21](EIMP-21.md) | Paired Python and JavaScript Project Euler answer suites | Draft | 2026-08-27 | OpenAI Codex (GPT-5) |

---

## The jia-sprint (current)

**Goal**: a functioning einmo library and review system, ready for foolish to
depend on it as a normal crates.io dependency instead of the stale vendored
copy at `/yolo/src/einmo`. **The sprint is still running** — what follows is a
rescope, not a wind-down.

### Rescoped 2026-08-11 — traded breadth for a fourth stage

`EIMP-01` adds a stage to a model that had three. That is foundational work:
it touches `Stage`, `ValidationLevel`, `TestConfig`, the promotion and
retraction rules, every surface that enumerates stages, and the integration
suite that exercises all of it. Absorbing it inside the sprint means giving
something up rather than letting the sprint grow, so **breadth was traded for
depth**:

- **Out**: the dhtml frontend. Backburnered, not cancelled — the shipped page
  keeps working and is knowingly left stale when the stage model changes.
- **Out for now**: `EIMP-9`, paused mid-flight; `EIMP-5` and `EIMP-8` stay
  where they were.
- **In**: `EIMP-01` end to end — library, tests, CLI promotion, the server,
  the TUI, and the `zweimomo` upgrade that proves it against real evaluators.

The trade is deliberate. A stage model that is right is worth more than a
frontend, and getting it right *before* `EIMP-4` publishes is the whole
reason it is worth doing now rather than after.

The sprint's EIMPs, in execution order:

**Nothing has been published yet.** The sprint targets `0.0.7` directly, and
API breakage before that point costs nothing — there is no consumer of a
published einmo to keep compatible. Sequencing is therefore driven by what
makes the *work* cheaper, not by release compatibility.

### Now

**Sprint scope: library, its tests, CLI promotion, the server, and the TUI
client. No GUI.** The dhtml frontend is **backburnered** as of 2026-08-11
(`EIMP-1.md` §S.9, `EIMP-1.plan.md` §Phase E) — the page that shipped
2026-07-31 keeps working, but no further dhtml work happens this sprint, and
it is knowingly left stale when the stage model changes.

1. **`EIMP-1`** (Implementing) — **the current focus, and effectively done**:
   77 of 84 plan checkboxes complete. Backburnering Phase E cleared its last
   substantive open item, so what remains is the maintainer-deferred
   `\d`/server-diff vim issue, the `status: complete` flip, and three
   post-EIMP follow-ups. Delivered: the `EinmoReview` surface, `ReviewMode`,
   multi-signer promote, flag semantics, the journal, the TUI-owned private
   server, and `CorpusSigner` on the **existing** byte-join construction
   (§S.11) with the new configurable collation (§S.11a).

### Urgently after `EIMP-1`

2. **`EIMP-8`** (Draft) — code-review findings across the einmo library, the
   review server, and `zweimomo`. 41 tracked findings.
3. **`EIMP-9`** (Implementing, **paused**) — the test-tooling contract.
   **Paused as of 2026-08-11, not deprioritized.** Its purpose is
   load-bearing and easy to mistake for hygiene: einmo's mutation gate has
   **never run to completion** (T1, T2), and mutation testing is the only
   mechanism einmo has for catching a test that *cannot fail*
   (`assert test_results() || True`). That failure mode has occurred in this
   repository, was detected once, and detection was then lost. See
   `EIMP-9.md` §Motivation, first subsection. Ten of forty-two plan
   checkboxes are done.

### Then

4. **`EIMP-01`** (Implementing) — a separate `einmo generate` phase writing the
   uncommitted `generated/` stage, `output/` as a committed baseline reached
   by an explicitly weak promotion, and gates that each compare only against
   their immediate predecessor. Supersedes `EIMP-3`. Changes `Stage`,
   `ValidationLevel`, and `TestConfig` — three of the five symbols `EIMP-4`
   publishes — so it lands before the split. Its Phase 6 updates the server
   and TUI for the fourth stage (**not** the dhtml page, which stays
   backburnered); its Phase 7 upgrades **`zweimomo`**, the integration
   testbed where einmo runs end-to-end against real Boa and pyo3 evaluators.
5. **`EIMP-4`** (Draft) — split into `einmo` + `einmo-review-server`,
   publish both at `0.0.7`, repoint `foolish-ubca` and `/yolo/src/zweimomo`
   at the published crate, delete the vendored copy.

### Completed sprint milestones

- **maintainer performance-verifies the review loop** — an explicit STOP
  in `EIMP-1.plan.md`, and `EIMP-4`'s first gate. **Done 2026-07-31**;
  it found thirteen defects (P0–P12), twelve of them fixed and merged to
  `jia`. The thirteenth, **P1**, was architectural and became `EIMP-7`.
- **`EIMP-7`** (complete) — the layered core: `EinmoCase`/`EinmoSuite`/
  `EinmoDirectory` behind an `EinmoStorage` trait, so `einmo test` and
  `einmo review` stopped maintaining parallel scanning, comparison, and
  promotion implementations. Carried `EIMP-1`'s P1 fix plus a second
  inconsistency found while drafting it (`einmo test` and `einmo review`
  answering differently about the same case).

Explicitly **outside** the sprint, each with its own specification so
nothing is dropped — both land after `EIMP-1`:

- **`EIMP-5`** — Merkle-tree corpus signing: faster to compute, cheaper to
  update. `EIMP-1` ships the byte-join construction, which is correct and
  sufficient at current corpus sizes; this EIMP's plan benchmarks *before*
  implementing, with "not worth merging" a legitimate outcome. Also carries
  the collation conformance harness (§S.1a) — stable-sort an alphabet,
  stable-sort its reverse, assert they agree — normative for every present
  and future `Collation`.
- **`EIMP-6`** (Brewing) — add structured JSONL logging to the test-run path.
  **Rescoped 2026-08-11**: the crash crumb is no longer retired and is now an
  explicit non-goal. `EIMP-01` moves crumb creation into the uncommitted
  `generated/` stage, dissolving the output-tree-pollution argument for
  removal; and a journal is weaker crash evidence unless it also flushes
  before every evaluation, at which point deleting a working crumb buys
  nothing. The prior §S.3 crumb-work freeze is lifted.

---

## Last Updated

**Date**: 2026-09-01
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Marked EIMP 11 Implementing after its clean Phase 0 baseline and
focused-test establishment on `jia`.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Added EIMP 21, a finite first range of paired Python and
JavaScript Project Euler solutions whose returned answer strings are written
and checked through einmo.

**Date**: 2026-08-26
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Added draft EIMP 11, which converts the 2026-08-25/26 review
into a priority-ordered hardening plan centered on domain-level atomicity,
transactional persistence, fail-closed behavior, and current-state
documentation replacement/deprecation.

**Date**: 2026-08-13
**Updated By**: Claude Code (Opus 5)
**Changes**: `EIMP-3` marked **superseded by EIMP-01** — it now carries a
forward-pointing note at its head, and its history is deliberately not
rewritten: the decision it recorded was correct and its requirement survives,
only the mechanism moved. `EIMP-01` status `Draft` → `Implementing` in the
table and the sprint list.

**Date**: 2026-08-11 (4)
**Updated By**: Claude Code (Opus 5)
**Changes**: Recorded the sprint's rescope explicitly — **the jia-sprint is
still running**, and adding a fourth stage (`EIMP-01`) is foundational enough
that breadth was traded for depth rather than letting the sprint grow: the
dhtml frontend backburnered, `EIMP-9` paused, `EIMP-01` taken end to end
including the `zweimomo` upgrade. Rationale stated: a stage model that is
right is worth more than a frontend, and it must be right before `EIMP-4`
publishes.

**Date**: 2026-08-11 (3)
**Updated By**: Claude Code (Opus 5)
**Changes**: Recorded the sprint's **scope boundary** — library, tests, CLI
promotion, server, TUI; **no GUI**. The dhtml frontend is backburnered
(`EIMP-1.md` §S.9, `EIMP-1.plan.md` §Phase E); the shipped page keeps working
and is knowingly left stale when the stage model changes. That cleared
`EIMP-1`'s last substantive open checkbox, so it is now noted as effectively
done at 77/84. `EIMP-01`'s entry updated to name its two new phases: the
review surface (server + TUI only) and the `zweimomo` upgrade.

**Date**: 2026-08-11 (2)
**Updated By**: Claude Code (Opus 5)
**Changes**: Re-sequenced §The jia-sprint on maintainer direction. **`EIMP-1`
is the current focus**, with `EIMP-8` and `EIMP-9` urgent follow-ups, then
`EIMP-01`, then `EIMP-4`. Recorded that nothing has been published and the
sprint targets `0.0.7` directly, so sequencing is driven by what makes the
work cheaper rather than by release compatibility (`EIMP-4` updated from
`0.0.6` in the sprint narrative; its own spec still says `0.0.6` and needs
updating when picked up). `EIMP-9` marked **paused, not deprioritized**, with
its load-bearing purpose surfaced into the sprint text: the mutation gate has
never run to completion, and mutation testing is einmo's only detector for a
test that cannot fail — a failure mode that occurred here and whose detection
was lost. `EIMP-7` and the maintainer performance-verification moved into a
"Completed sprint milestones" block (`EIMP-7` was still listed as "Final,
ready to implement" while `complete`). `EIMP-6` rescoped — see below.

**Date**: 2026-08-11 (1)
**Updated By**: Claude Code (Opus 5)
**Changes**: Rescoped `EIMP-6` to **add** structured JSONL logging to the
test-run path only; the crash crumb is no longer retired and is now an
explicit non-goal, with the retirement moved to a Rejected Alternative.
`status` `Draft` → `Brewing`, title updated. Added `EIMP-01` — separates
generation from gating. A new
`einmo generate` phase writes the gitignored `generated/` stage (a full
stage: outputs, signatures, `.einmo` files; it differs only in not being
committed and in existing to be compared against `output/`), so an agent or
human can materialize fresh results for direct inspection without disturbing
committed `output/`. `output/` becomes a committed baseline reached by an
explicitly weak `promote generated to output`, and the three gates stop
escalating — each compares only against its immediate predecessor, verifying
signatures on both sides. Gating stays on `output`/`checked`/`verified`
because `output` depends on generation and fails if it fails. Supersedes
`EIMP-3`, whose drift verdict moves into the `output` gate's comparison and
whose `regenerate-output` verb becomes the promotion. Adapted from the
Foolish project's `FOOP-06`.

**Date**: 2026-08-01
**Updated By**: Claude Code (Opus 5)
**Changes**: `EIMP-8` P0 closed. Root cause of the disagreement pinned: an
isolated probe showed neither `overly_complex_bool_expr` nor
`nonminimal_bool` is deny-by-default on `clippy 0.1.97`, so the repo's
mandated gate could never have caught `verify.rs:451` — the finding's
judgment was right, its mechanism wrong. The maintainer added
`[workspace.lints.clippy]` denying both; both members were wired to it with
a top-level `[lints]` / `workspace = true` (the workspace table is inert
without a per-member opt-in, and `lints.workspace = true` written inside
`[package]` is silently dropped). Clippy then failed at `verify.rs:451` as
predicted. `flags_fail_the_gate` is now `pub(crate)` and the test drives it
at all four operand corners instead of hand-inlining a copy with a
hardcoded `!true`. Gates green: 394 tests, clippy and fmt clean. `EIMP-8`
remains `Draft`; P26/P27/P28 and the rest are untouched.

**Date**: 2026-07-31 (5)
**Updated By**: Claude Code (Opus 5)
**Changes**: `EIMP-8` verified and extended. Ran the toolchain gates and
three targeted probes, added a **Triage** verdict to each of P0–P25, and
added sixteen findings (P26–P41). **P0's blocker classification is
withdrawn** — `cargo clippy --workspace --all-targets -- -D warnings` is
clean (verified after a forced recheck), so nothing was ever blocked and
the plan's Phase 0 is void. P1, P16 and P23 rejected as factually false;
P3, P7, P8 and P14 re-characterized; P9 upgraded. Three new High items:
**P26** stored XSS in `src/dhtml/review.html:174` via a reviewer-supplied
flag reason (reached through the exact data flow P14 examined and cleared);
**P27** `execute` applies decisions cleared between `plan()` and
`execute()`, contradicting its own comment; **P28** `POST /einmo/sessions`
opens a session over any filesystem path. Plan rewritten in triaged
priority order (nine phases, no Phase 0 gate). `status: Draft`,
`begun: [ ]` — awaiting ordinary maintainer triage.

**Date**: 2026-07-31 (4)
**Updated By**: opencode (z-ai/glm-5.2)
**Changes**: Added `EIMP-8` — a read-only code review of the einmo
library (`src/review.rs`), the review server (`src/review_server.rs` +
binary), and `zweimomo`, cataloguing twenty-five findings (P0–P25) with
locations, severities, and recommended remediations. `status: Draft`,
`begun: [ ]` — awaiting maintainer triage. P0 (a `verify.rs:451` dead-code
bug breaking `cargo clippy --all-targets -- -D warnings`) is called out as
the toolchain-gate blocker that must be fixed before any substantive work
per AGENTS.md.

**Date**: 2026-07-31 (3)
**Updated By**: Claude Code (Sonnet 5)
**Changes**: `EIMP-7` reached `complete` — all phases (0, A, A2, B, C, D,
E, F, G) plus the comprehensive test implemented and verified. `einmo
test` and `einmo review` now share one `EinmoCase`/`EinmoSuite`/
`EinmoDirectory` core behind an `EinmoStorage` trait; `CorpusSigner` can
be driven by an `EinmoSuite`'s own scan instead of walking a stage
directory a fourth, independent time (§S.8). The comprehensive test
(`review.rs`) proves `EinmoCase::agreement`, `EinmoTestRunner`, and
`EinmoReview` all agree on a COMMENTS-only-differing case and a tampered
artifact over the same on-disk corpus. `cargo test --workspace`: 356
passed, 0 failed. `EIMP-1.plan.md`'s **P1** finding (the whole reason
`EIMP-7` exists) marked resolved by it.

**Date**: 2026-07-31 (2)
**Updated By**: Claude Code (Sonnet 5)
**Changes**: Added `EIMP-7` — the layered core (`EinmoCase`/`EinmoSuite`/
`EinmoDirectory` behind an `EinmoStorage` trait), spun out of `EIMP-1`'s
**P1** maintainer finding per that finding's own recommendation, given a
blast radius across `einmo_suite.rs`, `review.rs`, `transitions.rs`,
`compare.rs`, `corpus_signer.rs`, and `cli.rs`. Drafting it surfaced a
second, previously unrecorded defect of the same family: `compare.rs`
holds a **third** stage-comparison implementation — section-aware and
`MatchSections`-policy-driven — that `einmo test` uses while `einmo
list`/`einmo review` use `scan_tests`'s single all-sections bool, so a
case differing only in `COMMENTS` reads clean to one and differing to the
other. `EIMP-7` makes `compare.rs`'s richer comparison the shared core
(raising review's fidelity to test's, not the reverse) and keeps the
`input/`/`output/`/`checked/`/`flagged/`/`verified/` directory split
explicitly unchanged — hand-authored suites are browsed and edited in
place, which `EinmoDirectory` is specified to preserve. Also inserted
`EIMP-7` into the jia-sprint sequence ahead of `EIMP-4`, and recorded in
`EIMP-5` §Open Questions that `EinmoSuite::directory_tree()` makes
directory-level Merkle hashing a supported option rather than a redesign.

**Date**: 2026-07-30 (5)
**Updated By**: Claude Code (Opus 5)
**Changes**: Corpus signing re-scoped. `EIMP-1` keeps the **existing**
byte-join construction (concatenate in manifest order, hash) — no
restructuring inside the sprint — but gains §S.11a, a **configurable
`Collation`** defaulting to `PathBytes` (component-wise, byte-wise within a
component, no locale, no normalization, no case folding, ties a hard
error). Because ordering determines the digest, the chosen collation's
identifier is recorded in `.section.sig`, so a verifier never mistakes a
configuration difference for tampering. The former `EIMP-5` (parallel
machinery) and `EIMP-6` (Merkle restructuring) are **merged into one
`EIMP-5`** — making hashing faster *and* cheaper to update is the whole
point of the restructuring, so splitting them would have left one EIMP
breaking the digest format for no measurable benefit. The logging EIMP
renumbered `EIMP-7` → `EIMP-6`.

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
