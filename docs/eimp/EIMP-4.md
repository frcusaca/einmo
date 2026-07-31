---
eimp: 4
title: Split einmo into core + einmo-review-server, publish both to crates.io at 0.0.6
author: Claude Code (Opus 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-30
supersedes: []
begun: [ ]
---

# EIMP-4: Split einmo into core + einmo-review-server, publish both to crates.io at 0.0.6

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

## Abstract

Split this repository's single `einmo` crate into two published crates —
`einmo` (the core testing/signing library and its `einmo` CLI) and
`einmo-review-server` (the `EinmoReview` session object, its HTTP server,
its TUI client, and the dhtml frontend) — and publish both to crates.io at
version `0.0.6`. The split exists because `einmo`'s only real consumer today
(`foolish-ubca`, in the `/yolo/src` workspace) imports exactly five symbols
— `EinmoSuite`, `Evaluator`, `Stage`, `TestConfig`, `ValidationLevel` — yet
publishing the crate as it stands would force `axum`, `tokio`, `hyper`,
`hyper-util`, `tower`, and `hyperlocal` onto every consumer of what is
fundamentally a *testing library*. After this EIMP, `/yolo/src/einmo` (a
stale vendored copy at `0.1.0` that `foolish-ubca` and `/yolo/src/zweimomo`
depend on by path) is deleted and replaced with a normal crates.io
dependency on the published `einmo`.

**This EIMP is executed only after `EIMP-1` reaches `complete`.** The review
loop must be feature-complete and performance-verified by the maintainer
before anything is published — publishing an API and then reshaping it
across `EIMP-1`'s remaining phases would burn version numbers for no gain.
The plan is written now so `EIMP-1`'s remaining work can be steered toward
it (see `EIMP-1.plan.md`'s crate-boundary tasks).

## Motivation

**Today.** `einmo` lives in one crate at `/yolo/einmo`, version `0.0.5`,
unpublished. A *stale copy* of it sits vendored inside the foolish
workspace at `/yolo/src/einmo` claiming version `0.1.0`; both
`/yolo/src/foolish-ubca` and `/yolo/src/zweimomo` depend on it via
`einmo = { path = "../einmo" }`. That vendored copy is strictly behind this
repository (it has no `review.rs`, no `review_server.rs`, no server
binaries), so every improvement made here is invisible to foolish until
someone manually re-vendors. The two copies' version numbers have already
diverged in the *wrong direction* — the stale one claims the higher number.

**Three problems this EIMP solves:**

1. **No published crate.** foolish cannot consume einmo as a normal
   dependency. `cargo publish --dry-run` already passes (verified
   2026-07-30), so the blocker is not mechanical — it is that the crate is
   not yet worth publishing in its current shape (problem 2) and its API is
   still moving (hence the `EIMP-1`-first ordering).
2. **Dependency weight.** `foolish-ubca`'s entire use of einmo is
   `use einmo::{EinmoSuite, Evaluator, Stage, TestConfig, ValidationLevel};`
   (`/yolo/src/foolish-ubca/src/ubca_snapshot_tester.rs:20`). Publishing
   the crate whole would make that five-symbol import drag in a full HTTP
   server stack. A testing library that costs an HTTP stack to depend on is
   a testing library people route around.
3. **Version divergence.** Publishing `0.0.5` would go *backwards* relative
   to the `0.1.0` the foolish workspace already references, producing a
   confusing downgrade at the exact moment the dependency becomes real.

**After this EIMP.** `einmo` publishes with a small dependency tree (no
axum/tokio/hyper/tower/hyperlocal). `einmo-review-server` publishes
alongside it, depending on `einmo` by version, carrying the whole review
experience. `/yolo/src/einmo` is deleted; `foolish-ubca` and
`/yolo/src/zweimomo` depend on `einmo = "0.0.6"` from crates.io.

## Specification

### S.1 The crate boundary

| Module / asset | Crate after the split | Rationale |
|---|---|---|
| `format.rs`, `signature.rs`, `verify.rs`, `stage.rs`, `transitions.rs`, `compare.rs`, `config.rs`, `error.rs`, `einmo_suite.rs` | `einmo` | The testing/signing core — what `foolish-ubca` actually uses. |
| `cli.rs` + `src/main.rs` + `src/bin/cargo_einmo.rs` (the `einmo` / `cargo-einmo` binaries) | `einmo` | Stage transitions, verify, compare, evaluate, `regenerate-output` — none of it needs the review session. |
| `CorpusSigner` (`EIMP-1` §S.11) | `einmo` | Corpus integrity is a core property, not a review-UI concern. Single-threaded by `EIMP-1`'s resolution, so it adds no async runtime to core. |
| `review.rs` (`EinmoReview`, `Decision`, `DecisionBook`, `VerifiedCache`, `SignerSet`, `Journal`) | `einmo-review-server` | Resolved 2026-07-30: the session object ships with the server rather than core, keeping core's surface to the test-runner/signing library. |
| `review_server.rs` + `src/bin/einmo_review_server.rs` + `src/bin/cargo_einmo_review_server.rs` | `einmo-review-server` | The HTTP layer and its binaries. |
| `scripts/einmo_review_client.sh` (the TUI) | `einmo-review-server` | Resolved 2026-07-30: "the TUI is a part of the review server crate." |
| dhtml frontend (`EIMP-1` §S.9) | `einmo-review-server` | Same crate as the server that embeds and serves it. |
| `scripts/experimental_reviewer.sh` | neither (stays repo-local, unpublished) | Reference material / fallback (`EIMP-2` §6); not part of either crate's published surface. |
| `zweimomo/` | neither (`publish = false`, unchanged) | Demo/fixture crate (`EIMP-2` §8). |

**Dependency direction is one-way**: `einmo-review-server` depends on
`einmo`; `einmo` never depends on `einmo-review-server`. Core `einmo`'s
dependency list after the split drops `axum`, `tokio`, `tower`, `hyper`,
`hyper-util`, and `hyperlocal` entirely.

**Consequence for `EIMP-1` Phase B.** `EIMP-1`'s planned
`einmo review plan|list|decide|undecide|execute` one-shot CLI verbs operate
on `EinmoReview`, which now lives in `einmo-review-server`. They therefore
belong to that crate's binary, not to core `einmo`'s `cli.rs`. `EIMP-1`'s
plan is updated accordingly.

### S.2 Version and release identity

Both crates publish at **`0.0.6`** (resolved 2026-07-30). Rationale: it
continues this repository's honest `0.0.x` line rather than claiming a
maturity the API does not yet have, and — unlike `0.0.5` — it is a number
the foolish workspace has never referenced, so there is no downgrade
confusion when `/yolo/src/einmo`'s vendored `0.1.0` is deleted rather than
upgraded. The two crates are versioned in lockstep for this release;
whether they diverge later is out of scope here.

`einmo-review-server` depends on core as
`einmo = { version = "0.0.6", path = ".." }` — the `version` field is what
crates.io publishes, the `path` is what the local workspace builds against.

### S.3 Publish preconditions

Each is a hard gate, checked in the plan before the irreversible upload:

1. `EIMP-1` is `complete` (review loop feature-complete, maintainer
   performance-verified).
2. `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D
   warnings`, `cargo fmt --check` all clean.
3. `cargo publish --dry-run` clean for **both** crates, in dependency order.
4. Core `einmo`'s dependency tree contains no `axum`/`tokio`/`hyper`/
   `tower`/`hyperlocal` (`cargo tree -p einmo` asserted, not eyeballed).
5. Both crates carry complete publish metadata: `description`, `license`,
   `repository`, `readme`, `keywords`, `categories`. Core already has all
   six; `einmo-review-server` needs its own.
6. `README.md` documents library usage as an external dependency — the
   five-symbol `foolish-ubca` shape at minimum — not only CLI usage.
7. The crates.io name `einmo-review-server` is confirmed available at
   publish time. (Core `einmo`'s availability could not be verified during
   drafting: crates.io's API refused the request under its data-access
   policy. Verify both interactively before uploading.)

**Publishing is irreversible** — a published version can be yanked but
never replaced or deleted. The plan therefore places an explicit
maintainer-confirmation step immediately before upload, and uploads core
first (`einmo-review-server` cannot resolve until core is live).

### S.4 Migrating the foolish workspace

After both crates are live, in `/yolo/src` (a *separate repository* — this
step is coordination, not work this repo's plan performs unilaterally):

1. Point `/yolo/src/foolish-ubca/Cargo.toml` and
   `/yolo/src/zweimomo/Cargo.toml` at `einmo = "0.0.6"` instead of
   `{ path = "../einmo" }`.
2. Confirm `foolish-ubca` builds and its snapshot tests pass against the
   published crate — its import surface is only
   `EinmoSuite, Evaluator, Stage, TestConfig, ValidationLevel`, so a
   compile failure here means the published API is genuinely missing
   something, not that the consumer needs rewriting.
3. Delete `/yolo/src/einmo` (the stale `0.1.0` vendored copy).
4. `/yolo/src/zweimomo` (the three-language original) is a separate
   question from this repo's JavaScript-only `zweimomo/` — this EIMP only
   repoints its dependency; it does not merge, delete, or reconcile the two.

## Test Plan

- **Unit/integration — unchanged behavior across the split.** The full
  existing suite must pass after the split with no test *deleted* and no
  assertion weakened: `einmo`'s tests stay with the modules they cover;
  `review.rs`/`review_server.rs`'s tests move to `einmo-review-server`
  wholesale. Total test count must not drop (any drop is a lost test, not
  a simplification).
- **Dependency-tree assertion.** A check (script or test) asserting
  `cargo tree -p einmo` contains none of `axum`, `tokio`, `hyper`,
  `hyper-util`, `tower`, `hyperlocal`. This is the whole point of the
  split, so it is pinned mechanically rather than trusted to review.
- **Both crates' `cargo publish --dry-run`** clean, in dependency order.
- **Consumer smoke test.** A scratch crate outside this workspace
  depending on the *packaged* core (`cargo package` output, not the path)
  that reproduces `foolish-ubca`'s exact five-symbol import and runs a
  minimal suite — proving the published surface is sufficient for the real
  consumer before the irreversible upload rather than after.
- **`einmo-review-server` end-to-end after the split.** `EIMP-2`'s
  pty-driven client/server flow (list → view → decide → execute) must still
  pass from the new crate, proving the split did not sever the TUI from its
  server.
- Comprehensive test: from a clean checkout, build both crates, run the
  dependency-tree assertion, run both dry-runs, and drive the full review
  loop end-to-end against `zweimomo`'s `day.1` fixture — the whole
  pre-publish gate in one runnable sequence.

## Rejected Alternatives

### A. Publish one crate with the server behind a Cargo feature

Keep a single `einmo` crate; put `axum`/`tokio`/`hyper` behind an optional
`review-server` feature, off by default. Rejected: the maintainer chose a
two-crate split. It is also the more honest structure here — the review
server is not a *variant* of the testing library, it is a separate program
that consumes it, with its own binaries, its own shell client, and (per
`EIMP-1` §S.9) its own web frontend. Feature-gating would additionally
create a test matrix (`--no-default-features` vs `--all-features`) that
two crates get for free.

### B. Publish now at 0.0.5, iterate in public

Publish immediately and let `EIMP-1`'s remaining phases land as `0.0.7`,
`0.0.8`, … Rejected: `EIMP-1` still reshapes `EinmoReview`'s public surface
(`diff`, `refresh`, `execute_one`, `ReviewMode`, the journal, claims), so
early publication would burn versions on an API known to be mid-flight, and
would publish the heavyweight dependency tree this EIMP exists to avoid.
The maintainer's explicit sequencing is review-loop-complete first, publish
second.

### C. Do nothing; keep vendoring einmo into the foolish workspace

Continue with `/yolo/src/einmo` as a manually-synced copy. Rejected: it is
already stale and already version-divergent in the wrong direction, and the
divergence is silent — nothing fails when the copy falls behind, so foolish
simply runs old einmo indefinitely. Manual vendoring also blocks the whole
reason for extracting einmo into its own repository.

### D. Name the review crate `einmo-tui`

An earlier draft used `einmo-tui`. Rejected in favor of
`einmo-review-server`: the crate's primary artifact is the *server* (a
resident process with an HTTP API that multiple frontends address); the TUI
is one client of it, alongside the planned dhtml frontend. Naming the crate
after one of its frontends would misdescribe it — and would leave the dhtml
frontend, which is not a TUI at all, homeless.

## Open Questions

- crates.io availability of both names (`einmo`, `einmo-review-server`) is
  unverified — the API refused programmatic access during drafting. Confirm
  interactively before the upload step; if `einmo` is taken, this EIMP
  needs a naming amendment before it can proceed.
- Whether the two crates stay version-locked after `0.0.6` or version
  independently. Deliberately left open: lockstep is right for the first
  release, and the answer for later releases depends on how often core
  changes without the server changing — information this EIMP does not yet
  have.

## References

- `EIMP-1` (`docs/eimp/EIMP-1.md`) — must reach `complete` before this
  EIMP is executed; §S.7 (server), §S.9 (dhtml frontend), §S.11
  (`CorpusSigner`) all sit on the crate boundary this EIMP defines.
- `EIMP-2` (`docs/eimp/EIMP-2.md`) — built `review.rs`/`review_server.rs`/
  `einmo_review_client.sh`, the code that moves to the new crate; §6
  records why `experimental_reviewer.sh` stays untouched.
- `EIMP-5` (`docs/eimp/EIMP-5.md`) — parallelized corpus signing, split out
  of `EIMP-1` §S.11 so core `einmo` stays single-threaded and
  runtime-free for this release.
- Consumers: `/yolo/src/foolish-ubca/src/ubca_snapshot_tester.rs:20` (the
  five-symbol import that defines the required published surface),
  `/yolo/src/zweimomo/Cargo.toml:11`, `/yolo/src/einmo` (the stale vendored
  copy this EIMP deletes).
- Code: root `Cargo.toml` (`[workspace] members`), `src/lib.rs` (the export
  surface that becomes core's published API).
