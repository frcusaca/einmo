# EIMP-8.plan — Code-review findings (einmo library, review server, zweimomo)

Derived from `docs/eimp/EIMP-8.md` (the specification). **Read both files
before executing any checkbox**, and in particular read each P-item's
**Triage** line in §S.2 — it is authoritative where it disagrees with the
original finding text above it. Four items are marked REJECTED and must not
be implemented; they appear in Phase 8 only to be recorded as closed.

**Status as of this revision:** `Draft`, `begun: [ ]`. Nothing below is
checked. The maintainer promotes the EIMP to `Implementing` and flips
`begun: [x]` before Phase 1 starts.

**No Phase 0 gate.** The first revision of this plan opened with "Phase 0 —
Unblock the toolchain gate", on P0's claim that `cargo clippy --all-targets
-- -D warnings` fails. It did not — neither lint P0 names is deny-by-default
on this toolchain (§S.0). There was nothing to unblock. **P0 itself is now
done** (Phase 8, checked off 2026-08-01): the maintainer added the two lints
at `deny`, both members were wired to the workspace table, and the dead line
was replaced with calls to the real predicate. The gates are green.

**Verify before you start**, so any later breakage is provably yours:

- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean —
      note this now denies `overly_complex_bool_expr` and `nonminimal_bool`
      workspace-wide
- [ ] `cargo test --workspace` green (expect 356 lib + 31 + 4 + 3 = 394)

---

## Phase 1 — High severity (§S.1 Tier 1)

Three independent items. P27 and P28 both touch execute/session plumbing, so
land them in that order; P26 is self-contained and is the highest-impact
single line in the EIMP — do it first.

- [ ] (read §S.3 / **P26** of EIMP-8.md) — stored XSS in `review.html` via a
      flag `reason`
  - [ ] Write the test first: record a `Flag` whose `reason` is
        `<img src=x onerror=alert(1)>`; assert `GET …/cases` still returns
        the reason verbatim in JSON (the server is deliberately not the
        escaping layer) and that the rendering sink escapes it
  - [ ] Fix `src/dhtml/review.html:174`: `${c.decision}` → `${escHtml(c.decision)}`
  - [ ] Audit every other interpolation in `review.html` for unescaped
        sinks; `${target}` at line 310 is the other one (server-controlled
        today, latent tomorrow)
  - [ ] Add a comment at `decision_tag` (`review_server.rs:267`) naming
        `review.html` as the escaping-responsible consumer, so the next
        person to add a renderer knows the string is reviewer-controlled
  - [ ] Closes **P14** — check it off in Phase 8 when this lands
- [ ] (read §S.3 / **P27** and §S.2 / **P2**, **P3** of EIMP-8.md) —
      `execute` applies cleared decisions
  - [ ] Write the tests first, all three:
    - [ ] `decide(Promote{Checked})` → `plan()` → `undecide()` → `execute()`
          → the id is in `skipped`, `checked/` untouched
    - [ ] `decide(Flag{Checked})` → `plan()` → `decide(Retract{Checked})` →
          `execute()` → `skipped` (same basis path, so only a kind check
          catches it — this is P3's real reproducer, not the one P3 states)
    - [ ] a hand-built `ExecutionPlan` naming a never-decided id →
          `skipped`
  - [ ] Rewrite the drift filter (`review.rs:899-919`) to distinguish three
        states: no live decision → skip; live decision of a different kind →
        skip; matching kind → compare basis fingerprints as today, and skip
        when the basis is absent
  - [ ] Correct the false comment at `review.rs:896-898` to state what is
        actually checked
  - [ ] Fold in **P2**: make `Fingerprint::of` distinguish `NotFound` (→
        legitimate `None`) from other I/O errors (→ `EinmoError::Io`), and
        have `decide` (`review.rs:643-644`) propagate rather than `.ok()`
  - [ ] Add the deliberate-fail-safe comment at `refresh()`'s own `.ok()`
        (`review.rs:674`) — a transient stat failure there reports spurious
        drift, which is the safe direction
  - [ ] Tests green; every existing `execute_*` test unchanged
- [ ] (read §S.3 / **P28** of EIMP-8.md) — `POST /einmo/sessions` accepts any
      filesystem path
  - [ ] Resolve the Open Question first: reject a mismatching `suite`, or
        drop the field and derive the suite server-side? (The latter also
        fixes **P33**.) Record the decision in EIMP-8.md
  - [ ] Write the test first: `POST /einmo/sessions` naming a directory other
        than the server's own suite is refused (403); naming its own suite
        succeeds
  - [ ] Add `allowed_suite: Option<PathBuf>` to `AppState`, set by
        `run_serve` from `args.suite`, canonicalized the way
        `suite_lock_path` already canonicalizes
  - [ ] Enforce it in `create_session`; document on `AppState` that a session
        can only ever be opened over the suite the process holds the
        `SuiteLock` for
  - [ ] Test green
- [ ] Commit: `EIMP-8 P26/P27/P28: XSS sink, execute drift filter, suite confinement`

## Phase 2 — Medium severity, integrity (§S.1 Tier 2)

- [ ] (read §S.3 / **P29** of EIMP-8.md) — `resume()` re-journals its own
      replay
  - [ ] Write the test first: journal a session with N decides, resume it
        twice, assert the file's `Decide` count is still N and exactly two
        resume markers were appended
  - [ ] Add a replay-suppressed path — a `replaying: AtomicBool` on
        `EinmoReview` honoured by the `log_at` calls in
        `decide`/`undecide`, set for the duration of `resume`'s replay loop
        (`review.rs:463-477`)
  - [ ] Journal one `SessionResume { replayed: usize }` instead of
        re-enacting the history (resolve the Open Question on whether resume
        should be recorded at all)
  - [ ] Test green
- [ ] (read §S.3 / **P30** of EIMP-8.md) — TCP bearer token exposed in argv
  - [ ] Write the test first: the env-var path authenticates; the token-file
        path authenticates
  - [ ] `#[arg(long, env = "EINMO_REVIEW_TOKEN", hide_env_values = true)]` on
        `ServeArgs::token` (`einmo_review_server.rs:101-105`); `clap`'s `env`
        feature is already enabled, no new dependency
  - [ ] Add `--token-file <path>` reading a mode-0600 file
  - [ ] When `--tcp` is given with no token, mint 32 bytes from `OsRng`,
        write to a 0600 file beside the session sidecar, print only the path
        — the shape `--private` already uses for the socket
  - [ ] Update the `--token` doc comment to state plainly that a token on the
        command line is readable by every local user via `/proc/<pid>/cmdline`
  - [ ] Test green
