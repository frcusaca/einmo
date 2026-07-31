# EIMP-4.plan — split into einmo + einmo-review-server, publish both at 0.0.6

Read `docs/eimp/EIMP-4.md` before acting on any task below. Tasks run top to
bottom. Work happens directly on `main` (`EIMP-0` §8).

**This plan does not begin until `EIMP-1` is `complete`.** The first STOP
below enforces that. Publishing is irreversible (a version can be yanked,
never replaced), so every gate here is a hard gate, not a formality.

- [ ] STOP — `EIMP-1` is `complete` (`grep '^status:' docs/eimp/EIMP-1.md`),
      and the maintainer has performance-verified the review loop. Do not
      begin otherwise; this is the whole reason the plan was written ahead
      of its execution.
- [ ] STOP — preconditions: `cargo test --workspace`, `cargo clippy
      --workspace --all-targets -- -D warnings`, `cargo fmt --check` all
      clean.
- [ ] Begin work: check `begun: [x]` in `EIMP-4.md` frontmatter, commit
      `EIMP-4.md` + `EIMP-4.plan.md` stating that work has commenced

## Phase A — carve out the `einmo-review-server` crate (EIMP-4.md §S.1)

Mechanical move first, behavior change never. Nothing in this phase may
alter what any existing test asserts.

- [ ] New crate directory `einmo-review-server/` with its own `Cargo.toml`:
      `einmo = { version = "0.0.6", path = ".." }` plus the review stack's
      own deps (`axum`, `tokio`, `tower`, `hyper`, `hyper-util`,
      `hyperlocal`, `serde`, `serde_json`, `thiserror`); full publish
      metadata (`description`, `license`, `repository`, `readme`,
      `keywords`, `categories`) per `EIMP-4.md` §S.3 item 5
- [ ] Add `einmo-review-server` to the root `[workspace] members`
- [ ] Move `src/review.rs` → `einmo-review-server/src/review.rs` and
      `src/review_server.rs` → `einmo-review-server/src/review_server.rs`,
      with their tests, verbatim; move `src/bin/einmo_review_server.rs` and
      `src/bin/cargo_einmo_review_server.rs` to the new crate's `src/bin/`
- [ ] Move `scripts/einmo_review_client.sh` into the new crate
      (`einmo-review-server/scripts/`), updating any path assumptions it
      makes about its own location; `scripts/experimental_reviewer.sh`
      stays where it is, unmoved and unmodified (`EIMP-4.md` §S.1,
      `EIMP-2.md` §6)
- [ ] Resolve the visibility fallout: everything `review.rs`/
      `review_server.rs` reached as `pub(crate)` inside the old single
      crate must now be genuinely `pub` on core `einmo` (or the code that
      needs it moves). Enumerate each such item as a sub-task here as it is
      found — each one is a deliberate decision to widen core's published
      API surface, not a mechanical fix
  - [ ] (sub-tasks recorded here during execution)
- [ ] Strip `axum`, `tokio`, `tower`, `hyper`, `hyper-util`, `hyperlocal`
      from the root `Cargo.toml`'s `[dependencies]`
- [ ] Phase A green: `cargo test --workspace` passes with **no test deleted
      and no test count drop** (`EIMP-4.md` §Test Plan); `cargo fmt` /
      `cargo clippy --workspace --all-targets -- -D warnings` clean

## Phase B — prove the boundary holds (EIMP-4.md §Test Plan)

- [ ] Write the dependency-tree assertion FIRST: a test (or a checked-in
      script the test invokes) asserting `cargo tree -p einmo` contains
      none of `axum`, `tokio`, `hyper`, `hyper-util`, `tower`,
      `hyperlocal`. This is the split's entire purpose — pin it
      mechanically so a future dependency addition cannot silently undo it
- [ ] Consumer smoke test: a scratch crate *outside* this workspace
      depending on the **packaged** core (`cargo package`'s output, not a
      path dependency) that reproduces `foolish-ubca`'s exact import —
      `use einmo::{EinmoSuite, Evaluator, Stage, TestConfig,
      ValidationLevel};` — and runs a minimal suite. A compile failure here
      means the published surface is genuinely insufficient for the real
      consumer, and must be fixed before upload, not after
