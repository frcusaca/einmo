# EIMP-1.plan — einmo-review-session

Read `docs/eimp/EIMP-1.md` before acting on any task below. Tasks run top to
bottom; each phase lands value on its own. This plan is adapted from the
original `FOOP-25.plan.md` (in `foolish-rust`), with worktree/branch
mechanics removed: einmo is a small, single-maintainer repository, so this
plan executes directly on `main` with regular commits (`EIMP-0` §8).

**Re-baselined 2026-07-30.** This plan was ported before `EIMP-2` existed,
and `EIMP-2` then implemented a substantial fraction of it while prototyping
the review server. Phase 0's drift re-survey (below) has now been performed;
every item `EIMP-2` already delivered is checked off with attribution, so
the unchecked boxes below are the *genuinely* remaining work. Two structural
resolutions also landed: `CorpusSigner` ships the **existing byte-join**
construction, single-threaded, with a configurable collation (restructuring
and parallelism both deferred to `EIMP-5`), and everything review-related
will ship in the `einmo-review-server` crate (`EIMP-4` §S.1) — Phase B's
verbs belong to that crate's binary, not core's `cli.rs`.

- [x] STOP — preconditions: all workspace tests pass (`cargo test`,
      `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`). Do
      not begin while any test is broken.
      (2026-07-30 06:05) — 190 workspace tests, clippy/fmt clean.
