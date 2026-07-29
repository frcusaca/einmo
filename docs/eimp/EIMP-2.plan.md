# EIMP-2.plan — einmo-review-server prototype

Read `docs/eimp/EIMP-2.md` before acting on any task below (and `EIMP-1.md`
for the full design this prototypes a slice of). Tasks run top to bottom.
Work happens directly on `main` — no worktree stage (`EIMP-0` §8).

**Incremental strategy**: each phase below adds exactly one capability to
the new `scripts/einmo_review_client.sh` and proves it end-to-end against
`zweimomo`'s real `suites/javascript/` tree before moving to the next.
`scripts/experimental_reviewer.sh` is NOT modified by this plan — it stays
as reference material and fallback (EIMP-2.md §6). By the end of Phase E
the new script can already list + view bodies; by Phase F it can also flag;
by Phase G it can also kick/retract; by Phase H it can also promote; by
Phase I it can also undo. Each phase's own verification step is a
precondition for starting the next. Feature scope stays fluid during this
prototyping phase (EIMP-2.md §6) — treat this sequence as a starting point,
adjust it if a phase surfaces a better shape for the next.

- [x] STOP — preconditions: `cargo test`, `cargo clippy --all-targets -- -D
      warnings`, `cargo fmt --check` all clean. Do not begin while any is
      broken.
      (2026-07-29 16:34) — fixed pre-existing fmt drift (3 files) and 4
      clippy findings (`is_none_or`, `.ok()`, `RawEvalRow` type alias
      replacing a repeated 4-tuple) before starting.
- [x] Sanity check: consult human on the one remaining Open Question
      (`EIMP-2.md` §Open Questions "Still open") — immediate-execute
      convenience endpoints for flag/retract vs. the two-call
      `PUT`-then-`POST` shape. Everything else was resolved during scoping
      (see §Open Questions "Resolved during scoping").
      (2026-07-29 16:34) — resolved: one convenience endpoint each for
      flag/retract (§Open Questions updated in `EIMP-2.md`).
- [x] Begin work: check `begun: [x]` in `EIMP-2.md` frontmatter, commit
      `EIMP-2.md` stating that work has commenced
      (2026-07-29 16:34)

## Phase A — `EinmoId` (EIMP-2.md §0)

Built first: every later phase (the review object, the server routes, the
script) addresses cases by `EinmoId`.

- [x] Write the unit tests FIRST (`EIMP-2.md` §Test Plan "Unit — `EinmoId`")
      (2026-07-29 16:34)
- [x] Implement `EinmoId` in `src/stage.rs` (alongside the existing
      `mirror_input_path` it formalizes), exported from `lib.rs`
      (2026-07-29 16:34)
- [x] `from_input_rel`, `from_stage_artifact_path`, `to_stage_path`,
      `as_str`/`Display`, `TryFrom<&str>` — validation rejects `..`,
      absolute paths, NUL bytes, empty segments
      (2026-07-29 16:34)
- [x] Phase A tests green; `cargo fmt` / `cargo clippy -D warnings` clean
      (2026-07-29 16:34) — 139 tests pass (133 pre-existing + 6 new)

## Phase B — `zweimomo` (Boa only) ported into this repo (EIMP-2.md §8)

Built next: every later phase needs a real suite to test against.