- [ ] (read §S.3 / **P32** of EIMP-8.md) — `serve_uds` deletes non-sockets
  - [ ] Write the tests first: `serve_uds` pointed at a regular file errors
        and leaves the file intact; pointed at a genuinely stale socket,
        rebinds as before
  - [ ] `symlink_metadata` + `FileTypeExt::is_socket()` before any
        `remove_file` (`review_server.rs:978-990`); refuse with an error
        naming the path
  - [ ] Narrow the stale inference to `ErrorKind::ConnectionRefused`; any
        other connect error means "cannot determine — refusing to reclaim"
  - [ ] Apply the same narrowing to `SuiteLock::acquire`'s probe
        (`suite_lock.rs:72-90`)
  - [ ] Tests green
- [ ] (read §S.3 / **P34** of EIMP-8.md) — `SuiteLock::acquire` is
      check-then-write
  - [ ] Write the test first: N threads calling `acquire` on one suite
        concurrently; exactly one succeeds
  - [ ] Acquire with `OpenOptions::new().write(true).create_new(true)`
        (atomic O_EXCL); on `AlreadyExists`, read + probe + reclaim-if-stale
        + retry a bounded number of times
  - [ ] Test green
- [ ] (read §S.3 / **P35** of EIMP-8.md) — predictable scratch paths under a
      shared `/tmp`
  - [ ] Resolve the Open Question: does a foreign-owned or symlinked scratch
        dir hard-error, or warn loudly and continue with journaling off?
        Record the decision in EIMP-8.md
  - [ ] Write the tests first: `harden_dir` refuses a symlink; refuses a
        foreign-owned directory; `journal_dir()` differs across uids
  - [ ] Add the uid to both base paths — `einmo-journal-{uid}`
        (`journal.rs:184-189`) and `einmo-review-private-{uid}`
        (`review_server.rs:1046-1051`)
  - [ ] Rewrite `harden_dir` (`journal.rs:207-211`) to create with the mode
        rather than chmod after: `DirBuilder::new().mode(0o700).create(dir)`,
        and on `AlreadyExists` verify instead of assume — `symlink_metadata`,
        `is_dir()`, not a symlink, `uid == getuid()`, `mode & 0o077 == 0`
  - [ ] Make a failed journal open visible: one stderr warning the first time
        `writer` is `None` (`journal.rs:231-238`). "Degrades silently" must
        mean "does not fail the review", not "tells no one"
  - [ ] Tests green; `suite_lock`'s tests still pass (it shares
        `journal_dir()`)
