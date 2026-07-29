# EIMP-2.plan — einmo-review-server prototype

Read `docs/eimp/EIMP-2.md` before acting on any task below (and `EIMP-1.md`
for the full design this prototypes a slice of). Tasks run top to bottom.
Work happens directly on `main` — no worktree stage (`EIMP-0` §8).

**Incremental strategy**: each phase below adds exactly one capability to
`experimental_reviewer.sh` and proves it end-to-end against `zweimomo`'s real
`suites/javascript/` tree before moving to the next. Nothing is "integrate
everything at the end" — by the end of Phase D the script can already list
+ view bodies; by the end of Phase E it can also flag; by Phase F it can also
kick/retract; by Phase G it can also promote; by Phase H it can also undo.
Each phase's own verification step is a precondition for starting the next.

- [ ] STOP — preconditions: `cargo test`, `cargo clippy --all-targets -- -D
      warnings`, `cargo fmt --check` all clean. Do not begin while any is
      broken.
- [ ] Sanity check: consult human to resolve `EIMP-2.md` §Open Questions
      (state-loss-on-restart acceptability, script JSON parsing approach,
      immediate-execute-for-flags/retracts endpoint shape, socket discovery
      path) enough to start Phase A. Remind them: "Above message comes from
      EIMP-2 working to build the einmo-review-server prototype; changes
      are on `main`. PTAL"
- [ ] Begin work: check `begun: [x]` in `EIMP-2.md` frontmatter, commit
      `EIMP-2.md` stating that work has commenced

## Phase A — `zweimomo` (Boa only) ported into this repo (EIMP-2.md §8)

Built first: every later phase needs a real suite to test against.

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
- [ ] Phase A tests green; `cargo fmt` / `cargo clippy -D warnings` clean
      (both crates)

## Phase B — the minimum `EinmoReview` slice (EIMP-2.md §2)