- [x] Sanity check: consult human to resolve `EIMP-1.md` §Open Questions
      (HTTP stack, journal location, differing default) enough to start
      Phase A. Remind them: "Above message comes from EIMP-1 working to
      build the EinmoReview session object; changes are on `main`. PTAL"
      (2026-07-30 06:05) — all six Open Questions resolved via direct
      conversation with the human (not just the three named above): HTTP
      stack (keep axum), journal location (scratch/state dir), claim TTL
      (5 min, auto-reclaim, shown in `plan()`), quorum (out of scope
      entirely, not just deferred), `ReviewOpts` mode (runtime-selectable
      `Full`/`Random`/`NewOrBroken`, not a boolean default), and Phase A2's
      parallel-read worker pool. Additionally resolved and written into
      `EIMP-1.md` §S.4a: the multi-signer content-then-key decision table
      for `checked`/`verified` promotion (paired with `EIMP-3` for
      `output`'s analogue).
- [x] Begin work: check `begun: [x]` in `EIMP-1.md` frontmatter, commit
      `EIMP-1.md` stating that work has commenced
      (2026-07-30 06:05)
- [x] Second resolution round (2026-07-30, after `EIMP-4`/`EIMP-5` were
      drafted, and revised again after `EIMP-5`/`EIMP-6` were merged):
      `CorpusSigner` keeps `EIMP-1` §S.11's **existing byte-join**
      construction, single-threaded — no Merkle restructuring inside this
      EIMP — with a new §S.11a configurable `Collation` (default
      `PathBytes`) recorded in `.section.sig`. Restructuring *and* its
      parallel machinery are `EIMP-5`, merged into one EIMP since making
      hashing faster and cheaper to update is the point of the
      restructuring. The earlier "use `tokio`" answer is withdrawn —
      `EIMP-4`'s crate split removes `tokio` from core, which is where
      `CorpusSigner` lives. `EinmoReview` and all frontends ship in the
      `einmo-review-server` crate; the journal is keyed by `EinmoId`
      with verbosity levels and must be *capable* of the crash crumb's
      purpose without retiring the crumb here.
      (2026-07-30 07:40)

## Phase 0 — drift re-survey (EIMP-1.md §S.10)

- [x] Read §S.10 of `EIMP-1.md`, then re-survey `src/einmo_suite.rs`,
      `transitions.rs`, `signature.rs`, `verify.rs`, `format.rs`,
      `compare.rs` for API drift since 2026-07-19 (the date this design was
      originally written, as `FOOP-25`)
      (2026-07-30 07:45) — surveyed. The dominant finding is not drift in
      those six files but that `EIMP-2` **already implemented much of this
      plan's Phase A and Phase C**; see the per-item attributions below.
- [x] Touch up `EIMP-1.md` §S.2–§S.7 sketches to match current einmo shapes;
      record notable drift in this plan as sub-tasks
      (2026-07-30 07:45) — §S.1 gained the crate-boundary note, §S.2 the
      `ReviewMode` resolution, §S.4a is new (multi-signer promote), §S.5/§S.6
      gained the claim-TTL and journal resolutions, §S.11 was rewritten
      single-threaded. Notable drift between §S.2's sketch and the shipped
      `review.rs`, recorded as sub-tasks:
  - [x] `MirrorPath` throughout the sketch is really `EinmoId` (`EIMP-2`
        Phase A formalized it). The sketch's type name is stale everywhere.
        (2026-07-30 07:45)
  - [x] `open` is `open(suite: impl Into<PathBuf>) -> Self`, not
        `open(&Path, ReviewOpts) -> Result<Self>` — no `ReviewOpts` exists
        yet, so no `ReviewMode` and no filter. Adding it is Phase A work
        below.
        (2026-07-30 07:45)
  - [x] No `ReviewerId` anywhere: a single implicit reviewer, deliberately
        (`EIMP-2` §2). `decide`/`undecide` take no reviewer argument.
        (2026-07-30 07:45)
  - [x] `EinmoReview`'s fields are `config`/`cache`/`decisions`/`exec` —
        **no `worklist: RwLock<Worklist>`** (`items()` rescans on every
        call, so there is no cached worklist to `refresh()` and no `version`
        for the sketch's If-Match story) and **no `journal`**.
        (2026-07-30 07:45)
  - [x] `body` returns `VerifiedBody`, not `Arc<VerifiedBody>`;
        `VerifiedCache` caches `Result<VerifiedBody, String>` rather than
        `Arc<OnceLock<VerifiedBody>>` (`EinmoError` isn't `Clone` —
        `EIMP-2` Phase C recorded this).
        (2026-07-30 07:45)
  - [x] `SignerSet` wraps einmo's existing `KeySource` with fields
        `to_checked`/`to_verified`, not the sketch's separate `Signer` type
        with fields `checked`/`verified`.
        (2026-07-30 07:45)
  - [x] `EIMP-2` added two methods the sketch never had: `flag_now` and
        `retract_now` (atomic, no decide/plan/execute ceremony). They stay.
        (2026-07-30 07:45)
  - [x] Absent entirely from the shipped code: `diff`, `execute_one`,
        `refresh`, `decision`. All are Phase A work below.
        (2026-07-30 07:45)

## Phase A — the session library (EIMP-1.md §S.2–§S.6)

- [x] Write the unit tests FIRST (`EIMP-1.md` §Test Plan: decisions, cache,
      signer, execute) as failing tests against the intended
      `einmo::review` surface
      (2026-07-29 17:38) — done by `EIMP-2` Phase C: 9 tests written first
      in `src/review.rs`. Journal tests are NOT covered (no journal yet) —
      see the journal task below.
- [x] Implement `review::Decision` + `DecisionBook` (replace-not-stack)
      (2026-07-29 17:38) — `EIMP-2` Phase C. **Not** per-reviewer and not
      versioned: single implicit reviewer, no `version` field. Whether to
      add either is deferred with claims/If-Match below.
- [x] Implement `review::VerifiedCache` (fingerprint → cached body,
      single-flight; verify-count test hook)
      (2026-07-29 17:38) — `EIMP-2` Phase C.
- [x] Implement `review::Signer` / `SignerSet` — §S.4 is the authority for
      what does NOT go in `EinmoReview`
      (2026-07-29 17:38) — `EIMP-2` Phase C, as a `KeySource` wrapper.
- [x] Implement `EinmoReview` (open/items/body/decide/undecide) over the
      above
      (2026-07-29 17:38) — `EIMP-2` Phase C. `diff`/`refresh`/`decision`
      remain; see below.
- [x] Implement `ExecutionPlan` + `execute` (exec mutex, fingerprint
      re-check, skip-and-report drift, confirm token plumbed but enforced by
      frontends)
      (2026-07-29 17:38, fix 2026-07-29 18:xx) — `EIMP-2` Phase C, incl. the
      exec mutex and the `undecide` pass after execute. Correction
      (2026-07-30 17:55): the "fingerprint re-check" this box claimed was
      only ever *presence*-based (`source_stage_for_promote` re-checking
      whether the source stage still exists) — no content fingerprint was
      actually compared, so a source that changed bytes without changing
      which stage held it went undetected. Genuine content-fingerprint
      re-check was added later alongside `refresh()` (see that checkbox
      below) once the gap surfaced from working through `refresh`'s own
      "decide first" instruction. `execute_one` and
      the retract cascade *within execute* remain; `retract_now` covers the
      cascade for the atomic path.
- [x] `ReviewOpts` + `ReviewMode` (`Full` default / `Random` /
      `NewOrBroken`) on `open`, with `filter` (`EIMP-1.md` §S.2). Note
      `ReviewItem.differing` already exists, so `NewOrBroken`'s predicate is
      available; `Random` needs a seed decision (record it — a fixed seed is
      reproducible, an entropy seed is a genuinely different sample per run)
      (2026-07-30 17:38) — added `ReviewOpts`/`ReviewMode` to `review.rs`;
      `EinmoReview::open` unchanged (defaults to `ReviewOpts::default()` —
      `Full`, no filter, zero churn for ~20 existing test call sites), new
      `EinmoReview::open_with(suite, opts)` for the configurable path.
      `NewOrBroken` filters `items()` on the existing `TestRow.differing`.
      `Random` seed decision: **OS entropy, not a fixed seed** — `items()`
      already rescans from scratch every call (no cached worklist to keep a
      stable order consistent with, per the Phase 0 drift finding), so a
      fixed seed would buy fake reproducibility over state that isn't
      stable anyway; a reviewer wanting a repeatable sample should record
      it via decisions, not order. Implemented as a small Fisher-Yates over
      `rand_core::OsRng` (already a dependency via `signature.rs`) rather
      than adding the `rand` crate. 5 new tests; exported from `lib.rs`.
      194 workspace tests, clippy/fmt clean.
- [x] `EinmoReview::diff(id, left, right) -> DiffHunks` + the server
      endpoint that exposes it; today the client renders whole panes with
      no server-computed diff (`similar` is already a dependency)
      (2026-07-30 17:47) — `DiffHunks`/`SectionDiff`/`DiffLine` in
      `review.rs`: `diff` calls `body()` on both sides (verify-on-inspect +
      single-flight cache reused, not bypassed), then diffs each section
      with `similar::TextDiff` (same crate `einmo_suite.rs`'s dependent-DIFF
      generation already uses). STAMPS is never a section here because
      `VerifiedBody` already excludes it. Server: `GET
      /einmo/<session>/cases/<id>/diff/<left>/<right>` — path segments for
      both stages, matching `body/<stage>`'s existing shape rather than the
      original sketch's `?l=&r=` query params, for one consistent way to
      name a stage across every route (same precedent Phase F/G set for
      `flag`/`retract`). DTOs (`DiffResponse`/`SectionDiffResponse`/
      `DiffLineResponse`) keep the domain type serde-free, per the
      established `PlannedAction`/`PlannedActionResponse` pattern.
      3 library tests + 3 endpoint tests (success, unknown case 404,
      invalid stage 400) + added to the unknown-session-404s-everywhere
      sweep. 200 workspace tests, clippy/fmt clean. Client wiring
      (rendering diff hunks instead of whole panes) deferred to Phase D,
      which already lists it as a follow-up task.
- [x] `EinmoReview::refresh()` — rescan and report changed cases. Note the
      Phase 0 finding: `items()` currently rescans every call, so decide
      first whether `refresh` means "invalidate a cache that does not yet
      exist" (i.e. also add the cached worklist) or is unnecessary as
      specified. Record the decision either way
      (2026-07-30 17:55) — decision: `refresh` is NOT "make items() fresh"
      (nothing to invalidate — `items()` already reads disk every call, no
      cached worklist exists to be stale). It IS real work: `decide()`
      never recorded what content a decision was actually based on, so
      nothing detected a decision going stale after the fact — the
      "fingerprint re-check, skip-and-report drift" earlier Phase A
      checkboxes claimed as EIMP-2-delivered turned out to only be
      *presence*-based (does the source stage still exist), never
      *content*-based (did the source stage's bytes change). Implemented
      properly: `DecisionBook` entries now carry a `Fingerprint` (the same
      type `VerifiedCache` already uses) of the decision's basis stage,
      captured at `decide()` time via new `decision_basis_stage`/
      `decision_basis_path` helpers. `refresh()` recomputes and compares,
      returning drifted ids **without clearing their decisions** — a
      frontend decides whether to re-prompt. `execute()` gained a
      pre-filter (checked against the *live* `DecisionBook`, not `plan`
      itself, so a decision changed between `plan()` and `execute()` is
      also caught) that skips-and-reports any action whose basis fingerprint
      no longer matches, via a mirrored `action_basis_path` helper (the two
      enums are shape-parallel, so this doesn't convert one into the
      other). 5 new tests, incl. one proving a drifted promote never
      touches `checked/` at all. 205 workspace tests, clippy/fmt clean.
- [x] `EinmoReview::execute_one(id, keys)` — per-item execution, and
      `decision(id)` ("answer so far")
      (2026-07-30 18:02) — `execute_one` builds a one-action
      `ExecutionPlan` and calls `execute` (`EIMP-1.md` §S.4's "individual
      vs batch collapses into one design" — not a separate code path, so it
      gets the same drift check, exec-mutex exclusivity, and
      undecide-on-completion for free); errors if no decision is pending,
      the decision is `Skip`, or the action was skipped (drifted/source
      gone) rather than executed — in the last case the decision is still
      cleared, same as batch `execute`. `decision(id)` is a direct
      `DecisionBook` read, no `items()` scan needed. 6 new tests. 211
      workspace tests, clippy/fmt clean.
- [x] §S.4a multi-signer promote: apply the content-then-key decision table
      to `checked`/`verified` inside `execute`/`execute_one` — content
      matches + my key present → no-op; content matches + new signer →
      append my stamp to the existing artifact; content differs → fresh
      write. Reuse `Stamps::has_stage_stamp_from` (added by `EIMP-3`) rather
      than writing a second lookup
      (2026-07-30 18:11) — new `promote_one_accumulating` (per-file,
      reusing `Stamps::has_stage_stamp_from` and
      `EinmoFile::append_stage_stamp_with` exactly as planned) replaces the
      single batched `transitions::promote` call in `execute`'s promote
      loop; the "derive the stage key once per (from, to) group" property
      is preserved (`StageKeypair::derive` moved out of the per-file
      function, called once by the caller,
      `execute_derives_stage_key_once_per_batch_not_per_case` still green).
      Unlike `EIMP-3`'s output-stage table, a content mismatch here is
      never a failure — checked/verified promotion always accepts the
      reviewer's approved content as the new baseline, which is what
      promoting *means* — only a broken source read is an error. Not
      shared as a common helper with `EIMP-3`'s `write_output` (its own
      "share where it falls out naturally, don't force it" note): the
      write-side stamping differs (append-one-stamp onto an
      already-certified source file here, vs. `Stamps::generate`'s full
      3-stamp chain there), so only the classification logic would be
      shareable, and extracting it now risked touching already-shipped,
      tested `EIMP-3` code without a concrete need. 3 new tests (true
      no-op, second-signer co-sign showing 2 `stage:checked` stamps, fresh
      baseline on genuine content change showing 1 fresh stamp); the
      existing `execute_promote_matches_cli_promote_byte_for_byte`
      equivalence test still passes unmodified (first-time promotion is
      the same "absent dest → fresh write" path as before). 214 workspace
      tests, clippy/fmt clean.
- [x] Flag = plaintext, **concatenating** (§S.3): `flagged/` is
      PLAINTEXT/unsigned/transient; execute writes the annotated note as
      plaintext and CONCATENATES a dated block on top when re-flagging;
      concurrent flags serialize under the exec mutex; `flagged/` stays
      exempt from verification. **Today it replaces** — `transitions.rs`'s
      `reflag_replaces_the_existing_flagged_file` test pins the current
      behavior and must be rewritten, not deleted
      (2026-07-30 18:22) — `transitions::flag` now reads the existing
      `flagged/<rel>` file's advisory (if any — verify-on-inspect still
      applies, only the unsigned trailing advisory is read) and writes
      `{new_block}\n{existing}` (newest on top). `reflag_replaces_...`
      rewritten to `reflag_concatenates_with_the_existing_flagged_note`
      (rewritten, not deleted, per the checkbox's own instruction) plus a
      new `triple_reflag_preserves_every_prior_block_in_order`.
      `flag_now` gained the `exec` mutex for the same reason `execute`
      holds it for its whole duration: concatenation is a
      read-then-write, and two concurrent flags racing that window could
      lose one's block.
      **Found and fixed a real format bug while testing this**:
      `format.rs`'s `split_advisory` used `rfind` on the *whole file text*
      to find "the last line starting with `# flagged:`", which was only
      ever correct for a single-block advisory — a concatenated ≥2-block
      advisory got split at the WRONG "# flagged:" occurrence, corrupting
      the STAMPS/advisory boundary and breaking verification entirely (the
      first failure surfaced as a stamp-JSON parse error, not something
      that looked like an advisory bug at first). Fixed by scoping the
      search to the STAMPS-and-after tail (everything after the last
      section `separator`) and taking the FIRST match within that tail,
      not the last — STAMPS is JSON-lines and never starts a line with
      `#`, so this is unambiguous and immune to body-section content that
      might coincidentally contain `# flagged:` text. Pinned with a new
      `format.rs` test,
      `concatenated_advisory_round_trips_and_keeps_the_chain_valid`.
      6 new/rewritten tests total across `format.rs`/`transitions.rs`/
      `review.rs`. 217 workspace tests, clippy/fmt clean.
- [x] New signed `notes/` stage (§S.3): a durable, attributed sibling to
      `flagged/`; a note is a valid signed `.einmo` (stamped,
      verify-on-inspect, participates in signature checks); support
      promoting a flag's concatenated content into `notes/` as a signed
      note body
      (2026-07-30 18:35) — **scoping decision**: `notes/` is deliberately
      NOT a new `Stage` enum variant. `Stage` has ~266 non-test call sites
      (`is_legal_transition`, `compare`, the CLI's `--stage` selection,
      suite-integrity walks, `StageDirs`, …); adding a variant would ripple
      through every exhaustive match over it for a stage that isn't part
      of the promotion pipeline at all (no retract, no compare, nothing to
      walk). Implemented instead as a narrow, self-contained function,
      `transitions::promote_flag_to_note`, matching how `flagged/` itself
      already sits outside most of that machinery. `TestConfig` gained
      `stage_dir_for_notes()` (a plain path, not a `StageDirs` entry).
      Building the note's full 3-stamp chain needed a real primitive fix
      first: `Stamps::generate` hardcoded `"stage:output"` as both the
      certified role and the stage key name, so it was unusable for any
      other stage — generalized into `Stamps::generate_for_stage(prior,
      configured, stage_key, stage_signer)`, with `generate` becoming a
      thin specialization (`generate(...) = generate_for_stage(...,
      "stage:output", ...)`), so all ~14 existing callers are unaffected
      (pinned by a new
      `generate_is_generate_for_stage_specialized_to_stage_output` test).
      `promote_flag_to_note` reads the note's body from the flagged file's
      advisory (verify-on-inspect first) and does NOT consume the flag —
      `flagged/<rel>` stays in place; resolving the flag is a separate,
      deliberate action. Broader integration (`einmo verify` scanning
      `notes/`, a CLI `--stage notes` selector) is explicitly deferred —
      not required by this checkbox's actual text, and the concrete need
      hasn't appeared yet. 5 new tests across `signature.rs`/
      `transitions.rs`. 222 workspace tests, clippy/fmt clean.
- [x] Flags break tests by default (§S.3): a flagged artifact fails the run
      (non-zero / red gate); `--flag-is-not-failure` downgrades to
      non-fatal but stderr STILL announces the flag count (no silent
      config); wire into the goal-state check (green = zero flags + signed
      + matching + valid signatures). Tests per §Test Plan "flag breaks
      tests". Note today's `flagged_orphan_is_not_a_violation` encodes the
      opposite policy for suite *shape* — reconcile the two deliberately
      (2026-07-30 18:43) — **reconciliation**: kept as two genuinely
      separate concepts rather than unified. `SuiteIntegrity` (shape:
      orphans/extraneous, R1/R2) is untouched —
      `flagged_orphan_is_not_a_violation` still passes unmodified, because
      a flagged artifact's presence was never what that check was about.
      New, independent library function `einmo_suite::count_flagged(config)
      -> Result<Vec<PathBuf>>` (walks `flagged/` directly — no `SuiteIntegrity`
      involvement at all) backs a new `--flag-is-not-failure` flag on
      `einmo verify`. The actual gate decision is a small pure function,
      `flags_fail_the_gate(count, flag_is_not_failure) -> bool`, kept
      separate from `ExitCode`/printing specifically so it is directly
      unit-testable (`std::process::ExitCode` has no public equality check
      in stable Rust, so a full `cmd_verify`-level assertion isn't
      practical — the pure decision function is where the real logic lives
      and where it's tested). The stderr announcement
      (`einmo: warning: <N> flagged artifact(s) present: …`) always prints
      when `count > 0`, in both JSON and text mode, regardless of
      `--flag-is-not-failure` — only the exit code changes. 5 new tests
      (2 library, 3 CLI: the pure gate function's three cases, plus a parse
      test for the new flag). 227 workspace tests, clippy/fmt clean.
- [x] Implement `Journal` (append-only JSONL, replay, truncated-tail
      tolerance) per the §S.6 resolution: keyed by `EinmoId`, verbosity
      levels (terse/normal/fine), `fine` recording each case as it is read
      in and verified. Must be *capable* of the crash crumb's purpose;
      retiring the crumb is explicitly NOT this EIMP's work (follow-up
      logging EIMP, see the repo TODO)
      (2026-07-30 18:58) — new `src/journal.rs`. `Journal::open` never
      fails (a plumbing failure just means events go unrecorded — logging
      must never block a reviewer's actual work); `JournalLevel::Terse <
      Normal(default) < Fine` gates `log_at` by a plain comparison.
      `replay` is truncated-tail tolerant by being *maximally* tolerant:
      any unparseable line (not just the last) is silently skipped rather
      than aborting the whole replay — simpler than distinguishing
      tail-truncation from other corruption, and still correct (every line
      that DOES parse survives). Scratch dir: `$EINMO_JOURNAL_DIR` or
      `temp_dir()/einmo-journal`, hardened to mode 0700 (mirrors
      `einmo_review_client.sh`'s `harden_dir`). Domain types stay
      serde-free (`review_server.rs`'s existing DTO convention): a new
      `JournalDecision` is the journal's own wire form for `Decision`,
      with round-trip conversions.

      Wired into `EinmoReview`: a random 128-bit session id per `open`/
      `open_with`; `decide`/`undecide` log at `Normal`; `execute` logs one
      `ExecuteBatch` at `Terse` (so `execute_one` inherits it for free, per
      its own "not a separate code path" design); `body` logs
      `VerifyStart`/`VerifyEnd` at `Fine` (an unmatched `VerifyStart`
      identifies the in-flight case after a crash — the crash-crumb-capable
      claim, without writing anything into `output/`); a `Drop` impl logs
      `SessionClose` so "terse: session open/close" needs no separate
      method to remember to call.

      New `EinmoReview::resume(suite, session_id, opts) -> Result<Self>`
      (`EIMP-1`'s "Reopen = replay", concretely: Use Case #7, "resume after
      a crash"): replays a session's journal and reconstructs pending
      decisions by calling the ordinary `decide`/`undecide` methods, in
      order — **not** by restoring the original decide-time fingerprint
      verbatim. This is a deliberate simplification found while
      implementing: recomputing the drift-check basis fresh at resume-time
      is actually the *safer* choice, not a shortcut — if nothing changed
      during the crash gap the fingerprints agree anyway, and if something
      DID change, that's a legitimate drift `execute`/`refresh` should
      still catch, not something a resume should paper over by trusting
      stale pre-crash state.

      11 new tests (7 in `journal.rs`, 6 in `review.rs` incl. two resume
      tests and a third pinning that resuming an unknown session id is
      just an ordinary fresh review, not an error). Found and fixed one
      test bug along the way: an early draft's test helper embedded a
      tempdir's full path (containing `/`) into a session id string later
      used as a filename component, corrupting the journal path — fixed by
      using only the tempdir's leaf name. `Journal`/`JournalEvent`/
      `JournalLevel`/`JournalDecision`/`JournalLine` exported from `lib.rs`.
      240 workspace tests, clippy/fmt clean.
- [x] Soft claims (§S.5): `claim(id, ttl)`, 5-minute default,
      auto-reclaimed on expiry, surfaced in `plan()` output. Advisory only —
      cannot wedge. (Single implicit reviewer today, so this is
      infrastructure for the multi-verifier story rather than something the
      current client exercises — note that when implementing)
      (2026-07-30 19:09) — `EinmoReview::claim(id)` (default 5-minute TTL,
      `DEFAULT_CLAIM_TTL` constant) and `claim_for(id, ttl)` (explicit TTL,
      used by tests to avoid a real 5-minute wait). Backed by `claims:
      RwLock<HashMap<EinmoId, Instant>>`; claiming an already-claimed case
      refreshes (replace-not-stack, same discipline as decisions) rather
      than stacking or erroring — advisory, nothing to contend over.
      Opportunistic pruning happens inside `claim_for` itself (the natural,
      cheap point to drop stale entries) rather than needing a background
      task or explicit release call — matches §S.5's "no action needed
      from the original claimant" exactly, since there genuinely is no
      release API at all, only auto-expiry. `ExecutionPlan` gained a
      `claims: Vec<ActiveClaim>` field (`{id, remaining: Duration}`),
      populated by a new private `active_claims()` read (filters expired,
      never mutates); confirmed advisory-only by a test that claims a case
      and then decides+executes it through to completion unimpeded. As
      noted in the plan, single-implicit-reviewer today means this is
      infrastructure the current client has no reason to call yet — it
      becomes load-bearing once multiple concurrent reviewers exist.
      4 new tests. `ActiveClaim` exported from `lib.rs`. 244 workspace
      tests, clippy/fmt clean.
- [x] All Phase A tests green; `cargo fmt` and `cargo clippy -D warnings`
      clean
      (2026-07-30 19:09) — confirmed: 244 einmo lib tests + 4 zweimomo unit
      + 3 zweimomo integration, all green (einmo alone grew from 189 tests
      at Phase A's start to 244). `cargo clippy --workspace --all-targets
      -- -D warnings` and `cargo fmt --check` both clean. **Phase A is
      complete.**

## Phase A2 — `CorpusSigner` (section PQ attestation), CRYPTO CORE ONLY, BYTE-JOIN + SINGLE-THREADED (EIMP-1.md §S.11, §S.11a)

Self-contained `CorpusSigner` object — NOT mixed into `EinmoReview` (§S.11).
NO real-corpus writes and NOT wired into the live promotion flow in this
EIMP (that integration is a later step). Prove the object in isolation;
`EinmoReview` will merely hold and call it later. **Single-threaded**
(resolved 2026-07-30): this ships in core `einmo`, which `EIMP-4` keeps free
of any async runtime. The digest construction stays `EIMP-1` §S.11's
byte-join — `EIMP-5` handles both the Merkle restructuring and the
parallelism, as one change, after this ships.

> **STASH — temporary implementation notes (2026-07-30), delete this block
> when Phase A2's checkboxes below are all checked.** Captured mid-design so
> nothing is lost if the sandbox reboots again before the code lands.
>
> **fips205 API, confirmed by reading the crate source
> (`fips205-0.4.1/src/{lib,traits}.rs`) — not just its docs:**
> - `slh_dsa_sha2_256s::N = 32` (seed size), `PK_LEN = 64`, `SIG_LEN = 29792`.
> - `KeyGen::keygen_with_seeds::<32>(sk_seed, sk_prf, pk_seed) -> (PublicKey,
>   PrivateKey)` is a **default trait method already in fips205** — fully
>   deterministic (internally drives a `DummyRng` that just replays the three
>   seeds via `try_fill_bytes`, no OS entropy touched). This is the
>   dual-derivation primitive; no new dependency (`rand_chacha` etc.) needed.
> - `PrivateKey::try_sign_with_rng(rng, message, ctx, hedged)` — verified in
>   `slh_sign_with_rng`'s body that when `hedged == false`, `opt_rand` is
>   fixed to `self.pk_seed` and `rng.try_fill_bytes` is **never called** —
>   signing is fully deterministic given the same key + message when
>   `hedged: false`, regardless of what rng is passed. Plan: pass
>   `&mut rand_core::OsRng` (already a dep, unused in practice) and
>   `hedged: false` throughout — same message ⇒ byte-identical signature,
>   every run.
> - Both are pure Rust, `#![no_std]`, `#![deny(unsafe_code)]` — better than
>   the `#![forbid(unsafe_code)]` floor this crate already holds.
>
> **Key derivation plan (extends `signature.rs`, not a new derivation
> scheme):** generalize `derive_keypair` to delegate to a new
> `pub(crate) fn derive_seed(passphrase: &str, salt: &[u8]) -> [u8; 32]`
> (same pinned Argon2id params, just parameterized on salt). `corpus_signer.rs`
> then derives a **master seed** via `derive_seed(passphrase,
> CORPUS_SIGNER_SALT)` (`CORPUS_SIGNER_SALT = b"einmo:corpus-signer-key:v1"`,
> domain-separated from the Ed25519 `SALT`), then SHA-256-expands it three
> ways (`expand(&master, b"sk_seed")` / `b"sk_prf"` / `b"pk_seed"`, each
> `Sha256(fixed-domain-prefix || label || master)`) into the three 32-byte
> seeds `keygen_with_seeds` wants. One Argon2id run (not three) — the
> expensive KDF step stays singular; SHA-256 fans it out. Same passphrase ⇒
> same master ⇒ same three seeds ⇒ same SLH-DSA keypair, always.
>
> **Digest plan, matching §S.11 step 3 ("hashed, and SPHINCS+ signs that
> digest") literally:** `digest()` streams a SHA-256 hasher over (a) a
> canonical header encoding stage name + param-set id (`"slh_dsa_sha2_256s"`)
> + collation id + the collation-ordered `EinmoId` list — so the *path set
> and order* are part of what's signed, not just concatenated file bytes —
> then (b) each file's full on-disk envelope bytes in manifest order,
> incrementally. The resulting 32-byte SHA-256 digest is the fips205 signing
> **message** (`sk.try_sign_with_rng(&mut OsRng, &digest, b"", false)`),
> matching the spec's explicit two-stage "hash, then SPHINCS+ signs that
> digest" rather than feeding the whole section to SLH-DSA directly.
> Mid-read size change on a file → hard error (record `len` from `metadata`
> before streaming, compare to bytes actually read after).
>
> **`Collation::PathBytes` comparator — the key insight:** compare
> `EinmoId::as_str().split('/')` (an iterator of `&str` components) via
> `Iterator::cmp`, **never** `EinmoId::as_str().cmp()` directly on the whole
> string. Rust's `str: Ord` is already raw-byte lexicographic (no locale, no
> normalization — satisfies items 3–4 of §S.11a for free), but comparing the
> *whole path string* would let `/` (0x2F) vs `-` (0x2D) decide before
> structure does, which is exactly backwards for the `a/b` vs `a-b` case the
> spec calls out: `["a","b"]` vs `["a-b"]` compares component 0 (`"a"` is a
> strict prefix of `"a-b"`) and puts `a/b` first, whereas a raw string
> compare would put `a-b` first (`-` < `/` in byte value). Splitting before
> comparing is what makes "a path boundary can never be confused with a
> character inside a name" (§S.11a item 2) actually true. Tie detection
> (item 5): sort, then check adjacent pairs — a tie can only mean duplicate
> input, so it's a hard error, not silently broken.
>
> **Module layout decided:** new `src/collation.rs` (the `Collation` enum,
> standalone, no `CorpusSigner` dependency — testable on bare `EinmoId`
> lists) and new `src/corpus_signer.rs` (`CorpusSigner`, `SectionManifest`,
> `SectionDigest`, `SectionSig`, the dual-derivation helpers). `sign` takes
> `&KeySource` (existing type from `config.rs`, not a new `Signer` type — the
> EIMP-1.md pseudocode's `&Signer` parameter predates `SignerSet`/`KeySource`
> actually being built in Phase A; `key.passphrase()` is the established
> idiom every other signing call site already uses, e.g.
> `transitions.rs::promote`). New `EinmoError::CorpusSignature(String)`
> variant for manifest/collation/digest/SLH-DSA-signature failures —
> deliberately distinct from `EinmoError::Verification` (the per-file Ed25519
> stamp-chain checker) so an unknown-collation error is never textually
> indistinguishable from a tampered-corpus mismatch, per §S.11a's explicit
> requirement. `einmo.toml` wiring: add `collation: Option<String>` to
> `config.rs`'s `SigningConfig`, parsed alongside the existing
> `output`/`checked`/`verified` fields; a `TestConfig` accessor resolves it
> via `Collation::parse` (default `"path-bytes"` when unset) — this is the
> "wire it to `einmo.toml`" checkbox, independent of *using* `CorpusSigner`
> from the live promotion flow, which stays out of scope per this phase's
> header.
>
> Not yet started: no `collation.rs`/`corpus_signer.rs` files exist yet, no
> tests written, no code beyond the already-committed `Cargo.toml` fips205
> dependency. Next action on resume: write `collation.rs` tests-first (the
> plan's own ordering — Collation before the `CorpusSigner` skeleton).

- [ ] Read §S.11 of `EIMP-1.md`; add `fips205` dep (feature
      `slh_dsa_sha2_256s` — conservative set) to `Cargo.toml`
- [ ] Write tests FIRST (§Test Plan "CorpusSigner" + section attestation):
      deterministic manifest; digest changes on add/remove/alter/reorder;
      SLH-DSA sign→verify round-trip; tamper fails; same-passphrase
      dual-derivation determinism; empty-section manifest — all exercising
      `CorpusSigner` standalone (no `EinmoReview`)
- [ ] Implement `EIMP-1.md` §S.11a's `Collation` — the configurable
      ordering, defaulting to `PathBytes`. Tests FIRST, against the
      variation it exists to eliminate: paths differing only by case; paths
      differing by Unicode normalization (NFC vs NFD of one grapheme);
      paths where a separator vs an in-name character would flip a naive
      string sort (`a/b` vs `a-b`); nested vs flat paths sharing a prefix;
      and that the ordering is identical for the same file set fed in
      several shuffled discovery orders. Wire it to `einmo.toml`
      (`[signing] collation = "path-bytes"`), and record its identifier in
      `.section.sig` so an unknown collation fails as *that*, never as a
      generic signature mismatch
- [ ] Implement `CorpusSigner` skeleton (`new`/`manifest`/`digest`/`sign`/`verify`)
      + the manifest builder (stage name + param-set id + collation id +
      the collation-ordered mirror-path list)
- [ ] Implement the sequential streaming read: files in manifest order,
      hasher fed incrementally, bounded memory, no whole-section buffer.
      This digest is the correctness reference and the performance baseline
      `EIMP-5` is measured against — write it to be the reference, not a
      placeholder
- [ ] Extend `Signer` (§S.4) to derive BOTH the Ed25519 stamp key and the
      section SLH-DSA key from one passphrase (Argon2id output expanded to
      the SLH-DSA seed; deterministic keygen)
- [ ] Implement `sign`/`verify` over the digest; `.section.sig` file shape
      defined but written only to fixtures/tempdirs in tests, never the
      real corpus
- [ ] Confirm core `einmo` gained no async runtime and no new heavy
      dependency beyond `fips205` — the constraint `EIMP-4`'s split depends
      on
- [ ] Phase A2 tests green; `cargo fmt` / `cargo clippy -D warnings` clean;
      `#![forbid(unsafe_code)]` still holds (fips205 is pure Rust)

## Phase B — review CLI verbs (in the `einmo-review-server` crate)

Per `EIMP-4` §S.1 these operate on `EinmoReview` and therefore belong to
`einmo-review-server`'s binary, not core `einmo`'s `cli.rs`. Until `EIMP-4`
performs the split they land in this repo's existing binary; the split then
moves them wholesale.

- [ ] `einmo review plan|list|decide|undecide|execute` one-shot subcommands
      (journal-backed session identity) with endpoint-equivalent semantics;
      unit tests
- [ ] Byte-for-byte equivalence test: `review execute` promotion == existing
      `einmo promote`
      (note: `EIMP-2` Phase C already proved `EinmoReview::execute` ==
      `transitions::promote` at the library level — this task is the *CLI*
      surface's equivalent)

## Phase C — the server (EIMP-1.md §S.7)

- [x] UDS listener by default; suite session identity; the server calls
      session-creation against itself at startup
      (2026-07-29 18:26) — `EIMP-2` Phase D: `einmo-review-server` binds a
      UDS at a configurable path, writes a `.session` sidecar, cleans both
      up on exit, and refuses to start against a live socket (rebinding a
      stale one).
- [x] JSON endpoints: sessions, cases, case detail, body, flag, retract,
      decision `PUT`/`DELETE`, plan, execute — with typed extractors and an
      `ApiError`→HTTP-status mapping
      (2026-07-29 18:26 → 2026-07-29, Phases D–I) — `EIMP-2`.
- [ ] **TUI-owned private server mode (`EIMP-1.md` §S.7a)** — the default
      launch shape: the client script starts its own server on a private,
      unpredictable socket inside its mode-700 scratch dir, drives it, and
      terminates it on exit (including `Ctrl-C`/abnormal exit, via the
      script's existing `trap` cleanup plus the server's `ctrl_c` handler).
      The standalone long-lived server stays available — this is an
      additional default mode, not a replacement
  - [ ] Upgrade `axum` 0.7.9 → 0.8.x and **delete** the hand-rolled UDS
        accept loop (`hyper` HTTP/1.1 builder + `hyper-util` `TokioIo`/
        `TowerToHyperService` + manual `UnixListener` loop) that `EIMP-2`
        Phase D only needed because 0.7's `serve()` is TCP-only. Verify the
        `Listener` impl for `tokio::net::UnixListener` is present in the
        enabled feature set before deleting the glue, and re-check whether
        `hyper`/`hyper-util` remain needed at all afterward
  - [ ] Keep `EIMP-2` Phase D's stale-vs-live socket probe
        (`UnixStream::connect` succeeds → refuse; fails → remove and
        rebind). §S.7a's illustrative snippet unlinks unconditionally,
        which would let a new server stomp a live one's socket
  - [ ] Socket path is per-session and unpredictable inside the hardened
        scratch dir — never a fixed world-known `/tmp` path
- [ ] `einmo review serve` as a *subcommand* spelling (today it is a
      standalone `einmo-review-server` binary); decide whether both spellings
      survive the `EIMP-4` split or the subcommand form is dropped —
      record the decision rather than doing it by default
- [ ] TCP 127.0.0.1 + bearer token behind a flag (UDS remains the default).
      Note `EIMP-2`'s standing caveat: the `checked to verified` passphrase
      travels plaintext in the execute body, which is materially riskier
      over TCP than over a UDS — resolve that *before* enabling TCP, not
      after
- [ ] Suite lockfile: a second server on the same suite refuses to start
- [ ] The remaining §S.7 endpoints: `diff` (needs Phase A's `diff`),
      claims, If-Match/409 (needs a `version` on items — see the Phase 0
      finding that no cached worklist or version exists), and SSE events
- [ ] Concurrency tests: N verifiers, single-flight verify counts, no lost
      updates, claims expire

## Phase D — the thin client script (EIMP-1.md §S.8)

**Re-scoped 2026-07-30.** As written this phase said "reduce
`scripts/experimental_reviewer.sh`". `EIMP-2` instead wrote a *new* thin
client (`scripts/einmo_review_client.sh`, 352 lines vs the old script's
1080) and deliberately left `experimental_reviewer.sh` untouched as
reference material and fallback (`EIMP-2` §6). The reduction this phase
existed to achieve has therefore already happened, by replacement rather
than by edit.

- [x] Server discovery + `fetch_body`/decision/plan/execute thin-client
      paths
      (2026-07-29) — `EIMP-2` Phases E–I, in the new client. No direct
      `einmo` fallback: the new script fails fast if no server is reachable
      (`EIMP-2` §6, a deliberate departure from this phase's original
      "keep the direct fallback" instruction).
- [x] Delete the superseded state machinery (decision arrays, undo/answer
      bookkeeping, results rendering, stats computation)
      (2026-07-29) — achieved by not carrying it into the new script;
      `experimental_reviewer.sh` retains it and is untouched by design.
- [x] Pty-driven end-to-end tests: promote, flag, kick, u-revisit,
      gate confirm, fail-fast-without-server
      (2026-07-29) — `EIMP-2` Phases E–J and its comprehensive test.
- [x] Measure and record here: line count, before vs after
      (2026-07-29) — 1080 → 352 lines (a *new* script, not a reduction of
      the old one). Per-test spawn/verification counts were not measured;
      `VerifiedCache`'s single-flight hook exists for it if wanted later.
- [ ] Decide the fate of `scripts/experimental_reviewer.sh`: keep as
      fallback/reference, or delete now that the new client covers the loop.
      `EIMP-4` §S.1 already excludes it from both published crates either
      way — this is a repo-hygiene decision, not a packaging one
- [ ] Client side of the TUI-owned private server (`EIMP-1.md` §S.7a):
      launch the server on a private socket in the hardened scratch dir,
      wait for it to be reachable, hold the session for the pass, and tear
      it down on exit via the existing `trap` cleanup. Replaces today's
      fail-fast-if-no-server startup check as the *default* path — keep an
      opt-in flag for attaching to an already-running standalone server,
      since `EIMP-2` Phase I's cross-invocation revisit test depends on
      that mode existing
- [ ] Wire the new client to whatever Phase A/C adds that it should surface:
      `diff` hunks, `ReviewMode` selection, claims

## Phase E — dhtml frontend (EIMP-1.md §S.9)

Ships in `einmo-review-server` (`EIMP-4` §S.1).

- [ ] Single embedded page: 4-pane view, server diff hunks, verb buttons,
      notes→Flag, plan view with typed-PROMOTE gate, SSE refresh
- [ ] Browser-path integration test (HTTP+token mode) reusing Phase C
      fixtures

## Comprehensive test + completion

- [ ] Comprehensive test, per `EIMP-1.md` §Test Plan: scripted
      multi-verifier end-to-end session (two reviewers, mixed
      individual/batch signing, crash-resume via the journal, drift) over a
      fixture suite; stamp chains asserted with `einmo verify`.
- [ ] STOP — maintainer performance-verifies the review loop end to end.
      `EIMP-4` (publish) gates on this having happened, not merely on the
      tests being green
- [ ] Verify all work is committed on `main` and all tests pass
      (`cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`)
- [ ] Update `EIMP-1.md` frontmatter `status: complete`
- [ ] Update `docs/eimp/INDEX.md` to reflect EIMP-1's completed status