- [ ] Commit: `EIMP-8 P29/P30/P32/P34/P35: journal integrity, token custody, scratch hardening`

## Phase 3 — Medium severity, robustness (§S.1 Tier 3)

- [ ] (read §S.2 / **P7** of EIMP-8.md) — sync `execute` blocks a tokio
      worker
  - [ ] Note the corrected rationale: Argon2id here measures **515 ms in
        debug** (m=19456/t=2/p=1, the OWASP minimum), not the "~1.8s by
        design" the finding claims. The real justification is the unbounded
        per-batch filesystem work under the `exec` mutex. Do not repeat the
        1.8s figure in a commit message
  - [ ] Write the test first: two concurrent `POST … /execute` at one
        session serialize, with no state corruption
  - [ ] Wrap `review.execute(&plan, &keys)` (`review_server.rs:732`) in
        `tokio::task::spawn_blocking`; `.await` the `JoinHandle`
  - [ ] **Re-run Phase 1's P27 tests** — `spawn_blocking` widens the
        `plan()`/`execute()` window, which is exactly the race P27 closes
  - [ ] Test green; `execute_with_confirm_promotes_to_checked` unchanged
- [ ] (read §S.2 / **P8** of EIMP-8.md) — default socket not hardened
  - [ ] **Do not implement the finding's recommendation** — hardening the
        socket's parent would `chmod 0700` the user's CWD (§S.2 / P8 Triage).
        The Open Question is answered: no
  - [ ] Write the test first: a default-mode socket file is created mode
        0600 **and** its parent directory's mode is unchanged
  - [ ] `set_permissions(socket_path, 0o600)` after `UnixListener::bind` in
        `serve_uds`
  - [ ] Document in `--socket`'s help text that the default mode relies on
        the socket's own permissions and `--private` is the unguessable-path
        alternative
  - [ ] Test green
- [ ] (read §S.2 / **P9** of EIMP-8.md) — unbounded sessions (upgraded from
      Low: each one opens a journal file, so it leaks fds and files, not just
      map entries)
  - [ ] Write the test first: create and close N sessions, assert the map
        empties; creating past the cap is refused
  - [ ] Add `AppState::close_session`, a `DELETE /einmo/{session}` route, and
        a hard cap on concurrent sessions (reject past it)
  - [ ] Document the session lifecycle on `AppState`
  - [ ] Test green
- [ ] (read §S.2 / **P5**, **P12** and §S.3 / **P41** of EIMP-8.md) — the
      worklist rescan, both halves
  - [ ] Benchmark `items()` on a synthetic 1000-case suite (before)
  - [ ] Resolve the Open Question: share `VerifiedCache`, or a sibling
        agreement cache? Pick whichever keeps `VerifiedCache` ignorant of
        `StagePairAgreement`. Record the decision in EIMP-8.md
  - [ ] Route `case.agreement`'s verify-on-inspect (`review.rs:549-552`)
        through the chosen cache (**P5**)
  - [ ] Add `EinmoReview::case(&id)` scanning one case's stage directories;
        rewire `case_detail` (`review_server.rs:285-297`) off the
        full-suite scan (**P12**)
  - [ ] Debounce the client's SSE handler (`review.html:346-366`) with a
        trailing ~150 ms timer; skip `selectCase` unless the event's `id` is
        the selected case; use the `Executed` payload's id lists to refresh
        only affected rows (**P41**)
  - [ ] Benchmark after: no regression on `day.1`, measurable improvement on
        the 1000-case suite
  - [ ] `comprehensive_multi_reviewer_end_to_end` passes unchanged
- [ ] (read §S.3 / **P31** of EIMP-8.md) — the DHTML client cannot
      authenticate over TCP
  - [ ] Resolve the Open Question first — this is a scope decision, not a
        technical one. Recommended: option (b), declare the DHTML UDS-only
        and remove `/` and `/review/{session}` from `router_tcp`. Option (a)
        requires answering how SSE carries the token without leaking it into
        logs and `Referer`. Record the decision in EIMP-8.md
  - [ ] Write the test first: over TCP, whichever routes are meant to be
        reachable are reachable and the rest 401
  - [ ] Implement the chosen option; update `EIMP-1` §S.7's framing if the
        browser-over-TCP goal is dropped
  - [ ] Test green