- [ ] Write the unit tests FIRST (`EIMP-2.md` §Test Plan "Unit —
      `EinmoReview` minimum slice") as failing tests against the intended
      `einmo::review` surface, run against `zweimomo`'s ported suite where a
      real suite is useful (not just synthetic fixtures)
- [ ] Implement `review::Decision` (all four variants: `Promote`, `Retract`,
      `Flag`, `Skip`) + `DecisionBook` (single implicit reviewer — no
      `ReviewerId` yet, per §2's note; replace-not-stack)
- [ ] Implement `review::VerifiedCache` (fingerprint →
      `Arc<OnceLock<VerifiedBody>>`, single-flight; verify-count test hook,
      reused from `EIMP-1`'s design as-is)
- [ ] Implement `EinmoReview::open`/`items`/`body`/`decide`/`undecide`/
      `plan`/`execute` over the above (§2's minimum surface — no `diff`,
      `execute_one`, or `refresh` in this EIMP)
- [ ] `Signer`/`SignerSet` (`EIMP-1` §S.4, unchanged) — computer key for
      `output to checked`, human passphrase-derived key for
      `checked to verified`
- [ ] `execute()` promote byte-for-byte equivalence test vs. the existing
      CLI `einmo promote`
- [ ] `execute()` retract byte-for-byte equivalence test vs. the existing
      CLI `einmo retract`, including its checked→verified cascade
- [ ] Flag execution matches the script's current `mv` behavior: moves to
      `flagged/`, writes the plaintext advisory line, no signing, no gate
- [ ] All Phase B tests green; `cargo fmt` and `cargo clippy -D warnings`
      clean

## Phase C — the HTTP server: read-only endpoints (EIMP-2.md §3)

First server increment: just enough to list and inspect — no mutation yet.

- [ ] Resolve Open Question: UDS socket location for the suite
- [ ] Write endpoint tests FIRST for `GET /api/review/items` and
      `GET /api/review/items/{m}/body/{stage}` and
      `GET /api/review/items/{m}`, against `zweimomo`'s ported suite
- [ ] Implement the three read-only routes; UDS binding, directory-
      permission inheritance (mirrors `scripts/experimental_reviewer.sh`'s
      existing mode-700 discipline)
- [ ] `src/bin/einmo_review_server.rs` + `src/bin/cargo_einmo_review_server.rs`
      binaries (EIMP-2.md §7), `Cargo.toml` `[[bin]]` entries
- [ ] Smoke test: `cargo einmo-review-server <zweimomo-suite>` starts,
      binds its socket, and `curl --unix-socket … GET /api/review/items`
      returns the real worklist
- [ ] Phase C tests green; `cargo fmt` / `cargo clippy -D warnings` clean

## Phase D — script increment 1: list + view (read-only)

First script change — prove the HTTP-only shape end to end before adding
any mutation.

- [ ] Startup check in `experimental_reviewer.sh`: fail fast with a clear
      message if no server socket is found for the suite — no silent
      direct-`einmo` fallback (EIMP-2.md §6)
- [ ] Replace `"$EINMO" list …` with `curl --unix-socket … GET
      /api/review/items`; keep the resulting `rows` array as the one local
      array the script still needs (EIMP-2.md §5)
- [ ] Replace `"$EINMO" body "$f"` calls with `GET
      /api/review/items/{m}/body/{stage}`
- [ ] Verify against `zweimomo`: run the rewired script (still read-only —
      no decisions made yet) against `einmo-review-server` pointed at
      `zweimomo`'s suite; confirm the worklist and pane bodies match what
      direct `einmo list`/`einmo body` produce
- [ ] Verify the no-server failure path: run the script with no server
      running, confirm it fails fast with the documented message

## Phase E — decision + execute endpoints: flag (EIMP-2.md §3)

Smallest mutating endpoint first: flag has no signing, no gate.

- [ ] Write endpoint tests FIRST for `PUT /api/review/items/{m}/decision`
      (`kind: flag`) and `POST /api/review/execute` (flag path, no
      `confirm` required)
- [ ] Implement `PUT … /decision` and `POST /execute`'s flag-execution path
- [ ] In `experimental_reviewer.sh`: replace the raw `mv … flagged/` with
      the `PUT`/`POST` sequence, sent immediately when the reviewer flags a
      test (EIMP-2.md §6)
- [ ] Verify against `zweimomo`, step by step: flag one test via the
      rewired script; confirm the plaintext advisory note lands in
      `flagged/` exactly as the old `mv`-based path produced; confirm the
      flagged test now fails `EinmoSuite` validation (`EIMP-1` §S.3)

## Phase F — decision + execute endpoints: retract/kick (EIMP-2.md §3)

- [ ] Write endpoint tests FIRST for `PUT … /decision` (`kind: retract`)
      and `POST /execute`'s retract-execution path (including the
      checked→verified cascade)
- [ ] Implement the retract decision + execution path
- [ ] In `experimental_reviewer.sh`: wire `\K` (kick) to the `PUT`/`POST`
      sequence — this closes the pre-existing gap where kicks were
      accumulated locally but never actually executed (EIMP-2.md §1, §5,
      §6)
- [ ] Verify against `zweimomo`, step by step: promote a test to `checked`
      by hand (direct `einmo promote`, to have something to retract), then
      kick it via the rewired script; confirm the checked artifact is
      removed and any verified cascade removal happens correctly

## Phase G — decision + execute endpoints: promote (EIMP-2.md §3, §4)

- [ ] Write endpoint tests FIRST for `PUT … /decision` (`kind: promote`)
      and `POST /execute`'s gated promote-execution path (`confirm:
      "PROMOTE"` required; passphrase handling for `checked to verified`)
- [ ] Implement the promote decision + execution path; passphrase arrives
      only inside the execute body, derived, used under the `exec` mutex,
      dropped, never logged
- [ ] In `experimental_reviewer.sh`: replace `promote_checked`/
      `promote_verified` accumulation with per-test `PUT … /decision` calls
      plus one gated `POST /execute` at the end of the pass, passphrase
      read from `/dev/tty` as today (EIMP-2.md §6)
- [ ] Verify against `zweimomo`, step by step: promote output→checked for
      one test via the rewired script; separately, promote checked→verified
      with a passphrase; confirm both resulting `.einmo` files are
      byte-for-byte what direct `einmo promote` would have produced

## Phase H — undo/revisit (EIMP-2.md §3, §5, §6)

- [ ] Resolve Open Question: does `u` map to a re-`PUT` (replace), a
      `DELETE` (undecide), or offer the reviewer a choice?
- [ ] Write endpoint tests FIRST for `DELETE /api/review/items/{m}/decision`
      (undecide — no-op on an already-undecided item, clears an existing
      one) and for re-`PUT` replace semantics
- [ ] Implement `DELETE … /decision`
- [ ] In `experimental_reviewer.sh`: delete the superseded state machinery
      (`undo_last_decision`, `answer_of`, `drop_from`, and the
      `promote_checked`/`promote_verified`/`retract_checked`/
      `retract_verified`/`flag_stage`/`flag_rel`/`flag_reason`/
      `send_to_agent_list`/`skip_list`/`noop_list` arrays); wire `u` to
      `GET … /items/{m}` + re-`PUT`-or-`DELETE` (EIMP-2.md §5, §6)
- [ ] Verify against `zweimomo`, step by step: decide a test (any kind),
      revisit it and change the decision, confirm only the new decision
      applies at execute time; revisit a test and back out to undecided,
      confirm execute treats it as untouched

## Phase I — summary rendering + cleanup

- [ ] Replace the end-of-pass local stats computation with a rendering of
      `GET /api/review/plan`'s response (EIMP-2.md §5)
- [ ] Resolve Open Question: script JSON parsing (`jq` dependency vs.
      script-friendly plain-text response mode) — implement whichever is
      chosen, across all endpoints the script calls
- [ ] Measure and record here: script line count before vs. after (compare
      against the ~700-line baseline noted in `EIMP-1` §S.8)
- [ ] Phase D–I tests green; `cargo fmt` / `cargo clippy -D warnings` clean

## Comprehensive test + completion

- [ ] Comprehensive test, per `EIMP-2.md` §Test Plan: a full review pass
      exercising the complete decision vocabulary (list, view, approve,
      kick, flag, undo) at least once each over `zweimomo`'s real suite,
      via the rewired script against a live server, chained end to end in
      one run (not just the per-phase step-by-step checks above)
- [ ] Integration test: script run with no server running fails fast with
      the documented message, no silent fallback
- [ ] All tests pass: `cargo test`, `cargo clippy --all-targets -- -D
      warnings`, `cargo fmt --check`
- [ ] Update `EIMP-2.md` frontmatter `status: complete`
- [ ] Update `docs/eimp/INDEX.md` to add EIMP-2 and reflect its completed
      status
