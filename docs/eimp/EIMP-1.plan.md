# EIMP-1.plan — einmo-review-session
Read `docs/eimp/EIMP-1.md` before acting on any task below. Tasks run top to
bottom; each phase lands value on its own. This plan is adapted from the
original `FOOP-25.plan.md` (in `foolish-rust`), with worktree/branch
mechanics removed: einmo is a small, single-maintainer repository, so this
plan executes directly on `jia` with regular commits (`EIMP-0` §8).

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

- [x] Read §S.11 of `EIMP-1.md`; add `fips205` dep (feature
      `slh_dsa_sha2_256s` — conservative set) to `Cargo.toml`
      (2026-07-30 22:51) — done in prep commit `16ef59f` before this session:
      `fips205 = { version = "0.4", default-features = false, features =
      ["slh_dsa_sha2_256s"] }`. Confirmed by reading the crate source
      (`fips205-0.4.1/src/{lib,traits}.rs`) rather than only its docs:
      `N = 32`, `PK_LEN = 64`, `SIG_LEN = 29792`;
      `KeyGen::keygen_with_seeds::<32>` is a deterministic default trait
      method (drives a `DummyRng` replaying three seeds, no OS entropy); with
      `hedged: false`, `Signer::try_sign_with_rng` never touches its `rng`
      argument — signing is fully deterministic given the same key+message.
- [x] Write tests FIRST (§Test Plan "CorpusSigner" + section attestation):
      deterministic manifest; digest changes on add/remove/alter/reorder;
      SLH-DSA sign→verify round-trip; tamper fails; same-passphrase
      dual-derivation determinism; empty-section manifest — all exercising
      `CorpusSigner` standalone (no `EinmoReview`)
      (2026-07-30 23:54) — 18 tests in `src/corpus_signer.rs`, written before
      the implementation they exercise (per project rules), covering exactly
      this list plus a few more: digest order-sensitivity (`a/b` vs `a-b`
      reordering under `PathBytes`), `.section.sig` excluded from its own
      manifest, unrecognized collation in a signature file fails as *that*,
      and tampered signature bytes fail verification rather than panicking.
- [x] Implement `EIMP-1.md` §S.11a's `Collation` — the configurable
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
      (2026-07-30 23:20) — new `src/collation.rs`: `Collation` (`#[non_exhaustive]`,
      one variant `PathBytes`) compares `EinmoId` by
      `as_str().split('/')` component vectors (never the whole string), so a
      shorter-components prefix sorts first and a path separator can never
      be confused with an in-name byte. Ties are a hard error (§S.11a item
      5). Wired to `einmo.toml`'s `[signing] collation` via a new
      `SigningConfig::collation` field and `TestConfig::collation()`
      accessor (defaults to `Collation::DEFAULT` when unset); an unknown
      identifier returns the new `EinmoError::CorpusSignature` variant,
      textually distinct from `EinmoError::Verification`. 12 tests in
      `collation.rs` + 3 in `config.rs` (default resolution, `einmo.toml`
      round-trip, unknown-identifier error).
- [x] Implement `CorpusSigner` skeleton (`new`/`manifest`/`digest`/`sign`/`verify`)
      + the manifest builder (stage name + param-set id + collation id +
      the collation-ordered mirror-path list)
      (2026-07-30 23:54) — new `src/corpus_signer.rs`. Manifest header is
      canonically length-prefixed (little-endian `u32` per field, including
      each path component) rather than separator-joined, so no character an
      `EinmoId` might contain could ever make two distinct manifests collide
      onto the same header bytes. Also added `CorpusSigner::for_suite(&TestConfig)`,
      a convenience constructor resolving `suite_root` + `[signing]
      collation` the same way every other signing setting resolves — not a
      promotion-flow wiring, just construction convenience (§S.11's
      "NOT wired into the live promotion flow yet" boundary is unchanged:
      nothing calls `sign`/`verify` from `EinmoReview` or the CLI).
- [x] Implement the sequential streaming read: files in manifest order,
      hasher fed incrementally, bounded memory, no whole-section buffer.
      This digest is the correctness reference and the performance baseline
      `EIMP-5` is measured against — write it to be the reference, not a
      placeholder
      (2026-07-30 23:54) — `digest_for` streams each file through a 64 KiB
      buffer into one running `Sha256`, comparing the byte count actually
      read against a `stat`-recorded length; a mismatch is
      `EinmoError::CorpusSignature`, never a silently truncated digest. The
      length-comparison itself is a separately unit-tested pure function
      (`check_read_len`) since a genuine read-time race isn't
      deterministically simulable in a unit test.
- [x] Extend `Signer` (§S.4) to derive BOTH the Ed25519 stamp key and the
      section SLH-DSA key from one passphrase (Argon2id output expanded to
      the SLH-DSA seed; deterministic keygen)
      (2026-07-30 23:54) — **scoping note**: no `review::Signer`/`SignerSet`
      change was needed. `CorpusSigner::sign` takes `&KeySource` (the
      existing `config.rs` type — `key.passphrase()` is the idiom every
      other signing call site already uses), not the review-level `Signer`.
      `signature.rs`'s `derive_keypair` was generalized into a new
      `pub(crate) fn derive_seed(passphrase, salt) -> [u8; 32]` (same pinned
      Argon2id instance, salt now a parameter); `derive_keypair` becomes a
      thin call to it with the existing `SALT`, so all existing Ed25519
      call sites are byte-for-byte unaffected. `corpus_signer::derive_slh_keypair`
      derives its own master seed via `derive_seed(passphrase,
      CORPUS_SIGNER_SALT)` (`b"einmo:corpus-signer-key:v1"`, domain-separated
      from the Ed25519 `SALT`), then SHA-256-expands it three ways
      (`sk_seed`/`sk_prf`/`pk_seed`) into `keygen_with_seeds`'s inputs — one
      Argon2id run, not three. Same passphrase ⇒ same three seeds ⇒ same
      SLH-DSA keypair, every time (tested), and the two key systems are
      confirmed domain-separated (tested).
- [x] Implement `sign`/`verify` over the digest; `.section.sig` file shape
      defined but written only to fixtures/tempdirs in tests, never the
      real corpus
      (2026-07-30 23:54) — `.section.sig` is a single JSON line (stage,
      param-set id, collation id, hex pubkey, base64 signature) written
      dot-named into the stage directory, so it is invisible to
      `walk_input_tree` — including this module's own manifest builder,
      pinned by a test. `verify` deliberately does NOT trust its own
      signer's configured collation: it re-derives the collation from the
      signature file's own recorded identifier first (failing as an
      unrecognized-collation error if unknown), THEN rebuilds the manifest
      under that collation — so a verifier always honors what a signature
      was actually produced under, per §S.11a. Every test (`sign`→`verify`
      round trip, tamper-after-signing, added-file-after-signing,
      empty-section, unrecognized-collation, tampered-signature-bytes) runs
      over `tempfile::tempdir()` fixtures only; nothing in this crate calls
      `sign`/`verify` against the real corpus.
- [x] Confirm core `einmo` gained no async runtime and no new heavy
      dependency beyond `fips205` — the constraint `EIMP-4`'s split depends
      on
      (2026-07-30 23:54) — confirmed via `cargo tree`: `fips205` pulls in
      only pure-Rust, `no_std`-internal transitive deps (no `tokio`/`hyper`/
      `axum`/`tower`). Core `einmo` *already* depends on `axum`/`tokio` from
      `EIMP-2`'s server prototype (that's `EIMP-4`'s split to fix, not this
      phase's concern) — the check here is narrower and holds: Phase A2's
      own new code (`collation.rs`, `corpus_signer.rs`) is fully
      synchronous, adds no async dependency of its own, and calls nothing
      from the `axum`/`tokio` dependency subtree.
- [x] Phase A2 tests green; `cargo fmt` / `cargo clippy -D warnings` clean;
      `#![forbid(unsafe_code)]` still holds (fips205 is pure Rust)
      (2026-07-30 23:54) — 276 einmo lib tests (from 244 at Phase A2's
      start) + 4 zweimomo unit + 3 zweimomo integration, all green.
      `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt
      --check` both clean. **Correction to this checkbox's own premise**: no
      crate-wide `#![forbid(unsafe_code)]` attribute actually exists in this
      repo today (checked directly — `lib.rs` has none). What holds: zero
      `unsafe` blocks in this phase's new production code
      (`collation.rs`/`corpus_signer.rs`); the crate's only existing
      `unsafe` usage is `std::env::set_var` in test-only code (Rust 2024
      requires `unsafe` for it), unrelated to this phase; and `fips205`
      itself is `#![no_std]` + `#![deny(unsafe_code)]` internally. **Phase
      A2 is complete.**

## Phase B — review CLI verbs (in the `einmo-review-server` crate)