- [ ] (read §S.3 / **P33** of EIMP-8.md) — the `/` entry point is broken
  - [ ] If Phase 1's P28 dropped the `suite` field, `/`'s auto-create now
        works — verify it and add the HTTP-level test. Otherwise make `/`
        redirect to the process's existing session rather than minting one
  - [ ] Add `.catch(e => toast(…))` to `init()` (`review.html:372`) so a
        failure shows instead of a blank page
  - [ ] Fold in **P11**: dedupe `serve_review_dhtml`/`serve_review_dhtml_root`
        to one handler and add `Path<SessionId>` on `/review/{session}` so an
        unknown session 404s before the shell is served
  - [ ] Test green
- [ ] (read §S.3 / **P36** of EIMP-8.md) — `?differing=true` silently ignored
  - [ ] Write the test first: `GET …/cases?differing=true` returns strictly
        the differing subset
  - [ ] Add `Query<ListCasesParams>` to `list_cases` and an optional
        per-call `ReviewMode` override on `items()` (the capability exists as
        `ReviewMode::NewOrBroken` but is fixed at `open_with` time)
  - [ ] Test green
- [ ] Commit: `EIMP-8 P5/P7/P8/P9/P12/P31/P33/P36/P41: blocking, caching, session lifecycle, client paths`

## Phase 4 — Low severity, library

- [ ] (read §S.2 / **P4**) — `refresh()` holds the read lock across stats
  - [ ] Snapshot `(id, basis, path)` under the lock into a `Vec`, drop the
        guard, stat outside it (`review.rs:667-678`); comment that a
        `decide` landing mid-stat simply isn't in this call's report
- [ ] (read §S.2 / **P6**) — `shuffle` modulo bias
  - [ ] Add the `// non-uniform; ordering only, not security` comment at
        `review.rs:277`. Do not add a dependency
- [ ] (read §S.3 / **P37**) — `plan()` order is nondeterministic
  - [ ] Write the test first: two `plan()` calls on an unchanged decision set
        are equal
  - [ ] Sort `actions` by `id` in `plan()` (`review.rs:837-862`)
  - [ ] Correct all three doc comments (`review.rs:356-358`,
        `review.rs:361-362`, `review_server.rs:633`) to describe the real
        execution order: promotions grouped by stage pair first, then
        retracts and flags
- [ ] Commit: `EIMP-8 P4/P6/P37: refresh lock scope, shuffle comment, plan ordering`

## Phase 5 — Low severity, review server

- [ ] (read §S.2 / **P10**) — predictable `SessionId`, "opaque" doc
  - [ ] Take option (a), per the resolved Open Question: correct the doc at
        `review_server.rs:31-32` to "sequential, minted by
        `AppState::create_session`; not a secret — the access control is the
        socket's permissions (UDS) or the bearer token (TCP)". Do not mint
        from `OsRng`
- [ ] (read §S.2 / **P13**) — `delete_decision` event taxonomy
  - [ ] **Deferred until Phase 3's P41 lands** — the finding's premise is
        that a consumer wants to skip a re-fetch on clear, and today's client
        re-fetches on every event regardless. Re-evaluate afterwards
  - [ ] For now, document the contract: any `decision-made` means "re-fetch
        the affected case"
- [ ] (read §S.2 / **P15**) — non-constant-time bearer compare
  - [ ] Note the ordering: **P30** (Phase 2) hands over the whole token with
        no timing at all and is the one that mattered. This is defence in
        depth
  - [ ] Replace `presented == guard.token` (`review_server.rs:875-878`) with
        a short manual constant-time compare (length check plus a folding XOR
        over the bytes). Do not add `subtle` for eight lines — `EIMP-4` §S.1
        keeps core einmo dependency-light
- [ ] (read §S.3 / **P38**) — `escHtml` does not escape quotes
  - [ ] Add `.replace(/"/g,'&quot;')` and `.replace(/'/g,'&#39;')`, coerce
        with `String(s ?? '')` (`review.html:368-370`)
  - [ ] Comment that it is safe for text and quoted-attribute contexts but
        not unquoted attributes, URLs, or script contexts
- [ ] (read §S.3 / **P39**) — wire DTOs are `pub` in a private module
  - [ ] Decide: is the HTTP wire contract public API? Recommended yes — an
        external client wanting typed responses is what the `Deserialize`
        derives are for. Record the decision in EIMP-8.md
  - [ ] Either export the full DTO set from `lib.rs`, or demote all of them
        — including the three currently exported (`DiffLineResponse`,
        `DiffResponse`, `SectionDiffResponse`) — to `pub(crate)`. Do not
        leave the boundary split
  - [ ] While in `journal.rs`, apply P1's one surviving nit:
        `self.writer.lock()` → `unwrap_or_else(|e| e.into_inner())`
        (`journal.rs:273`), so a poisoned mutex does not silently kill
        journaling for the rest of the session