- [ ] `einmo-review-server` end-to-end after the split: re-run `EIMP-2`'s
      pty-driven client/server flow (list → view → decide → execute)
      against `zweimomo`'s `day.1`, from the new crate, proving the split
      did not sever the TUI from its server
- [ ] Phase B green; `cargo fmt` / `cargo clippy -D warnings` clean

## Phase C — publish metadata and documentation (EIMP-4.md §S.2, §S.3)

- [ ] Set both crates' `version = "0.0.6"`
- [ ] `README.md`: add a library-consumer section documenting einmo as an
      external dependency (`[dependencies] einmo = "0.0.6"` plus the
      five-symbol usage shape `foolish-ubca` actually needs). Today's
      README is CLI-centric — a crates.io reader arriving at the front page
      must be able to see how to *depend on* it, not only how to run it
- [ ] `einmo-review-server/README.md` (its own crates.io front page):
      what it is, that it depends on `einmo`, how to start the server and
      drive the client
- [ ] Verify both crates' publish metadata is complete per `EIMP-4.md`
      §S.3 item 5

## Phase D — the irreversible part (EIMP-4.md §S.3)

- [ ] Confirm the crates.io names `einmo` and `einmo-review-server` are
      both available/owned (interactively — the API refused programmatic
      access during drafting, `EIMP-4.md` §Open Questions). If `einmo` is
      taken, STOP: this EIMP needs a naming amendment before proceeding
- [ ] `cargo publish --dry-run -p einmo` clean
- [ ] `cargo publish --dry-run -p einmo-review-server` clean
- [ ] Assert the §S.3 gate list one final time as a checklist: EIMP-1
      complete, all tests/clippy/fmt clean, both dry-runs clean, core's
      dep tree free of the HTTP stack, metadata complete, README documents
      library usage
- [ ] STOP — maintainer confirmation to upload. A published version can be
      yanked but never replaced or deleted; this is the point of no return
- [ ] `cargo publish -p einmo` (core first — the server cannot resolve
      until core is live on crates.io)
- [ ] Wait for core to be resolvable from crates.io, then
      `cargo publish -p einmo-review-server`
- [ ] Verify both crates install from crates.io into a clean scratch
      directory (`cargo add einmo` in a fresh crate; `cargo install
      einmo` for the binary)

## Phase E — migrate the foolish workspace (EIMP-4.md §S.4)

Coordination across repositories: `/yolo/src` is a **separate repository**
from this one. These tasks are performed there, and are listed here so the
migration is tracked to completion rather than assumed.

- [ ] Point `/yolo/src/foolish-ubca/Cargo.toml` at `einmo = "0.0.6"`
      instead of `{ path = "../einmo" }`
- [ ] Point `/yolo/src/zweimomo/Cargo.toml` at `einmo = "0.0.6"` likewise
- [ ] Confirm `foolish-ubca` builds and its snapshot tests pass against the
      published crate
- [ ] Delete `/yolo/src/einmo` (the stale vendored `0.1.0` copy)
- [ ] Record here whether `/yolo/src/zweimomo` (the three-language
      original) and this repo's JavaScript-only `zweimomo/` should later be
      reconciled — noting the question, not answering it (out of scope,
      `EIMP-4.md` §S.4 item 4)

## Comprehensive test + completion

- [ ] Comprehensive test, per `EIMP-4.md` §Test Plan: from a clean
      checkout — build both crates, run the dependency-tree assertion, run
      both `--dry-run`s, and drive the full review loop end-to-end against
      `zweimomo`'s `day.1` fixture. The entire pre-publish gate as one
      runnable sequence, so it can be re-run before any future release
      rather than reconstructed from this plan
- [ ] All tests pass: `cargo test --workspace`, `cargo clippy --workspace
      --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] Update `EIMP-4.md` frontmatter to `status: complete`
- [ ] Update `docs/eimp/INDEX.md` to reflect EIMP-4's completed status
