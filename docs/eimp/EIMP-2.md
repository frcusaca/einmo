---
eimp: 2
title: einmo-review-server — a minimal HTTP prototype of the review/sign/promote/flag loop
author: Claude Code (Sonnet 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-29
supersedes: []
begun: [ ]
---

# EIMP-2: einmo-review-server — a minimal HTTP prototype of the review/sign/promote/flag loop

## Abstract

Stand up a minimal, single-suite HTTP server — `einmo-review-server` — that
hosts one `EinmoReview` (per `EIMP-1`) for the current directory, started via
`cargo einmo-review-server` (mirroring the existing `cargo-einmo` alias-binary
pattern). `scripts/experimental_reviewer.sh` is rewired so that everywhere it
currently shells out to `einmo` directly (`einmo list`, `einmo body`, `einmo
promote`) or mutates the corpus itself (the raw `mv … flagged/` it uses for
flagging today), it instead makes an HTTP request to this server. This is
explicitly a **prototype slice** of `EIMP-1`'s full server design (§S.7) — not
the dhtml frontend, not SSE, not multi-verifier claims, not the journal, not
`CorpusSigner`. It exists to prove out the core review loop — list, inspect,
decide (promote/flag), execute — hosted behind HTTP before investing in the
rest of `EIMP-1`'s surface. `experimental_reviewer.sh`'s name says exactly
what this is: an experiment, not the finished tool.

State ownership moves server-side too (§5): the script's current parallel
bash arrays (`promote_checked`, `flag_stage`/`flag_rel`/`flag_reason`, etc.)
are exactly `EinmoReview`'s `DecisionBook` reimplemented in shell; once
decisions are `PUT` to the server as they're made, the script keeps only the
one array it needs to iterate (the worklist itself) and asks the server for
everything else. To exercise this against real, non-trivial content rather
than synthetic tempdir fixtures, this EIMP also brings a JavaScript-only
(Boa) copy of the `foolish-rust` workspace's `zweimomo` test crate into this
repository (§8) — its own crate, its own real signed test suite, no
cross-repo dependency.

## Motivation

`EIMP-1` specifies the full `EinmoReview` design, but it is large: a
thread-safe session object, a journal, multi-verifier concurrency semantics, a
dhtml frontend, and (§S.11) a whole separate post-quantum attestation
subsystem. Building all of it before proving the *shape* of "the script talks
to a server instead of calling `einmo` directly" is a lot of investment before
the first feedback. This EIMP carves out the smallest useful slice:

- One resident process, one suite (the current directory) — no multi-suite
  routing, no auth beyond what a unix-domain socket's file permissions give
  for free.
- Just enough server surface for `experimental_reviewer.sh`'s actual current
  needs: list the worklist, fetch a verified body, record a decision
  (promote-to-checked, promote-to-verified, or flag), and execute it.
- No journal yet — decisions live in memory only; a server restart loses
  in-flight (undecided/unexecuted) state. This is an explicit, acceptable
  prototype limitation (see Open Questions).

If this slice works and feels right, `EIMP-1`'s fuller design (journal,
concurrency, dhtml, `CorpusSigner`) is the natural next step, informed by
what this prototype teaches. If it doesn't, the cost of finding that out is
small.

## Specification

### 1. What the script actually calls today

`scripts/experimental_reviewer.sh` currently touches einmo in exactly these
ways:

| Script does today | Becomes |
|---|---|
| `"$EINMO" list "$SUITE" [--filter …] [--differing]` | `GET /api/review/items` |
| `"$EINMO" body "$f"` (verified body of a stage file) | `GET /api/review/items/{m}/body/{stage}` |
| raw `mv "$SUITE/$stage/$rel" "$SUITE/$stage/flagged/$rel"` (writes the plaintext advisory note itself) | `PUT /api/review/items/{m}/decision` with `{"kind":"flag","reason":…}` then `POST /api/review/execute` (or an immediate-execute variant — see §3) |
| `"$EINMO" promote output to checked "$SUITE" -- <files>` | `PUT /api/review/items/{m}/decision` `{"kind":"promote","to":"checked"}` per file, then `POST /api/review/execute` |
| `"$EINMO" promote checked to verified "$SUITE" --interactive -- <files>` | same shape, `{"to":"verified"}`, passphrase carried in the execute call body (§4) |
| `\K` (kick) — accumulated locally as `retract_checked`/`retract_verified`; the existing script does not appear to actually invoke `einmo retract` to execute these today (a pre-existing gap, not introduced by this EIMP) | `PUT /api/review/items/{m}/decision` `{"kind":"retract","from":"checked"\|"verified"}`, then `POST /api/review/execute` — this EIMP closes the gap by actually executing kicks, cascade included (`einmo retract`'s existing checked→verified cascade, `transitions.rs`) |
| `u` (revisit) — local array surgery (`drop_from`, `answer_of`) | `GET /api/review/items/{m}` to read the current decision, then `PUT … /decision` to replace it (§5) — or `DELETE … /decision` (`undecide`) if the reviewer backs out to "no decision yet" rather than replacing with a new one |

This table is the entire scope of this EIMP's HTTP surface. Beyond adding
`retract`/`undecide` support (closing the pre-existing kick-execution gap),
nothing in `EIMP-1`'s §S.7 table beyond these rows is built here.

### 2. `EinmoReview` — the minimum viable slice

Only the parts of `EIMP-1` §S.2's `EinmoReview` needed to back the rows
in §1's table above:

```rust
pub struct EinmoReview {
    suite: EinmoSuite,                // immutable after open()
    worklist: RwLock<Worklist>,       // read-mostly; refresh() takes the write lock
    cache: VerifiedCache,             // fingerprint -> verified body; single-flight verification
    decisions: RwLock<DecisionBook>,  // single implicit reviewer — see Open Questions
    exec: Mutex<()>,                  // execution (disk mutation + signing) is exclusive
}

impl EinmoReview {
    pub fn open(suite: &Path, opts: ReviewOpts) -> Result<Self>;
    pub fn items(&self) -> Vec<ReviewItem>;
    pub fn body(&self, m: &MirrorPath, s: Stage) -> Result<Arc<VerifiedBody>>;
    pub fn decide(&self, m: &MirrorPath, d: Decision) -> Result<Option<Decision>>;
    pub fn undecide(&self, m: &MirrorPath) -> Option<Decision>;
    pub fn plan(&self) -> ExecutionPlan;
    pub fn execute(&self, plan: &ExecutionPlan, keys: &SignerSet) -> ExecutionReport;
}
```

`Decision` (`EIMP-1` §S.3) keeps `Promote { to }`, `Retract { from }`, `Flag {
stage, reason }`, and `Skip` — all four, not a subset. The script's existing
verb set (`\C`/`\V` promote, `\K` kick/demote, note→flag, `q`/skip, `u`
revisit) exercises all four, so this prototype needs all four to actually
replace it; only paring the *object model* down (below), not the decision
vocabulary, is in scope for "minimum viable."

Dropped from `EIMP-1`'s full sketch, for this EIMP only: `diff` (the script
doesn't call it — vim's own diff mode handles that locally today),
`execute_one` (batch-only for now — the script already gates promotions as
one batch, per the existing "promoted (%d): …" summary), `refresh` (no
live-mutation detection yet), and `ReviewerId` (single implicit reviewer per
server instance — see Open Questions). `undecide` **is** kept — see §5 — as
the server-side mechanism a revisit (`u`) resolves through, alongside
`decide`'s own replace-not-stack behavior. `Signer`/`SignerSet` (`EIMP-1`
§S.4) are unchanged and still used exactly as specified — key custody stays
out of `EinmoReview` here too.

### 3. The HTTP surface