- [ ] Commit: `EIMP-8 P10/P13/P15/P38/P39: server hygiene, escaping, DTO visibility`

## Phase 6 — Low severity, zweimomo and workspace

- [ ] (read §S.3 / **P40** and §S.2 / **P18**) — no `[workspace.lints]`
  - [ ] Add `[workspace.lints.rust] unsafe_code = "deny"` alongside the
        existing `[workspace.lints.clippy]` table in the root `Cargo.toml`.
        Both members already carry the top-level `[lints]` /
        `workspace = true` opt-in (added with P0 on 2026-08-01), so this is
        now a two-line change. The root crate — the published one, carrying
        `ed25519-dalek`, `fips205`, `chacha20poly1305`, `argon2` — is the one
        P18 omitted
  - [ ] `cargo clippy --workspace --all-targets -- -D warnings` unchanged
  - [ ] Closes **P18**
- [ ] (read §S.2 / **P25**) — `pub mod evaluators;`
  - [ ] `zweimomo/src/lib.rs:11`: `pub mod evaluators;` → `mod evaluators;`,
        keeping `pub use evaluators::BoaEvaluator;`
  - [ ] `cargo test -p zweimomo` green
- [ ] (read §S.2 / **P23**) — pin the Boa serialization
  - [ ] The finding's claim is **false** (measured: `evaluate("'hi'")` →
        `["hi"]`, no quotes — §S.0). Add the tests anyway, for the real
        surprises: `'hi' → "hi"`, `({a:1}) → "[object Object]"`,
        `[1,2,3] → "1,2,3"` in `evaluators.rs`'s `mod tests`
  - [ ] Do not "confirm the quote-wrapping in `day.1`'s baselines" — there is
        none to confirm
- [ ] (read §S.2 / **P17**) — `Context`-per-call cost undocumented
  - [ ] Extend `BoaEvaluator`'s doc comment (`evaluators.rs:6-11`) with the
        `!Send` reasoning and `thread_local!` caching as a recorded rejected
        alternative. No code change
- [ ] (read §S.2 / **P19**) — stack-overflow test brittleness
  - [ ] Comment at `zweimomo/tests/suites.rs:99` noting the dependency on the
        default thread stack triggering a guard-page SIGSEGV, and that the
        fix if it flakes is
        `std::thread::Builder::new().stack_size(64 * 1024)`
- [ ] (read §S.2 / **P20**) — `run_tier` does not exercise the checked gate
  - [ ] Comment in `run_tier` (`suites.rs:59-80`) noting the checked-baseline
        gate is a human-review action, and that
        `eimp3_output_drift_comprehensive` covers the output/checked
        relationship
- [ ] (read §S.2 / **P21**) — fixture passphrase in the test body
  - [ ] Comment at `suites.rs:194-196` noting it is a fixture string in a
        `publish = false` crate, not a secret
- [ ] Commit: `EIMP-8 P17/P19/P20/P21/P23/P25/P40: zweimomo notes, workspace lints, pub mod`

## Phase 7 — The EIMP-8 comprehensive test

One test exercising the Tier-1 and Tier-2 fixes in combination — each one's
failure mode is another's precondition (see §"The EIMP-8 comprehensive test"
in EIMP-8.md).

- [ ] Write the comprehensive test:
  - [ ] `decide(Promote{Checked})` → `plan()` → `undecide()` → `execute()` →
        `skipped`, `checked/` untouched (**P27**)
  - [ ] a flag reason of `<img src=x onerror=alert(1)>` survives verbatim in
        the JSON and is escaped at the render sink (**P26**, **P14**)
  - [ ] `decide(Flag{Checked})` → `plan()` → `decide(Retract{Checked})` →
        `execute()` → `skipped` (**P3** via **P27**)
  - [ ] `POST /einmo/sessions` naming a foreign directory → 403; naming the
        server's suite → 200 (**P28**)
  - [ ] decide, close, resume twice → `Decide` count unchanged, two
        `SessionResume` lines (**P29**)
  - [ ] two concurrent `POST …/execute` serialize under `spawn_blocking`,
        with no cleared decision applied (**P7** × **P27**)
  - [ ] `serve_uds` at a regular file refuses and leaves it intact; a
        default-mode socket is 0600 with its parent unchanged (**P32**,
        **P8**)
  - [ ] N threads racing `SuiteLock::acquire` → exactly one wins (**P34**)
