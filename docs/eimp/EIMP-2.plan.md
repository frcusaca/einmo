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

- [x] New crate `zweimomo/` at this repo's root: `Cargo.toml` depending on
      `einmo` (path dependency, `.` / workspace-relative) and `boa_engine`
      (pinned to the version used in `foolish-rust`'s `zweimomo`) — no
      `foolish-ubca`/`foolish-core`, no `rustpython-vm` (EIMP-2.md §8,
      Rejected Alternative G)
      (2026-07-29 16:59)
- [x] Port `BoaEvaluator` + its unit tests from `foolish-rust`'s
      `zweimomo/src/evaluators.rs` (Boa-only slice)
      (2026-07-29 16:59)
- [x] Port the `suites/javascript/` tree (`input/`, `output/`, `checked/`)
      from `foolish-rust`'s `zweimomo`
      (2026-07-29 16:59) — organized into progressive-difficulty tiers
      rather than copied flat: `suites/javascript/day.1/` (the 8 ported
      inputs), plus scaffolded `week.2/`/`month.2/`/`years.later/` tiers
      with design-notes READMEs and no content yet (elaboration beyond the
      original plan — see EIMP-2.md §8, updated).
- [x] Write and pass: the ported `BoaEvaluator` unit tests; a test that
      evaluates every ported input via `EinmoSuite` and matches the ported
      `checked/` baselines byte-for-byte (Test Plan "Unit — zweimomo (Boa)
      port")
      (2026-07-29 16:59) — `javascript_tiers_generate_and_verify` iterates
      all four tier directories, skipping any without an `input/` dir yet
      (only `day.1/` currently populated); also ported
      `crash_crumb_survives_stack_overflow` (renamed from the original's
      `_foolish_` name, since it's evaluator-agnostic).
- [x] Add `zweimomo` to this repo's workspace
      (2026-07-29 16:59) — resolved: yes, root `Cargo.toml` gained
      `[workspace] members = [".", "zweimomo"]`. Only `einmo` (root) is
      published; `zweimomo` is `publish = false`.
- [x] Phase B tests green; `cargo fmt` / `cargo clippy -D warnings` clean
      (both crates)
      (2026-07-29 16:59) — 145 tests total across the workspace (139 einmo
      + 6 zweimomo), clippy/fmt clean

## Phase C — the minimum `EinmoReview` slice (EIMP-2.md §2)

- [x] Write the unit tests FIRST (`EIMP-2.md` §Test Plan "Unit —
      `EinmoReview` minimum slice") as failing tests against the intended
      `einmo::review` surface, run against `zweimomo`'s ported suite where a
      real suite is useful (not just synthetic fixtures)
      (2026-07-29 17:38) — 9 tests written in `src/review.rs`; used
      lightweight `Echo`-evaluator tempdir suites rather than the ported
      `zweimomo` fixtures directly (unit-level, not integration-level, per
      the module's own scope) — `zweimomo`'s real suite is exercised at the
      integration-test phase (E onward) instead.
- [x] Implement `review::Decision` (all four variants: `Promote`, `Retract`,
      `Flag`, `Skip`) + `DecisionBook` (keyed by `EinmoId`; single implicit
      reviewer — no per-request `ReviewerId` yet, per §2's note;
      replace-not-stack)
      (2026-07-29 17:38)
- [x] Implement `review::VerifiedCache` (fingerprint →
      `Arc<OnceLock<...>>`, single-flight; verify-count test hook)
      (2026-07-29 17:38) — caches `Result<VerifiedBody, String>` per slot
      rather than `Arc<OnceLock<VerifiedBody>>` directly (`EinmoError` isn't
      `Clone`, so the original EIMP-1 sketch's exact type doesn't compile);
      a verification failure is memoized too (re-verifying a still-tampered
      file is wasted work; a changed fingerprint mints a fresh slot).
- [x] Implement `EinmoReview::open`/`items`/`body`/`decide`/`undecide`/
      `plan`/`execute` over the above (§2's minimum surface — no `diff`,
      `execute_one`, or `refresh` in this EIMP), all keyed by `EinmoId`
      (2026-07-29 17:38) — also moved `scan_tests`/`body_sections`/`TestRow`
      from `cli.rs` to `einmo_suite.rs` (`pub(crate)`) as a prep refactor so
      `items()` reuses the CLI's existing suite-scan logic rather than
      duplicating it.
- [x] `Signer`/`SignerSet` (`EIMP-1` §S.4, unchanged) — computer key for
      `output to checked`, human passphrase-derived key for
      `checked to verified`
      (2026-07-29 17:38) — `SignerSet` wraps the crate's existing
      `KeySource` (already the "resolved passphrase" type `promote`/
      `retract` take) rather than inventing a new key type; `to_verified`
      is `Option<KeySource>` so a promotion needing it without one supplied
      errors (`EinmoError::NoKey`), never silently falls back to the
      computer key.
- [x] `execute()` promote byte-for-byte equivalence test vs. the existing
      CLI `einmo promote`
      (2026-07-29 17:38) — compares section bodies (not raw bytes, since
      each independent run's stamp carries its own generation timestamp)
      between an `EinmoReview::execute` promotion and a direct
      `transitions::promote` call on identically-seeded content; both must
      produce the same INPUT/OUTPUT/COMMENTS bodies.
- [x] `execute()` retract byte-for-byte equivalence test vs. the existing
      CLI `einmo retract`, including its checked→verified cascade
      (2026-07-29 17:38) — `execute_retract_matches_cli_retract_and_cascades`
- [x] Flag execution matches the script's current `mv` behavior: moves to
      `flagged/`, writes the plaintext advisory line, no signing, no gate
      (2026-07-29 17:38) — `execute_flag_moves_and_writes_advisory_no_signing`
- [x] Key hygiene: `execute()` groups pending promotions by `(from, to)`
      stage pair and issues one `transitions::promote` call per group
      (2026-07-29 17:44) — preserves `transitions::promote`'s existing KEK
      discipline (`StageKeypair::derive` once per call, `with_signing_key`
      unwraps/signs/zeroizes once per individual file inside that call);
      `execute()` itself never derives or holds plaintext key material,
      only forwards `SignerSet`'s `KeySource`s into `transitions::promote`.
      Added `execute_derives_stage_key_once_per_batch_not_per_case`
      (timing-bounded: 5 cases in one batch must complete well under
      5×1.8s, the Argon2id cost of deriving per case). Required deriving
      `Hash` on `Stage` (additive, no behavior change) to key a
      `HashMap<(Stage, Stage), _>`.
- [x] All Phase C tests green; `cargo fmt` and `cargo clippy -D warnings`
      clean
      (2026-07-29 17:44) — 149 einmo tests (139 pre-Phase-C + 10 new in
      `review.rs`) + 6 zweimomo tests, clippy/fmt clean

## Phase D — the HTTP server: session + read-only endpoints (EIMP-2.md §3, §3a, §7)

First server increment: create the one session, list, and inspect — no
mutation yet.

- [x] Add `axum`, `tokio`, `tower`, `hyperlocal` dependencies (EIMP-2.md §7);
      confirm exact version pins at implementation time
      (2026-07-29 18:26) — also `hyper` (explicit, `http1`+`server`
      features) and `hyper-util` (`tokio`+`service` features): axum 0.7's
      `serve()` is TCP-only, so the UDS accept loop is hand-rolled with
      hyper's HTTP/1.1 `Builder` + hyper-util's `TokioIo`/
      `TowerToHyperService` glue over `tokio::net::UnixListener`, rather
      than the originally-planned `hyperlocal` `UnixListenerExt::serve`
      (that API is hyper-1.x-native-service-shaped, not a drop-in for an
      axum `Router`). `hyperlocal` is still used client-side, by the test
      suite's end-to-end UDS test. Resolved versions: axum 0.7.9, hyper
      1.11.0, hyper-util 0.1.20, hyperlocal 0.9.1, tower 0.5.3.
- [x] Define `ApiError` (`thiserror`-derived, `IntoResponse` impl mapping
      each variant to its HTTP status per EIMP-2.md §3a's table); a
      `SessionId` newtype
      (2026-07-29 18:26) — the `Decision` serde-tagged enum for `PUT …
      /decision` bodies is deferred to Phase F (the first phase that
      actually needs a mutating decision body); this phase is read-only.
- [x] Implement `Path<EinmoId>` support
      (2026-07-29 18:26) — via `serde::Deserialize` (axum's `Path<T>`
      extractor requires `DeserializeOwned`, not `FromStr`, in this axum
      version — the plan's original assumption was off), routed through
      the existing `EinmoId::try_from` validation; a malformed segment is
      therefore a deserialization error, which axum's extractor turns into
      a `400` before the handler runs. Also added `EinmoId: Serialize` for
      JSON responses, and a `SessionId: Deserialize` following the same
      pattern.
- [x] Implement session creation: `POST /einmo/sessions` opens an
      `EinmoReview` for the suite given on the server's command line,
      returns the session id; the server calls this against itself once at
      startup (EIMP-2.md §2, §7)
      (2026-07-29 18:26)
- [x] Write endpoint tests FIRST for `POST /einmo/sessions`,
      `GET /einmo/<session>/cases`, `GET /einmo/<session>/cases/<id>`, and
      `GET /einmo/<session>/cases/<id>/body/<stage>`, against `zweimomo`'s
      ported suite; unknown session id 404s on every route; a malformed
      `<id>` segment 400s at the extractor (EIMP-2.md §Test Plan "Unit —
      typed extractors and ApiError mapping")
      (2026-07-29 18:26) — 9 tests in `src/review_server.rs`, incl. an
      invalid-stage-400s case and 3 UDS-lifecycle integration tests (below)
- [x] Implement the four routes above using typed extractors throughout
      (`Path<EinmoId>`, `Path<SessionId>` — never `Path<String>` parsed by
      hand, EIMP-2.md §3a); UDS binding at a configurable path (default
      `./.einmo-review.sock`, §7); write `<socket-path>.session` alongside
      the socket containing the session id; both files removed on exit
      (normal exit and `SIGINT` via `tokio::signal::ctrl_c`); refuse to
      start if a stale socket file exists at the target path and cannot be
      connected to
      (2026-07-29 18:26) — directory-permission (mode-700) inheritance
      deferred: that discipline belongs to the *client's* scratch dirs
      (`experimental_reviewer.sh`'s existing pattern), not the socket file
      itself, which has no analogous scratch content of its own to
      protect; revisit if the session-id sidecar file turns out to need
      it. Stale-vs-live socket detection implemented via a connect probe
      (`UnixStream::connect` succeeds → live, refuse; fails → stale,
      remove and rebind) — covered by
      `serve_uds_refuses_a_live_socket`/`serve_uds_rebinds_a_stale_socket_file`.
- [x] `src/bin/einmo_review_server.rs` + `src/bin/cargo_einmo_review_server.rs`
      binaries (`--socket <path>` flag, `<suite>` positional), `Cargo.toml`
      `[[bin]]` entries
      (2026-07-29 18:26)
- [x] Smoke test: `cargo einmo-review-server <zweimomo-suite>` starts, binds
      its socket, writes the session file, and
      `curl --unix-socket … GET /einmo/<session>/cases` (session id read
      from the session file) returns the real worklist
      (2026-07-29 18:26) — done manually against
      `zweimomo/suites/javascript/day.1` (real 8-case suite): list/detail/
      body all returned correct data; live-socket refusal and clean-shutdown
      cleanup also verified manually, then captured as permanent automated
      tests (`serve_uds_end_to_end_and_cleans_up_on_shutdown` et al.)
      rather than left as a one-off manual check.
- [x] Phase D tests green; `cargo fmt` / `cargo clippy -D warnings` clean
      (2026-07-29 18:26) — 158 einmo tests (was 149) + 6 zweimomo tests,
      clippy/fmt clean

## Phase E — new script increment 1: list + view (read-only)

Create `scripts/einmo_review_client.sh` — first increment, prove the
HTTP-only shape end to end before adding any mutation.
`scripts/experimental_reviewer.sh` is untouched (EIMP-2.md §6); copy its
vim invocation/pane/statusline setup as the starting point, not its
decision-tracking arrays.

- [x] Complete 2026-07-29. New file `scripts/einmo_review_client.sh`; port
      the vim invocation, pane layout, and statusline setup from
      `experimental_reviewer.sh` as the starting point (not its
      decision-tracking arrays) (EIMP-2.md §6); startup check: fail fast
      with a clear message if no server socket (and session file) is found
      for the suite — no fallback to `experimental_reviewer.sh` or direct
      `einmo` calls (EIMP-2.md §6); read the session id once and hold it as
      a constant for the run. Implementation note: vim caps `-c`/`--cmd`
      arguments at 10 (`MAX_ARG_CMDS`); the original plan to pass each split/
      mapping/function as its own `-c` hit that ceiling, so all setup
      commands are joined into a single newline-separated `-c` argument
      instead (same effect as sourcing a script) — documented inline in the
      script so later phases adding more panes/mappings don't reintroduce
      the ceiling. Also carried over the scratch-directory hardening
      (`umask 077`, `harden_dir`, `EINMO_REVIEW_CLIENT_DIR` override, trap
      cleanup) from `experimental_reviewer.sh` since verified bodies
      fetched from the server are still signed content written to local
      scratch files for vim to display.
- [x] Complete 2026-07-29. Added `jq` as a new script dependency (EIMP-2.md
      §6, Rejected Alternative H); checked availability at startup.
- [x] Complete 2026-07-29. Implemented listing: `curl --unix-socket … GET
      /einmo/<session>/cases`, parsed with `jq` into the one local array the
      script needs — `ids`, a list of `EinmoId`s (EIMP-2.md §5), with an
      optional substring filter (positional arg) mirroring
      `experimental_reviewer.sh`'s name-filter usage.
- [x] Complete 2026-07-29. Implemented body viewing: `GET
      /einmo/<session>/cases/<id>/body/<stage>` for `output`/`checked`/
      `verified`, rendered into vim panes; a missing/errored stage (e.g. no
      `verified/` artifact yet) renders its pane's `error` field inline
      instead of a blank pane or a script crash.
- [x] Complete 2026-07-29. Verified against `zweimomo`: started
      `einmo-review-server` against `zweimomo/suites/javascript/day.1`,
      drove `einmo_review_client.sh` end-to-end over a pty (`python3 pty` +
      programmatic `:qa!`); confirmed the worklist (8 cases), per-case
      stage line, and all four panes (input placeholder, output, checked,
      verified-unavailable-fallback) render the expected content sourced
      entirely from the server's JSON responses.
- [x] Complete 2026-07-29. Verified the no-server failure path: ran the
      script against a socket path with no server listening; confirmed it
      fails fast (exit 1) with the documented message and does not attempt
      any fallback.

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
