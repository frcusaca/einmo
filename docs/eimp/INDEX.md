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
||||||| 9c8589a
| [EIMP-31](EIMP-31.md) | Extra-repository verification and assurance | Draft | 2026-09-04 | OpenAI Codex (GPT-5) |

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