- [ ] New crate `zweimomo/` at this repo's root: `Cargo.toml` depending on
      `einmo` (path dependency, `.` / workspace-relative) and `boa_engine`
      (pinned to the version used in `foolish-rust`'s `zweimomo`) — no
      `foolish-ubca`/`foolish-core`, no `rustpython-vm` (EIMP-2.md §8,
      Rejected Alternative G)
- [ ] Port `BoaEvaluator` + its unit tests from `foolish-rust`'s
      `zweimomo/src/evaluators.rs` (Boa-only slice)
- [ ] Port the `suites/javascript/` tree (`input/`, `output/`, `checked/`)
      from `foolish-rust`'s `zweimomo` verbatim
- [ ] Write and pass: the ported `BoaEvaluator` unit tests; a test that
      evaluates every ported input via `EinmoSuite` and matches the ported
      `checked/` baselines byte-for-byte (Test Plan "Unit — zweimomo (Boa)
      port")
- [ ] Add `zweimomo` to this repo's workspace, if/when one is introduced —
      for now this repo has no `[workspace]`; confirm at begun-time whether
      one is needed or whether `zweimomo` stays a sibling crate built
      independently
- [ ] Phase B tests green; `cargo fmt` / `cargo clippy -D warnings` clean
      (both crates)

## Phase C — the minimum `EinmoReview` slice (EIMP-2.md §2)

- [ ] Write the unit tests FIRST (`EIMP-2.md` §Test Plan "Unit —
      `EinmoReview` minimum slice") as failing tests against the intended
      `einmo::review` surface, run against `zweimomo`'s ported suite where a
      real suite is useful (not just synthetic fixtures)
- [ ] Implement `review::Decision` (all four variants: `Promote`, `Retract`,
      `Flag`, `Skip`) + `DecisionBook` (keyed by `EinmoId`; single implicit
      reviewer — no per-request `ReviewerId` yet, per §2's note;
      replace-not-stack)
- [ ] Implement `review::VerifiedCache` (fingerprint →
      `Arc<OnceLock<VerifiedBody>>`, single-flight; verify-count test hook,
      reused from `EIMP-1`'s design as-is)
- [ ] Implement `EinmoReview::open`/`items`/`body`/`decide`/`undecide`/
      `plan`/`execute` over the above (§2's minimum surface — no `diff`,
      `execute_one`, or `refresh` in this EIMP), all keyed by `EinmoId`
- [ ] `Signer`/`SignerSet` (`EIMP-1` §S.4, unchanged) — computer key for
      `output to checked`, human passphrase-derived key for
      `checked to verified`
- [ ] `execute()` promote byte-for-byte equivalence test vs. the existing
      CLI `einmo promote`
- [ ] `execute()` retract byte-for-byte equivalence test vs. the existing
      CLI `einmo retract`, including its checked→verified cascade
- [ ] Flag execution matches the script's current `mv` behavior: moves to
      `flagged/`, writes the plaintext advisory line, no signing, no gate
- [ ] All Phase C tests green; `cargo fmt` and `cargo clippy -D warnings`
      clean

## Phase D — the HTTP server: session + read-only endpoints (EIMP-2.md §3, §7)

First server increment: create the one session, list, and inspect — no
mutation yet.

- [ ] Implement session creation: `POST /einmo/sessions` opens an
      `EinmoReview` for the suite given on the server's command line,
      returns the session id; the server calls this against itself once at
      startup (EIMP-2.md §2, §7)
- [ ] Write endpoint tests FIRST for `POST /einmo/sessions`,
      `GET /einmo/<session>/cases`, `GET /einmo/<session>/cases/<id>`, and
      `GET /einmo/<session>/cases/<id>/body/<stage>`, against `zweimomo`'s
      ported suite; unknown session id 404s on every route
- [ ] Implement the four routes above; UDS binding at a configurable path
      (default `./.einmo-review.sock`, §7), directory-permission inheritance
      (mirrors `scripts/experimental_reviewer.sh`'s existing mode-700
      discipline); write `<socket-path>.session` alongside the socket
      containing the session id; both files removed on exit (normal exit and
      `SIGINT`/`SIGTERM`); refuse to start if a stale socket file exists at
      the target path and cannot be connected to
- [ ] `src/bin/einmo_review_server.rs` + `src/bin/cargo_einmo_review_server.rs`
      binaries (`--socket <path>` flag, `<suite>` positional), `Cargo.toml`
      `[[bin]]` entries
- [ ] Smoke test: `cargo einmo-review-server <zweimomo-suite>` starts, binds
      its socket, writes the session file, and
      `curl --unix-socket … GET /einmo/<session>/cases` (session id read
      from the session file) returns the real worklist
- [ ] Phase D tests green; `cargo fmt` / `cargo clippy -D warnings` clean

## Phase E — new script increment 1: list + view (read-only)

Create `scripts/einmo_review_client.sh` — first increment, prove the
HTTP-only shape end to end before adding any mutation.
`scripts/experimental_reviewer.sh` is untouched (EIMP-2.md §6); copy its
vim invocation/pane/statusline setup as the starting point, not its
decision-tracking arrays.

- [ ] New file `scripts/einmo_review_client.sh`; port the vim invocation,
      pane layout, and statusline setup from `experimental_reviewer.sh`
      verbatim (EIMP-2.md §6); startup check: fail fast with a clear
      message if no server socket (and session file) is found for the
      suite — no fallback to `experimental_reviewer.sh` or direct `einmo`
      calls (EIMP-2.md §6); read the session id once and hold it as a
      constant for the run
- [ ] Add `jq` as a new script dependency (EIMP-2.md §6, Rejected
      Alternative H)
- [ ] Implement listing: `curl --unix-socket … GET
      /einmo/<session>/cases`, parsed with `jq` into the one local array
      the script needs — `ids`, a list of `EinmoId`s (EIMP-2.md §5)
- [ ] Implement body viewing: `GET
      /einmo/<session>/cases/<id>/body/<stage>`
- [ ] Verify against `zweimomo`: run `einmo_review_client.sh` (still
      read-only — no decisions made yet) against `einmo-review-server`
      pointed at `zweimomo`'s suite; confirm the worklist and pane bodies
      match what direct `einmo list`/`einmo body` produce
- [ ] Verify the no-server failure path: run the script with no server
      running, confirm it fails fast with the documented message

## Phase F — flag convenience endpoint (EIMP-2.md §3)

Smallest mutating endpoint first: flag has no signing, no gate, and is now
a single atomic call (resolved Open Question).

- [ ] Write endpoint tests FIRST for
      `POST /einmo/<session>/cases/<id>/flag` (`{"reason":string}`,
      records `Decision::Flag` and executes it in one call, no `confirm`
      required)
- [ ] Implement the flag convenience endpoint
- [ ] In `einmo_review_client.sh`: implement flagging as one
      `POST … /flag` call, sent immediately when the reviewer flags a case
      (EIMP-2.md §6) — the equivalent of `experimental_reviewer.sh`'s raw
      `mv … flagged/`, but over HTTP
- [ ] Verify against `zweimomo`, step by step: flag one test via the new
      script; confirm the plaintext advisory note lands in `flagged/`
      exactly as `experimental_reviewer.sh`'s `mv`-based path would
      produce; confirm the flagged test now fails `EinmoSuite` validation
      (`EIMP-1` §S.3)

## Phase G — retract convenience endpoint (EIMP-2.md §3)

- [ ] Write endpoint tests FIRST for
      `POST /einmo/<session>/cases/<id>/retract`
      (`{"from":"checked"|"verified"}`, records `Decision::Retract` and
      executes it in one call, including the checked→verified cascade)
- [ ] Implement the retract convenience endpoint
- [ ] In `einmo_review_client.sh`: wire `\K` (kick) to one `POST … /retract`
      call — this closes the pre-existing gap where
      `experimental_reviewer.sh` accumulated kicks locally but never
      actually executed them (EIMP-2.md §1, §5, §6)
- [ ] Verify against `zweimomo`, step by step: promote a test to `checked`
      by hand (direct `einmo promote`, to have something to retract), then
      kick it via the new script; confirm the checked artifact is removed
      and any verified cascade removal happens correctly

## Phase H — decision + execute endpoints: promote (EIMP-2.md §3, §4)

- [ ] Write endpoint tests FIRST for `PUT … /decision` (`kind: promote`)
      and `POST /execute`'s gated promote-execution path (`confirm:
      "PROMOTE"` required; passphrase handling for `checked to verified`)
- [ ] Implement the promote decision + execution path; passphrase arrives
      only inside the execute body, derived, used under the `exec` mutex,
      dropped, never logged
- [ ] In `einmo_review_client.sh`: implement promotion as per-case
      `PUT … /decision` calls plus one gated `POST /execute` at the end of
      the pass, passphrase read from `/dev/tty` (same UX as
      `experimental_reviewer.sh` today — note the plaintext-transport
      caveat, EIMP-2.md §Open Questions "Still open") (EIMP-2.md §6)
- [ ] Verify against `zweimomo`, step by step: promote output→checked for
      one test via the new script; separately, promote checked→verified
      with a passphrase; confirm both resulting `.einmo` files are
      byte-for-byte what direct `einmo promote` would have produced

## Phase I — undo/revisit (EIMP-2.md §3, §5, §6)

- [ ] Write endpoint tests FIRST for
      `DELETE /einmo/<session>/cases/<id>/decision` (undecide — no-op on an
      already-undecided case, clears an existing one) and for re-`PUT`
      replace semantics
- [ ] Implement `DELETE … /decision`
- [ ] In `einmo_review_client.sh`: implement `u` (revisit) as `GET …
      /cases/<id>` followed by re-`PUT`-or-`DELETE` — there is no local
      array surgery to delete here, because unlike
      `experimental_reviewer.sh`'s `undo_last_decision`/`answer_of`/
      `drop_from`, this script never had that state to begin with
      (EIMP-2.md §5, §6)
- [ ] Verify against `zweimomo`, step by step: decide a test (any kind),
      revisit it and change the decision, confirm only the new decision
      applies at execute time; revisit a test and back out to undecided,
      confirm execute treats it as untouched

## Phase J — summary rendering + cleanup

- [ ] Implement the end-of-pass summary in `einmo_review_client.sh` as a
      rendering of `GET /einmo/<session>/plan`'s response — there is no
      local stats computation to replace, since this script never
      accumulated the counts itself (EIMP-2.md §5)
- [ ] Measure and record here: `einmo_review_client.sh`'s line count vs.
      `experimental_reviewer.sh`'s (~700 lines, per `EIMP-1` §S.8) — the new
      script is expected to be substantially shorter (EIMP-2.md §6)
- [ ] Note in this plan whether the plaintext-passphrase-transport weakness
      (EIMP-2.md §Open Questions "Still open") needs a follow-up EIMP, or
      stays an accepted prototype limitation
- [ ] Phase E–J tests green; `cargo fmt` / `cargo clippy -D warnings` clean

## Comprehensive test + completion

- [ ] Comprehensive test, per `EIMP-2.md` §Test Plan: a full review pass
      exercising the complete decision vocabulary (list, view, approve,
      kick, flag, undo) at least once each over `zweimomo`'s real suite,
      via `einmo_review_client.sh` against a live server, chained end to
      end in one run (not just the per-phase step-by-step checks above)
- [ ] Integration test: `einmo_review_client.sh` run with no server running
      fails fast with the documented message, no silent fallback
- [ ] All tests pass: `cargo test`, `cargo clippy --all-targets -- -D
      warnings`, `cargo fmt --check`
- [ ] Update `EIMP-2.md` frontmatter `status: complete`; remove the
      "Resolved during scoping" record from §Open Questions per `EIMP-0`'s
      Open-Questions-emptied-when-frozen convention
- [ ] Update `docs/eimp/INDEX.md` to add EIMP-2 and reflect its completed
      status