Per `EIMP-4` §S.1 these operate on `EinmoReview` and therefore belong to
`einmo-review-server`'s binary, not core `einmo`'s `cli.rs`. Until `EIMP-4`
performs the split they land in this repo's existing binary; the split then
moves them wholesale.

- [x] `einmo review plan|list|decide|undecide|execute` one-shot subcommands
      (journal-backed session identity) with endpoint-equivalent semantics;
      unit tests
      (2026-07-31 00:13) — landed in `src/bin/einmo_review_server.rs` (per
      `EIMP-4` §S.1's crate-boundary note, restated at the top of this
      phase: these verbs belong to `einmo-review-server`, not core's
      `cli.rs`, and this repo has no split-off crate yet so they land in
      the existing binary). Restructured the binary from a plain
      `#[tokio::main] async fn main()` into a top-level clap `Parser` with a
      `Command` enum: `Serve(ServeArgs)` (today's exact bind/session-file/
      shutdown behavior, byte-for-byte unchanged, just moved under a
      subcommand and driven by a manually-built
      `tokio::runtime::Builder::new_multi_thread().enable_all()` runtime
      instead of the macro — the only subcommand that touches async at
      all) plus `Plan`/`List`/`Decide`/`Undecide`/`Execute`, all synchronous
      over `EinmoReview`'s ordinary API. `main()`/`cli_main`/`dispatch`
      mirror `src/cli.rs`'s own shape (`ExitCode`-returning, no
      `process::exit`, clap error handling identical to `cli_main`'s).

      **Session identity**: every one-shot subcommand takes `--session
      <id>` (optional); given, it calls `EinmoReview::resume(work_dir, id,
      opts)` (replays the journal, EIMP-1 §S.6); omitted, it
      `open`/`open_with`s a fresh session. Every subcommand announces (to
      stderr, so `--json` stdout stays one clean payload) the session id it
      operated under, so `decide` (no `--session`) mints one and a later
      `execute --session <that-id>` picks the decision back up — proven by
      `decide_then_undecide_across_separate_calls_survives_journal_replay`
      and `list_reflects_a_pending_decision_across_a_resumed_session`,
      which chain two/three separate `cmd_*` calls (each opening and
      dropping its own `EinmoReview`, exactly like separate process
      invocations) purely through a shared `--session` string.

      **`decide`'s decision grammar** mirrors `split_promote_args`'s spoken
      `to`/`from` convention rather than inventing something unrelated:
      `promote to <stage>` (or bare `promote <stage>`), `retract from
      <stage>` (or bare), `flag <stage> <reason…>`, `skip` — parsed by a
      small pure `parse_decision`/`parse_decidable_stage` pair (checked/
      verified only, matching `review_server::DecidableStage`), directly
      unit-tested without a suite fixture.

      **`execute`'s confirm gate and `SignerSet`** mirror
      `review_server::post_execute` exactly: any pending `Promote` action
      requires the literal `--confirm PROMOTE` (`confirm_gate`, a pure
      function pinned by 3 tests, same "kept separate from I/O for direct
      testability" precedent as `cli.rs`'s `flags_fail_the_gate`);
      `to_checked` is unconditionally `KeySource::from_passphrase("")`
      (never goes through the cascade at all, matching the server's
      hard-coded computer key); `to_verified` is resolved only when
      `plan_needs_verified_key` (pure, 3 tests) finds a pending `Promote {
      to: Verified }`, via `src/cli.rs`'s own established key-cascade
      (`KeyCascadeInputs`, `resolve_stage_key`, `--passphrase`/
      `--stdin-passphrase`/`--interactive`/`EINMO_PASSPHRASE`/
      `einmo.toml`/interactive-prompt) — a deliberate, documented widening
      from the HTTP endpoint's bare `Option<String>` passphrase field: a
      CLI naturally has an interactive-prompt tier an HTTP request body
      cannot offer, and the *shape* of the gate and the `SignerSet` (the
      part `EIMP-1` §S.7 actually specifies) matches exactly; only the
      *mechanism* resolving `to_verified`'s value is richer.

      **Discrepancies found and resolved, documented per this plan's own
      convention**:
      - `ExecutionPlan` has no `Display`/render method to reuse (`EIMP-1`
        §S.7's "the plan's rendered text if one exists already" — it does
        not). Added presentation-only rendering (`action_line`,
        `decision_tag`, `action_json`) local to the bin, mirroring how
        `review_server.rs` already has its own private `decision_tag`/
        `PlannedActionResponse` — presentation lives at the frontend, not
        the core session object.
      - `KeyCascadeInputs` was not re-exported from `lib.rs` (only
        `resolve_stage_key`/`KeySource` were); added it to the curated
        `pub use config::{…}` list — the minimum surface a second CLI
        binary needs to reuse the established cascade rather than
        reinventing passphrase resolution.
      - `EinmoError::io`/`review_server::decision_tag`/`cli.rs`'s
        `read_stdin_line`/`prompt_tty` are `pub(crate)`/private to core's
        own modules and therefore invisible across the crate boundary a
        `src/bin/*.rs` file sits on (a `src/bin` file is a separate crate
        linking the package's lib as `extern crate einmo`, even though
        both live in one Cargo package) — small, unavoidable local
        duplicates were written instead (`io_err` via `EinmoError::Io`'s
        public struct-variant fields, `read_stdin_line`, `prompt_tty`,
        `decision_tag`), each documented at its definition as a deliberate
        cross-boundary duplication rather than an oversight.
      - `--json` output uses `serde_json::json!`/`Value`'s `Display`
        (proper escaping) rather than `cli.rs`'s hand-rolled
        `format!("{{\"x\":\"{}\"}}", …)` strings: `cli.rs`'s JSON values are
        always path-safe substrings, but `decide`'s `flag` reason is
        free-text a reviewer types, which can contain quotes — correctness
        (rust_instructions.md §1a's optimization order) wins over
        cosmetic-only convention match here.
      - `Serve`'s move under a subcommand changes its invocation syntax
        (`einmo-review-server <suite>` becomes `einmo-review-server serve
        <suite>`) — `ServeArgs`' own flags/behavior are byte-for-byte
        unchanged, only the subcommand wrapper is new. This repo has no
        automated test or script that invokes the bare old form
        programmatically (checked: no `tests/` integration test, no
        `Command::new` spawn of this binary anywhere; only
        `scripts/einmo_review_client.sh`'s help text mentions the old
        invocation prosaically) — left as a Phase C/D follow-up
        (`scripts/einmo_review_client.sh`'s two help-text lines) rather
        than touched here, per this phase's own scope boundary.

      25 new tests in `src/bin/einmo_review_server.rs` (clap-parsing smoke
      tests; `parse_decision`/`parse_decidable_stage`/`list_opts`/
      `confirm_gate`/`plan_needs_verified_key` as pure, directly-tested
      functions; functional tests over a seeded tempdir suite exercising
      the full decide→execute chain, the confirm-gate refusal, and a
      `checked`-then-`verified` promotion using an explicit `--passphrase`
      so no test ever reaches the interactive-prompt tier). einmo lib stays
      at 276 tests (only `lib.rs`'s export list changed); workspace total
      276 (lib) + 25 (this bin) + 4 (zweimomo unit) + 3 (zweimomo
      integration) = 308, all green. `cargo clippy --workspace --all-targets
      -- -D warnings` and `cargo fmt --check` both clean.
- [x] Byte-for-byte equivalence test: `review execute` promotion == existing
      `einmo promote`
      (note: `EIMP-2` Phase C already proved `EinmoReview::execute` ==
      `transitions::promote` at the library level — this task is the *CLI*
      surface's equivalent)
      (2026-07-31 00:13) —
      `cli_execute_promote_matches_einmo_promote_cli_at_the_cli_surface` in
      `src/bin/einmo_review_server.rs`: the baseline runs the actual `einmo`
      binary's CLI path via the exported `einmo::cli_main` entry point
      (`einmo promote output to checked <suite>`, not a re-implementation);
      the review path chains `cmd_decide`+`cmd_execute` across a shared
      `--session` id, exactly like two separate process invocations. Both
      run over independently-seeded, identical fixtures. Compared via
      `EinmoFile::sections()` filtered to exclude `STAMPS` (a local
      `body_sections_excluding_stamps`, since the library's own
      `body_sections` helper is `pub(crate)` and invisible across the bin's
      crate boundary) rather than raw bytes — timestamps legitimately
      differ between two independent runs (each stamp signs its own
      generation time), the same reasoning `review.rs`'s own
      `execute_promote_matches_cli_promote_byte_for_byte` (the Phase A,
      library-level equivalence test this one complements) already
      documented. Passes.

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
- [x] **TUI-owned private server mode (`EIMP-1.md` §S.7a)** — the default
      launch shape: the client script starts its own server on a private,
      unpredictable socket inside its mode-700 scratch dir, drives it, and
      terminates it on exit (including `Ctrl-C`/abnormal exit, via the
      script's existing `trap` cleanup plus the server's `ctrl_c` handler).
      The standalone long-lived server stays available — this is an
      additional default mode, not a replacement
      (2026-07-31 00:41) — the SERVER-side capability is complete; the
      CLIENT-side driving script is explicitly Phase D's job (unchanged,
      not touched here). See the three sub-items below for what shipped.
  - [x] Upgrade `axum` 0.7.9 → 0.8.x and **delete** the hand-rolled UDS
        accept loop (`hyper` HTTP/1.1 builder + `hyper-util` `TokioIo`/
        `TowerToHyperService` + manual `UnixListener` loop) that `EIMP-2`
        Phase D only needed because 0.7's `serve()` is TCP-only. Verify the
        `Listener` impl for `tokio::net::UnixListener` is present in the
        enabled feature set before deleting the glue, and re-check whether
        `hyper`/`hyper-util` remain needed at all afterward
        (2026-07-31 00:41) — `Cargo.toml`: `axum = "0.8"` (resolved to
        0.8.9). Verified `#[cfg(unix)] impl Listener for
        tokio::net::UnixListener` exists unconditionally (not feature-gated)
        by reading `axum-0.8.9/src/serve/listener.rs` directly before
        deleting anything — matches §S.7a's own instruction to verify
        against the installed crate's source, not assume. `serve_uds`
        (`src/review_server.rs`) is now `axum::serve(listener,
        app).with_graceful_shutdown(shutdown)`; `run_accept_loop` (the
        hand-rolled `hyper`/`hyper-util` glue) is deleted entirely. Route
        syntax updated `:param` → `{param}` (matchit 0.8's breaking change,
        confirmed via axum's own CHANGELOG.md before editing). Also added
        `serve_tcp`/`router_tcp` (see the TCP checkbox below) using the same
        `axum::serve` shape, since one `Listener`-generic `serve()` now
        covers both transports.

        **`hyper`/`hyper-util`/`tower`/`hyperlocal` re-check**: grepped
        every use site first, per this checkbox's own instruction. Nothing
        in production code (`src/**/*.rs` outside `#[cfg(test)]`) uses any
        of the four anymore — the only remaining call sites are
        `review_server.rs`'s own tests, which drive a real UDS/TCP client
        against a real bound socket (`hyper_util::client::legacy::Client`
        + `hyperlocal::Uri`) to prove `serve_uds`/`serve_tcp` end-to-end,
        and `tower::ServiceExt::oneshot` for the rest of the test suite's
        in-process request harness. All four moved from `[dependencies]` to
        `[dev-dependencies]` in `Cargo.toml` accordingly — axum itself still
        pulls its own private copies transitively for production use, this
        crate simply no longer depends on them directly outside tests.
  - [x] Keep `EIMP-2` Phase D's stale-vs-live socket probe
        (`UnixStream::connect` succeeds → refuse; fails → remove and
        rebind). §S.7a's illustrative snippet unlinks unconditionally,
        which would let a new server stomp a live one's socket
        (2026-07-31 00:41) — untouched by the axum rewrite: `serve_uds`
        still probes with `tokio::net::UnixStream::connect` before ever
        calling `UnixListener::bind`, exactly as before. Pinned by the three
        pre-existing tests (`serve_uds_end_to_end_and_cleans_up_on_shutdown`,
        `serve_uds_refuses_a_live_socket`, `serve_uds_rebinds_a_stale_socket_file`),
        all still green after the rewrite — the axum 0.8 migration changed
        `serve_uds`'s internals, never its externally-observed probe
        behavior.
  - [x] Socket path is per-session and unpredictable inside the hardened
        scratch dir — never a fixed world-known `/tmp` path
        (2026-07-31 00:41) — new `review_server::private_socket_path()`
        (`src/review_server.rs`, re-exported from `lib.rs`): mints a random
        128-bit directory name (`rand_core::OsRng`, the same primitive
        `review.rs`'s own `random_session_id` already uses) under
        `$EINMO_REVIEW_PRIVATE_DIR` (or a fixed temp-dir subdirectory),
        hardens it to mode 0700 via `journal.rs`'s `harden_dir` (promoted
        `pub(crate)` and reused rather than duplicated — DRY per
        `rust_instructions.md` §2b), and returns
        `<random-dir>/review.sock` inside it — the directory's randomness
        plus its 0700 permissions are the whole access control, mirroring
        `einmo_review_client.sh`'s own `mktemp -d`-then-`harden_dir`
        pattern (an unguessable directory, a predictably-named file inside
        it) exactly as this checkbox asked.

        **Design decision on shape**: this function ONLY mints and reserves
        the path; it does not itself start a server. Wired into
        `einmo-review-server serve` as a new `--private` flag
        (`src/bin/einmo_review_server.rs`): when set, `run_serve` calls
        `private_socket_path()` instead of using `--socket`, then binds it
        with the ordinary `serve_uds` exactly like the standalone path —
        the capability and its consumption are the same code path, only
        the socket's origin differs. `--private` mode prints the resulting
        socket path as the FIRST (and only) line of stdout — everything
        else already went to stderr — so a driving script can do `SOCKET=$(
        einmo-review-server serve --private <suite>)`. This is deliberately
        the server-side capability only; the bash script that calls it with
        its own `trap`-based lifecycle is Phase D's work, per this task's
        own scope boundary.

        6 new tests: 2 for `private_socket_path` itself (uniqueness +
        mode-0700 hardening; end-to-end bind-over-it via `serve_uds`), plus
        a clap-parsing test and a `run_serve` suite-lock-interaction test in
        the bin (see the suite-lockfile checkbox below).
- [x] `einmo review serve` as a *subcommand* spelling (today it is a
      standalone `einmo-review-server` binary); decide whether both spellings
      survive the `EIMP-4` split or the subcommand form is dropped —
      record the decision rather than doing it by default
      (2026-07-31 00:41) — **decision, not implemented (per this checkbox's
      own instruction)**: do NOT add a new `einmo review serve` spelling
      now. Reasoning, read against `EIMP-1.md` §S.1 and this plan's own
      repeated framing:

      1. **Naming-surface decisions belong to `EIMP-4`, not this EIMP.**
         §S.1 is explicit that `EIMP-4` "splits the repository into
         published `einmo` (core) and published `einmo-review-server`" and
         that "Phase B's `einmo review …` verbs belong to
         `einmo-review-server`" as THAT crate's binary's subcommands — not
         core `einmo`'s `cli.rs`. Phase B already followed this: the
         one-shot verbs (`plan`/`list`/`decide`/`undecide`/`execute`) landed
         in `src/bin/einmo_review_server.rs`, not `src/cli.rs`, specifically
         because this repo has no split-off crate yet and "until `EIMP-4`
         performs the split they land in this repo's existing binary; the
         split then moves them wholesale." The exact same reasoning applies
         to `serve`: it already lives on the `einmo-review-server` binary
         (`einmo-review-server serve <suite>`, moved under a `Serve`
         subcommand in Phase B). Inventing a SECOND spelling
         (`einmo review serve`) on core's `einmo` binary today would mean
         building a naming surface `EIMP-4` might delete or reshape anyway
         once the crates are physically separate — exactly the
         "make anything that would make the split harder" risk §S.1 warns
         against, restated for a binary name instead of a dependency.
      2. **There is no `einmo review` subcommand namespace on core's `einmo`
         binary to attach `serve` to.** `src/cli.rs` has no `review`
         subcommand at all (checked directly) — building one now, for one
         verb, ahead of `EIMP-4` deciding the actual crate/binary shape,
         would be speculative surface with no consumer.
      3. **Both spellings surviving is plausible but not this EIMP's call.**
         A bare `einmo-review-server` binary (direct, scriptable, matches
         every other one-shot verb already living there) and a hypothetical
         `einmo review serve` (discoverable under the more commonly-typed
         `einmo` binary) are not mutually exclusive — `EIMP-4` could ship
         both, the way `cargo-einmo` already aliases `einmo`. But deciding
         THAT is squarely `EIMP-4` §S.1 territory (it owns the split and
         therefore the binary-naming consequences of the split), not
         something to pre-empt here by writing code for a binary that
         doesn't exist yet.

      **Net**: `einmo-review-server serve <suite>` (and its `cargo
      einmo-review-server` alias) remains the only spelling. `EIMP-4`
      inherits this exact recorded reasoning when it performs the split and
      decides binary/subcommand naming for real.
- [x] TCP 127.0.0.1 + bearer token behind a flag (UDS remains the default).
      Note `EIMP-2`'s standing caveat: the `checked to verified` passphrase
      travels plaintext in the execute body, which is materially riskier
      over TCP than over a UDS — resolve that *before* enabling TCP, not
      after
      (2026-07-31 00:41) — **resolution chosen**: (a) TCP+bearer-token as an
      explicit opt-in (`einmo-review-server serve --tcp <addr> --token
      <token>`; UDS always binds regardless — TCP is additive, never a
      replacement), PLUS (b) a hard, non-silent refusal of any `execute`
      request over that TCP listener whose body carries a non-null
      `passphrase`, UNLESS the caller also passes the separate, explicit
      `--allow-insecure-tcp-verified-passphrase` flag. This is option
      (a)+(b) from this task's own menu of reasonable resolutions, not the
      "document loudly and defer to `EIMP-6`" alternative — because (b) was
      concretely implementable in-scope (a body-buffering `axum` middleware
      the size of the existing `ApiError` mapping) and a hard-refusal
      default is strictly safer than a documentation-only mitigation for a
      risk this exact EIMP is the one introducing (TCP support did not
      exist before this checkbox). TLS/mTLS remain explicitly out of
      scope — the opt-in flag is a deliberate, informed-consent escape
      hatch for a trusted network path, not a substitute for real
      transport security, and is named and documented as exactly that.

      **What shipped** (`src/review_server.rs`): `serve_tcp` (the TCP
      analogue of `serve_uds`, loopback-only — refuses to bind any
      non-127.0.0.1/::1 address, a hard invariant checked in code, not just
      documented); `router_tcp` (wraps `router` in a bearer-token +
      passphrase-guard `axum::middleware::from_fn_with_state` layer, called
      `tcp_guard`); the guard rejects any request missing/mismatching
      `Authorization: Bearer <token>` (401) before it rejects a
      passphrase-carrying `execute` body (403) — auth and the
      transport-confidentiality risk are checked independently, since a
      valid token does not make plaintext-over-TCP any less plaintext. UDS
      (`router`) carries NONE of this layer — a local socket's directory
      permissions remain the whole access control there, unchanged.
      `src/bin/einmo_review_server.rs`'s `ServeArgs` gained `--tcp
      <SocketAddr>` (clap `requires = "token"` — TCP can never bind without
      a token, not even by omission), `--token <String>`, and
      `--allow-insecure-tcp-verified-passphrase`; `run_serve` binds UDS
      always and additionally spawns TCP when `--tcp`/`--token` are given,
      sharing one `AppState` (one review, reachable both ways at once — the
      spec's own "multiple verifiers, concurrently" / "browser review" use
      cases), torn down together via one broadcast shutdown channel feeding
      both listeners plus the `Ctrl-C` handler.

      11 new tests: bearer-token accept/reject (3), passphrase-refused vs.
      passphrase-less-allowed vs. explicit-opt-in-allowed over TCP (3),
      non-loopback-bind-refused (1), a real end-to-end TCP loopback test
      proving the bearer gate over an actual bound socket (1, mirrors the
      existing UDS end-to-end test's harness shape), plus clap-parsing
      tests for the three new flags including `--tcp` without `--token`
      being rejected at parse time (3).
- [x] Suite lockfile: a second server on the same suite refuses to start
      (2026-07-31 00:41) — new module `src/suite_lock.rs`
      (`SuiteLock`/`suite_lock_path`, re-exported from `lib.rs`). Path
      precedent: follows `journal.rs`'s §S.6 reasoning exactly (ephemeral
      session/process state, not part of the reviewed corpus, so it must
      NOT live inside the suite or show up in the suite's own `git
      status`) rather than `EIMP-1.md`'s own literal "inside the suite's
      own scratch/state area" phrasing, which this task's instructions
      flagged as worth checking against precedent — `journal_dir()`'s
      scratch base already is that precedent, so the lock lives there too:
      `journal_dir().join("suite-<hex>.lock")`, where `<hex>` is a
      truncated SHA-256 of the suite's CANONICALIZED path (so two different
      relative/symlinked spellings of the same suite collide on the same
      lock, as they must; not a security boundary, just a stable filesystem
      -safe name).

      **Same stale-vs-live discipline as the socket probe**, reusing it
      directly rather than re-inventing: the lock file's *content* is the
      new server's own socket path, so `SuiteLock::acquire` probes that
      recorded path with `std::os::unix::net::UnixStream::connect` — the
      exact test already used to distinguish a live server from a crashed
      one's leftover file. Connect succeeds → refuse (`EinmoError::Io` with
      `ErrorKind::AddrInUse`); connect fails/empty/unreadable → the lock is
      stale, reclaimed silently. Released on `Drop` (a clean shutdown
      leaves no trace), applies to BOTH standalone and (once minted, see
      above) TUI-private sockets, since `run_serve`
      (`src/bin/einmo_review_server.rs`) acquires it unconditionally,
      before either socket path variant is bound — "a second server of ANY
      kind" per this checkbox's own text.

      11 new tests: 6 in `suite_lock.rs` (path stability/uniqueness,
      acquire-then-drop releases, refuses-while-live, reclaims-stale,
      errors-on-nonexistent-suite) + 1 functional test in the bin
      (`run_serve_refuses_when_suite_lock_is_held`, asserting the
      observable consequence — no socket ever bound — rather than the
      `ExitCode` value, which has no public equality check on stable Rust
      per `cli.rs`'s own established note).
- [x] The remaining §S.7 endpoints: `diff` (needs Phase A's `diff`),
      claims, If-Match/409 (needs a `version` on items — see the Phase 0
      finding that no cached worklist or version exists), and SSE events
      (2026-07-31 00:41) — checked each sub-item against the actual code
      before touching anything, per this task's own instruction:
  - `diff`: **already shipped** by Phase A
    (`GET /einmo/<session>/cases/<id>/diff/<left>/<right>`, `case_diff` in
    `src/review_server.rs`, landed 2026-07-30 17:47 per Phase A's own
    checkbox above) — confirmed by reading the router and its 5 existing
    tests; not re-implemented.
  - claims: `EinmoReview::claim`/`claim_for` existed (Phase A, 2026-07-30
    19:09) but had NO HTTP endpoint — genuinely missing, now implemented:
    `POST /einmo/<session>/cases/<id>/claim` (`claim_case`), body
    `{ttl_secs?}` (0/absent = the library's own 5-minute default). 2 new
    tests.
  - If-Match/409: **still blocked, explained rather than force-built** —
    re-confirmed the Phase 0 finding this checkbox itself cites still
    holds (`grep -n "version" src/review.rs` finds nothing but this
    checkbox's own citation): `EinmoReview` keeps no cached worklist
    (`items()` rescans disk every call) and no `version` field anywhere.
    `EinmoReview`'s actual staleness story is content-fingerprint drift
    checking (`decide`/`refresh`/`execute`, the Phase A `refresh()`
    checkbox above), a deliberately different mechanism than optimistic
    -concurrency versioning. Retrofitting a `version` counter solely to
    satisfy this one HTTP row would invent a staleness signal the object
    was not designed to carry, as a side effect of one endpoint — exactly
    what this checkbox's own text warned against forcing. Documented in
    place as a doc comment on `put_decision` in `src/review_server.rs`
    rather than silently dropped; left for whichever future work actually
    needs HTTP-layer optimistic concurrency to design `version` properly.
  - SSE events: **genuinely missing, now implemented** —
    `GET /einmo/<session>/events` (`session_events`), backed by a
    per-session `tokio::sync::broadcast` channel added to `AppState`
    (`SessionEntry` groups a session's `Arc<EinmoReview>` with its event
    sender so the two can never desync). Every mutating handler
    (`put_decision`/`delete_decision`/`flag_case`/`retract_case`/
    `post_execute`/`claim_case`) publishes a `ReviewEvent`
    (`DecisionMade`/`ItemChanged`/`Executed`, matching this row's own
    "decision-made / item-changed / executed") AFTER its mutation already
    succeeded — a failed request never announces an event that didn't
    happen. Streamed via `axum::response::sse::Sse` over
    `tokio_stream::wrappers::BroadcastStream` (new direct dependencies
    `futures-util`/`tokio-stream`, both already unconditional transitive
    dependencies of axum's own `response::sse` module — its own doc
    example uses exactly this pair). 2 new tests, including a real
    streaming-body test (subscribe, then decide, then read the pushed SSE
    frame off `Body::into_data_stream()` with a timeout) rather than only
    asserting a 200 status.
- [x] Concurrency tests: N verifiers, single-flight verify counts, no lost
      updates, claims expire
      (2026-07-31 00:41) — 3 new tests in `src/review_server.rs`, all
      exercising the HTTP layer concurrently per this checkbox's own
      framing (not new library infrastructure — reusing Phase A's existing
      single-flight cache and claim-expiry mechanisms, driven through real
      concurrent HTTP requests):
  - `concurrent_body_requests_single_flight_verify_exactly_once`: 16
    concurrent `GET .../body/output` requests for the SAME artifact, via
    `tokio::spawn` + the shared `Router`'s `Clone`, then asserts the
    underlying `VerifiedCache`'s verify counter is exactly 1 — reached via
    a new `#[cfg(test)] pub(crate) EinmoReview::cache_verify_count()` hook
    (`src/review.rs`) exposed specifically because `review_server.rs` is a
    sibling module and the field is private.
  - `concurrent_decisions_on_distinct_cases_lose_none`: 20 verifiers
    deciding 20 DIFFERENT cases concurrently (simulating "N verifiers"
    against one session, since today's design is single-implicit-reviewer
    per the Phase 0 drift finding — there is no `ReviewerId` to
    parameterize by, so concurrency is exercised across cases instead of
    across reviewer identities); asserts `plan()` afterward reflects all
    20, none lost to a race on the shared `DecisionBook`.
  - `claim_via_http_expires_and_is_auto_reclaimed`: a short-TTL claim
    (`claim_for`, 20ms) is active immediately, then gone from
    `plan().claims` after it expires — the same auto-reclaim behavior
    `review.rs`'s own library-level claim tests already prove, reached
    through the HTTP-adjacent path this checkbox specifically asked for.

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
- [x] Decide the fate of `scripts/experimental_reviewer.sh`: keep as
      fallback/reference, or delete now that the new client covers the loop.
      `EIMP-4` §S.1 already excludes it from both published crates either
      way — this is a repo-hygiene decision, not a packaging one
      (2026-07-31) — **keep for now.** Deprecation is premature until the
      review server's functionality is proven and the old script's
      capabilities have been fully replicated. Revisit when `EIMP-4`
      performs the crate split.
- [x] Client side of the TUI-owned private server (`EIMP-1.md` §S.7a):
      launch the server on a private socket in the hardened scratch dir,
      wait for it to be reachable, hold the session for the pass, and tear
      it down on exit via the existing `trap` cleanup. Replaces today's
      fail-fast-if-no-server startup check as the *default* path — keep an
      opt-in flag for attaching to an already-running standalone server,
      since `EIMP-2` Phase I's cross-invocation revisit test depends on
      that mode existing
      (2026-07-31) — `einmo_review_client.sh` now accepts `-p SUITE` to
      launch a private `einmo-review-server serve --private` in the
      background, captures the socket path from stdout, probes with curl
      before proceeding, and kills the server + removes the socket on exit
      via `trap`. `-s SOCKET` remains for attaching to a standalone server
      (mutually exclusive with `-p`). Script grew from 352 to 460 lines.
- [x] Wire the new client to whatever Phase A/C adds that it should surface:
      `diff` hunks, `ReviewMode` selection, claims
      (2026-07-31) — `\d` in vim fetches server-side diff hunks
      (`GET .../diff/output/checked`) via a helper script, renders them in
      a split. `-n` flag passes `?differing=true` to the cases endpoint
      (NewOrBroken mode). Claims are shown in the plan display before
      execute: `CLAIMED: {id} ({remaining}s remaining)`.

## Phase E — dhtml frontend (EIMP-1.md §S.9)

Ships in `einmo-review-server` (`EIMP-4` §S.1).

- [x] Single embedded page: 4-pane view, server diff hunks, verb buttons,
      notes→Flag, plan view with typed-PROMOTE gate, SSE refresh
      (2026-07-31) — `src/dhtml/review.html` (self-contained, ~400 lines):
      dark-themed 4-pane layout (input/output/checked/verified), verb
      buttons (c=checked, v=verified, f=flag, k=kick, u=undo, claim),
      diff toggle (fetches `GET .../diff/output/checked`), plan overlay
      with PROMOTE gate and passphrase input, SSE-driven live refresh
      (decision-made/item-changed/executed events auto-refresh the
      worklist). Served via `include_str!` at `GET /` and `GET
      /review/{session}` by `review_server.rs`.
- [ ] Browser-path integration test (HTTP+token mode) reusing Phase C
      fixtures

## Comprehensive test + completion

**Test Plan items intentionally deferred (documented, not forgotten):**
- **Per-reviewer isolation** and **version bump / If-Match / 409**: the
  current design is single-implicit-reviewer with no `version` field
  (`EIMP-2` Phase C, re-confirmed Phase 0 drift finding). These tests
  require multi-reviewer infrastructure that does not exist yet. They are
  not blocked — they are genuinely not applicable to the current design.
  When multi-reviewer support is added (future EIMP), these tests become
  normative.
- **Zeroize on drop**: `StageKeypair` seals the seed under a random
  per-process KEK (`Zeroizing<[u8; 32]>`), and `ed25519-dalek`'s
  `SigningKey` zeroizes on drop. No plaintext key material exists at rest.
  A best-effort memory inspection test is deferred — the design's
  zeroization story is structural, not behavioral.

- [x] Comprehensive test, per `EIMP-1.md` §Test Plan: scripted
      multi-verifier end-to-end session (two reviewers, mixed
      individual/batch signing, crash-resume via the journal, drift) over a
      fixture suite; stamp chains asserted with `einmo verify`.
      (2026-07-31) — `comprehensive_multi_reviewer_end_to_end` in
      `src/review.rs`: 3-case fixture suite, all promoted to checked;
      Reviewer A promotes `a.foo` to verified, Reviewer B promotes `b.foo`
      to verified (different session ids, same passphrase — proves
      multi-reviewer stamp coexistence); crash-resume reconstructs a
      pending `c.foo` decision from the journal; drift detection via
      `refresh()` after content change; fresh decide+execute for the
      drifted case; every `.einmo` in `checked/` and `verified/` passes
      `from_file` (verify-on-inspect) with valid stamp chains; journal
      replay confirms SessionOpen/Decide/ExecuteBatch events.
      311 einmo lib tests + 30 binary tests = 341 total, clippy/fmt clean.
- [x] STOP — maintainer performance-verifies the review loop end to end.
      `EIMP-4` (publish) gates on this having happened, not merely on the
      tests being green
      (2026-07-31 15:10) — performed via a fresh Claude Code session, not
      the sprint's own agent: two independent static code-review passes
      (crypto/signing core; server/CLI/client + plan-vs-code consistency)
      plus hands-on driving of `einmo_review_client.sh` against a live
      private server (promote-to-checked, promote-to-verified with
      passphrase, flag, kick, undo, the pending-plan summary, and the
      typed-`PROMOTE` gate — confirmed with `einmo verify --all`
      afterward). This *did* surface real defects — see "Maintainer-found
      defects" immediately below, now tracked as its own checkbox block
      per the maintainer's direction rather than silently folded in here.
      **`cargo test --workspace` itself deadlocked mid-run** (never
      returned) during this verification — the "STOP" is satisfied
      (verification happened), but per this repo's own rule ("never start
      substantive work while any test is broken") nothing past that first
      sub-item below should be treated as unblocked until it's fixed.

## Maintainer-found defects (jia-sprint review, 2026-07-31)

Found via the STOP-gate performance-verification above. Ordered by
priority; the plan is to fix top-down. Each item below is independently
actionable — file/line, what's wrong, why it matters — so any one can be
picked up without re-deriving the finding. `\d`/server-diff (see the
explicitly-deferred item near the bottom) is the one exception: the
maintainer asked to leave it alone for now despite the diagnosis, since it
apparently used to work and the regression (if any) isn't understood yet.

- [x] **P0 — `cargo test --workspace` deadlocks partway through; never
      completes.** Confirmed, not merely slow: the run made genuine
      progress for ~14 minutes (100%+ CPU, `corpus_signer::*` and most of
      `review::tests::*` passing, including the flagship
      `comprehensive_multi_reviewer_end_to_end`), then **all 21 OS threads
      of the test binary parked in `futex_do_wait`, with `utime` in
      `/proc/<pid>/stat` frozen bit-for-bit identical across a 20+ second
      sampling window** — a real deadlock, not contention.
      (2026-07-31 15:57) — root-caused and fixed. Bisected one test at a
      time (`cargo test --lib <exact_test_name> -- --exact
      --test-threads=1`, per-test timeout) to
      `review::tests::execute_promote_matches_cli_promote_byte_for_byte`,
      then reproduced it live under `gdb --args ... run`, interrupted with
      Ctrl-C once frozen, and got a full `thread apply all bt`: the test
      thread was blocked in `std::sync::Mutex::lock` on
      `review::tests::JOURNAL_ENV_LOCK`, a `static` mutex. Root cause:
      `TestContext` (both in `src/review.rs` and, identically
      copy-pasted, in `src/bin/einmo_review_server.rs`) stores that
      mutex's `MutexGuard` as a struct field, so acquiring one keeps the
      lock held for the *entire* `TestContext`'s lifetime, not just the
      brief window it needs to mutate `EINMO_JOURNAL_DIR`. That's correct
      and required for every ordinary test (it's what serializes
      concurrently-running tests' use of that shared env var), but
      `std::sync::Mutex` isn't reentrant, and exactly one test per file
      creates a *second*, independent `TestContext` on the same thread
      before dropping the first
      (`execute_promote_matches_cli_promote_byte_for_byte`'s `tmp`/`tmp2`,
      comparing the review path against a fresh CLI-driven baseline; the
      binary's `tests::cli_execute_promote_matches_einmo_promote_cli_at_
      the_cli_surface` has the identical shape) — the second
      `test_context()` call deadlocks waiting on a lock the very same
      thread already holds. Fix: added `TestContext::release_journal_lock`
      to both files (drops the guard and its scratch tempdir early,
      once the first context's journal-writing work is actually done),
      called from each of the two affected tests right before opening
      their second context. Confirmed via `gdb`/`strace`-style isolation
      as the maintainer suggested, not guessed. Verified: the previously-
      hanging test now passes standalone (`--exact --test-threads=1`,
      ~16s); the full `review::` module (58 tests, `--test-threads=4`,
      229s) and the `einmo-review-server` binary's own module (30 tests,
      29s) each pass completely on their own; and a final clean
      `cargo test --workspace` (output redirected straight to a file,
      not piped through `tail`, to avoid losing progress visibility)
      completes end-to-end: **348 tests total (311 `einmo` lib + 30
      `einmo-review-server` bin + 4 `zweimomo` lib + 3 `zweimomo`
      `tests/suites.rs`), 0 failed**, in ~330s. `cargo fmt --check` and
      `cargo clippy --all-targets -- -D warnings` both still clean.
- [ ] **P1 — the `differing`/`NewOrBroken` flag conflates two different
      concerns that happen to share one boolean.** `TestRow::differing`
      (`src/einmo_suite.rs` `scan_tests`) is `true` if *any* of
      output/checked/verified is absent, OR any two present stages'
      bodies disagree — `bodies.iter().any(Option::is_none) ||
      bodies.windows(2).any(|w| w[0] != w[1])`. That's a reasonable
      "needs attention" signal for `einmo test`/`einmo list`'s own
      fail-the-gate use (an unpopulated `verified/` is fine at the
      `Checked` level, but the row is still worth a human glance in a
      listing). `review::EinmoReview::items` (`src/review.rs`) reuses
      that exact same field to implement `ReviewMode::NewOrBroken`, whose
      own doc comment (and `einmo_review_client.sh -n`'s help text)
      promises cases that "differ **between output and checked**
      stages" — output vs checked specifically, not "any stage missing."
      Confirmed empirically: on a fresh suite where `verified/` simply
      hasn't been populated yet (the normal starting state), every case
      shows `differing: true` regardless of whether output and checked
      actually agree; promoting one case to `verified` was the only thing
      that flipped its flag to `false`. This makes `-n`/`NewOrBroken`
      nearly useless on a typical suite.
      **Direction from the maintainer**: don't just patch the boolean —
      `einmo test` (must *fail* when conditions aren't met) and `einmo
      review` (must *prompt the reviewer to act*) are different
      consumers of the same underlying stage-comparison, and the code
      should reflect that: share the comparison core, but let each
      consumer derive its own, correctly-scoped meaning from it instead
      of both reading one overloaded `bool`.
      **(2026-07-31, after further discussion) — broadened into an
      architectural direction, to be *thought through* only after P0 is
      fixed, not acted on yet:** the maintainer's proposal is a layered
      core the whole crate shares, not just a `scan_tests` patch —
      - `EinmoCase`: one case's full cross-stage bundle (input/output/
        checked/verified) plus every operation performable on it as
        itself — read each stage's file, verify its stamp chain,
        pairwise-compare stage bodies, promote/flag/retract *this one
        case*. Formalizes what `TestRow` (`einmo_suite.rs`) and
        `review.rs`'s per-id operations each already approximate ad hoc,
        without a shared type.
      - `EinmoSuite`: matches files across stage directories into
        `EinmoCase`s and owns suite-wide case management, including
        being the one place batch/signature promotion logic lives —
        replacing the two independent promote implementations that
        already exist and have already drifted (`transitions::promote`
        vs. `review.rs`'s `promote_one_accumulating`; see P3, a direct
        symptom of this duplication).
      - `EinmoSuiteDirectory`: the file/directory-operations layer
        (walking, path mirroring, stage-dir resolution) `EinmoCase`/
        `EinmoSuite` sit on. Largely already exists informally as
        `stage.rs`/`config.rs` (`walk_input_tree`, `mirror_input_path`,
        `stage_dir`) and is what `CorpusSigner` already builds on — main
        remaining work is naming it explicitly and having the case/suite
        layer share it rather than each doing its own path-juggling.
      - `einmo test`'s FAE/FF policy and user-facing error formatting,
        and `einmo review`'s gather-and-prompt worklist, become two thin,
        purpose-specific layers over the *same* `EinmoSuite`/`EinmoCase`
        core, instead of review re-deriving its own scanning/comparison/
        promotion logic as it does today. The FAE/FF half of this split
        already exists (`einmo_suite.rs`'s `Problem`/`ProblemLevel`); the
        review half is what currently bypasses it.
      Given the blast radius (`einmo_suite.rs`, `review.rs`,
      `transitions.rs`, `corpus_signer.rs`, `cli.rs`), consider spinning
      this into its own EIMP (design doc first, via `eimp-write-plan`)
      rather than executing it ad hoc inside EIMP-1's tail end — a
      decision for whoever picks this up, not pre-made here.
- [x] **P2 — `EinmoReview::execute` can partially apply a batch and then
      discard the whole report on error** (`src/review.rs:845-915`).
      Promotions are grouped by `(from, to)` in a `HashMap` and applied
      group-by-group; `Stage::Verified` groups do
      `keys.to_verified.as_ref().ok_or_else(...)?` — a `?` *inside* the
      loop. If a plan mixes a `→checked` and a `→verified` promotion and
      no verified key was supplied, and (arbitrary) `HashMap` iteration
      hits the checked-group first, that promotion is written to disk —
      then the next iteration's missing-key error propagates out of the
      whole function. The flag/retract loop after the promote loop never
      runs; the already-applied item's pending decision is never
      cleared; no `JournalEvent::ExecuteBatch` is ever logged for a batch
      that *did* mutate disk. Fix: validate all needed keys for the
      entire plan before mutating anything, or make the per-group loop
      resilient (accumulate per-group errors into `report`, never
      short-circuit the whole function once a group has mutated disk).
      Add a test with a mixed checked+verified batch and a missing
      verified key.
      (2026-07-31 16:05) — split the promote-groups loop into two passes:
      the first resolves every `(from, to)` group's key (still returning
      `EinmoError::NoKey` on a missing verified key, preserving the
      documented "returns an error if a promotion needs a verified-stage
      key" contract) *before* the second pass does any mutation at all;
      a missing key now aborts the whole batch with zero side effects,
      regardless of `HashMap` iteration order, rather than after some
      other group has already been written to disk. Added
      `execute_missing_verified_key_aborts_the_whole_batch_untouched`: a
      two-case batch (`a.foo → verified` with no verified key, `b.foo →
      output → checked` in the same plan) asserts neither promotion lands
      on disk and both decisions remain pending after the `Err`. All 19
      `execute_*`/promote-related tests in `review::tests` pass
      (including the new one and every pre-existing one, no
      regressions); `cargo fmt --check` / `cargo clippy --all-targets --
      -D warnings` both clean.
- [x] **P3 — the reviewer-session promote path drops `is_computer_key`/
      `non_human` detection.** `transitions::promote` computes and
      returns `Promoted.non_human` for empty-passphrase (computer-key)
      `verified` promotions, and it's tested
      (`empty_passphrase_verified_is_flagged_non_human`). The session
      path added by this EIMP, `promote_one_accumulating`
      (`src/review.rs:1140-1187`), returns only a plain `String` detail —
      `is_computer_key`/`non_human` appears nowhere in `review.rs`,
      `review_server.rs`, or `src/bin/einmo_review_server.rs`. An
      empty-passphrase `verified` promotion executed through `POST
      /execute` or the TUI is currently indistinguishable from a genuine
      human attestation in the API response and `ExecutionReport`. Thread
      `non_human` through `promote_one_accumulating` → `Executed` →
      the HTTP/CLI JSON, same as the pre-existing CLI path already does.
      (2026-07-31 16:20) — added `non_human: bool` to `review::Executed`
      (mirrors `transitions::Promoted::non_human`); `promote_one_
      accumulating` now computes it once
      (`to == Stage::Verified && is_computer_key(&keypair.pubkey_hex())`)
      and returns it alongside the detail string from all three of its
      return paths (no-op / co-signed / fresh baseline), threaded through
      the one call site in `execute()`; the retract/flag call sites set
      `non_human: false` (not applicable to non-promote actions).
      Surfaced at both API surfaces: `review_server.rs`'s `ExecuteResponse
      .executed` changed from an undocumented `(String, String)` tuple to
      a named `ExecutedResponse { id, detail, non_human }` (verified no
      current consumer — the shell client, the dhtml page — reads
      anything but `.length`, so this was a safe, non-breaking shape
      change); the CLI's `print_execute_report`
      (`src/bin/einmo_review_server.rs`) now includes `non_human` in its
      JSON output and prints the same plaintext warning `einmo promote`'s
      own CLI path already does (`cli.rs`) for any non-human verified
      promotion. Three new tests: `review::tests::execute_reports_non_
      human_for_a_computer_key_verified_promotion_but_not_a_real_one`
      (library level, both computer-key `true` and real-passphrase
      `false` cases), `review_server::tests::execute_response_surfaces_
      non_human_for_a_computer_key_verified_promotion` (proves it reaches
      the actual HTTP JSON), plus P2's
      `execute_missing_verified_key_aborts_the_whole_batch_untouched`
      picked up along the way. Full re-run: `review::tests` 60/60,
      `review_server::tests` 46/46, `einmo-review-server` binary 30/30,
      all passing; `cargo fmt --check` / `cargo clippy --all-targets --
      -D warnings` both clean.
- [x] **P4 — `claims` never reaches the HTTP `plan` endpoint.**
      `PlanResponse` (`src/review_server.rs`) only has `actions`;
      `get_plan` maps `plan.actions` and drops `plan.claims` even though
      `EinmoReview::plan()` returns it. Both consumers expect a `.claims`
      field that never arrives: `scripts/einmo_review_client.sh`'s
      `jq -r '.claims // [] | ...'` (always empty) and
      `src/dhtml/review.html`'s claims banner (always hidden). The CLI
      binary's `cmd_plan` (`src/bin/einmo_review_server.rs`) already
      includes claims correctly — mirror that shape into `PlanResponse`.
      (2026-07-31 16:32) — added `ClaimResponse { id, remaining_secs }`
      (same shape `cmd_plan` already prints) plus a
      `From<review::ActiveClaim>` conversion, and a `claims:
      Vec<ClaimResponse>` field on `PlanResponse`, populated in `get_plan`
      from `plan.claims`. Two existing tests had literally documented the
      gap in their own comments (`claim_endpoint_surfaces_in_plan`:
      "PlanResponse only carries actions; claims aren't in the wire
      type"; `claim_via_http_expires_and_is_auto_reclaimed`: "PlanResponse
      (the wire DTO) doesn't carry claims" with a `let _ = plan;`
      side-stepping it) — both now actually assert on `plan.claims`
      instead of working around the hole. Manually verified live end to
      end: started a real server on a fixture suite, claimed a case over
      HTTP, and confirmed `GET .../plan` now returns
      `"claims":[{"id":"name_binding.js","remaining_secs":299}]` — exactly
      the shape the shell client's `jq` was always expecting. All 46
      `review_server::tests` pass; `cargo fmt --check` / `cargo clippy
      --all-targets -- -D warnings` both clean.
- [x] **P5 — dhtml diff view renders every line wrong**
      (`src/dhtml/review.html`). Reads `line.Equal`/`.Removed`/`.Added`,
      but `DiffLineResponse` is `#[serde(tag = "tag", rename_all =
      "lowercase")]` (`src/review_server.rs`) — the wire shape is
      `{"tag":"equal","text":"..."}`. All three property reads are
      `undefined`, so every line currently renders green ("added") with
      the literal text `undefined`. Fix the JS to branch on `.tag`, same
      as `einmo_review_client.sh`'s `diff-helper.sh` already does
      correctly.
      (2026-07-31 16:29) — `toggleDiff()` now reads `line.tag`/`line.text`
      directly instead of the nonexistent per-variant keys. No Rust test
      covers static HTML/JS, so verified against a REAL signed diff
      instead of just reading the diff: built a tiny throwaway suite
      (`einmo evaluate` with content "one" → promote to checked, then
      changed the input to "two" and `einmo regenerate-output`, so
      output/checked genuinely disagree, both validly signed), started a
      real server, fetched `GET .../diff/output/checked`, and fed the
      exact response through the fixed JS logic verbatim (via `node`,
      outside a browser — no headless browser available in this
      environment). Confirmed correct output: `removed`/`added` lines
      with the right CSS classes and `-`/`+` prefixes, matching
      `.diff-line.removed`/`.diff-line.added`'s existing styling.
- [x] **P6 — dhtml SSE live-refresh never fires**
      (`src/dhtml/review.html`). Uses `es.onmessage`, but the server
      sends every event with an explicit name
      (`Event::default().event(event.name())` —
      `decision-made`/`item-changed`/`executed`,
      `src/review_server.rs`). Per the SSE spec, `onmessage` only fires
      for unnamed (`"message"`) events; named events need
      `addEventListener(name, ...)`. Add listeners for all three event
      names instead of the single `onmessage` handler.
      (2026-07-31 16:34) — `connectSSE()` now registers one shared handler
      via `addEventListener` for each of the three event names (dropped
      the now-redundant `data.event === '...'` branch inside the old
      `onmessage` handler — with per-name listeners already scoped to
      exactly those three names, the check could never be false). Verified
      against the REAL wire protocol, not just the code: started a live
      server, opened a raw `curl -N` connection to `GET .../events`,
      triggered a decision change on another connection, and captured the
      actual SSE frame — `event: decision-made` followed by
      `data: {"event":"decision-made","id":"..."}` — confirming the
      server really does send exactly the named-event shape
      `addEventListener` needs and `onmessage` was silently dropping.
- [x] **P7 — private server can be orphaned on early script exit**
      (`scripts/einmo_review_client.sh`). `trap cleanup EXIT INT TERM`
      isn't registered until after the private server is already
      spawned; at least six early `exit 1` paths between spawn and the
      trap registration (no socket special file, no `.session` sidecar,
      empty session, stale/unreachable socket, missing `jq`,
      `harden_dir` failure) leave `$SERVER_PID` running with no cleanup —
      and no way to find its (deliberately unpredictable/unexposed)
      socket path again afterward. Move `trap cleanup EXIT INT TERM`
      to immediately after the private server is spawned (right after
      `SERVER_PID=$!`), before any later fail-fast check can exit.
      (2026-07-31 16:32) — moved `cleanup()`'s definition and `trap
      cleanup EXIT INT TERM` to the very top of the script, right after
      the initial `SOCKET`/`PRIVATE_SUITE`/`SERVER_PID`/`TMP` variable
      declarations and before `getopts` even runs — every variable
      `cleanup()` touches was already `${VAR:-}`-guarded, so registering
      it this early is a safe no-op in every mode (`-s`, `-p`, or a bare
      `exit 0` from `-h`) until there's actually something to clean up.
      Removed the now-redundant manual `kill "$SERVER_PID"` on the
      socket-timeout path (the trap already covers it) and the old,
      later `cleanup`/`trap` definitions. Verified against the REAL bug,
      not just by reading the diff: built a `PATH` with every tool the
      script needs (`curl`, `mktemp`, etc.) *except* `jq`, ran
      `./einmo_review_client.sh -p <suite>` under it so the script spawns
      the private server and then fails at the "jq is required" check —
      confirmed on the unfixed code (via `git stash`) that this leaves
      `einmo-review-server serve --private ...` running as an orphan
      with no socket path ever exposed to find it again, and confirmed
      the fixed code kills it cleanly every time.
- [x] **P8 — private-socket base directory is never hardened**
      (`src/review_server.rs` `private_socket_base_dir`). The per-session
      leaf directory gets `crate::journal::harden_dir` (mode 0700), but
      its parent (`$EINMO_REVIEW_PRIVATE_DIR` or
      `$TMPDIR/einmo-review-private`) is only `create_dir_all`'d, never
      chmod'd. Under a typical umask (022) that's mode 0755 —
      world-readable/executable — so any local user can list it and see
      every "unpredictable" session directory name the moment it's
      created, contradicting `EIMP-1.md` §S.7a's discovery-resistance
      intent (the leaf's own 0700 still blocks traversal into it, but the
      name itself leaks). Apply `harden_dir` to the base dir the same way
      `journal_dir()` already hardens itself.
      (2026-07-31 16:40) — `private_socket_path()` now calls
      `crate::journal::harden_dir(&base)?` in place of the bare
      `create_dir_all`, matching `journal_dir()`'s own discipline exactly.
      Extended the existing `private_socket_path_is_hardened_and_unique`
      test to also assert the base dir itself ends up mode 0700 (not just
      the leaf) — confirmed this genuinely catches the bug by reverting
      the fix locally and re-running: it failed with `left: 493, right:
      448` (0755 vs the expected 0700, exactly the leak under umask 022),
      then passed again once restored. All 46 `review_server::tests`
      pass; `cargo fmt --check` / `cargo clippy --all-targets -- -D
      warnings` both clean.
- [x] **P9 — `retract_now` doesn't take the `exec` mutex that
      `flag_now`/`execute` do** (`src/review.rs`). `flag_now`'s own doc
      comment explains why it takes `self.exec.lock()` (concurrent flags
      must serialize against `execute`'s own batch). `retract_now` has no
      equivalent guard, so a concurrent `execute()` promoting an id and a
      `retract_now()` for the same id can interleave unserialized. Add
      the same `let _guard = self.exec.lock()...` `retract_now` opens
      with.
      (2026-07-31 16:44) — added the identical guard, same place in the
      function `flag_now` takes it. New test
      `retract_now_serializes_against_a_concurrent_execute_on_the_same_id`:
      spawns a real `retract_now`/`execute` race on the SAME id (execute
      promoting checked→verified, retract demoting from checked) and
      asserts neither call panics and, whichever actually won the race,
      `checked/`+`verified` end up either both absent or both
      stamp-chain-valid — never a torn/half-written file. Verified
      non-flaky across several manual runs before the full-suite
      confirmation. All 61 `review::tests` pass (60 pre-existing + the
      new one); `cargo fmt --check` / `cargo clippy --all-targets -- -D
      warnings` both clean.
- [x] **P10 — passphrase-bearing types derive `Debug` with no
      redaction.** `config::KeySource`, `review::SignerSet`,
      `review_server::ExecuteRequest`, and
      `src/bin/einmo_review_server.rs`'s `ExecuteArgs` all derive `Debug`
      over a field holding a raw passphrase — inconsistent with
      `signature::StageKeypair`'s hand-written redacted `Debug`
      (`"seed": "<sealed>"`) and this project's own "never log secrets"
      rule (`rust_instructions.md`). No live `{:?}` call site was found
      during review, so this is latent rather than currently
      triggered — fix before it is. Hand-write redacting `Debug` impls
      for all four.
      (2026-07-31 16:53) — hand-wrote redacted `Debug` impls for
      `KeySource`, `ExecuteRequest`, and `ExecuteArgs`, all printing
      `"<redacted>"` in place of the passphrase (same shape as
      `StageKeypair`'s). `SignerSet` needed no separate impl: it still
      derives `Debug`, but that's already safe once `KeySource`'s own
      `Debug` redacts, since a derived impl calls each field's own `fmt` —
      proved this transitively-safe reasoning with a real test rather
      than just asserting it. Four new tests, one per type, each
      confirming the raw passphrase string never appears in `{:?}`
      output: `config::tests::key_source_debug_never_renders_the_raw_
      passphrase`, `review::tests::signer_set_debug_never_renders_
      either_raw_passphrase`, `review_server::tests::execute_request_
      debug_never_renders_the_raw_passphrase`,
      `tests::execute_args_debug_never_renders_the_raw_passphrase`
      (`src/bin/einmo_review_server.rs`). All affected clusters re-run
      clean: `config::tests` 15/15, `review_server::tests::execute*`
      5/5, full `einmo-review-server` binary 31/31; `cargo fmt --check`
      / `cargo clippy --all-targets -- -D warnings` both clean.
- [x] **P11 — dhtml case list is the one render path that doesn't escape
      `c.id`** (`src/dhtml/review.html`, `renderCaseList()`). Every other
      render path in the file (`showPlan`, `toggleDiff`) uses
      `escHtml()`; this one interpolates directly into `innerHTML`. Case
      ids derive from suite input filenames, so a crafted filename could
      execute script in the reviewer's browser. Use `escHtml()` here too.
      (2026-07-31 16:56) — wrapped `c.id` in `escHtml()`, matching
      `showPlan`/`toggleDiff`. No Rust test covers static HTML/JS;
      verified with a standalone `node` reproduction of the exact
      template-literal logic on both versions: the OLD code turned a case
      id of `<img src=x onerror=alert(1)>evil.js` into a live `<img
      onerror=...>` tag in the resulting HTML (confirmed
      `html.includes('<img')` was `true`); the FIXED code renders it as
      inert escaped text (`&lt;img src=x onerror=alert(1)&gt;evil.js`,
      `includes('<img')` `false`).
- [x] **P12 — stage path segments in `review_server.rs` use hand-rolled
      parsing instead of a typed extractor.** `case_body`, `case_diff`,
      `flag_case`, `retract_case` take `Path<(.., String)>` and validate
      via a hand-written `parse_stage()` inside each handler — the exact
      shape `rust_instructions.md`'s "HTTP services (axum)" section
      calls out as wrong, and inconsistent with this same file's header
      comment claiming "typed extractors throughout." Give `Stage` a
      `Deserialize` impl usable as a path-segment extractor (mirroring
      `SessionId`'s), and use `Path<(SessionId, EinmoId, Stage)>` etc.
      instead.
      (2026-07-31 17:03) — added `impl<'de> Deserialize<'de> for Stage`
      in `stage.rs`, right where `EinmoId`'s own impl already lives,
      routing through the existing `Stage::parse` (same shape/rationale
      as `EinmoId`'s). All four handlers switched to
      `Path<(SessionId, EinmoId, Stage)>` /
      `Path<(SessionId, EinmoId, Stage, Stage)>`; `parse_stage()` and its
      four call sites deleted entirely — nothing left hand-rolling this.
      Purely an internal refactor: the four pre-existing tests that
      already assert `400` on an invalid stage string
      (`invalid_stage_400s`, `case_diff_400s_on_invalid_stage`,
      `flag_endpoint_400s_on_invalid_stage`,
      `retract_endpoint_400s_on_invalid_stage`) needed no changes and all
      still pass, since axum's default `Path` rejection is `400` too —
      exactly the point of the refactor: identical external behavior,
      no more hand-rolled parsing to keep in sync. `stage::tests` 16/16,
      `review_server::tests` 47/47; `cargo fmt --check` / `cargo clippy
      --all-targets -- -D warnings` both clean.
- [ ] Deferred by explicit maintainer direction (not a fix-it item):
      `\d`/server-diff in `einmo_review_client.sh` is broken —
      `EinmoReviewServerDiff()`'s Vimscript function body uses
      backslash line-continuation inside a single string passed to
      `vim -c "..."`; that continuation syntax is a `:source`-time
      parser feature and does not apply to a `-c` argument, confirmed by
      reproducing the exact `E116`/`E10` errors in isolation. Left alone
      per the maintainer ("it worked before, I'm not sure what's going
      on, but leave it") pending further diagnosis of whether/how it
      ever worked.

- [ ] Verify all work is committed on `jia` and all tests pass
      (`cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`)
      **Note (2026-07-31, superseding the 341-tests note below)**: all
      work is currently on branch `agent/1-0`, not `jia` — needs a merge
      before this can be checked. `cargo fmt --check` and `cargo clippy
      --all-targets -- -D warnings` are still clean.
      **Update (2026-07-31 15:57)**: `cargo test` deadlocking is now
      fixed (P0). A clean `cargo test --workspace` completes end-to-end:
      **348 tests, 0 failed** (311 `einmo` lib + 30 `einmo-review-server`
      bin + 4 `zweimomo` lib + 3 `zweimomo` `tests/suites.rs`; the earlier
      "341" count predates several tests added since and, more
      importantly, was never actually re-confirmed to complete before
      this review — see P0 for the full story). `cargo fmt --check` /
      `cargo clippy --all-targets -- -D warnings` both still clean. Still
      blocked on the branch-vs-`jia` merge, and on the remaining P1–P12
      defects below before this box should be checked.
      **Update (2026-07-31 17:03) — P2 through P12 all now fixed too**
      (only P1, the architectural discussion, remains open by design —
      see its entry above: explicitly deferred, think-through-only per
      the maintainer, not an implementation task). Final full re-run
      after all of P0/P2–P12 together: **356 tests, 0 failed** (318
      `einmo` lib + 31 `einmo-review-server` bin + 4 `zweimomo` lib + 3
      `zweimomo` `tests/suites.rs` — the count grew again since the
      348-test checkpoint above because P2, P3, P9, and P10 each added
      their own regression test alongside the fix). `cargo fmt --check`
      / `cargo clippy --all-targets -- -D warnings` both clean. Still
      not checking this box: the branch-vs-`jia` merge hasn't happened
      (all of this is still on `agent/1-0`) and P1 is still open —
      genuinely not done yet, not just unconfirmed.
- [ ] Update `EIMP-1.md` frontmatter `status: complete`
- [ ] Update `docs/eimp/INDEX.md` to reflect EIMP-1's completed status

## Post-EIMP follow-ups (recorded for later)

- [ ] Create an EIMP to pull in best practices for using git worktrees from
      the Foolish project (einmo's origin). Foolish uses
      `jia`-trunk-plus-worktrees for feature isolation; einmo currently does
      not, but as the contributor base grows the pattern may become valuable.
- [ ] Investigate test-suite performance: Argon2 key derivation (~1.8s per
      invocation) makes `cargo test` very slow when many tests each derive a
      key. Consider batching key derivation across tests, using a shared
      cached keypair fixture, or running slow tests behind a feature gate.
- [ ] Wire `ReviewMode::Random` to the CLI: add `--random` flag to
      `einmo-review-server list` (the library already supports it via
      `ReviewOpts { mode: ReviewMode::Random }` — only the CLI flag is
      missing).
