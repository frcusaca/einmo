---
eimp: 8
title: Code-review findings — einmo library, review server, and zweimomo
author: opencode (z-ai/glm-5.2) <noreply@anthropic.com>; Claude Code (Opus 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-31
supersedes: []
begun: [ ]
---


# EIMP-8: Code-review findings — einmo library, review server, and zweimomo

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

## Abstract

A code review of three surfaces — the `einmo` library (`src/review.rs`,
`src/journal.rs`, `src/suite_lock.rs`), the review server
(`src/review_server.rs`, `src/dhtml/review.html`,
`src/bin/einmo_review_server.rs`), and the `zweimomo` companion crate.

The document has two layers. **Layer one** is the original z-ai/glm-5.2
review (P0–P25), preserved verbatim in §S.2 with its numbering intact.
**Layer two** is a second-agent verification pass that ran each claim
against the code and the toolchain, recorded a **Triage** verdict on every
P-item, and added sixteen findings the first pass missed (P26–P41, §S.3).

The verification pass **rejected four of the twenty-six original findings
as factually false** — including P0, which claimed to be a build-gate
Blocker and is not — **re-characterized four more**, and found that the
first review's single most consequential omission is a **stored XSS in the
review UI (P26)** reachable through the very field P14 examined and cleared.

Net accepted work: **thirty-seven items**, of which three are High
(P26, P27, P28), eleven Medium, and the rest Low. `status: Draft`,
`begun: [ ]` — awaiting maintainer triage.

## Motivation

`EIMP-1`'s maintainer-review pass produced a numbered, location-stamped
findings list (P0–P12), and `EIMP-7` was spun out of that record's own P1
recommendation. That established the repo's convention: a thorough read of
an already-`complete` surface produces a findings list, which is then
triaged into "fix in place" vs "spin into its own EIMP" — rather than being
silently patched or dropped.

This EIMP continues that discipline, and extends it in one way the earlier
passes did not need: **a review is itself a work product subject to
review.** The first pass here was produced by a different model, and roughly
one finding in six did not survive contact with the code. Some failures were
citation-level (P1 quotes an `.expect("…poisoned")` that does not exist in
`Journal::log_at`), some were measurement-level (P7's "~1.8s by design"
Argon2id cost measures at 515 ms in a debug build), and one recommends a
remediation that would actively make things worse (P8 would `chmod 0700` the
user's working directory). Recording *that* — with the verification command
next to each verdict — is as much a part of the record as the findings
themselves, because the next agent to read this file needs to know which
claims were checked and how.

The three surfaces under review:

1. **`src/review.rs` and friends** — `EinmoReview`, the thread-safe
   review-session object (`EIMP-1`/`EIMP-2`, `complete`), plus `journal.rs`
   and `suite_lock.rs` which it depends on. Well-encapsulated and
   well-tested, but the `execute` drift filter has a hole its own comment
   denies, `resume()` corrupts the audit journal it replays, and the
   scratch-directory discipline assumes a private `/tmp`.
2. **`src/review_server.rs` + `src/dhtml/review.html` + the binary** — the
   axum HTTP app, the browser client it serves, and the CLI
   (`EIMP-2`, `complete`). Strong typed-extractor discipline (`EinmoId`
   validation genuinely closes path traversal) and good authz separation,
   but the DHTML client has an unescaped `innerHTML` sink fed by a
   reviewer-controlled string, `POST /einmo/sessions` accepts any path on
   the filesystem, and the client cannot authenticate over the one
   transport it was built for.
3. **`zweimomo/`** — the JavaScript-only companion crate (`EIMP-2` §8,
   `complete`). Small and clean; the findings here are one real one-line
   fix (`pub mod`) and a set of comment-only notes.

None of the thirty-seven accepted items is a "the design is wrong" finding.
The architectures are sound. The findings are localized defects, each with a
file:line pointer and a recommended fix.

## Specification

This EIMP's "specification" is the findings list. Each P-item is a
self-contained record with **Location**, **Finding**, **Severity**, and
**Recommended remediation**; every item in §S.2 additionally carries a
**Triage** line recording the verification pass's verdict, and that verdict
is authoritative where it disagrees with the original text above it.

Severity tiers:

- **High** — a real correctness, integrity, or security defect; fix before
  the next release.
- **Medium** — a robustness, concurrency, or scalability concern; fix when
  the affected code is next touched.
- **Low** — hygiene, documentation, or a minor-convention deviation.
- **Rejected** — the finding does not hold; recorded with why, so it is not
  re-derived by the next reviewer.

There is no **Blocker** tier in use: §S.1 establishes that no finding blocks
the toolchain gate.

### S.0 — Verification method and toolchain status

Everything below was checked against the working tree at `f87a97b` on `jia`.
The commands, and their actual results:

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --check` | clean |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | **clean** (re-run after `touch src/verify.rs` to defeat caching) |
| Tests | `cargo test --workspace` | green (356 lib + 31 + 4 + 3 integration; exit 0) |

**No gate was broken. `begun: [ ]` was not blocked by a Blocker, because
there was not one.** P0's Blocker classification — and the first revision of
the plan's Phase 0 built on it — rested on a claim that clippy denies
`flagged_count > 0 && !true` under the repo's mandated invocation. On the
tree as reviewed it did not, and an isolated probe pinned down why:

| Configuration | `n > 0 && !true` |
|---|---|
| default clippy + `-D warnings` | **silent** |
| `overly_complex_bool_expr = "deny"` + `nonminimal_bool = "deny"` | **caught** (both fire) |

Toolchain: `cargo 1.97.1` / `rustc 1.97.1` / `clippy 0.1.97`. Neither lint is
at deny level by default here, so the repo's gate could never have caught
this line.

**Resolved 2026-08-01.** The maintainer added `[workspace.lints.clippy]`
denying both lints, and both workspace members were wired to it with a
top-level `[lints]` / `workspace = true` (the `[workspace.lints]` table is
inert on its own — a member that does not opt in gets nothing, and writing
`lints.workspace = true` *inside* `[package]` yields a silent
`unused manifest key: package.lints`). With that in place clippy fails at
`src/verify.rs:451` exactly as P0 predicted, and the line was fixed per P0's
revised remediation. Gates re-verified green afterwards.

The distinction worth carrying forward: P0's **judgment** was correct — the
line was dead and asserted nothing. Its **mechanism** was not, and the
difference mattered, because "the build is already failing, fix it first"
and "no tool will ever tell you about this" call for opposite responses. The
second is the one that warranted adding a lint.

Three empirical probes backed specific verdicts (each reverted after
measuring):

- **Argon2id cost** (bears on P7): a temporary `#[test]` timing
  `StageKeypair::derive` printed `DERIVE_MS=515` in a **debug** build. The
  pinned parameters are `m=19456 KiB, t=2, p=1` (`signature.rs:35-37`) —
  the OWASP 2025 *minimum* baseline, deliberately cheap. P7's "~1.8s by
  design" is off by an order of magnitude and by more in release.
- **Boa string rendering** (bears on P23): `BoaEvaluator.evaluate("'hi'")`
  returns `["hi"]` — no quotes. `to_std_string_escaped` escapes unpaired
  surrogates; it does not quote-wrap. P23 read `Debug` formatting as
  content.
- **`--flag-is-not-failure`** (bears on P0's Open Question): the feature
  **exists** — `flags_fail_the_gate(flagged_count, flag_is_not_failure)` at
  `cli.rs:714`, wired at `cli.rs:700`, CLI flag at `cli.rs:205`. The Open
  Question is answered; the fix is to call the real function.

### S.1 — Execution priority

Highest-severity, lowest-blast-radius first. The plan file follows this
order.

| Tier | Items | Theme |
|---|---|---|
| **1 — High** | P26, P27, P28 | Stored XSS; `execute` applies cleared decisions; unconstrained suite path |
| **2 — Medium, integrity** | P2, P29, P30, P32, P34, P35 | Drift detection; journal integrity; token exposure; destructive socket handling; lock race; shared-`/tmp` hardening |
| **3 — Medium, robustness** | P5, P7, P8, P9, P12, P31, P33, P36, P41 | Cache/scan cost; runtime blocking; socket mode; session lifecycle; broken client paths |
| **4 — Low** | P0, P3, P4, P6, P10, P11, P13, P14, P15, P17–P25, P37, P38, P39, P40 | Hygiene, docs, comment-only notes |
| **Rejected** | P1, P16, P23 (as stated), P0 (as Blocker) | Do not implement; see each Triage |

### S.2 — The z-ai/glm-5.2 findings (P0–P25), triaged

The **Finding** and **Recommended remediation** text in this section is the
original review's, unedited. The **Triage** line is the verification pass's.

#### P0 — `verify.rs:451` breaks `cargo clippy --all-targets -- -D warnings`

- **Location:** `src/verify.rs:451`
- **Finding:**
  ```rust
  let gate_fails_with_override = flagged_count > 0 && !true; // !flag_is_not_failure
  ```
  The `--flag-is-not-failure` override is dead code: `!true` is always
  `false`, so `gate_fails_with_override` is always `false` regardless of
  `flagged_count`. clippy denies this (`clippy::overly_complex_bool_expr` +
  `clippy::nonminimal_bool`) and the **entire** `cargo clippy --all-targets
  -- -D warnings` gate fails.
- **Severity (claimed):** Blocker.
- **Triage: REJECTED AS BLOCKER; the underlying defect is real, Low.
  FIXED 2026-08-01.**
  `cargo clippy --workspace --all-targets -- -D warnings` was **clean**,
  verified twice, the second time after `touch src/verify.rs` to force a
  fresh check of the test target. Neither `overly_complex_bool_expr` nor
  `nonminimal_bool` is at deny level by default on this toolchain (§S.0
  pins the isolated probe), so the repo's mandated gate could not have
  caught this and nothing was ever blocked. The first revision of the
  plan's "Phase 0 gate" premise was void.

  That is a narrow rejection, and it should not be read as a defence of the
  line. `!true` is `false`, so `gate_fails_with_override` was `false` for
  every input and `assert!(!gate_fails_with_override)` asserted nothing
  whatsoever — it would have passed with `flags_fail_the_gate` deleted
  outright. The finding's judgment was right; only its account of what
  would catch it was wrong.

  What *is* real is smaller and different from what the finding describes:
  lines 442–455 **re-implement the gate inline** instead of calling the
  production predicate. `flags_fail_the_gate(flagged_count,
  flag_is_not_failure)` exists at `cli.rs:714` and is the thing under test.
  A test that hand-inlines `flagged_count > 0 && !true` cannot catch a
  regression in `flags_fail_the_gate` — it asserts against a copy of the
  logic, and the copy has a hardcoded operand.
- **Recommended remediation (revised): DONE.** `flags_fail_the_gate`
  (`cli.rs:714`) is now `pub(crate)`, and `verify.rs`'s gate test calls it
  for both operand values at both `flagged_count > 0` and `flagged_count ==
  0`, pinning all four corners so neither operand can be quietly hardcoded
  again. The two hand-inlined `let` bindings are gone. The Open Question
  about whether the feature exists is answered — it does, and the function's
  own doc comment already claimed it was "kept as a small pure function …
  so the actual decision is directly unit-testable", which is precisely what
  the inlined copy prevented.
- **Follow-on (maintainer, 2026-08-01):** `[workspace.lints.clippy]` now
  denies `overly_complex_bool_expr` and `nonminimal_bool`, with both members
  opted in. This class of defect is a gate failure from here on rather than
  something a reviewer has to notice. This also shrinks **P40**: adding
  `[workspace.lints.rust] unsafe_code = "deny"` is now a two-line edit
  beside the existing clippy table, because both members already carry the
  `[lints]` / `workspace = true` opt-in it would otherwise have had to
  introduce.

#### P1 — `EinmoReview::Drop` can panic-on-poison and abort the process

- **Location:** `src/review.rs:1128-1140`, depending on `Journal::log_at`
  taking its internal `Mutex<Option<File>>` via `.expect("…poisoned")`.
- **Finding:** If a panic during the session poisoned that mutex, `drop`
  panics-on-unwind → aborts the process.
- **Severity (claimed):** High.
- **Triage: REJECTED — the cited code does not exist.** `Journal::log_at`
  (`journal.rs:262-278`) takes the lock as:
  ```rust
  if let Ok(mut guard) = self.writer.lock()
      && let Some(file) = guard.as_mut()
  { let _ = writeln!(file, "{json}"); }
  ```
  There is no `.expect()` anywhere in `journal.rs`. On poison it silently
  skips, which is exactly the documented "infallible / degrades silently"
  contract. `Drop` has no panic path. The finding's entire mechanism is a
  fabricated citation.

  A genuine, much smaller nit survives in the neighbourhood: once poisoned,
  the journal is silently dead for the **rest of the session** — every
  subsequent event is dropped, and the audit record just stops with no
  indication. `unwrap_or_else(|e| e.into_inner())` would recover, and the
  `Option<File>` behind the lock has no invariant a poisoning panic could
  have broken. Folded into P39 as a one-line hygiene change; not worth its
  own item.

#### P2 — `decide` swallows transient fingerprint I/O errors, silently disabling drift detection

- **Location:** `src/review.rs:643-644`
  ```rust
  let basis = decision_basis_path(&self.config, &id, &decision)
      .and_then(|p| Fingerprint::of(&p).ok());
  ```
- **Finding:** `Fingerprint::of` returns `Err` on a transient `metadata()`
  failure. The `.ok()` turns that into `None`, which `execute` later treats
  as "no recorded basis → nothing to compare → proceed"
  (`review.rs:905-907`). A transient stat failure during `decide` therefore
  silently disables drift detection for that decision. `None` is a
  *legitimate* value, so the fix must distinguish "file absent" from "stat
  errored".
- **Severity:** Medium (confirmed; the original said High).
- **Triage: CONFIRMED.** Locations and mechanism check out exactly. Downgraded
  from High only because the trigger is a transient I/O fault, not
  reviewer-reachable input — but the *consequence* (silent loss of drift
  detection) is severe, and it is the same `else { return true }` branch
  that P27 reaches by a far more likely route. Fix P27 and P2 together: once
  `execute` distinguishes "no basis recorded" from "no decision recorded",
  P2's `None` stops meaning "proceed unchecked".
  The converse case at `refresh()` (`review.rs:674`, `Fingerprint::of(…).ok()`
  on the *current* path) is fail-safe — a transient failure reports spurious
  drift rather than hiding it — and needs no change beyond a comment saying
  so deliberately.
- **Recommended remediation:** Have `Fingerprint::of` distinguish `NotFound`
  (→ legitimate `None`) from other I/O errors (→ propagate as
  `EinmoError::Io`). In `decide`, propagate rather than `.ok()`.

#### P3 — `execute` applies the caller's `plan.actions` but only re-checks the *basis* against the live `DecisionBook`

- **Location:** `src/review.rs:888-919`, with the claim at `review.rs:896-898`.
- **Finding:** The drift filter re-derives the *basis fingerprint* from the
  live `DecisionBook`, but the *action* it appends is the one from
  `plan.actions`. If a concurrent `decide` replaces the decision with a
  different *kind* whose basis fingerprint matches, `execute` applies the
  stale action.
- **Severity:** Low as stated; the mechanism is promoted to High as **P27**.
- **Triage: CONFIRMED MECHANISM, WRONG EXAMPLE, INCOMPLETE.** The stated
  example does not reproduce: `Decision::Promote { to: Checked }` bases on
  `source_stage_for_promote(…, Checked)` = the **output/** path, while
  `Decision::Retract { from: Checked }` bases on the **checked/** path
  (`decision_basis_stage`, `review.rs:1160-1167`). Different paths →
  different `Fingerprint.path` field → mismatch → correctly skipped. The
  claim that both "fingerprint the same `checked/` path" is wrong.

  A case that *does* reproduce: `Flag { stage: Checked }` replaced by
  `Retract { from: Checked }`. Both base on `checked/`, the fingerprint
  matches, and the stale **Flag** executes against the reviewer's fresh
  **Retract** intent.

  More importantly, the finding stops one branch short of the real hole. See
  **P27** — the `None` arm of the same `let … else`, which is reached by an
  ordinary `undecide`, needs no race, and is what makes the quoted comment
  outright false. Implement P27; P3's kind-check falls out of it.

#### P4 — `refresh()` holds the decisions read lock across per-entry filesystem stat calls

- **Location:** `src/review.rs:667-678`
- **Finding:** `refresh()` holds `self.decisions.read()` for the whole body,
  which calls `Fingerprint::of` once per pending decision, blocking writers.
- **Severity:** Low (confirmed; the original said Medium).
- **Triage: CONFIRMED, DOWNGRADED.** The code does what the finding says.
  Downgraded because `refresh` is advisory and rarely called, the pending set
  is bounded by what one reviewer decided in one session, and `metadata()` on
  a local path is microseconds. The fix is cheap and harmless, so do it —
  but it is not a Medium.
- **Recommended remediation:** Snapshot `(id, basis, path)` under the read
  lock into a `Vec`, drop the guard, then stat outside it. Add the comment
  that a `decide` landing mid-stat simply isn't reflected in this call's
  report, which is acceptable for an advisory API.

#### P5 — `items()` re-walks the suite and re-runs `case.agreement` on every call, bypassing `VerifiedCache`

- **Location:** `src/review.rs:538-574` (esp. `case.agreement(…)` at 549-552).
- **Finding:** `GET /einmo/{session}/cases` re-scans the suite directory and
  re-runs `case.agreement` for every case on every request, verify-on-
  inspecting `output/` and `checked/` artifacts. The single-flight
  `VerifiedCache` (used by `body()`) is not consulted.
- **Severity:** Medium.
- **Triage: CONFIRMED, and worse than described.** The finding treats this as
  a per-request cost. It is per-*event*: the DHTML client calls
  `refreshCases()` **plus** `selectCase()` on every SSE event
  (`review.html:352-355`), and `selectCase` issues three `body` fetches. One
  reviewer keystroke that records a decision therefore re-scans the whole
  suite and re-verifies every output+checked artifact, for every connected
  subscriber. See **P41**, which is the client half of the same problem and
  should be fixed in the same pass.
- **Recommended remediation:** Route `agreement`'s verify-on-inspect through
  `VerifiedCache` (or a sibling agreement cache keyed by the same
  `Fingerprint`). Benchmark on a synthetic 1000-case suite before and after.

#### P6 — `shuffle` uses modulo reduction, introducing negligible bias

- **Location:** `src/review.rs:274-280`, `(OsRng.next_u32() as usize) % (i + 1)`.
- **Severity:** Low.
- **Triage: CONFIRMED.** Non-uniform, ordering-only, not security-sensitive.
- **Recommended remediation:** Accept and add the comment. Do not add a
  dependency for this.

### S.2b — Review server findings (`src/review_server.rs` + binary)

#### P7 — `post_execute` runs synchronous `review.execute(...)` on the tokio worker thread

- **Location:** `src/review_server.rs:732`, called from `async fn
  post_execute` (`review_server.rs:705`).
- **Finding:** `EinmoReview::execute` is synchronous: filesystem I/O and
  Argon2id key derivation (~1.8s by design), holding a `std::sync::Mutex`
  for the whole duration, on a tokio worker thread.
- **Severity:** Medium.
- **Triage: CONFIRMED, RATIONALE CORRECTED.** The blocking is real and
  `spawn_blocking` is the right fix. The stated *reason* is not: the pinned
  Argon2id parameters (`signature.rs:35-37`) are `m=19456 KiB, t=2, p=1` —
  the OWASP 2025 **minimum** baseline, chosen to be cheap — and a measured
  derivation takes **515 ms in a debug build**, well under a tenth of that in
  release. "~1.8s by design" is fabricated; do not repeat it in a commit
  message.

  The honest justification is the *unbounded* part: `execute` walks every
  action in the batch doing filesystem promote/retract/flag work while
  holding the `exec` `Mutex`, and the batch size is the reviewer's pending
  set. A large batch blocks a runtime worker for arbitrarily long. That is
  reason enough.
- **Recommended remediation:** Wrap `review.execute(&plan, &keys)` in
  `tokio::task::spawn_blocking` and `.await` the `JoinHandle`. `execute`'s own
  serialization is unchanged. Add a test firing two concurrent
  `POST … /execute` at one session and asserting they serialize.

#### P8 — Default `--socket .einmo-review.sock` is not hardened; only `--private` mode hardens

- **Location:** `src/bin/einmo_review_server.rs:80-83` (default
  `.einmo-review.sock`), `src/review_server.rs:971-998` (`serve_uds`).
- **Finding:** Standalone `serve` binds `--socket` without hardening the
  socket file or its containing directory; `--private` hardens both to 0700,
  the default mode does not.
- **Severity:** Medium.
- **Triage: CONFIRMED PROBLEM, HARMFUL REMEDIATION — DO NOT IMPLEMENT AS
  WRITTEN.** The gap is real. But the recommended fix — "harden the socket
  file's containing directory in `serve_uds` (reuse `journal::harden_dir` on
  the socket's parent)" — would `chmod 0700` **the user's current working
  directory**, since the default socket path is relative and `serve_uds` is
  also the function `--private` calls. Silently changing the mode of a
  directory the user merely happened to `cd` into is a worse bug than the one
  being fixed. The EIMP's own Open Question ("harden unconditionally?")
  should be answered **no**, on these grounds.
- **Recommended remediation (revised):** Harden the **socket file**, not its
  parent. After `UnixListener::bind`, `set_permissions(socket_path, 0o600)`
  — a mode a connecting client must satisfy, affecting nothing but the file
  einmo itself created. (Note `bind` already applies the process umask, so
  this closes the permissive-umask case specifically.) Additionally, document
  in `--socket`'s help text that the default mode relies on the socket's own
  permissions and that `--private` is the unguessable-path alternative. Add a
  test asserting a default-mode socket file is mode 0600 and that the parent
  directory's mode is **unchanged**.

#### P9 — `AppState::sessions` grows unboundedly; no `close_session`/eviction

- **Location:** `src/review_server.rs:155-175`.
- **Finding:** `create_session` inserts but nothing removes.
- **Severity:** Medium (confirmed; the original said "Low today").
- **Triage: CONFIRMED, UPGRADED.** The finding calls this "moot today"
  because standalone `serve` opens one session. It is not moot: the
  `POST /einmo/sessions` route is mounted unconditionally
  (`review_server.rs:809`) and any client that can reach the transport can
  call it in a loop. Each call constructs an `EinmoReview`, which opens a
  journal file — so this leaks **file descriptors and files in
  `journal_dir()`**, not just map entries. Combined with **P28** (the suite
  path is caller-supplied), it is a remote-ish resource-exhaustion primitive,
  not a tidiness issue.
- **Recommended remediation:** Add `close_session`, a `DELETE
  /einmo/{session}` route, and a hard cap on concurrent sessions (reject with
  429/507 past it). Document the lifecycle on `AppState`. Add a test that
  creates and closes N sessions and asserts the map empties.

#### P10 — `SessionId` is minted predictably (incrementing `u64`); doc claims "opaque"

- **Location:** `src/review_server.rs:31-40`, `164-167`.
- **Severity:** Low.
- **Triage: CONFIRMED.** Ids are `0, 1, 2, …`. Predictability is acceptable
  under the current trust model (UDS directory permissions / TCP bearer
  token are the real gates); the doc is what is wrong.
- **Recommended remediation:** Option (a) — correct the doc to "sequential,
  minted by `AppState::create_session`; not a secret — the access control is
  the socket's permissions (UDS) or the bearer token (TCP)." Do **not** mint
  from `OsRng`: it would imply a secrecy property the transport does not
  need and the URL bar would leak anyway.

#### P11 — `serve_review_dhtml` and `serve_review_dhtml_root` are byte-identical; `/review/{session}` ignores the `{session}` segment

- **Location:** `src/review_server.rs:806-833`, `1104-1118`.
- **Severity:** Low.
- **Triage: CONFIRMED.** Both handlers are identical and neither takes a
  `Path` extractor.
- **Recommended remediation:** Dedupe to one handler and add
  `Path<SessionId>` on `/review/{session}` so an unknown session 404s before
  the shell is served. Note this interacts with **P33**: the `/` route's
  auto-create flow is broken independently, and both should be fixed
  together or the 404 will simply move.

#### P12 — `case_detail` calls `items()` and filters — full suite re-scan to fetch one case

- **Location:** `src/review_server.rs:285-297`.
- **Severity:** Low.
- **Triage: CONFIRMED.** Shares P5's fix.
- **Recommended remediation:** Add `EinmoReview::case(&id)` scanning only
  that case's stage directories. Pairs with P5's cache routing.

#### P13 — `delete_decision` publishes `DecisionMade` (not a distinct `decision-cleared` event)

- **Location:** `src/review_server.rs:488-501`.
- **Severity:** Low.
- **Triage: CONFIRMED, but the premise is weaker than stated.** The finding's
  motivation is "a DHTML that wants to skip a full re-fetch on clear can't
  distinguish" — but the shipped DHTML re-fetches on *every* event regardless
  of name (`review.html:352-355`), so no consumer is currently penalized.
  Fix **P41** first; if the client still can't avoid a re-fetch afterwards,
  the event split becomes worth it.
- **Recommended remediation:** Defer. Document the current contract
  ("any `decision-made` means re-fetch the affected case") and revisit after
  P41.

#### P14 — `CaseSummary.decision` embeds the flag `reason` verbatim in the worklist response

- **Location:** `src/review_server.rs:267-274` (`decision_tag` →
  `format!("flag {stage}: {reason}")`).
- **Finding:** A reviewer-entered flag `reason` travels in the worklist
  payload.
- **Severity:** the observation is correct; its **conclusion is wrong**.
- **Triage: CONFIRMED OBSERVATION, WRONG CONCLUSION — see P26.** The finding
  traces exactly the right data flow and then closes it with "No change
  needed for the current trust model." That is the miss. The string it
  follows from `PUT /decision` into `CaseSummary.decision` continues into
  `review.html:174`, where it is interpolated into `innerHTML` **without
  escaping**. The confidentiality question the finding asks is the minor one;
  the injection question it does not ask is High. Superseded by **P26**;
  implement P26 and this item closes with it.

#### P15 — `tcp_guard` bearer-token comparison is not constant-time

- **Location:** `src/review_server.rs:875-878`.
- **Severity:** Low.
- **Triage: CONFIRMED, and it is the lesser of the two token problems.** The
  comparison is `==` on `String`. Over a loopback-only listener this is not
  practically exploitable, as the finding says. **P30** — the same token
  sitting in `/proc/<pid>/cmdline` for every local user to read — hands an
  attacker the whole token with no timing at all, and should be fixed first.
- **Recommended remediation:** A short manual constant-time compare (length
  check plus a `fold`ing XOR over the bytes). Do not add the `subtle`
  dependency for eight lines; `EIMP-4` §S.1 keeps core einmo dependency-light.

#### P16 — `ExecuteResponse`/`PlanResponse` DTOs `derive(Deserialize)` though only ever returned

- **Location:** `src/review_server.rs:558`, `631`, and others.
- **Severity:** Low.
- **Triage: REJECTED — the derives are used.** The crate's own HTTP tests
  deserialize every one of these: `let plan: PlanResponse = body_json(resp)
  .await` and siblings appear at `review_server.rs:1221, 1255, 1377, 1505,
  1517, 1543, 1593, 1646, 1725, 1735, 1813, 1975, 2445, 2480`. Removing
  `Deserialize` breaks roughly fifteen tests. The finding did not check for
  in-crate consumers.

  The real hygiene issue in this area is the opposite one: these DTOs are
  `pub` inside a **private** module and absent from `lib.rs`'s curated
  `pub use` list, so they are unreachable public API. Recorded as **P39**.

### S.2c — zweimomo findings (`zweimomo/`)

#### P17 — `BoaEvaluator` mints a fresh `Context::default()` per `evaluate()` call; cost is undocumented

- **Location:** `zweimomo/src/evaluators.rs:19`.
- **Severity:** Low.
- **Triage: CONFIRMED.** Doc-only. `Context` is `!Send`; the per-call
  construction is required, not accidental. There is a one-line comment
  (`// A fresh Context per call (Context is !Send).`) but no note of the cost.
- **Recommended remediation:** Extend the `BoaEvaluator` doc comment with the
  `!Send` reasoning and the `thread_local!`-caching alternative as a recorded
  rejected option. No code change.

#### P18 — `zweimomo/Cargo.toml` has no `[lints.rust] unsafe_code` gate

- **Location:** `zweimomo/Cargo.toml`.
- **Severity:** Low.
- **Triage: CONFIRMED, WIDEN THE SCOPE.** At the time of review neither
  `zweimomo/Cargo.toml` **nor the root `Cargo.toml`** declared `[lints]`.
  Fixing only the demo crate leaves the published crate — the one that
  actually touches Ed25519, SLH-DSA, ChaCha20-Poly1305 and Argon2id —
  ungated. Do it once, workspace-wide, as `[workspace.lints.rust]
  unsafe_code = "deny"`; both members were given the top-level `[lints]` /
  `workspace = true` opt-in on 2026-08-01 when P0 was fixed, so only the
  table itself is still missing. Tracked as **P40** (which carries the
  syntax note); this item closes with it.

#### P19 — `crash_crumb_survives_stack_overflow` relies on runtime stack-overflow detection

- **Location:** `zweimomo/tests/suites.rs:90-140` (`recurse(usize::MAX)` at 99).
- **Severity:** Low.
- **Triage: CONFIRMED.** Comment-only, as recommended.
- **Recommended remediation:** Note at the `recurse` line that the test
  depends on the default thread stack triggering a guard-page SIGSEGV, and
  that the fix if it ever flakes is
  `std::thread::Builder::new().stack_size(64 * 1024)`.

#### P20 — `run_tier` exercises generation+self-verification but not the `output==checked` gate

- **Location:** `zweimomo/tests/suites.rs:59-80`.
- **Severity:** Low.
- **Triage: CONFIRMED.** Comment-only.

#### P21 — `eimp3_output_drift_comprehensive` hardcodes a checked passphrase in the test body

- **Location:** `zweimomo/tests/suites.rs:194-196`.
- **Severity:** Low.
- **Triage: CONFIRMED.** Fixture string in a `publish = false` crate; not a
  secret. Comment-only.

#### P22 — `copy_dir_recursive` uses `.unwrap()` on every fs op

- **Location:** `zweimomo/tests/suites.rs:145-157`.
- **Severity:** Low (no change needed).
- **Triage: CONFIRMED AS SELF-CLOSED.** Test-only; `rust_instructions`
  exempts tests. Record and move on. No plan checkbox.

#### P23 — `BoaEvaluator`'s `to_std_string_escaped()` quotes string OUTPUT

- **Location:** `zweimomo/src/evaluators.rs:23-26`.
- **Finding:** `to_std_string_escaped()` renders a JS string value `"hello"`
  as `"hello"` (with quotes) in OUTPUT, not `hello`.
- **Triage: REJECTED — measured false.** A temporary probe test (§S.0)
  printed:
  ```
  evaluate("'hi'")     => Ok(["hi"])
  evaluate("({a:1})")  => Ok(["[object Object]"])
  evaluate("[1,2,3]")  => Ok(["1,2,3"])
  ```
  The value is `hi`, three characters, no quotes. The quotes in the finding
  are `Debug` formatting of the `Vec<String>`, misread as content.
  `to_std_string_escaped` escapes **unpaired surrogates** as `\uXXXX`; it
  does not quote-wrap. There is nothing to confirm in `day.1`'s baselines
  and nothing to change.

  The *pinning test* the finding suggests is still worth adding, for the
  unrelated reason that no unit test currently covers string-valued or
  object-valued results at all — and the real surprises there are
  `[object Object]` and the surrogate escaping, neither of which is
  obvious. Kept as a Low test-addition under this number.
- **Recommended remediation (revised):** Add unit tests to
  `evaluators.rs`'s `mod tests` pinning `'hi' → "hi"`, `({a:1}) →
  "[object Object]"`, and `[1,2,3] → "1,2,3"`. Delete the false claim.

#### P24 — `boa_engine = "=0.21.1"` exact pin is correct but unverified-by-test

- **Location:** `zweimomo/Cargo.toml:19`.
- **Severity:** Low.
- **Triage: CONFIRMED; take option (a).** The `=` pin *is* the machine
  check — Cargo enforces it at resolve time, and `Cargo.lock` is committed.
  A test asserting the version would only restate what the manifest already
  enforces. Accept the comment as the explanation and close.

#### P25 — `zweimomo/src/lib.rs` `pub mod evaluators;` violates `rust_instructions` §"Don't `pub mod` internal modules"

- **Location:** `zweimomo/src/lib.rs:11`.
- **Severity:** Low.
- **Triage: CONFIRMED — the one unambiguous, unambiguously-correct fix in the
  original review.** And the inconsistency is sharp: `src/lib.rs` follows the
  private-`mod` + curated-`pub use` convention exactly, across nineteen
  modules. `zweimomo` is the sole deviation.
- **Recommended remediation:** `mod evaluators;` + keep
  `pub use evaluators::BoaEvaluator;`.

### S.3 — Findings the first pass missed (P26–P41)

#### P26 — Stored XSS in the review UI via a flag `reason`

- **Location:** `src/dhtml/review.html:174`:
  ```js
  if (hasDecision) badge = `<span class="badge decided">${c.decision}</span>`;
  ...
  div.innerHTML = `<span class="id">${escHtml(c.id)}</span>${badge}`;
  ```
  fed by `src/review_server.rs:271`:
  ```rust
  Decision::Flag { stage, reason } => format!("flag {stage}: {reason}"),
  ```
- **Finding:** `c.decision` is interpolated into `innerHTML` **without
  `escHtml`** — note that `c.id` on the very same line *is* escaped, so this
  is an omission, not a convention. `c.decision` is `decision_tag`'s output,
  which for a `Flag` decision embeds the reviewer-supplied `reason` verbatim.
  A decision recorded as
  ```
  PUT /einmo/<session>/cases/<id>/decision
  {"kind":"flag","stage":"checked","reason":"<img src=x onerror=fetch('…')>"}
  ```
  executes attacker-controlled JavaScript in the review page, for **every**
  reviewer whose client renders that worklist — and the SSE stream pushes it
  to them automatically (`review.html:352-355`) without any navigation.

  What the injected script inherits is the whole review capability: it is
  same-origin with the API, so it can `GET` every case body in the suite,
  `PUT` decisions, `POST … /execute` (the confirm string is a typo gate, not
  a security boundary — its own doc says so, `review_server.rs:658-659`), and
  read the value of `#plan-passphrase`, which is where the human types the
  `checked → verified` signing passphrase. An XSS here forges human
  attestations in a system whose entire purpose is to make human attestation
  cryptographically meaningful.

  The reachable-reason argument: `reason` strings arrive from `PUT
  /decision` and `POST … /flag/{stage}`, from a second reviewer in the
  multi-reviewer shape `EIMP-1` §S.5 explicitly anticipates, from a scripted
  or CI-driven flagging pass, and — once a flag is applied — from the
  `flagged/` artifact itself, which travels **with the corpus in git**. A
  flag reason authored in a fork and merged upstream is stored XSS that ships
  in the repository.
- **Severity:** **High.** The highest-impact finding in this document.
- **Recommended remediation:** Escape it: `${escHtml(c.decision)}`. Then
  audit every remaining interpolation in `review.html` — the `${target}` at
  line 310 is also unescaped (server-controlled stage names today, so not
  currently exploitable, but it is the same latent shape). Then harden
  `escHtml` per **P38**. Add a test that records a flag whose reason contains
  `<script>` and asserts the served worklist JSON is unchanged (the server
  must keep passing it through — escaping belongs at the sink) *and* a
  DHTML-level assertion, or at minimum a comment at `decision_tag` naming
  `review.html:174` as the escaping-responsible sink.

#### P27 — `execute` applies actions whose decision was cleared between `plan()` and `execute()`

- **Location:** `src/review.rs:899-919`, specifically:
  ```rust
  let Some(basis) = decisions.get_entry(id).and_then(|e| e.basis.as_ref()) else {
      return true; // no recorded basis: nothing to compare, proceed
  };
  ```
  and the comment at `review.rs:896-898`:
  > Checked against the LIVE DecisionBook (not `plan` itself, which carries
  > no fingerprint) so a decision changed or cleared between plan() and
  > execute() is caught the same way.
- **Finding:** The comment is false for the "cleared" half it explicitly
  claims. `get_entry(id)` returns `None` for an id that has been
  `undecide`d — and the `let … else` treats `None` the same as "the decision
  exists but has no content basis", falling through to `return true`, i.e.
  **execute it**. A reviewer who records a promotion, then clears it, and
  whose clear lands after a concurrent `plan()` snapshot, gets the promotion
  applied anyway. Signed. To `checked` or `verified`.

  The window is not theoretical in the shipped server: `post_execute`
  (`review_server.rs:711-732`) computes `plan()` and then calls
  `execute(&plan, …)` with an `await`-free but genuinely interleavable gap —
  `DELETE …/decision` (`review_server.rs:488`) takes only the `decisions`
  write lock, which `execute` is not holding between those two statements.
  P7's `spawn_blocking` fix *widens* this window, so the two must land
  together.

  Two further ids reach the same `return true` arm: an id whose decision
  legitimately has `basis: None` (a `Skip`, or a `Promote` whose source
  vanished), and an id that was never decided at all but appears in a
  caller-constructed `ExecutionPlan` — `execute` is `pub`, and nothing
  validates that a plan's actions correspond to live decisions. P2's
  transient-stat case lands here too.
- **Severity:** **High** — it defeats the drift guarantee `EIMP-1` §S.2/§S.5
  is built on, and the comment asserting otherwise means a future reader will
  not look.
- **Recommended remediation:** Restructure the filter to distinguish three
  states rather than two:
  1. **no live decision for `id`** → `skipped` (this is the bug);
  2. **live decision of a different kind than the planned action** → `skipped`
     (this closes P3);
  3. **live decision, same kind, basis present** → compare fingerprints as
     today; **basis absent** → `skipped` unless the decision kind genuinely
     has no basis (`Skip` produces no action at all, so in practice this
     means "skip", closing P2's silent-proceed path).

  Then rewrite the comment to state what is actually checked. Tests: (a)
  `decide(Promote)` → `plan()` → `undecide()` → `execute()` → asserts
  `skipped`, artifact untouched; (b) `decide(Flag{Checked})` → `plan()` →
  `decide(Retract{Checked})` → `execute()` → asserts `skipped`; (c) an
  `ExecutionPlan` hand-built with an action for a never-decided id →
  `skipped`.

#### P28 — `POST /einmo/sessions` opens a review session over any path the caller names

- **Location:** `src/review_server.rs:215-237`:
  ```rust
  pub struct CreateSessionRequest { pub suite: std::path::PathBuf }
  async fn create_session(State(state): …, Json(req): Json<CreateSessionRequest>)
      -> impl IntoResponse { let id = state.create_session(req.suite); … }
  ```
  route mounted at `review_server.rs:809`.
- **Finding:** The suite directory is taken from the request body with no
  validation, no confinement, and no relation to the suite the process was
  started against — even though the module doc one screen above declares
  "One process, one suite, one session" (`review_server.rs:2-3`) and the
  binary acquires a `SuiteLock` for exactly one suite before serving
  (`einmo_review_server.rs:322`).

  A client that clears the transport gate can therefore `POST {"suite":
  "/home/victim/anything"}` and then, against that directory: enumerate it
  (`GET …/cases`), read file contents out of it (`GET …/cases/{id}/body/
  {stage}`), and **mutate** it — `POST …/flag/{stage}` moves files into a
  `flagged/` subtree it creates, `POST …/retract/{stage}` deletes checked and
  verified artifacts (`EinmoCase::retract`), and `POST …/execute` writes
  freshly signed files. Reads and writes are confined to einmo's stage-
  directory naming by `EinmoId` validation (see the note below), but the
  *root* those names are resolved against is fully caller-chosen.

  This also removes the `SuiteLock` guarantee for any suite reached this way,
  and combines with **P9** into unbounded session/journal-file creation.

  **Credit where due:** the id half of this surface is genuinely well built.
  `validate_id_str` (`stage.rs:249-262`) rejects empty, NUL, absolute, and
  `..`-containing ids, and `EinmoId`'s `Deserialize` routes through it
  (`stage.rs:229-241`), so `Path<EinmoId>` closes traversal at the extractor.
  The gap is that nothing analogous exists for `suite`.
- **Severity:** **High** under the deployment `EIMP-1` §S.7 contemplates
  (loopback TCP with a bearer token, browser client); Medium under
  UDS-with-restrictive-directory-permissions alone. Rated High because the
  route is mounted identically on both.
- **Recommended remediation:** Bind the server to its suite. Give `AppState`
  an `allowed_suite: Option<PathBuf>` set by `run_serve` from `args.suite`
  (canonicalized, as `suite_lock_path` already does), and have
  `create_session` reject — `403`, not silently substitute — any `suite` that
  does not canonicalize to it. Keep the field in the request for wire
  compatibility, or drop it and derive the suite entirely server-side, which
  is simpler and also fixes **P33**. Test: `POST /einmo/sessions` naming a
  different directory is refused; naming the server's own suite succeeds.

#### P29 — `resume()` re-journals every replayed event, corrupting and doubling the audit log

- **Location:** `src/review.rs:454-479`.
- **Finding:** `resume` reads the journal with `Journal::replay`, opens the
  **same** journal file in append mode via `open_internal`
  (`review.rs:487-495`, which also logs a fresh `SessionOpen`), and then
  replays each `Decide`/`Undecide` through the ordinary `review.decide(…)` /
  `review.undecide(…)` calls — each of which writes its own journal entry
  (`review.rs:645-651`, `review.rs:765-770`).

  So every resume **re-appends the entire decision history it just read**.
  The journal is not a cache; `EIMP-1` §S.6 makes it the audit record —
  "session id, reviewer, timestamp, … every decide/undecide/claim/execute
  with outcomes". After a resume it contains decisions stamped with times the
  reviewer was not deciding anything, and it does so recursively: resume *n*
  times and the file holds roughly 2ⁿ copies of the original history.
  `EinmoReview::resume` is on the shipped CLI path — every one-shot
  subcommand invoked with `--session <id>` goes through it
  (`einmo_review_server.rs:440-445`), so a scripted reviewer doing ten
  chained one-shot commands has already written a thousand-fold journal.

  Decision *state* stays correct (replace-not-stack makes replay idempotent),
  which is why no test caught this — the tests assert the reconstructed
  `DecisionBook`, never the journal's contents after a resume.
- **Severity:** Medium — integrity of the audit record, and unbounded file
  growth on a normal usage pattern.
- **Recommended remediation:** Add a replay-suppressed path: either a
  `decide_silent`/`undecide_silent` pair used only by `resume`, or a
  `replaying: AtomicBool` on `EinmoReview` that `log_at` honours (the flag is
  simpler and cannot be reached by accident from a public API). Journal a
  single `SessionResume { replayed: usize }` event instead, so the resume is
  *recorded* rather than *re-enacted*. Test: journal a session, resume it
  twice, assert the file's `Decide` count is unchanged and exactly two
  `SessionResume` lines were added.

#### P30 — The TCP bearer token is passed on the command line, exposing it to every local user

- **Location:** `src/bin/einmo_review_server.rs:101-105`:
  ```rust
  /// The bearer token TCP clients must present …
  #[arg(long)]
  token: Option<String>,
  ```
- **Finding:** `--token <secret>` puts the token in the process's `argv`,
  which on Linux is world-readable at `/proc/<pid>/cmdline` and printed by
  `ps -ef` for any local user, for as long as the server runs. It also lands
  in the invoking shell's history. The token is the **sole** access control
  on the TCP listener (`tcp_guard`, `review_server.rs:866-888`), so this
  hands a local attacker complete API access — including
  `POST …/execute` — with no timing attack, no guessing, and no race.

  This is the same threat model P15 addresses at the far end (constant-time
  comparison) while the token is being broadcast at the near end.

  `clap`'s `env` feature is already enabled (root `Cargo.toml`,
  `clap = { version = "4", features = ["derive", "env"] }`), so the fix costs
  no new dependency.
- **Severity:** Medium (High on a shared host).
- **Recommended remediation:** `#[arg(long, env = "EINMO_REVIEW_TOKEN",
  hide_env_values = true)]` and, better, support `--token-file <path>` reading
  a mode-0600 file. Best of all: when `--tcp` is given without a token, mint
  32 bytes from `OsRng`, write them to a 0600 file next to the session
  sidecar, and print only the path — the same shape `--private` already uses
  for the socket. Update the `--token` doc comment to say plainly that
  passing a token on the command line exposes it to every local user. Test:
  the env-var and file paths both authenticate; the help text carries the
  warning.

#### P31 — The DHTML client cannot authenticate over TCP, the only transport it exists for

- **Location:** `src/dhtml/review.html:125-132`:
  ```js
  async function api(method, path, body) {
    const opts = { method, headers: { 'content-type': 'application/json' } };
    …
  ```
  against `src/review_server.rs:866-881` (`tcp_guard` rejects any request
  without `Authorization: Bearer <token>`) and `review_server.rs:940`
  (the guard layers over the **whole** router, including `GET /` and
  `GET /review/{session}`).
- **Finding:** `api()` never sends an `Authorization` header, and
  `EventSource` (`review.html:346`) cannot send one at all. Over TCP the
  browser therefore gets `401` on the page load itself, on every API call, and
  on the SSE stream. `EIMP-1` §S.7's rationale for the TCP listener existing
  is verbatim "TCP on 127.0.0.1 with a bearer token **only when a browser
  needs it**" — and the browser client, as shipped, cannot use it.

  Over UDS the client works, but a browser cannot open a unix socket, so the
  DHTML is only reachable there through a proxying helper.
- **Severity:** Medium — a shipped feature that does not function, and the
  kind of gap that invites someone to "fix" it by weakening `tcp_guard`.
- **Recommended remediation:** Decide the shape deliberately and write it
  down: (a) exempt `GET /` and `GET /review/{session}` from the token check
  and have the page prompt for the token (or read it from `#token`
  fragment), storing it in memory and attaching it in `api()`; SSE then needs
  the token as a query parameter, since `EventSource` has no header API — and
  a token in a URL is itself a leak into logs, so this needs a real decision,
  not a default. Or (b) declare the DHTML UDS-only and document that TCP is
  for programmatic clients, removing `/` and `/review/{session}` from
  `router_tcp` entirely. Option (b) is smaller, honest, and preserves the
  security posture; recommend it unless a browser-over-TCP workflow is
  actually wanted. Test: over TCP, whichever routes are meant to be reachable
  are reachable, and the rest 401.

#### P32 — `serve_uds` deletes whatever file sits at `--socket`, without checking it is a socket

- **Location:** `src/review_server.rs:978-990`:
  ```rust
  if socket_path.exists() {
      match tokio::net::UnixStream::connect(socket_path).await {
          Ok(_) => return Err(… AddrInUse …),
          Err(_) => std::fs::remove_file(socket_path)?,
      }
  }
  ```
- **Finding:** Two problems in five lines.

  **(a) It deletes non-sockets.** Nothing verifies the path is a socket. A
  connect to a regular file fails with `ENOTSOCK`, which lands in the `Err`
  arm, and the file is removed. `einmo-review-server serve --socket
  ~/notes.txt <suite>` deletes `~/notes.txt`. A mistyped flag, or a
  `--socket` inherited from a config or script, destroys user data. Directories
  are safe only incidentally (`remove_file` fails and `?` propagates).

  **(b) The liveness probe conflates "dead" with "busy".** Every `connect`
  failure is read as "stale". A live server whose `listen` backlog is
  momentarily full, or one whose socket the current user cannot connect to
  (`EACCES` — precisely the case after **P8**'s 0600 hardening, if two users
  share a path), is judged dead and has its socket file unlinked out from
  under it. It keeps running, now unreachable. `SuiteLock` catches the
  same-suite case; it does not catch two suites configured with the same
  socket path.

  `SuiteLock::acquire` (`suite_lock.rs:72-90`) makes the same
  every-error-means-stale inference on the lock file, so both should be
  fixed together.
- **Severity:** Medium — data loss on a plausible typo.
- **Recommended remediation:** Before removing, `symlink_metadata` the path
  and require `file_type().is_socket()` (`std::os::unix::fs::FileTypeExt`);
  refuse with a clear error otherwise, naming the path and saying einmo will
  not delete a non-socket. For (b), narrow the stale inference to
  `ECONNREFUSED` specifically (`e.kind() == ErrorKind::ConnectionRefused`)
  and treat other errors as "cannot determine — refusing to reclaim". Test:
  `serve_uds` pointed at a regular file errors and leaves the file intact;
  pointed at a genuinely stale socket, rebinds as before.

#### P33 — The `/` entry point is broken: it POSTs a session-create with no body

- **Location:** `src/dhtml/review.html:141-149`:
  ```js
  const m = path.match(/\/review\/([0-9a-f]+)/);
  if (m) { sessionId = m[1]; }
  else {
    const s = await api('POST', '/einmo/sessions');   // no body
    sessionId = s.session;
    history.replaceState(null, '', `/review/${sessionId}`);
  }
  ```
  against `create_session`'s `Json<CreateSessionRequest>` extractor
  (`review_server.rs:229-237`), whose `suite` field is required.
- **Finding:** `api()` omits `opts.body` when `body === undefined`, so the
  request carries `content-type: application/json` and an empty body. Axum's
  `Json` extractor fails to deserialize, returning `400`/`422` before the
  handler runs. `api()` throws, `init()` rejects, and the page renders its
  empty shell with no error shown — the rejection is unhandled
  (`review.html:372` calls `init()` without a `.catch`). The route exists
  (`review_server.rs:831`) and has its own dedicated handler
  (`serve_review_dhtml_root`), so this is a shipped path that has apparently
  never been exercised.

  Only `/review/<session>` works, using the id the operator reads from stderr
  or the `<socket>.session` sidecar file.
- **Severity:** Medium (a broken advertised entry point), Low if `/` is
  simply retired.
- **Recommended remediation:** This resolves cleanly if **P28** is fixed by
  deriving the suite server-side: `POST /einmo/sessions` then needs no body,
  and `/`'s auto-create starts working as designed. Otherwise, make `/`
  redirect to the process's existing session (the server knows its id) rather
  than minting one. Either way add `.catch(e => toast(...))` to `init()` so a
  failure is visible instead of a blank page. Test: an HTTP-level assertion
  that `GET /` followed by the client's own create call succeeds.

#### P34 — `SuiteLock::acquire` is check-then-write; two servers can both win

- **Location:** `src/suite_lock.rs:72-102`.
- **Finding:** The sequence is: `lock_path.exists()` → read → probe the
  recorded socket → `remove_file` if stale → `harden_dir(parent)` →
  `fs::write(&lock_path, …)`. Every step is a separate syscall with no
  atomicity anywhere. Two servers starting concurrently against the same
  suite both observe "stale" (or "absent"), both write, and both proceed to
  bind different sockets over the same corpus. The lock's entire purpose —
  `EIMP-1` §S.5, "a second review server … must refuse to start" — is
  defeated by the ordinary case of two shells racing.

  The module doc's own framing ("the lock file's *content* is the live
  server's own socket path") is what forces the read-then-write shape, but
  the write can still be made atomic.
- **Severity:** Medium.
- **Recommended remediation:** Acquire with `OpenOptions::new()
  .write(true).create_new(true).open(&lock_path)` — atomic O_EXCL — and treat
  `AlreadyExists` as "someone else holds or held it": then, and only then,
  read the existing file, probe its socket, and if genuinely stale,
  `remove_file` and retry the `create_new` a bounded number of times. Write
  the socket path into the handle already opened. Test: N threads calling
  `acquire` on one suite concurrently; exactly one succeeds.

#### P35 — Fixed, predictable scratch paths under a shared `/tmp`

- **Location:** `src/journal.rs:184-189` (`journal_dir()` →
  `std::env::temp_dir().join("einmo-journal")`),
  `src/review_server.rs:1046-1051` (`private_socket_base_dir()` →
  `…/einmo-review-private`), `src/journal.rs:207-211` (`harden_dir`), and
  `src/suite_lock.rs:47` (every suite lock lives in `journal_dir()`).
- **Finding:** Three scratch areas resolve to fixed, guessable names in a
  world-writable directory when the corresponding env override is unset —
  which is the default. `harden_dir` is
  ```rust
  std::fs::create_dir_all(dir)?;
  std::fs::set_permissions(dir, Permissions::from_mode(0o700))
  ```
  which is `create_dir_all` (succeeds silently if the path already exists,
  whoever owns it) followed by a `set_permissions` that **follows symlinks**.
  On a multi-user host a local attacker can, before the victim's first run:

  - **Kill the audit trail.** Pre-create `/tmp/einmo-journal` owned by the
    attacker. The victim's `create_dir_all` succeeds, `set_permissions` fails
    with `EPERM`, `harden_dir` returns `Err`, and `Journal::open`'s
    `.and_then(…).ok()` (`journal.rs:231-238`) quietly yields `writer: None`.
    Journaling is off for the whole session, silently, by design — the
    "never fail the review" contract makes this failure invisible. `EIMP-1`
    §S.6's audit record simply does not exist.
  - **Chmod an arbitrary directory the victim owns.** Plant
    `/tmp/einmo-journal` as a symlink to any directory; `set_permissions`
    follows it and applies 0700. Low impact (0700 is restrictive), but it is
    an unintended write primitive on paths einmo never meant to touch.
  - **Deny review-server startup indefinitely.** Write
    `/tmp/einmo-journal/suite-<sha256-prefix>.lock` containing the path of a
    socket the attacker keeps alive. The suite hash is derived from the
    canonical suite path (`suite_lock.rs:38-47`) with no secret, so it is
    computable for any target. `SuiteLock::acquire` probes, finds the socket
    live, and refuses to start — permanently.

  `private_socket_path` fails closed (it propagates `harden_dir`'s error,
  `review_server.rs:1084`) which is correct, but it means an attacker can
  deny `--private` mode too.
- **Severity:** Medium — no confidentiality loss (the 0700 leaf still
  protects contents when hardening succeeds), but silent audit loss and
  persistent denial of service, both trivially triggered.
- **Recommended remediation:** Three changes, all small:
  1. Include the uid in the base path — `einmo-journal-{uid}` and
     `einmo-review-private-{uid}` — so users cannot collide by default.
  2. Rewrite `harden_dir` to create with the right mode rather than fixing it
     afterwards: `DirBuilder::new().mode(0o700).create(dir)`, treating
     `AlreadyExists` as "verify, don't assume" — `symlink_metadata` the path,
     require `is_dir()` and not a symlink, require `uid == getuid()`, require
     `mode & 0o077 == 0`; error out otherwise instead of chmod-ing.
  3. Make journal-open failure **visible**: keep the operation infallible as
     documented, but print a one-line warning to stderr the first time
     `writer` is `None`. "Degrades silently" should mean "does not fail the
     review", not "does not tell anyone".

  Tests: `harden_dir` refuses a symlink; refuses a foreign-owned directory;
  `journal_dir()` differs across uids.

#### P36 — The client's `?differing=true` filter is silently ignored by the server

- **Location:** `src/dhtml/review.html:157-160`:
  ```js
  const differing = new URLSearchParams(location.search).get('differing');
  let url = `/einmo/${sessionId}/cases`;
  if (differing) url += `?differing=true`;
  ```
  against `list_cases` (`review_server.rs:276-283`), which extracts only
  `State` and `Path` — there is no `Query` extractor anywhere in the module.
- **Finding:** Axum ignores unextracted query parameters, so the request
  succeeds and returns the unfiltered worklist. The user gets the full list
  with no indication their filter did nothing. The underlying capability
  exists — `ReviewMode::NewOrBroken` (`review.rs:230-233`) is exactly this
  predicate — but it is fixed at `open_with` time and `items()` takes no
  per-call override, so wiring the parameter requires a small library change,
  not just an extractor.
- **Severity:** Low.
- **Recommended remediation:** Either add `Query<ListCasesParams>` to
  `list_cases` and give `items()` an optional per-call `ReviewMode` override,
  or delete the dead parameter from the client. Prefer the former — a
  reviewer filtering to new-or-broken cases is the primary large-suite
  workflow §S.2 describes. Test: `GET …/cases?differing=true` returns strictly
  the differing subset.

#### P37 — `plan()` returns actions in nondeterministic order while documenting an order

- **Location:** `src/review.rs:837-862` iterating `decisions.iter()` over a
  `HashMap`, versus `ExecutionPlan::actions`'s doc ("The actions this plan
  will apply, **in order**", `review.rs:361-362`), `plan()`'s own doc ("in
  the order they'll run", `review.rs:356-358`), and `PlanResponse::actions`
  ("in order", `review_server.rs:633`).
- **Finding:** `HashMap` iteration order is unspecified and randomized per
  process by `RandomState`, so two `GET …/plan` calls on the same unchanged
  decision set return the actions in different orders — a reviewer's
  confirmation dialog reshuffles between refreshes. Separately, `execute`
  does **not** run them in `plan.actions` order regardless: it hoists all
  promotions into `(from, to)` groups first (`review.rs:923-1000`) and only
  then walks retracts and flags (`review.rs:1002-1032`). Both docs are wrong
  in both directions.
- **Severity:** Low.
- **Recommended remediation:** Sort `actions` by `id` before returning, and
  correct all three doc comments to describe the real execution order
  (promotions grouped by stage pair first, then retracts and flags in id
  order). Test: two `plan()` calls on an unchanged decision set are equal.

#### P38 — `escHtml` does not escape quotes, and is one refactor away from an attribute-context hole

- **Location:** `src/dhtml/review.html:368-370`:
  ```js
  function escHtml(s) {
    return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
  }
  ```
- **Finding:** Sufficient for element-text context, which is where every
  current call sits — but the file already interpolates into **attribute**
  positions from other sources (`class="badge decided"`, `class="kind
  ${kind}"` at line 310), so the two contexts are mixed in the same
  templates. The moment any escaped value moves into an attribute, `"` and
  `'` escape the quoting and the function's name implies it was safe.
  `escHtml` also assumes a string and throws on `null`/number input.
- **Severity:** Low (defence in depth for **P26**).
- **Recommended remediation:** Add `.replace(/"/g,'&quot;')` and
  `.replace(/'/g,'&#39;')`, coerce with `String(s ?? '')`, and note in a
  comment that it is safe for both text and quoted-attribute contexts but not
  for unquoted attributes, URLs, or script contexts.

#### P39 — Wire DTOs are `pub` inside a private module: unreachable public API

- **Location:** `src/lib.rs:23-40` (every module is private) and
  `src/review_server.rs` — `CreateSessionRequest`/`Response` (215-227),
  `CaseSummary` (240-250), `BodyResponse` (299-304), `DecisionRequest` (411),
  `ClaimRequest` (505), `PlannedActionResponse` (560), `ClaimResponse` (611),
  `PlanResponse` (631), `ExecuteRequest` (656), `ExecutedResponse` (684),
  `ExecuteResponse` (697), `FlagRequest` (763) — none of which appear in
  `lib.rs`'s `pub use review_server::{…}` list (`lib.rs:64-67`, which exports
  only `ApiError`, `AppState`, `DiffLineResponse`, `DiffResponse`,
  `SectionDiffResponse`, `SessionId`, and the four functions).
- **Finding:** These types are `pub` but unreachable from outside the crate,
  so `cargo doc` never renders them and no external client can deserialize a
  response into the crate's own DTO. Meanwhile three siblings of the same
  kind (`DiffLineResponse`, `DiffResponse`, `SectionDiffResponse`) *are*
  exported — so the boundary is inconsistent as well as unreachable. `lib.rs`
  otherwise follows the private-`mod` + curated-`pub use` convention
  impeccably across nineteen modules; this is the one soft spot.

  This is the finding P16 was reaching for, correctly identified as a
  visibility problem rather than an unused-derive problem.
- **Severity:** Low.
- **Recommended remediation:** Decide whether the HTTP wire contract is
  public API. If yes (recommended — an external client wanting typed
  responses is the whole reason the `Deserialize` derives earn their keep),
  export the full set from `lib.rs`. If no, demote all of them, including the
  three currently exported, to `pub(crate)`. Do not leave it split. While in
  `journal.rs`, apply P1's surviving nit: `writer.lock()` →
  `unwrap_or_else(|e| e.into_inner())`.

#### P40 — No `[workspace.lints]`; neither crate denies `unsafe_code`

- **Location:** `Cargo.toml` (root) — as of 2026-08-01 it carries
  `[workspace.lints.clippy]` but no `[workspace.lints.rust]`.
- **Finding:** `rust_instructions` §2f requires crypto-touching crates to
  declare `unsafe_code = "deny"`. The original review recorded this for
  `zweimomo` only (P18), which is the *less* important of the two crates —
  the root crate is the one carrying `ed25519-dalek`, `fips205`,
  `chacha20poly1305`, `argon2` and `zeroize`, and the one published to
  crates.io. Neither declares the lint. Both are `unsafe`-free today, so the
  deny is a no-op that only prevents future drift.
- **Severity:** Low.
- **Recommended remediation:** Add one table to the root manifest, beside
  the clippy one already there:
  ```toml
  [workspace.lints.rust]
  unsafe_code = "deny"
  ```
  Nothing else is needed: both members already opt in, having been wired
  when P0 was fixed. Verify with
  `cargo clippy --workspace --all-targets -- -D warnings`. Closes P18.
- **Syntax note — this is where P0's fix went wrong the first time.** The
  per-member opt-in is a **top-level** table:
  ```toml
  [lints]
  workspace = true
  ```
  Writing `lints.workspace = true` *inside* `[package]` parses fine and does
  nothing; cargo reports only `warning: unused manifest key: package.lints`,
  which is easy to scroll past, and every lint in the workspace table stays
  silently inert. If a newly-denied lint appears to fire on nothing, check
  this before concluding the code is clean.

#### P41 — Every SSE event triggers a full worklist rescan plus three body fetches, per subscriber

- **Location:** `src/dhtml/review.html:346-366`:
  ```js
  const onEvent = (e) => {
    … refreshCases();
    if (currentIdx >= 0) selectCase(currentIdx);
  };
  for (const name of ['decision-made','item-changed','executed'])
      es.addEventListener(name, onEvent);
  ```
- **Finding:** Every event — including the one this client just caused — runs
  `refreshCases()` (a `GET …/cases`, which is P5's full-suite rescan and
  re-verification) and `selectCase()` (three `GET …/body/{stage}` calls).
  There is no debounce, no coalescing, and no self-event suppression. A batch
  `Executed` event with fifty cases produces exactly the same single event, but
  a reviewer working quickly generates one full-suite re-verification per
  keystroke-driven decision, multiplied by every connected client.

  This is the demand side of P5's supply-side cost; fixing only the cache
  halves the problem, fixing only the client leaves the per-request cost. Both
  belong in one pass.
- **Severity:** Low on `day.1` (a handful of cases); Medium on any real
  corpus.
- **Recommended remediation:** Debounce `onEvent` (a trailing ~150 ms timer
  coalescing bursts), skip `selectCase` unless the event's `id` matches the
  currently-selected case, and use the `Executed` payload's `executed`/
  `skipped` lists to refresh only affected rows. Pairs with P5 and P12.

## Test Plan

Per-item tests are named in each P-item's remediation. The EIMP-wide
verification is:

- `cargo fmt --check` clean.
- `cargo clippy --workspace --all-targets -- -D warnings` clean — it is clean
  **now** (§S.0), so any breakage is introduced by this EIMP's own work, not
  inherited.
- `cargo test --workspace` green.
- `comprehensive_multi_reviewer_end_to_end` (`src/review.rs`) and
  `eimp3_output_drift_comprehensive` (`zweimomo/tests/suites.rs:167`) pass
  unchanged — the regression sentinels for the library and zweimomo.

### The EIMP-8 comprehensive test

One test exercising the Tier-1 and Tier-2 fixes *in combination*, since each
one's failure mode is another's precondition:

1. Open a session; record a `Promote { to: Checked }`; `plan()`; `undecide`;
   `execute` → the id is in `skipped` and `checked/` is untouched (**P27**).
2. Record a `Flag { stage: Checked, reason: "<img src=x onerror=alert(1)>" }`;
   `GET …/cases` → the JSON still carries the reason verbatim (the server is
   not the escaping layer) **and** a rendering-layer assertion confirms the
   sink escapes it (**P26**, **P14**).
3. Record a `Flag { stage: Checked }`; `plan()`; replace with
   `Retract { from: Checked }`; `execute` → `skipped`, not a stale flag
   (**P3** via **P27**).
4. `POST /einmo/sessions` naming a directory other than the server's suite →
   `403`; naming the server's own suite → `200` (**P28**).
5. Decide, close, `resume` twice → the journal's `Decide` count is unchanged
   and exactly two `SessionResume` lines were appended (**P29**).
6. Two concurrent `POST …/execute` against one session serialize under
   `spawn_blocking`, with no state corruption and no cleared decision applied
   (**P7** interacting with **P27**).
7. `serve_uds` pointed at a regular file refuses and leaves the file intact;
   a default-mode socket is created mode 0600 and its parent directory's mode
   is unchanged (**P32**, **P8**).
8. N threads racing `SuiteLock::acquire` on one suite → exactly one succeeds
   (**P34**).

## Rejected Alternatives

### A. Do nothing — the surfaces are `complete` and tested

`cargo test --workspace` is green and all three surfaces carry `complete`
EIMPs. But "tested" is not "reviewed": `EIMP-1`'s own maintainer pass found
twelve defects *despite* `EIMP-2` being green, and `EIMP-7` was spun out of
one of them. The tests here document the happy path — which is precisely why
**P27** (a cleared decision still executes), **P29** (resume corrupts its own
audit log) and **P26** (a flag reason is an XSS payload) all survived a
356-test suite. Doing nothing leaves a High-severity injection in a tool
whose purpose is trustworthy human attestation.

### B. Spin each finding into its own EIMP

`EIMP-7` was spun out because its blast radius spanned six modules. Nothing
here does; most items are one line to one function. Thirty-seven EIMPs would
drown the index. One EIMP with a prioritized plan is the right granularity.

### C. Implement every finding before triage

Rejected, and this EIMP is the demonstration of why: four of the original
twenty-six findings do not survive verification, one recommends a change
(`chmod 0700` on the user's CWD) that would be a regression, and the item
labelled Blocker blocks nothing. An agent that had executed the plan
top-to-bottom without checking would have started by "fixing" a clean clippy
gate and ended by breaking fifteen tests removing load-bearing `Deserialize`
derives. Triage before implementation is not ceremony.

### D. Accept the first review verbatim and simply extend it

The tempting shape: append new findings, leave P0–P25 alone, let the
implementing agent sort it out. Rejected because the false items are not
inert — P0 gates the entire plan behind a phantom Blocker, P8 prescribes a
harmful change, P16 prescribes a test-breaking one, and P14 explicitly
*clears* the field that P26 shows is exploitable. A findings list is read as
authority. Leaving refuted claims in it, unmarked, converts this document
from a review into a trap. Hence the **Triage** line on every item, with the
command or measurement that settled it.

## Open Questions

Resolved during this pass (recorded so they are not re-opened):

- ~~P0's intent~~ — **answered**: `--flag-is-not-failure` is implemented;
  `flags_fail_the_gate` at `cli.rs:714` is the predicate the test should call.
- ~~P8's hardening scope~~ — **answered: no**, never harden the parent of a
  caller-supplied `--socket`. Harden the socket file itself.
- ~~P10's direction~~ — **answered**: doc fix (a). Random ids imply a secrecy
  property the transport does not provide.
- ~~P24~~ — **answered**: the `=` pin plus committed `Cargo.lock` is already
  the machine check.

Still open, for the maintainer:

- **P31's shape.** Is a browser-over-TCP workflow actually wanted? If yes,
  the SSE token-in-query-string leak needs a real answer (token in the URL
  lands in logs and `Referer`). If no, drop `/` and `/review/{session}` from
  `router_tcp` and say so in `EIMP-1` §S.7. This is a scope decision, not a
  technical one.
- **P28's mechanism.** Reject a mismatching `suite` (keeps the wire shape,
  keeps multi-session plausible) or drop the field and derive the suite
  server-side (simpler, also fixes P33, forecloses multi-suite)? The answer
  depends on whether multi-session review is still a live goal.
- **P5's cache shape.** Share `VerifiedCache` (one cache, two consumers) or a
  sibling agreement cache (separate invalidation)? Pick whichever avoids
  making `VerifiedCache` know about `StagePairAgreement`.
- **P35's `harden_dir` strictness.** Refusing a foreign-owned or symlinked
  scratch directory is correct, but it turns a currently-silent degradation
  into a hard error on shared CI images where `/tmp` may be pre-populated.
  Error, or warn-and-continue with journaling disabled *loudly*?
- **P29's replay marker.** Is `SessionResume { replayed: n }` the right
  event, or should resume append nothing at all and stay invisible in the
  record?

## References

- Prior EIMPs:
  - `EIMP-1` (`EinmoReview`) — the session object under review; its own
    P0–P12 maintainer-defect record is the convention this EIMP follows.
    §S.2 (drift), §S.5 (claims, suite lock), §S.6 (journal), §S.7/§S.7a
    (transport and the private-socket shape) are all load-bearing here.
  - `EIMP-2` (review server, `complete`) — the HTTP surface; §8 ports
    `zweimomo`; §3a the typed-extractor discipline; §4 the passphrase
    handling.
  - `EIMP-7` (`EinmoCase`/`EinmoSuite`/`EinmoDirectory`, `complete`) — the
    precedent for spinning a finding out when blast radius warrants. This
    EIMP does not (Rejected Alternative B).
- External docs:
  - `rust_instructions.md` §1a, §"Don't" (`pub mod`, `.unwrap()`),
    §Concurrency, §HTTP services, §2f (`[lints.rust] unsafe_code`), §7
    (Testing).
  - `AGENTS.md` "Development Rules" — note §S.0: no gate is currently broken.
- Code locations by item:
  - `src/verify.rs`, `src/cli.rs` — P0
  - `src/review.rs` — P2–P6, P27, P29, P37
  - `src/journal.rs` — P1 (refuted), P35, P39
  - `src/suite_lock.rs` — P34, P35
  - `src/review_server.rs` — P7–P16, P28, P32, P36, P39
  - `src/dhtml/review.html` — P26, P31, P33, P38, P41
  - `src/bin/einmo_review_server.rs` — P8, P30
  - `Cargo.toml`, `zweimomo/Cargo.toml` — P18, P24, P40
  - `zweimomo/src/lib.rs`, `zweimomo/src/evaluators.rs`,
    `zweimomo/tests/suites.rs` — P17, P19–P23, P25

## Last Updated

**Date**: 2026-07-31 (2)
**Updated By**: Claude Code (Opus 5)
**Changes**: Verification-and-extension pass over the original z-ai/glm-5.2
review. Ran the toolchain gates and three targeted probes (§S.0), added a
**Triage** verdict to each of P0–P25, and added sixteen findings (P26–P41).
Rejected P1, P16 and P23 as factually false and P0's Blocker classification
as unfounded (clippy is clean); re-characterized P3, P7, P8 and P14; upgraded
P9. New High items: **P26** (stored XSS in `review.html` via a flag reason,
reached through the exact data flow P14 examined and cleared), **P27**
(`execute` applies decisions that were cleared between `plan()` and
`execute()`, contradicting its own comment), **P28** (`POST /einmo/sessions`
accepts any filesystem path). Added §S.1's execution priority, rewrote the
Test Plan around a combined comprehensive test, added Rejected Alternative D,
and resolved four Open Questions. `status: Draft`, `begun: [ ]` — the
Blocker-driven urgency is withdrawn; this now awaits ordinary maintainer
triage.

**Date**: 2026-07-31
**Updated By**: opencode (z-ai/glm-5.2)
**Changes**: Created EIMP-8 — a read-only code review of the einmo library
(`src/review.rs`), the review server (`src/review_server.rs` + binary),
and `zweimomo`, cataloguing twenty-five findings (P0–P25) with locations,
severities, and recommended remediations. `status: Draft`, `begun: [ ]` —
awaiting maintainer triage before any P-item is implemented.