Binds a **unix-domain socket** by default, same rationale as `EIMP-1` §S.7
(inherits directory permissions; no token machinery needed for a
localhost-only prototype). TCP + bearer token is **out of scope** for this
EIMP (`EIMP-1` §S.7 already specs it; add it there when a browser frontend
actually needs it).

| Method | Path | Body | Meaning |
|--------|------|------|---------|
| GET    | `/api/review/items` | — (query: `filter`, `differing`) | worklist rows, mirrors `einmo list` |
| GET    | `/api/review/items/{m}` | — | one item's detail, incl. its current decision (if any) — read before a revisit (§5) |
| GET    | `/api/review/items/{m}/body/{stage}` | — | verified body content, mirrors `einmo body` |
| PUT    | `/api/review/items/{m}/decision` | `{"kind":"promote","to":"checked"\|"verified"} \| {"kind":"retract","from":"checked"\|"verified"} \| {"kind":"flag","reason":string} \| {"kind":"skip"}` | record (or replace — replace-not-stack, `EIMP-1` §S.3) a decision — all four `Decision` variants |
| DELETE | `/api/review/items/{m}/decision` | — | `undecide` — clear a decision back to "untouched" (§5's revisit path, when the reviewer backs out rather than replaces) |
| GET    | `/api/review/plan` | — | structured plan preview (what execute would do) — also doubles as the end-of-pass summary the script renders (§5) |
| POST   | `/api/review/execute` | `{"confirm":"PROMOTE","passphrase"?:string}` | apply all pending decisions; flags AND retracts execute unconditionally (no gate — flags per `EIMP-1` §S.3; retracts are a local demotion, not a new signature, so the same "no gate" treatment applies), promotions require the `confirm` token |

**`PUT … /decision` is sent the moment the reviewer decides, not batched
client-side** — see §5: the server's `DecisionBook` is the single
accumulating store, so the script never needs a local copy to dump at the
end. Flags execute (via their own `POST /execute`) right after their `PUT`;
promotions accumulate as pending decisions across the whole pass and are
applied together by one gated `POST /execute` at the end.

Retracts execute like flags do — immediately, no `confirm` gate (they demote
an already-signed artifact locally; they do not themselves produce a new
signature the way a promotion does). No `/api/review` session-summary
endpoint, no SSE — not needed by the script's current flow. Every response
is JSON; the script parses with whatever the shell has available (see §6 —
this may mean adding a `jq` dependency to the prototype, or having the
server format script-friendly plain text as an alternative — an Open
Question).

### 4. Signing stays exactly as `EIMP-1` §S.4 specifies

No change from `EIMP-1`: `Signer`/`SignerSet` is a separate object from
`EinmoReview`; the passphrase for a `checked to verified` execute arrives
only inside the `POST /api/review/execute` request body, is derived into a
key, used under the `exec` mutex, and dropped. `output to checked`
promotions use the computer/empty-passphrase key, same as today's
`--passphrase ""`-style default (see `Cargo.toml`/`einmo.toml` conventions
already in this crate).

### 5. State ownership moves server-side — the script tracks indices, not decisions

Today `experimental_reviewer.sh` re-implements a decision store in bash: the
parallel arrays `promote_checked`, `promote_verified`, `retract_checked`,
`retract_verified`, `flag_stage`/`flag_rel`/`flag_reason`,
`send_to_agent_list`, `skip_list`, and `noop_list` are, collectively, exactly
what `EinmoReview`'s `DecisionBook` already is (`EIMP-1` §S.3) — a map from
test to the reviewer's current decision. Once decisions are recorded
server-side via `PUT … /decision` as the reviewer makes each one (not
accumulated locally and dumped in a batch at the end), the script no longer
needs to hold or replay its own copy. It becomes a thin loop:

- The one array the script still needs is `rows` — the ordered worklist from
  `GET /api/review/items` — because the script drives a `for i in
  "${!rows[@]}"` loop and needs a stable per-test path to reference in each
  request. Everything about *what has been decided so far* is a question the
  script asks the server, not state it accumulates.
- The results/stats summary at the end of a review pass (today computed by
  iterating the local arrays) becomes a `GET /api/review/plan` call — the
  structured plan **is** the summary; the script renders it rather than
  building its own count.
- Kicks (`\K`, demote/retract) get the same treatment: the script currently
  accumulates `retract_checked`/`retract_verified` locally but — per §1's
  table — does not appear to actually execute them against `einmo` today.
  This EIMP both moves that state server-side (a `Decision::Retract`, same
  as any other decision) **and** closes the execution gap: a kick becomes a
  `PUT … /decision` (`kind: retract`) that actually executes (§3), so
  `\K` in the rewired script does what its name has always implied.
- `undo_last_decision`, `answer_of`, and `drop_from` (the array-surgery
  functions this file's bash currently needs for revisits — see `EIMP-1`
  Motivation, which names these as exactly the ~80 lines of shell
  reimplementing "a map from test to current decision") are **deleted**.
  A revisit becomes: `GET /api/review/items/{m}` to read the current
  decision (if any, for display), then either a fresh `PUT … /decision`
  that replaces it (`EinmoReview::decide`'s replace-not-stack semantics,
  `EIMP-1` §S.3) or, if the reviewer backs out to "no decision yet" rather
  than choosing a new one, a `DELETE … /decision` (`undecide`, kept in
  this EIMP's slice — §2).

### 6. `experimental_reviewer.sh` changes

- A new startup check: does a `einmo-review-server` UDS socket exist for this
  suite? If not, **fail with a clear message telling the user to start it**
  (`cargo einmo-review-server <suite>` or `einmo-review-server <suite>`) —
  unlike `EIMP-1` §S.8's eventual "no-server fallback," this prototype does
  **not** keep a direct-`einmo` fallback path. The whole point of this EIMP
  is to prove the HTTP-only shape; a silent fallback would hide whether it
  actually works end to end.
- Replace the `"$EINMO" list …` call with a `curl --unix-socket` GET against
  `/api/review/items`, kept as the one local array (`rows`) per §5.
- Replace the `"$EINMO" body "$f"` calls with GETs against
  `/api/review/items/{m}/body/{stage}`.
- Replace the raw `mv … flagged/` with a `PUT … /decision` (`kind: flag`)
  followed immediately by its own `POST /execute`, sent the moment the
  reviewer flags a test (not batched) — flags still run unconditionally, no
  gate, matching current behavior (§3).
- Replace the local `retract_checked`/`retract_verified` accumulation
  (currently dead state — §1/§5) with a `PUT … /decision` (`kind: retract`)
  followed immediately by its own `POST /execute`, wired to `\K` — kicks now
  actually execute, closing the pre-existing gap (§5).
- Replace the local `promote_checked`/`promote_verified` accumulation with a
  `PUT … /decision` sent per-test as each decision is made (§5); at the end
  of the pass, one gated `POST /execute` (reading the plan the server already
  holds) carries the typed `PROMOTE` confirmation and, when any pending
  decision promotes to `verified`, the passphrase (read from `/dev/tty`
  exactly as the script does today).
- Revisits (`u`) become a `GET … /items/{m}` followed by either a re-`PUT`
  (replace) or a `DELETE … /decision` (`undecide`, to back out entirely) on
  the same test path, per §5 — no local array surgery.
- `Cargo-installed` binary requirement: `curl` must exist on the reviewer's
  machine (already true — the script's environment assumptions don't
  change).

### 7. Binary and installation

Mirrors the existing `einmo`/`cargo-einmo` pattern (`Cargo.toml`'s `[[bin]]`
entries):

```toml
[[bin]]
name = "einmo-review-server"
path = "src/bin/einmo_review_server.rs"

[[bin]]
name = "cargo-einmo-review-server"
path = "src/bin/cargo_einmo_review_server.rs"
```

`cargo einmo-review-server <suite>` runs the resident process in the
foreground (Ctrl-C to stop); it is the reviewer's responsibility to run it in
a second terminal/tmux pane/background job — no daemonization, no
service-manager integration in this EIMP.

### 8. A real test suite — `zweimomo` (Boa/JavaScript only) in this repo

The integration tests in §Test Plan need a suite with **real, non-trivial
`.einmo` content** across `input/`/`output/`/`checked/` — hand-built fixtures
would either be too thin to catch real bugs or amount to reinventing what
`zweimomo` (the `foolish-rust` workspace's companion einmo test crate,
`FOOP-92` §Use Case D) already is. Rather than build a second, bespoke fixture
generator, this EIMP brings a **JavaScript-only** copy of `zweimomo` into this
repository as its own crate:

- **New crate `zweimomo/`** at this repo's root, alongside `src/`, with its
  own `Cargo.toml` — depends on `einmo` (path dependency to this repo, `.` /
  workspace-relative) and `boa_engine` (pinned, matching the version already
  used in `foolish-rust`'s `zweimomo`). It does **not** depend on
  `foolish-ubca`/`foolish-core` — no `UbcaEvaluatorAdapter`, no Foolish input,
  no cross-repo path dependency into `/yolo/src`. Ported components, from
  `foolish-rust`'s `zweimomo/src/evaluators.rs`: `BoaEvaluator` and its unit
  tests only (the `RustPythonEvaluator`/`UbcaEvaluatorAdapter` code and their
  tests are not copied).
- **Ported test suite**: the `suites/javascript/` tree (`input/`, `output/`,
  `checked/`) from `foolish-rust`'s `zweimomo`, copied as-is — eight `.js`
  concept inputs (integer arithmetic, nested expressions, name binding, data
  structures, function application, division-by-zero, search-query,
  nested-expressions-with-division-by-zero) with their existing signed
  `output/`/`checked/` baselines. This is real, previously-reviewed content —
  not fixtures invented for this EIMP.
- **Purpose here, specifically**: `zweimomo`'s `suites/javascript/` tree is
  what `einmo-review-server` (§2–§7) is pointed at for the integration tests
  in §Test Plan — a real suite with real signed baselines, several files,
  and a mix of already-`checked`/already-verified-or-not content, so the
  end-to-end script test in §Test Plan exercises list/body/flag/promote
  against actual data instead of a synthetic one-file tempdir.
- **Not a general `foolish-rust` `zweimomo` replacement.** The
  `foolish-rust` workspace's own `zweimomo` crate is untouched by this EIMP
  and keeps all three evaluators (Foolish, Python, JavaScript) for now;
  removing `rustpython-vm` there is tracked separately (see
  `docs/todo/AIAGENT-einmo-extraction.todo.md` in `foolish-rust`) and is not
  part of this EIMP's scope.

## Test Plan

Tests are written first, per project rules.

- **Unit — `EinmoReview` minimum slice**: `items()` reflects `einmo list`'s
  existing walk semantics (reuse/adapt any existing `EinmoSuite`-level test
  fixtures); `body()` is single-flight verified (N concurrent requests for
  one artifact → one verification, per `EIMP-1`'s cache design, test-hook
  counter); `decide()` replace-not-stack for a single implicit reviewer, all
  four `Decision` variants (`Promote`, `Retract`, `Flag`, `Skip`);
  `undecide()` clears back to untouched; `execute()` promotion is
  byte-for-byte equivalent to the existing CLI `einmo promote`; `execute()`
  retract is byte-for-byte equivalent to the existing CLI `einmo retract`
  (including its checked→verified cascade, `transitions.rs`); flag execution
  matches the script's current `mv` behavior (moves to `flagged/`, writes
  the plaintext advisory line).
- **Unit — server endpoints**: each route in §3, against a tempdir suite;
  malformed decision bodies (including an invalid `kind`) rejected with a
  clear 4xx; execute without `confirm: "PROMOTE"` refused for promotions but
  not for flags or retracts (§3's "no gate" rule for those two); `DELETE`
  on an undecided item is a no-op, not an error; passphrase never logged,
  never retained past the execute call.
- **Integration — script against a live server**: a pty-driven end-to-end
  run of the updated `experimental_reviewer.sh` (reusing the stub-vim
  technique referenced in `EIMP-1`'s test plan) against a real
  `einmo-review-server` instance **pointed at `zweimomo`'s `suites/javascript/`
  tree** (§8) — not a synthetic tempdir. Exercises the full decision
  vocabulary, step by step: list the worklist; view a body; **approve**
  (promote output→checked, and separately checked→verified with a
  passphrase); **kick** (retract a checked artifact, confirm the
  verified-cascade removal per `transitions.rs`); **flag** (with a reason,
  confirm the plaintext note lands and the test now fails
  `EinmoSuite`'s validation per `EIMP-1` §S.3); **undo** (revisit a decided
  test, both the replace path and the back-out-to-undecided/`DELETE` path,
  confirm the prior decision truly no longer applies at execute time). Each
  of these is verified independently before being chained, per the
  step-by-step incremental approach in the plan. Confirm every resulting
  `.einmo`/`flagged/` state matches what the equivalent direct `einmo` CLI
  calls would have produced.
- **Integration — no-server behavior**: starting `experimental_reviewer.sh`
  with no server running fails fast with the documented message; it does
  NOT silently fall back to direct `einmo` calls (§6).
- **Unit — `zweimomo` (Boa) port (§8)**: `BoaEvaluator`'s existing unit
  tests (integer arithmetic, `Math.floor` division, division-by-zero →
  `Infinity`, a thrown error → `Err`) pass unchanged in the new crate; the
  ported `suites/javascript/` fixtures evaluate via `EinmoSuite` and match
  their existing `checked/` baselines byte-for-byte (proving the port didn't
  silently change anything).

## Rejected Alternatives

### A. Build the full `EIMP-1` server (§S.7) first, use only the rows this script needs

Rejected: `EIMP-1`'s full server includes SSE, claims, multi-verifier
concurrency, and a journal — real design and implementation weight that this
prototype does not need to answer the question "does routing the script
through HTTP work at all." Building the minimum slice first, then growing
into `EIMP-1`'s full shape once the prototype validates the approach, is
cheaper and gives earlier feedback.

### B. Keep a direct-`einmo` fallback in the script (as `EIMP-1` §S.8 eventually wants)

Rejected for this EIMP specifically (though correct for the eventual
production tool): a fallback path would let the script "work" even if the
HTTP wiring were subtly broken, masking exactly the thing this prototype
exists to test. `EIMP-1` §S.8 is unchanged and still the target for the
fuller tool.

### C. Multi-reviewer support from the start

Rejected: `EIMP-1` §S.5's per-reviewer decision isolation and soft claims are
real design surface. A single implicit reviewer (whoever is talking to the
one resident server process) is sufficient to prove the HTTP-routing shape
and defers that complexity to when `EIMP-1` proper is implemented.

### D. Do nothing — keep `experimental_reviewer.sh` calling `einmo` directly

Rejected: the entire point of `EIMP-1`'s eventual design is that repeated
`einmo` process spawns re-verify the same stamp chains on every call; a
resident, cache-holding server is the fix. This EIMP is the smallest step
that proves that fix's shape works before committing to the rest of
`EIMP-1`.

### E. Keep the script's local decision arrays, batch-`PUT` them at the end

The script could still accumulate `promote_checked`/`flag_stage`/etc.
locally during the review pass and send them all as one batch of `PUT`
calls right before `execute`, rather than sending each decision the moment
it's made (§5). Rejected: this keeps exactly the bash-side bookkeeping
(`undo_last_decision`, `answer_of`, `drop_from`) that `EIMP-1`'s Motivation
names as the ~80 lines of shell reimplementing a decision map — the local
arrays would just be a second, redundant copy of what
`EinmoReview::DecisionBook` already tracks. Sending each decision as it's
made keeps the server as the single source of truth throughout the pass
(not just at the end), makes a revisit a simple re-`PUT` instead of local
array surgery, and means `GET /api/review/plan` can serve as the running
summary at any point, not just after a client-side replay.

### F. Hand-build synthetic fixtures instead of porting `zweimomo`

Rejected: hand-built fixtures for the integration tests would either be too
thin (a couple of trivial files) to exercise the review loop meaningfully,
or would amount to rebuilding a smaller, worse version of `zweimomo`'s
existing, already-reviewed `suites/javascript/` tree. Porting the real
suite is less work and gives more realistic coverage — see §8.

### G. Port all three `zweimomo` evaluators (Foolish, Python, JS), not just Boa

Rejected for this EIMP: `UbcaEvaluatorAdapter` needs `foolish-ubca`/
`foolish-core`, which live in a different repository (`foolish-rust`) — a
cross-repo path dependency this EIMP explicitly avoids (see the einmo-repo
`zweimomo`'s scope note in §8). `RustPythonEvaluator` pulls in
`rustpython-vm`, which is exactly the FFI-ish/apt-install build weight the
user separately wants trimmed from `foolish-rust`'s own `zweimomo` (tracked
in that repo's own todo list, not part of this EIMP). Boa alone is
pure-Rust, has no cross-repo dependency, and is sufficient to provide a
real, non-trivial signed test suite.

## Open Questions

- **State loss on restart.** Decisions live in memory only (no journal in
  this EIMP). Is that an acceptable prototype limitation, or should a
  minimal append-only decision log be included even at this stage? Leaning
  "acceptable for now" — confirm with human before Phase A.
- **Script JSON parsing.** Does `experimental_reviewer.sh` gain a `jq`
  dependency, or does the server offer a script-friendly plain-text mode
  (`Accept: text/plain` or a `--script`-style query param) for the specific
  endpoints the script needs? Affects both the server response format and
  what the script's environment now requires.
- **Immediate-execute for flags.** §3 has flag-then-execute as two calls
  (`PUT decision` + `POST execute`); should there instead be a single
  `POST /api/review/items/{m}/flag` convenience endpoint that does both
  atomically, closer to matching the script's current single `mv` call
  site? Leaning toward the convenience endpoint; confirm at begun-time.
- **Socket discovery path.** Where does the UDS socket file live (suite-root
  dot-file vs. a scratch/state dir)? Same open question as `EIMP-1`'s
  journal-location question; should probably be answered consistently for
  both.

## References

- **EIMP-1** — the full `EinmoReview` design this EIMP prototypes a slice
  of. §S.2 (`EinmoReview` object), §S.4 (`Signer`/`SignerSet`), §S.7 (the
  full server table), and §S.8 (the eventual thin-client script with a
  no-server fallback) are all referenced above and remain the target design
  once this prototype has proven itself.
- **EIMP-0** — the EIMP process itself.
- **FOOP-92** (`foolish-rust`, Complete) §Use Case D — `zweimomo`'s origin
  design (why three interpreters, why pure-Rust-only, the `Evaluator`
  contract each satisfies). §8 of this EIMP ports only the Boa slice.
- Code: `src/{einmo_suite,transitions,signature,verify,format,compare}.rs`;
  `scripts/experimental_reviewer.sh`; `Cargo.toml` (existing
  `einmo`/`cargo-einmo` `[[bin]]` pattern this EIMP's binaries follow);
  `foolish-rust`'s `zweimomo/src/evaluators.rs` (`BoaEvaluator`, ported by
  §8) and `zweimomo/suites/javascript/` (ported test fixtures, §8).