- [ ] Test green; placed alongside the relevant module's existing tests
- [ ] Commit: `EIMP-8: comprehensive test`

## Phase 8 — Record the rejected findings, then close

The rejected items are not implemented. They are checked off here so the
record shows they were adjudicated rather than forgotten, and so a future
reviewer does not re-derive them.

- [x] **P0** — rejected as Blocker (clippy was clean under the default lint
      set; §S.0), but the line itself was dead and is now fixed
      (2026-08-01 04:24)
  - [x] Confirmed why the gate stayed silent: neither
        `overly_complex_bool_expr` nor `nonminimal_bool` is deny-by-default
        on `clippy 0.1.97`. Isolated probe in §S.0
        (2026-08-01 04:24)
  - [x] Maintainer added `[workspace.lints.clippy]` denying both; wired both
        members with a top-level `[lints]` / `workspace = true` (a
        `[workspace.lints]` table alone is inert, and `lints.workspace =
        true` written *inside* `[package]` is silently ignored as
        `unused manifest key: package.lints`). Clippy then failed at
        `verify.rs:451` exactly as P0 predicted
        (2026-08-01 04:24)
  - [x] Made `flags_fail_the_gate` (`cli.rs:714`) `pub(crate)`
        (2026-08-01 04:24)
  - [x] Replaced the hand-inlined gate at `verify.rs:442-455` with calls to
        it at all four corners — `(n>0,false)`, `(n>0,true)`, `(0,false)`,
        `(0,true)` — so neither operand can be hardcoded again; deleted the
        `!true` tautology
        (2026-08-01 04:24)
  - [x] `cargo fmt` / `cargo clippy --workspace --all-targets -- -D warnings`
        clean; `cargo test --workspace` green (356 + 31 + 4 + 3 = 394, 0
        failed)
        (2026-08-01 04:24)
- [ ] **P1** — rejected: `Journal::log_at` has no `.expect()`
      (`journal.rs:273` uses `if let Ok(…)`). No panic path in `Drop`. The
      surviving one-line nit is folded into Phase 5's P39
- [ ] **P16** — rejected: the `Deserialize` derives are used by ~15 in-crate
      HTTP tests (`review_server.rs:1221, 1255, 1377, 1505, …`). Removing
      them breaks the suite. The real issue is visibility — Phase 5's P39
- [ ] **P22** — self-closed by the original review: `.unwrap()` in test code
      is exempt per `rust_instructions`. No change
- [ ] **P24** — rejected as a work item: the `=` pin plus a committed
      `Cargo.lock` already is the machine check. No test to add

## Phase 9 — Final verification and close

- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` green
- [ ] `comprehensive_multi_reviewer_end_to_end` (`src/review.rs`) passes
      unchanged
- [ ] `eimp3_output_drift_comprehensive` (`zweimomo/tests/suites.rs:167`)
      passes unchanged
- [ ] Every Open Question in EIMP-8.md is resolved or explicitly deferred
- [ ] Update `EIMP-8.md` frontmatter `status: complete`
- [ ] Update `docs/eimp/INDEX.md` (EIMP-8 row `Draft` → `complete`, plus a
      "Last Updated" entry summarizing what was fixed and what was rejected)

## Notes for the implementing agent

- **Triage first, and trust the Triage line over the finding text.** §S.2's
  finding bodies are the original review, unedited; the **Triage** line under
  each is the verified verdict. Where they disagree, the Triage wins. Four
  items are REJECTED — implementing them would break tests (P16), chmod the
  user's CWD (P8 as originally written), or chase code that does not exist
  (P1).
- **Tests first**, per `rust_instructions` §7 and `AGENTS.md`. Every
  behavioral item above has a "Write the test first" sub-task.
- **Order matters in three places.** P27 before P7 (`spawn_blocking` widens
  the race P27 closes). P28 before P33 (dropping the `suite` field is what
  makes `/` work). P41 before re-deciding P13.
- **Do not repeat the "~1.8s Argon2id" figure.** It measures 515 ms in a
  debug build and less in release (§S.0). Cite the unbounded batch I/O
  instead.
- **Commit at phase boundaries**, message referencing `EIMP-8 P<n>`.
- **No worktree/branch mechanics.** This executes directly on `jia` per
  `eimp.md`.
