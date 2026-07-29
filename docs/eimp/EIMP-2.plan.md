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

- [x] Complete 2026-07-29. Wrote endpoint tests FIRST for
      `POST /einmo/<session>/cases/<id>/flag/<stage>`
      (`{"reason":string}`) — success moves the case and returns `200`
      (`flag_endpoint_moves_the_case_and_returns_ok`), unknown case `404`s
      (`flag_endpoint_404s_on_unknown_case`), invalid stage `400`s
      (`flag_endpoint_400s_on_invalid_stage`) — plus `EinmoReview::flag_now`
      unit tests (atomic no-decide-needed, clears any pending decision,
      errors when the stage holds nothing for the id). Implementation note:
      the stage is a path segment (`/flag/<stage>`), not a body field —
      matches the `body/<stage>` route's shape and reuses the same
      `Path<(SessionId, EinmoId, String)>` + `parse_stage` pattern already
      used by `case_body`, rather than inventing a second way to name a
      stage.
- [x] Complete 2026-07-29. Implemented the flag convenience endpoint:
      `EinmoReview::flag_now` (calls `transitions::flag` directly — no
      `decide`/`plan`/`execute` ceremony, since flag needs no signing/gate)
      plus the `flag_case` handler wired into the router. Also fixed
      `ApiError`'s mapping: an `EinmoError::Io` whose underlying
      `ErrorKind` is `NotFound` now maps to `404` instead of falling into
      the `_ => 500` catch-all — this also corrected `case_body`'s
      previously-500 response for a not-yet-promoted stage (observed
      during Phase E's smoke test) into the `404` it should have been all
      along.
- [x] Complete 2026-07-29. In `einmo_review_client.sh`: implemented
      flagging as one `POST … /flag/<stage>` call, sent immediately when
      the reviewer types `f` at the between-tests prompt (EIMP-2.md §6) —
      the equivalent of `experimental_reviewer.sh`'s raw `mv … flagged/`,
      but over HTTP. The stage defaults to whichever stage currently holds
      the case, preferring the highest one present (mirrors
      `source_stage_for_promote`'s stage preference in `review.rs`); the
      reviewer is prompted for a reason, which is sent as the request body.
- [x] Complete 2026-07-29. Verified against `zweimomo`: ran the flag flow
      end-to-end over a pty (server pointed at a scratch copy of `day.1`),
      typed `f` → reason → confirmed the case moved from `checked/` to
      `flagged/` on disk with the correct `# flagged: <reason> <timestamp>`
      advisory line, matching `experimental_reviewer.sh`'s `mv`-based
      output byte for byte in content. (Caught and fixed a stale-binary
      test artifact along the way — the running server process needed a
      rebuild to pick up the new route; not a code bug.)

## Phase G — retract convenience endpoint (EIMP-2.md §3)

- [x] Complete 2026-07-29. Wrote endpoint tests FIRST for
      `POST /einmo/<session>/cases/<id>/retract/<stage>` — success removes
      the case and returns `200` (`retract_endpoint_removes_the_case_and_returns_ok`),
      unknown case `404`s, invalid stage name `400`s, `output` stage
      (not retractable) `400`s — plus `EinmoReview::retract_now` unit tests
      (atomic, cascades checked→verified, clears any pending decision,
      errors when nothing to retract, refuses `output`). Implementation
      note: `stage` is a path segment (`/retract/<stage>`), matching
      `flag`'s and `body`'s shape, not a `{"from": ...}` body field as the
      plan originally sketched — one consistent way to name a stage across
      every route, no request body needed at all for retract.
- [x] Complete 2026-07-29. Implemented the retract convenience endpoint:
      `EinmoReview::retract_now` (calls `transitions::retract` directly,
      cascade included, no signing/gate needed) plus the `retract_case`
      handler. Also added `EinmoError::Config` → `400` to `ApiError`'s
      mapping (retracting `output`/`flagged` returns `Config`, a client
      error — was falling into the `500` catch-all before this).
- [x] Complete 2026-07-29. In `einmo_review_client.sh`: wired `k` (kick) at
      the between-tests prompt to one `POST … /retract/<stage>` call, sent
      immediately — no local queue, closing the gap where
      `experimental_reviewer.sh` accumulated kicks in its `flag_*` arrays
      but never actually executed them (EIMP-2.md §1, §5, §6). Defaults to
      the highest-present of `checked`/`verified` (the only two retractable
      baselines) for this case.
- [x] Complete 2026-07-29. Verified against `zweimomo`: promoted a case to
      `checked` (direct `transitions::promote` in the smoke-test setup, to
      have something to retract), kicked it via the running script over a
      pty, confirmed the `checked/` artifact was removed on disk.

## Phase H — decision + execute endpoints: promote (EIMP-2.md §3, §4)

- [x] Complete 2026-07-29. Wrote endpoint tests FIRST for `PUT … /decision`
      (all four `kind`s: `promote`/`retract`/`flag`/`skip`, replace-not-stack,
      invalid `kind` rejected by the `Json<DecisionRequest>` extractor
      itself), `DELETE … /decision` (clears back to untouched), `GET …/plan`,
      and `POST … /execute`'s gated promote path (`confirm: "PROMOTE"`
      required whenever any pending decision promotes; passphrase required
      for `checked to verified`, absent passphrase refused with nothing
      written). Implementation note: `Json<T>` extractor rejections (bad
      `"kind"`, missing required field) come back `422` from axum itself,
      not `400` — different from `Path<EinmoId>`'s `400` (its rejection is
      the id's own `Deserialize` impl, caught by `Path`'s rejection
      mapping, not `Json`'s) — the two tests originally written for `400`
      were corrected to expect `422` after confirming this against actual
      axum behavior, not assumed from the spec sketch.
- [x] Complete 2026-07-29. Implemented the promote decision + execution
      path: `PUT … /decision` (`DecisionRequest`, a `#[serde(tag="kind")]`
      enum over a `DecidableStage` restricted to `checked`/`verified`,
      converts into `Decision`) → `EinmoReview::decide`; `GET …/plan` →
      `EinmoReview::plan`, rendered through a `PlannedActionResponse` DTO
      (the domain `PlannedAction`/`Stage` types stay serde-free); `POST …
      /execute` builds a `SignerSet` from the request body's optional
      `passphrase` field — constructed inside the handler, held only for
      the duration of the `EinmoReview::execute` call, then dropped when
      the handler returns; never stored on `AppState`, never logged.
      **Found and fixed a real pre-existing bug while writing this
      phase's tests**: `EinmoReview::execute` (from Phase C) applied
      actions but never cleared their entries from the `DecisionBook` —
      an executed (or drift-skipped) decision kept showing up as
      "pending" in the next `plan()`. Added the missing `undecide` pass
      at the end of `execute`, plus two new `review.rs` unit tests
      (`execute_clears_pending_decisions_it_applied`,
      `_it_skipped`) pinning the fix — caught by
      `execute_with_confirm_promotes_to_checked` asserting
      `plan.actions.is_empty()` after execute, not by design foresight.
- [x] Complete 2026-07-29. In `einmo_review_client.sh`: `c`/`v` at the
      between-tests prompt record a promote-to-checked/verified decision
      via `PUT … /decision` immediately (applied later, not immediately —
      unlike flag/kick); after the case loop ends, one end-of-pass block
      renders the pending plan (`GET …/plan`), prompts for the `checked to
      verified` passphrase via `/dev/tty` with `read -s` (hidden input,
      same UX as `experimental_reviewer.sh`) only if any pending action
      needs one, requires typing the literal word `PROMOTE` to proceed,
      then calls `POST … /execute` — the plaintext-over-the-socket caveat
      is called out inline in the prompt text itself (EIMP-2.md §Open
      Questions "Still open").
- [x] Complete 2026-07-29. Verified against `zweimomo`: promoted
      `integer_arithmetic.js` output→checked via the script over a pty
      (confirmed the computer-key-signed `checked/` artifact appeared on
      disk with the expected `stage:checked` stamp), then separately
      checked→verified with a passphrase (confirmed the `verified/`
      artifact's `stage:verified` stamp carries a different pubkey than
      the computer key, i.e. genuinely signed under the supplied
      passphrase, and that the passphrase itself never echoed to the
      terminal).

## Phase I — undo/revisit (EIMP-2.md §3, §5, §6)

- [x] Complete 2026-07-29. `DELETE … /decision` and its handler were
      already implemented in Phase H (a natural companion to `PUT`, needed
      by that phase's own test setup); this phase added the two tests the
      plan specifically calls for: `delete_decision_is_a_no_op_when_already_undecided`
      (bare `DELETE` with no prior `PUT`, still `200`) and
      `put_decision_after_delete_starts_a_fresh_decision` (`PUT` → `DELETE`
      → `PUT` a *different* kind → the new decision, not a stack of both,
      is what's visible — replace-not-stack survives a revisit).
- [x] Complete 2026-07-29. In `einmo_review_client.sh`: the between-tests
      display now shows `GET … /cases/<id>`'s `decision` field inline
      (`pending decision: … (u to undo)`) whenever one exists — reusing the
      `detail_json` already fetched for the stages line, no extra request.
      `u` sends `DELETE … /decision` unconditionally (safe: a no-op if
      nothing was pending) — there is no local array surgery to perform,
      unlike `experimental_reviewer.sh`'s `undo_last_decision`/`answer_of`/
      `drop_from`, because this script never had that state to begin with
      (EIMP-2.md §5, §6).
- [x] Complete 2026-07-29. Verified against `zweimomo` across two script
      runs sharing one server (decisions live server-side, so this is a
      valid way to simulate "revisit later in the same session"): run 1
      decided promote-to-checked on `name_binding.js` and aborted execute
      (left it pending); run 2 showed `pending decision: promote to
      checked (u to undo)`, undo cleared it, and the end-of-pass plan
      immediately reported "nothing pending to execute" — confirming the
      revisit is genuinely server-side, not something the second script
      invocation could have faked locally.

## Phase J — summary rendering + cleanup

- [x] Complete 2026-07-29. Already in place from Phase H: the end-of-pass
      block calls `GET /einmo/<session>/plan` and renders its `actions`
      directly (`kind`, `id`, `to` where present) before asking for
      confirmation, then reports `POST … /execute`'s own `executed`/
      `skipped` counts afterward — no local stats computation exists to
      replace, since this script never accumulated counts itself
      (EIMP-2.md §5). No further work needed for this checkbox.
- [x] Complete 2026-07-29. Measured: `einmo_review_client.sh` is **341
      lines**, `experimental_reviewer.sh` is **1080 lines** (`wc -l`) — the
      new script is not merely "substantially shorter" but under a third
      the size, consistent with EIMP-2.md §5's prediction that moving
      state ownership server-side (no `promote_checked`/`promote_verified`/
      `retract_checked`/`retract_verified`/`flag_stage`/`flag_rel`/
      `flag_reason`/`send_to_agent_list`/`skip_list`/`noop_list` parallel
      arrays, no `undo_last_decision`/`answer_of`/`drop_from` array
      surgery) removes most of what made the old script long.
- [x] Complete 2026-07-29. The plaintext-passphrase-transport weakness
      (EIMP-2.md §Open Questions "Still open") stays an accepted prototype
      limitation, not a blocker for this EIMP's completion — the doc
      already states this explicitly ("not blocking this EIMP", "revisit
      later", "not an emergency, but should not be silently carried
      forward either"). No follow-up EIMP is being opened for it now;
      revisit before `EIMP-1`'s TCP+bearer-token mode is ever built, per
      the existing note.
- [x] Complete 2026-07-29. Phase E–J tests green (184 einmo + 6 zweimomo);
      `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D
      warnings` clean.

## Comprehensive test + completion

- [x] Complete 2026-07-29. Comprehensive test, per `EIMP-2.md` §Test Plan:
      one chained run over a scratch copy of `zweimomo`'s `day.1` suite (8
      cases), driven over a pty against a live server, exercising every
      decision kind in the same pass: `data_structures.js` promoted
      output→checked; `division_by_zero.js` promoted output→verified with
      a genuine passphrase (confirmed via a distinct signing pubkey);
      `function_application.js` kicked (retracted from `checked`);
      `integer_arithmetic.js` flagged with an advisory reason; and
      `name_binding.js` decided-then-undone (confirmed removed from the
      pending plan by a second script invocation reading the same
      server-side session). Every artifact-level effect was verified on
      disk, not just from the script's own success messages.
      **Found and fixed a real bug while running this test**: a killed
      (not cleanly shut down) server leaves its socket file in place, so
      the startup check's `-S "$SOCKET"` (file exists, is socket-typed)
      passed even though nothing was listening — the script would then
      fail deep inside with a raw, unhelpful `curl: (7) Failed to
      connect`. Added a connectivity probe (`curl` a real request against
      the socket) immediately after the existing existence checks, with
      the same fail-fast messaging as the missing-socket case; verified
      both the no-socket-at-all and the killed-server/stale-socket-file
      cases now produce the same clear, documented error.
- [x] Complete 2026-07-29. Integration test: ran `einmo_review_client.sh`
      against a socket path with nothing listening (both "never existed"
      and "existed, server was killed" variants); confirmed it fails fast
      with the documented message and does not attempt any fallback in
      either case.
- [x] Complete 2026-07-29. All tests pass: 184 einmo + 6 zweimomo tests,
      `cargo clippy --workspace --all-targets -- -D warnings` clean,
      `cargo fmt --check` clean.
- [x] Complete 2026-07-29. Updated `EIMP-2.md` frontmatter to
      `status: complete`; removed the "Resolved during scoping" record
      from §Open Questions (kept "Still open" — the plaintext-passphrase
      note — per `EIMP-0`'s convention, which empties only the resolved
      record, not genuinely still-open items).
- [x] Complete 2026-07-29. Updated `docs/eimp/INDEX.md` to add EIMP-2 and
      reflect its completed status.
