---
eimp: 1
title: EinmoReview — a thread-safe review-session object; thin bash, server, and dhtml frontends
author: Atlas <hc.busy@gmail.com> (ported by Claude Code (Sonnet 5) <noreply@anthropic.com>)
status: Implementing
type: Standards
created: 2026-07-19
supersedes: []
begun: [x]
---

# EIMP-1: EinmoReview — a thread-safe review-session object; thin bash, server, and dhtml frontends

**Retroactive / ported document.** This EIMP documents a decision originally
made on 2026-07-19 as `FOOP-25` in the `foolish-rust` workspace (before
einmo was extracted into its own repository). The content below is ported
with the following adaptations from the original:

- All references to `poor_einmo.sh` refer to what is now
  `scripts/experimental_reviewer.sh` in this repository.
- Worktree/branch mechanics (`WORKTREE_ORIGIN_BRANCH=jia`, the
  `foop-25-einmo-review-session` worktree, merge-to-`jia`) are dropped —
  einmo's own EIMP process (`EIMP-0` §8) executes plans directly on `main`.
  A corresponding `EIMP-1.plan.md` is provided, adapted to einmo's simpler
  single-repo workflow.
- Cross-references to `FOOP-15` (secured interactive review, MCP) and
  `FOOP-64` (the einmo suite migration and `poor_einmo.sh`'s original
  prototype work) point at documents that exist only in `foolish-rust`, not
  in this repository. They are kept as historical citations (§References)
  since they explain the design's provenance, but they are not actionable
  EIMPs here — if this repository ever needs their content, it should be
  ported the same way this document was.
- The status was left as `Draft`/`begun: [ ]` when first ported (2026-07-29):
  the design had not been implemented and no `einmo::review` module existed
  in `src/`. As of 2026-07-30, the Open Questions below are resolved and
  work has begun (`status: Implementing`, `begun: [x]`) — see
  `EIMP-1.plan.md` for progress.

Everything else below is the original specification (adjusted only where
the Open Questions section below records a resolution), unchanged in
substance otherwise.

## Abstract

Extract the einmo review *session* — the worklist, the reviewer's evolving
decisions, verified-body access, and deliberate signed execution — out of
`scripts/experimental_reviewer.sh` and into a thread-safe Rust object,
`EinmoReview`, in the einmo crate. Key custody (passphrase → key → sign) is
deliberately **not** part of the review object: it lives in a separate
`Signer` object that the review *uses* at execution time, supporting both
promote-one-at-a-time and accumulate-then-sign-at-the-end from a single
passphrase entry. One running review is exposed through a small server API;
`scripts/experimental_reviewer.sh` shrinks to a thin, fast client (vim stays
the editor), and a dhtml page talking to the same server replaces vimdiff as
the first browser frontend. This EIMP is the session layer that a future
secured-interactive-review EIMP (perspectives, MCP — see the original
`FOOP-15` in `foolish-rust`) would attach to. It also adds (§S.11) a
**layered post-quantum section attestation**: a conservative
SPHINCS+/SLH-DSA signature over a whole stage section (manifest +
byte-joined files), recomputed when the section updates, on top of — never
replacing — the existing per-file Ed25519 stamps.

## The Aspirational Goal

**One review, one object, every surface a thin view.**

A review session should be a first-class thing: it knows which tests need
attention, what every stage's verified body says, what the reviewer has
decided so far, and what will happen when those decisions are executed.
Humans and agents should meet the *same* object through whatever surface is
at hand — a bash loop driving vim, a browser page, an MCP tool — and the
invariants (verify-on-inspect, replace-not-stack decisions, deliberate
attested execution) should hold identically everywhere because they are
enforced in exactly one place. Verification is paid once and remembered, so
review runs at the speed of reading, not the speed of re-verifying. Signing
stays a human act with its own object, its own lifetime, and its own
confirmation — never a side effect. And the corpus's story ("who decided
what, when, and with which key") is written down as it happens.

When this EIMP is complete, `scripts/experimental_reviewer.sh` is a dumb
terminal loop an evening's read long, the browser page is a courtesy view
over the same API, and the next frontend — a perspective-rich SPA or an
agent reviewer — costs an afternoon, not a rewrite.

**The goal state, stated plainly (once, in one place):** a healthy suite has
**no flags, every artifact signed, and every artifact matching** — output
matches checked matches verified (at the suite's level), and for every stamp
the **public signature verifies against the key the passphrase derives**
(the signer is who they claim to be; no computer key masquerading as a human
`verified` stamp). Flags are the explicit exception to "healthy": a flag is
a red mark that breaks the suite until a human resolves it. "Green"
therefore means zero flags + all-signed + all-matching + all-signatures-valid;
anything less is not done.

## Motivation

The review stopgap (`scripts/experimental_reviewer.sh`, née
`poor_einmo.sh`) proved the review *protocol* — panes, verbs,
replace-not-stack decisions, revisits, the PROMOTE gate — and every lesson
was learned the hard way, in bash arrays:

- The revisit/undo machinery (`undo_last_decision`, `answer_of`,
  `drop_from`) is ~80 lines of shell reimplementing "a map from test to
  current decision".
- Loop-control bugs (a `continue` that re-opened the same test forever) and
  ordering bugs (`show_cmd` called before its definition) shipped and were
  found by users — state machines in bash have no tests.
- Every run re-verifies every stamp chain via 3 `einmo body` spawns per test
  (~500 process spawns per full pass of a 161-test suite). The review is
  slower than it has any right to be.
- The pane-verb protocol ("type `promote` as the whole pane") exists only
  because vim has no action channel back to bash. It works, but it is a
  workaround, not a design.

The session logic wants to be a library with unit tests; the speed wants a
resident process that verifies once; the UIs want a real action channel. All
three are the same refactor.

## Supported Use Cases

1. **The solo loop, faster** — a reviewer runs
   `scripts/experimental_reviewer.sh`; between tests the bodies arrive from
   the server's verified cache in milliseconds instead of hundreds of ms of
   spawn+verify.
2. **Accumulate, then sign once** — review 40 tests, decide on each, and at
   the end type the PROMOTE confirmation and one passphrase; every
   checked→verified promotion is signed from that single entry.
3. **Promote as you go** — for a long session, execute each decision
   immediately after it is made; the session-scoped `Signer` (derived once)
   makes per-item signing as cheap as batch.
4. **Browser review** — the same 4-pane inspection (input│output│checked│verified)
   as a dhtml page: server-computed diffs, verbs as buttons, the gate as a
   typed confirmation. Replaces vimdiff for reviewers who prefer a browser;
   vim remains fully supported.
5. **Multiple verifiers, concurrently** — two humans (or a human and an
   agent) review the same suite at once; decisions are per-reviewer, stamps
   accumulate (two `verified` stamps are *stronger*, not a conflict), and
   soft claims prevent duplicated effort.
6. **Agent reviewers** — an AI agent lists, inspects, decides, and (with its
   own key) executes through the same API.
7. **Resume after a crash** — the journal replays a session's decisions;
   nothing a reviewer decided is lost to a dropped ssh connection.
8. **Audit** — "who decided what, when, with which key" is answerable from
   the journal plus the stamp chains, per file.

## Specification

### S.1 The three layers

```
einmo core (exists)   format · signature · verify · stage · transitions · compare · EinmoSuite · CorpusSigner
review session (NEW)  EinmoReview — session state, decisions, cache, plan/execute, journal
frontends (thin)      review CLI verbs · `einmo review serve` · einmo_review_client.sh · dhtml
```

All frontends call the same `EinmoReview`; no frontend writes `.einmo` bytes
or touches key material.

**Crate boundary (resolved 2026-07-30, specified by `EIMP-4` §S.1).** These
three layers do not all ship in one crate. `EIMP-4` splits the repository
into published `einmo` (core: the top line above, `CorpusSigner` included)
and published `einmo-review-server` (the bottom two lines: `EinmoReview`
itself, the HTTP server, the TUI client script, and the dhtml frontend).
Two consequences bind this EIMP's remaining work:

- **Core must stay dependency-lean.** No `axum`/`tokio`/`hyper`/`tower` may
  land in core — which is why §S.11's `CorpusSigner` ships single-threaded
  here and its parallel machinery is deferred to `EIMP-5`.
- **Phase B's `einmo review …` verbs belong to `einmo-review-server`.**
  They operate on `EinmoReview`, which lives in that crate; they are that
  crate's binary's subcommands, not core `einmo`'s `cli.rs`.

The split itself is `EIMP-4`'s work, executed *after* this EIMP completes.
This EIMP need only avoid building anything that would make the split
harder — chiefly, keeping core free of the HTTP stack and of an async
runtime.

### S.2 The `EinmoReview` object

Thread-safe by construction (`Send + Sync`); the server holds one
`Arc<EinmoReview>`. Interior mutability is partitioned by contention:

```rust
pub struct EinmoReview {
    suite: EinmoSuite,               // immutable after open()
    worklist: RwLock<Worklist>,      // read-mostly; refresh() takes the write lock
    cache: VerifiedCache,            // fingerprint -> verified body; single-flight verification
    decisions: RwLock<DecisionBook>, // per-item, per-reviewer, versioned
    journal: Journal,                // Mutex<append-only writer>
    exec: Mutex<()>,                 // execution (disk mutation + signing) is exclusive
}

impl EinmoReview {
    pub fn open(suite: &Path, opts: ReviewOpts) -> Result<Self>;   // opts: mode, filter
    pub fn items(&self) -> Vec<ReviewItem>;                        // worklist rows + current decisions
    pub fn body(&self, m: &MirrorPath, s: Stage) -> Result<Arc<VerifiedBody>>;
    pub fn diff(&self, m: &MirrorPath, l: Stage, r: Stage) -> Result<DiffHunks>;
    pub fn decide(&self, r: ReviewerId, m: &MirrorPath, d: Decision) -> Result<Option<Decision>>;
    pub fn undecide(&self, r: ReviewerId, m: &MirrorPath) -> Option<Decision>;
    pub fn decision(&self, r: ReviewerId, m: &MirrorPath) -> Option<Decision>;  // "answer so far"
    pub fn plan(&self, r: ReviewerId) -> ExecutionPlan;            // pure preview
    pub fn execute(&self, plan: &ExecutionPlan, keys: &SignerSet) -> ExecutionReport;   // batch
    pub fn execute_one(&self, r: ReviewerId, m: &MirrorPath, keys: &SignerSet) -> Result<Executed>;
    pub fn refresh(&self) -> Vec<MirrorPath>;                      // rescan; stale decisions flagged
}
```

**`ReviewOpts.mode` (resolved — was "does `differing_only` default on?").**
Not a boolean: a runtime-selectable `ReviewMode`.

```rust
pub enum ReviewMode {
    Full,               // every item in the worklist
    Random,             // worklist in randomized order (sampling a large suite)
    NewOrBroken,        // only items with no baseline yet, or a content mismatch
}
```

`NewOrBroken` is what the old `differing_only` boolean was reaching for,
generalized: an item qualifies when its candidate stage's content doesn't
match the next stage up (the same content-section comparison `compare.rs`
already performs — INPUT/OUTPUT[*]/PERSPECTIVE/DIFF sections, never STAMPS),
OR when the next stage has no artifact at all yet. `Full` is the default
(matches `EIMP-2`'s existing unfiltered list behavior — no surprise
narrowing for a script that's always shown everything); `NewOrBroken` and
`Random` are opt-in via `ReviewOpts`.

**Single-flight verification**: `VerifiedCache` maps
`Fingerprint → Arc<OnceLock<VerifiedBody>>`. The map lock is held only to
fetch/insert the entry; verification runs inside `get_or_init` outside the
map lock — concurrent readers of the same artifact trigger exactly one
stamp-chain verification and never block readers of other files.
Verify-on-inspect is preserved (nothing renders unverified); it is paid once
per byte-content, not once per look.

### S.3 Decisions — replace, never stack

```rust
pub enum Decision {
    Promote { to: Stage },              // "output to checked" | "checked to verified"
    Retract { from: Stage },            // checked cascades to verified (library enforces)
    Flag    { stage: Stage, reason: String },
    Skip,                               // looked, deliberately chose not to rule
}
// DecisionBook: MirrorPath -> { ReviewerId -> (Decision, version) }; absence = untouched.
```

`decide` replaces that reviewer's previous decision and returns it;
`undecide` clears it; absence means untouched. This map-shaped invariant
replaces the equivalent bash script's entire `drop_from`/
`undo_last_decision`/`answer_of` machinery. Every item carries a `version`
bumped on decision change or byte change; frontends send it back (If-Match)
so a stale view cannot silently decide about changed content.

**A flag BREAKS THE TEST by default — this is the newly-designed behavior as
of this EIMP.** Previously a flag only moved an artifact into the
`flagged/` sink and it was a matter of interpretation whether that should
fail a run. This EIMP makes it definite: **the presence of any flagged
artifact for a test fails that test** (`einmo test` returns non-zero; the
gate is red). A suite CAN be configured to not treat flags as failures —
`--flag-is-not-failure` (per-suite config or CLI flag) downgrades a flag
from "failure" to "advisory" — **but even then, a flag ALWAYS produces
stderr output announcing its existence** (`einmo: warning: <N> flagged
artifact(s) present: …`). There is no configuration under which a flag is
silent; the most it can be made is non-fatal-but-loud. This keeps flags
impossible to lose: the default punishes them, and the opt-out still shouts.

**Flags break the test and do not diff.** A `Flag` is not a comparison
against a baseline; it is a deliberate "this is wrong, stop and look". The
reviewer's note is kept **in full and in context** (they annotate the
rendered body right where the error is; the whole annotated text is the
note, not just an added line).

**`flagged/` is PLAINTEXT, UNSIGNED, and TRANSIENT — a development-process
component, not a durable signed record.** This EIMP settles a question the
corpus had left open: **flagging writes a plaintext message with no
signature.** A flag is a short-lived "in progress, broken" marker meant to
be resolved and removed, not to persist or be cryptographically attributed.
So `EinmoReview` executing a `Flag` simply writes the note as plaintext into
`flagged/<test>` — and re-flagging **concatenates**: the new dated,
annotated content goes ON TOP, the existing flagged content BELOW, in the
same path. Because it is plaintext by design, there is no envelope to
corrupt and no verification to fail; `flagged/` remains **exempt from the
escalation** exactly as today. Its only job is to **break the test by
default** (S.3, above) until a human resolves it. A pending `Flag` still
replaces on re-edit (normal rule); on execute it concatenates newest-on-top;
concurrent multi-verifier flags serialize under the `exec` mutex so both
dated blocks land, none lost. The journal records who flagged, when, and
with what note.

**Durable, attributed observations go in a NEW signed `notes/` stage — not
`flagged/`.** For an observation meant to LAST — a design note, a reviewed
finding, an attributable comment that should survive past the bug it
describes — `flagged/` is the wrong home (it is transient and unsigned).
This EIMP adds a `notes/` sibling stage that **is signed** (a proper
`.einmo` envelope, verify-on-inspect, stamped like any stage). The same
concatenated annotated content that a flag holds as plaintext can be
promoted into `notes/` as the **signed body of a note** — so a throwaway
flag can graduate into a durable, attributed record. `notes/` participates
in signature checks (its stamps must verify against their
passphrase-derived keys, per the goal state); `flagged/` never does. Rule of
thumb: **`flagged/` is for the development loop and should trend to empty;
`notes/` is for what you want to keep.**

### S.4 Signing is a separate object — the design answer

**Question posed**: should signing-from-passphrase (individually or in
batch) be part of the review process, or a separate object? **Answer: a
separate object.** The review object holds *decisions*; a `Signer` holds
*key custody*. They meet only at execution:

```rust
pub struct Signer { /* Argon2id-derived Ed25519 key; zeroized on drop */ }
impl Signer {
    pub fn from_passphrase(pass: Passphrase) -> Signer;   // derive once; pass is consumed & wiped
    pub fn computer() -> Signer;                          // the empty-passphrase computer/agent key
}
pub struct SignerSet { pub checked: Signer, pub verified: Option<Signer> }
```

Rationale for the separation:

- **Different lifetimes.** Decisions live for the whole session and survive
  crashes (journal); key material should live as briefly as possible and
  never be persisted. One object cannot honor both.
- **Different owners.** A server can hold the review for many verifiers, but
  a key belongs to one human. With a separate `Signer`, the server stages
  decisions all day without ever touching key material; the passphrase
  enters only inside an execute call, is derived, used, dropped.
- **Individual vs batch collapses into one design.** `execute_one` and batch
  `execute` take the same `&SignerSet`. A session-scoped signer derived once
  makes per-item signing as cheap as batch — the reviewer chooses cadence,
  not cost. Deriving per-call remains possible (highest caution mode).
- **Attestation stays honest.** Stage promotions to `checked` may use the
  computer key; promotions to `verified` require a human signer — the
  `SignerSet` shape makes that rule visible in the types.

Execution is always deliberate: `plan()` renders exactly what will run
(today's results block, kept), and the frontend must present it and pass an
explicit confirmation (the typed `PROMOTE` word survives as the API's
`confirm` token). Retractions carry their own confirmation and are never
batched silently.

**Multi-stage promotion of one file, one passphrase.** Because pending
promotions live in the session's decision set, a reviewer deciding a file
needs BOTH `output to checked` and `checked to verified` is just two
decisions on one file (or a `Decision::Promote { to: Verified, through: true }`
convenience meaning "carry it up from wherever it is"). `execute`/
`execute_one` then apply the stages **in lifecycle order** (`output to
checked` before `checked to verified` — the later hop reads the freshly
written checked) under a **single derived `Signer`**, so the human is
prompted at most once for the whole batch, mixed stages included. This is
the durable home of "promote several stages in one go, one passphrase."
Ordered-apply-under-one-key lives in the library so every frontend (bash,
server, MCP) inherits it.

### S.4a Content-then-key decision for `execute`'s promote (multi-signer accumulation)

**Resolved — replaces `transitions::promote`'s always-fresh-copy behavior
for the `EinmoReview::execute` path.** Today's `transitions::promote`
(`src/transitions.rs`) always copies the source stage's file to the
destination and appends exactly one stamp — it never inspects whatever
might already be sitting at the destination. `EinmoReview::execute`
promoting into `checked`/`verified` instead applies the same
content-then-key decision table this EIMP's sibling, `EIMP-3`, gives the
core test-run path for `output` (`EIMP-3.md` §Specification "Content/key
decision table") — restated here for `checked`/`verified`:

| Existing destination file | Content sections match the promotion candidate? | Promoting signer's key already among existing `stage:<dest>` stamps? | Outcome |
|---|---|---|---|
| absent (or corrupt) | n/a | n/a | write fresh, sign, done (today's behavior, unchanged) |
| present | no | n/a | this is a genuine new baseline: write fresh content, fresh stamp chain from scratch (old stamps do not carry over onto different content) |
| present | yes | yes | no-op: destination file stays byte-for-byte untouched, no rewrite, no timestamp change |
| present | yes | no | **append** the promoting signer's `stage:<dest>` stamp to the *existing* destination file in place; every prior stamp (including other signers') is preserved |

"Was it signed by me" (today's implicit single-signer assumption) becomes
"is at least one of the existing stamps mine — others may also be present
and are left alone." This is the semantics `S.5` below already describes in
prose ("Multiple `verified` stamps are accumulated attestation"); this
subsection makes it a concrete, implementable decision table and extends it
to `checked` as well as `verified` (previously only `verified` was
described as accumulating). Content comparison and the exact-pubkey stamp
lookup are the same primitives `EIMP-3` introduces (`Stamps`'s exact-pubkey
lookup, alongside the existing prefix-based `stamped_by`) — implementers
should share that helper between the two EIMPs where it falls out naturally
rather than duplicating it, per `EIMP-3.md`'s own scope-boundary note.

### S.5 Concurrency semantics for multiple verifiers

- Per-reviewer decisions coexist; replace-not-stack holds *within* a
  reviewer. Executing appends that reviewer's stamps; a second verifier
  executing later appends theirs (§S.4a's decision table). Multiple
  `checked`/`verified` stamps are accumulated attestation, surfaced via
  `Stamps::stamped_by`.
- Soft claims (`claim(m, ttl)`) advertise "I'm on this one" in listings;
  advisory only, cannot wedge. **Default TTL: 5 minutes (resolved)** —
  short enough to suit an interactive review pass; an expired claim is
  reclaimed automatically (silently released back to the pool, no action
  needed from the original claimant) rather than requiring an explicit
  release call. **Active claims ARE surfaced in `plan()`'s output
  (resolved)** — a reviewer sees what another reviewer currently holds (and
  its remaining TTL) before deciding, so two reviewers don't collide on the
  same item.
- The `exec` mutex serializes disk mutation; each write re-checks the file
  fingerprint first — anything drifted since planning is skipped-and-
  reported, never clobbered.
- An advisory lockfile makes a second *server* on the same suite refuse to
  start; external CLI mutations are caught by `refresh()`.

### S.6 The journal

Append-only JSONL per session, under a **scratch/state directory** (resolved
— not a suite dot-file: the journal is ephemeral session/process state, not
part of the reviewed corpus, and should not travel with it or show up in
`git status` for the suite's own repository). Path follows the same
scratch-dir hardening `EIMP-2`'s client script already established
(`einmo_review_client.sh`'s `umask 077`/`harden_dir` pattern) — one journal
file per session id. Contents: session id, reviewer, timestamp,
produced_by, every decide/undecide/claim/execute with outcomes. Reopen =
replay. This is the audit and crash-recovery substrate.

**Keyed by `EinmoId` end to end (resolved 2026-07-30).** Every entry that
concerns a case carries its `EinmoId` (§0) as the identifying field — not a
path, not an index, not a display name. One identifier from the client's
keystroke through the server's handler to the journal line means a journal
can be joined against `items()`, against a plan, and against the corpus
itself without any translation layer that could disagree.

**Verbosity levels.** The journal writes at a configurable level, so a
routine session stays readable while a debugging session records
everything:

| Level | Records |
|---|---|
| terse | session open/close, `execute` batches and their outcomes |
| normal (default) | the above, plus every decide/undecide/claim |
| fine | the above, plus **each case as it is read in and verified** — one entry per `EinmoId` per verification, which is what makes the journal able to answer "which case was in flight when this crashed?" |

**Enough to serve the crash crumb's purpose — but not (yet) its
replacement.** At `fine`, a case that begins verification and never records
its completion leaves an unmatched entry, which identifies the in-flight
case after a crash *strictly more precisely* than today's crash crumb does
— and without the crumb's side effect of writing a placeholder `.einmo`
into `output/`. (That side effect is not hypothetical: it is exactly what
forced the `"TEST IN PROGRESS"` special-case into `EIMP-3`'s content/key
decision table in `write_output`.) **This EIMP only makes the journal
*capable* of that role; it does not retire the crash crumb.** Retirement
touches `einmo_suite.rs`'s test-run path — a different layer from the
review session — and would invalidate existing tests
(`crash_crumb_survives_stack_overflow` in `zweimomo`, einmo's
`catastrophe_crumb_*` tests). It is carried as a follow-up logging EIMP in
`EIMP-7` (`docs/eimp/EIMP-7.md`), alongside the broader logging design.
Per `EIMP-7` §S.3, crash-crumb work is frozen as of 2026-07-30: the
mechanism keeps working untouched, but gains no new features or consumers
while it is scheduled for removal.

**Quorum policies are explicitly OUT of scope for this EIMP (resolved).**
"Quorum" is not yet a defined concept in einmo — this EIMP only facilitates
*multiple parties independently signing* the same stage (§S.4a's
accumulation), with no N-of-M policy engine deciding when that's "enough."
Whether some future gate should require e.g. 2 distinct human stamps before
treating `verified` as complete is left for a follow-up EIMP if and when
it's needed; nothing here blocks building it later, since the journal
already records every stamping event a quorum policy would need to read.

### S.7 The server — one running review

`einmo review serve <suite>`: binds a **unix-domain socket by default**
(`curl --unix-socket`; inherits directory permissions — the mode-700
discipline `scripts/experimental_reviewer.sh` already established), TCP on
127.0.0.1 with a bearer token only when a browser needs it. Handlers are
thin translations onto `Arc<EinmoReview>`.

#### S.7a TUI-owned private server (resolved 2026-07-30)

**The TUI starts its own server and kills it on exit.** Rather than the TUI
attaching to a pre-existing, externally-managed daemon (`EIMP-2`'s shape,
where the script fails fast if no server is found), the review script
launches a server configured to listen on a **private socket of its own**,
drives it for the session, and terminates it when the pass ends. Nothing
else on the machine knows the socket path, so no other process can
accidentally interfere with the TUI's session — the socket *is* the access
control, and its lifetime is exactly the TUI's lifetime.

Client side stays plain `curl` over that socket:

```bash
# GET
curl --unix-socket /tmp/server.sock http://localhost/users

# PUT
curl --unix-socket /tmp/server.sock -X PUT \
     -H "Content-Type: application/json" \
     -d '{"name":"alice"}' http://localhost/users/1
```

Server side is the straightforward axum/`UnixListener` binding:

```rust
use axum::{routing::get, routing::put, Router};
use std::fs;
use std::path::Path;
use tokio::net::UnixListener;

#[tokio::main]
async fn main() {
    let socket_path = "/tmp/server.sock";

    // 1. Remove old socket file if it exists
    if Path::new(socket_path).exists() {
        fs::remove_file(socket_path).unwrap();
    }

    // 2. Build your API routes
    let app = Router::new()
        .route("/users", get(|| async { "Get users output" }))
        .route("/users", put(|| async { "Put users output" }));

    // 3. Bind to the Unix Domain Socket
    let listener = UnixListener::bind(socket_path).unwrap();
    println!("Listening securely on Unix socket: {}", socket_path);

    // 4. Run the server
    axum::serve(listener, app).await.unwrap();
}
```

**Implementation notes on adopting this shape:**

- **It requires axum 0.8.** `axum::serve(listener, app)` accepting a
  `tokio::net::UnixListener` is an 0.8 capability; this repo pins **0.7.9**,
  where `serve()` is TCP-only — which is exactly why `EIMP-2` Phase D had to
  hand-roll the accept loop out of `hyper`'s HTTP/1.1 builder, `hyper-util`'s
  `TokioIo`/`TowerToHyperService`, and a manual `UnixListener` loop. Upgrading
  to axum 0.8 (0.8.9 current as of 2026-07-30) lets that whole glue layer be
  **deleted** in favor of the four-line binding above. Treat the upgrade as
  part of this work, and verify the `Listener` impl is available in the
  feature set actually enabled rather than assuming it.
- **Socket path must not be `/tmp/server.sock`.** The snippet's fixed path is
  illustrative; a private per-session socket needs an unpredictable path in a
  mode-700 directory (the scratch-dir hardening
  `einmo_review_client.sh` already performs). A fixed world-known path in
  `/tmp` is the opposite of the isolation this design is for.
- **`remove_file`-if-exists is not sufficient on its own.** Blindly unlinking
  whatever sits at the path would let this server stomp a *live* server's
  socket. `EIMP-2` Phase D already solved this: probe with `UnixStream::connect`
  first — connect succeeds → a live server owns it, refuse to start; connect
  fails → the file is stale, remove and rebind. Keep that logic.
- **Termination must be reliable.** The script kills the server on exit; that
  has to hold for `Ctrl-C` and for an abnormal exit too, or sockets and
  orphaned servers accumulate. The client's existing `trap`-based cleanup is
  the hook, paired with the server's own `ctrl_c` shutdown and socket removal.
- **This does not remove the standalone-server mode.** A long-lived server
  that several clients address remains meaningful (it is what makes
  server-side session state observable across two script runs, which `EIMP-2`
  Phase I used to prove decisions really live server-side). The TUI-owned
  private server is an additional, *default* launch mode, not a replacement.

| Method | Path | Meaning |
|--------|------|---------|
| GET    | `/api/review`                        | session summary: counts, cursor, dirty, verifiers |
| GET    | `/api/review/items?differing&filter=`| worklist rows incl. per-reviewer decisions |
| GET    | `/api/review/items/{m}`              | item detail; `version` for If-Match |
| GET    | `/api/review/items/{m}/body/{stage}` | verified body (ETag: fingerprint) |
| GET    | `/api/review/items/{m}/diff?l=&r=`   | hunks between stages, stamp lines excluded |
| PUT    | `/api/review/items/{m}/decision`     | decide (If-Match: version → 409 when stale) |
| DELETE | `/api/review/items/{m}/decision`     | undecide |
| POST   | `/api/review/items/{m}/claim`        | soft lease (TTL) |
| GET    | `/api/review/plan`                   | structured plan + rendered results-block text |
| POST   | `/api/review/execute`                | `{confirm:"PROMOTE", scope: all\|[m…], passphrase?}` |
| POST   | `/api/review/refresh`                | rescan; returns changed items |
| GET    | `/api/review/events`                 | SSE: decision-made / item-changed / executed |

The passphrase appears only in the execute request body (or is typed at the
server's own terminal when executing via CLI), is derived into a `Signer`,
used under the `exec` mutex, and dropped.

### S.8 The reduced `scripts/experimental_reviewer.sh`

The script keeps exactly what bash+vim are good at and sheds all session
state:

- **Keeps**: the per-test loop, the vim invocation (top info panel + 4
  tiles, `\d`/`\D`/`\i`/`\I`, statusline), reading the reviewer's pane
  intent, the between-tests prompt.
- **Sheds**: decision arrays and all undo/answer bookkeeping (→ PUT/DELETE/GET
  decision), body rendering and verification (→ GET body from cache), the
  differing computation (→ server), results rendering (→ GET plan), the
  gate execution (→ POST execute), stats-table computation (→ item detail).
- Server discovery via the suite's socket file; **no server → the current
  direct-`einmo` path remains as a degraded fallback** (one `fetch_body`-style
  switch), so the script never hard-depends on the server.
- Success measure: script size roughly halves, and a full no-decision pass
  over a large suite performs zero stamp verifications after the server's
  first pass (spawn count per test drops from 3 einmo processes to 3 socket
  reads).

### S.9 The dhtml frontend

A single self-contained page embedded in the binary (`include_str!`),
served by the same server: the 4-pane layout with server-computed diff
hunks (one diff implementation — `compare.rs` — everywhere), verb buttons, a
notes box (→ `Flag`), the plan view with the typed-PROMOTE gate, SSE-driven
refresh so concurrent verifiers see each other's decisions and claims live.
Byte-steadiness: signed bytes are never mutated by presentation. No
framework required at this phase.

### S.10 Drift tolerance

Einmo will likely evolve before this EIMP is undertaken. This specification
therefore binds to einmo's *behaviors* — stages, stamp chains,
verify-on-inspect, body extraction, promotion/retraction/flag transitions —
not to exact function signatures. The plan's first implementation task is a
re-survey of `src/` (`einmo_suite.rs`, `transitions.rs`, `signature.rs`,
`verify.rs`, `format.rs`, `compare.rs`) with spec touch-ups before any code.
The Rust sketches above are shape, not letter.

### S.11 Section-level post-quantum attestation (SPHINCS+)

**Layered, not a replacement.** The per-artifact Ed25519 stamps stay exactly
as they are (fast, per-file, the existing approval chain). This adds a
SECOND, coarser signature over a whole **section** (`output/`, `checked/`,
or `verified/` as a unit) using **SPHINCS+ / SLH-DSA** at a **conservative
parameter set** (the large-signature, slow-signing variant — e.g.
`slh_dsa_sha2_256s` via the pure-Rust `fips205` crate; this attestation runs
rarely, so size and speed do not matter and we buy the biggest security
margin). Because it is additive, **no existing `.einmo` signature is
invalidated** — the migration pain of a scheme swap is avoided entirely.

**Encapsulated in a `CorpusSigner` object — NOT mixed into `EinmoReview`.**
The whole section-attestation pipeline (build the manifest, read the
section in parallel into one buffer, hash, SLH-DSA sign/verify) is one
cohesive responsibility and lives in its own object. `EinmoReview` *uses*
it; it does not contain the logic. This mirrors the S.4 discipline that
keeps key custody (`Signer`) out of the review object — `CorpusSigner` is
the section-level analogue.

```rust
/// Owns section-level post-quantum attestation for one suite. Stateless w.r.t.
/// review; given a stage it (re)builds the manifest, reads the section, and
/// signs or verifies. Send + Sync so the server's single review can call it.
pub struct CorpusSigner {
    suite_root: PathBuf,
    params: SlhDsaParams,     // the conservative set, e.g. slh_dsa_sha2_256s
    read_workers: usize,      // bounded read-parallelism (S.11 read pass)
}

impl CorpusSigner {
    pub fn new(suite_root: &Path, params: SlhDsaParams, read_workers: usize) -> Self;
    /// Deterministic manifest for a stage (sorted mirror-paths + sizes/offsets).
    pub fn manifest(&self, stage: Stage) -> Result<SectionManifest>;
    /// Manifest + parallel read + hash → the message digest to sign/verify.
    pub fn digest(&self, stage: Stage) -> Result<SectionDigest>;
    /// (Re)sign a stage's section with the SLH-DSA key; writes `.section.sig`.
    pub fn sign(&self, stage: Stage, signer: &Signer) -> Result<SectionSig>;
    /// Recompute and check a stage's `.section.sig`; Ok(()) or a mismatch error.
    pub fn verify(&self, stage: Stage) -> Result<()>;
}
```

`EinmoReview::execute` holds an `Arc<CorpusSigner>` (or constructs one from
the suite) and calls `sign(stage, signer)` as the final step of promoting
into that stage — the review object orchestrates, `CorpusSigner` does the
work. Verification (CLI `einmo verify`, the server, the review script) calls
`verify(stage)` without any review session at all.

**What `CorpusSigner` signs.** For a section, the signed message is built
deterministically:

1. A **manifest** header: the stage name, the parameter set id, and the
   ordered list of included mirror-paths. Order is einmo's existing sorted
   walk (`walk_input_tree` sorts; deterministic), so the manifest is
   reproducible.
2. Then, in manifest order, each file's **bytes byte-joined** onto the
   running message (the signed envelope bytes as they sit on disk — the
   whole artifact, not just its body).
3. The whole thing is **hashed**, and SPHINCS+ signs that digest. The
   section signature + its manifest live in one file per stage (e.g.
   `checked/.section.sig` — dot-named, so einmo's walkers skip it).

**Reading the section — SINGLE-THREADED for this EIMP (resolved
2026-07-30).** Earlier drafts of this section specified a two-pass
metadata→offsets→disjoint-slice parallel read as the default, with the
worker pool resolved to `tokio`. **That is no longer this EIMP's scope.**
`EIMP-4` splits the repository into a lean core `einmo` and a separate
`einmo-review-server`, moving `tokio` and the whole HTTP stack out of core
— and `CorpusSigner` belongs in core. Implementing the parallel read with
`tokio` would therefore drag an async runtime straight back into the crate
the split exists to keep lean, for a workload with no async character.

So `CorpusSigner` ships here as the streaming, sequential implementation:
read the files in manifest order and feed the hasher incrementally
(`update(chunk)` per read block), never materializing the whole section in
memory. Correct, deterministic, bounded-memory, and zero new dependencies.
A sketch:

```rust
// manifest order fixes the byte order; the hasher sees the same stream a
// byte-join would have produced, without the intermediate buffer.
for path in manifest.paths() {
    let mut f = File::open(path)?;
    loop {
        let n = f.read(&mut chunk)?;
        if n == 0 { break }
        hasher.update(&chunk[..n]);
    }
}
```

This serial digest is the **oracle**: any future parallel implementation
must reproduce it bit-for-bit. Parallelizing it is deferred to `EIMP-5`,
and the structural work that makes parallelism cheap — restructuring the
corpus digest around a Merkle tree rather than a monolithic byte-join — is
specified by `EIMP-6` (`docs/eimp/EIMP-6.md`). Doing
the structure first is deliberate: mapping an independent digest over each
file and folding the results needs no shared buffer, no offset
choreography, and no defense against files changing size between two
passes.

**Concurrency caveat (carried forward).** A file changing underneath the
signer must be a hard error that aborts the signature, never a silently
truncated digest. The section sign runs under `execute`'s write lock
(S.2/S.4), which already excludes concurrent mutation from within einmo;
an external mutation mid-read simply fails verification later, which is the
correct outcome.

**Determinism is structural.** The manifest's sorted walk fixes the byte
order before any read begins, so the digest does not depend on read timing,
buffering, or (later) worker count.

**When it runs.** Whenever the section updates —
`EinmoReview::execute`/`execute_one`, promoting into a stage, calls
`CorpusSigner::sign(stage, signer)` as its final step (execution already
holds the write lock and the `Signer`). Verification calls
`CorpusSigner::verify(stage)`, which recomputes the manifest+hash and checks
the SLH-DSA signature; a mismatch means a file was added, removed,
reordered, or altered under the section as a whole — integrity above the
per-file level.

**Keys.** By default the **same passphrase** derives BOTH the existing
Ed25519 stamp key and the section SPHINCS+ key (via the S.4 `Signer`,
extended to expose both a per-file Ed25519 signer and a section SLH-DSA
signer from one derivation). SPHINCS+ keygen takes a seed; the Argon2id
output is expanded to the required seed length and fed to deterministic
keygen, preserving einmo's "same passphrase ⇒ same key" invariant. A future
option may separate the two keys, but same-passphrase is the default.

**Scope for THIS EIMP: crypto core + tests only.** Build and unit-test the
section-signature primitive (manifest builder, deterministic hash, SLH-DSA
sign/verify, same-passphrase dual derivation) as a self-contained module. Do
NOT wire it into the live promotion flow or write `.section.sig` into the
real corpus yet — that corpus-touching integration is a later step. The
primitive is proven in isolation first.

## Test Plan

Tests are written first, per project rules.

- **Unit — decisions**: replace-not-stack (second `decide` returns the
  first); `undecide` then unchanged pass = untouched; per-reviewer
  isolation; version bump on decide and on byte change; If-Match/409 on
  stale version.
- **Unit — cache**: N threads requesting one artifact → exactly one
  verification (test hook counter); tampered file → refused object, never
  content; fingerprint change invalidates.
- **Unit — signer**: derive-once reuse across N signings; zeroize on drop
  (best-effort assertion); computer vs human key selection per stage;
  passphrase never reachable after construction.
- **Unit — section PQ attestation (S.11, crypto core only)**: manifest is
  deterministic for a fixed file set (reorder inputs on disk → same sorted
  manifest → same message); adding/removing/altering one file changes the
  signed digest; SLH-DSA sign→verify round-trips; a tampered signature or a
  changed file fails verify; **same passphrase derives both** the Ed25519
  stamp key and the section SLH-DSA key (dual-derivation determinism: same
  passphrase ⇒ same section pubkey across runs); empty-section manifest is
  well-formed. NO real-corpus writes in this EIMP — pure module tests over
  fixtures.
- **Unit — CorpusSigner read strategies (S.11)**: `ParallelBuffer` (default)
  and `Stream` produce a **byte-identical digest** over the same fixture
  set, independent of worker count and read completion order; the parallel
  two-pass buffer has exactly `sum(len)` bytes with each file at its
  manifest offset; a file that shrinks between the metadata and read pass
  (short read) or grows (leftover bytes) is a hard error, not a silent
  mis-hash; `Stream` holds bounded memory (never materializes the whole
  section). Stress with a mix of many tiny files and a few large ones.
  `CorpusSigner` is exercised as a standalone object (no `EinmoReview`),
  proving the encapsulation.
- **Unit — execute**: plan/execute equivalence with CLI `einmo promote`
  byte-for-byte; skip-and-report on mid-plan drift; retract cascade;
  exclusive exec under concurrent decide traffic (no lost updates).
- **Unit — flag = plaintext, concatenating (S.3)**: `flagged/<test>` is
  PLAINTEXT, unsigned; executing a `Flag` on a fresh test writes the
  annotated note as plaintext; re-flagging CONCATENATES the new dated block
  ON TOP of the existing content (same path); two reviewers flagging the
  same test → both dated blocks present, ordered, none lost (serialized by
  the exec mutex); a pending `Flag` replaces on re-edit; `flagged/` stays
  exempt from verification (a plaintext/broken file there fails no gate);
  the journal has both flag events.
- **Unit — signed `notes/` stage (S.3)**: a note promoted into `notes/` is a
  valid SIGNED `.einmo` (verify-on-inspect passes; stamp verifies against
  the passphrase-derived key); the same concatenated content that was a
  plaintext flag can be signed as a note's body; `notes/` participates in
  signature checks while `flagged/` does not.
- **Unit — flag breaks tests (S.3)**: a flagged artifact makes the run FAIL
  by default (non-zero exit / red gate); `--flag-is-not-failure` downgrades
  it to non-fatal BUT stderr still announces the flag count; there is no
  config that makes a flag silent; the goal-state check (zero flags + all
  signed + all matching + all signatures valid against their
  passphrase-derived keys) is green only when no flags exist.
- **Journal**: replay reconstructs the DecisionBook exactly; a truncated
  tail (crash) replays cleanly.
- **Server**: endpoint tests against a tempdir suite
  (list/body/diff/decide/plan/execute/SSE); UDS permission inheritance; 409
  flows; token required on TCP.
- **Thin client**: a pty-driven end-to-end run of the reduced
  `scripts/experimental_reviewer.sh` against a live server (the stub-vim
  technique): promote, note→flag, `u`-revisit keeps answer, gate
  skip/confirm; plus the no-server fallback path.
- **Comprehensive test**: a scripted multi-verifier session (two reviewers,
  mixed individual/batch execution, one crash-resume, one drift) over a
  fixture suite, asserted against the resulting stamp chains via `einmo
  verify`.

## Rejected Alternatives

### A. Signing inside `EinmoReview`

Fold `Signer` into the session (review holds the derived key after first
passphrase entry). Rejected: the lifetimes and owners differ (S.4) — a
server-held review would hold human key material for the whole session,
violating derive-use-drop; testing key custody would entangle with decision
logic; and per-reviewer keys in one shared object invite cross-signing bugs.
The separation costs one extra parameter at the two execute calls.

### B. Stateless server (re-verify per request)

Simplest server: every request re-runs `einmo` logic like the CLI does.
Rejected: it re-imports the exact cost this EIMP exists to remove — the
review script's slowness *is* repeated verification; a stateless server
makes the browser UI equally slow and makes multi-verifier coordination
(versions, claims, events) impossible.

### C. Keep growing `scripts/experimental_reviewer.sh`

Continue enriching the bash script (it works today). Rejected on this
session's own evidence: the revisit machinery, the infinite-loop `continue`
bug, and the `show_cmd` ordering bug are all state-machine defects bash
cannot unit-test. The script's proper role is a thin terminal frontend.

### D. Do nothing

Review remains vimdiff-over-temp-files with per-run re-verification.
Rejected: the corpus is growing, a session substrate is needed anyway, and
every future frontend would re-implement review semantics.

## Open Questions

All resolved at begun-time (2026-07-30) — design is frozen:

- **HTTP stack**: `axum` (0.7.9) + `hyper`/`hyper-util`/`tower`, matching
  `EIMP-2`'s already-proven stack over a unix-domain socket. No switch to a
  more minimal framework — reuse what's already working.
- **Journal location**: a scratch/state directory (not a suite dot-file).
  See §S.6.
- **Claim lease TTL**: 5 minutes, auto-reclaimed on expiry; active claims
  ARE surfaced in `plan()`. See §S.5.
- **Quorum policies**: out of scope for this EIMP entirely (not deferred as
  "maybe later in this EIMP" — genuinely not a defined concept yet). This
  EIMP only facilitates multi-party signature accumulation (§S.4a); a
  quorum policy engine, if ever needed, is a distinct follow-up EIMP. See
  §S.6.
- **`ReviewOpts` mode default**: not a boolean — a runtime-selectable
  `ReviewMode` (`Full` default, plus `Random` and `NewOrBroken`). See §S.2.
- **Parallel section-read (§S.11)**: superseded. `CorpusSigner` ships
  **single-threaded** here; the parallel machinery moves to `EIMP-5`, and
  the Merkle-tree restructuring that makes it cheap is a design TODO. The
  earlier "use `tokio`" resolution is withdrawn — `EIMP-4`'s crate split
  removes `tokio` from core `einmo`, which is where `CorpusSigner` lives.
  See §S.11.
- **Phase A2 (`CorpusSigner`) scope**: confirmed in-scope for this EIMP
  (crypto core + tests only, per §S.11's existing "not wired into the live
  promotion flow yet" boundary — that boundary is unchanged), now
  explicitly single-threaded.
- **Crate boundary**: `EinmoReview` and every frontend ship in
  `einmo-review-server`; core `einmo` keeps the test runner, signing, and
  `CorpusSigner`. See §S.1 and `EIMP-4` §S.1.
- **Journal**: keyed by `EinmoId` throughout, with verbosity levels
  (finest level records each case as read in and verified), logging enough
  to serve the crash crumb's purpose without retiring the crumb in this
  EIMP. See §S.6.

## References

- **EIMP-3** — the core-test-run (`output`-stage) analogue of this EIMP's
  §S.4a multi-signer content-then-key decision table; the two EIMPs share
  the same decision shape over separate code paths (`transitions::promote`
  here, `write_output` there).
- **FOOP-25** (`foolish-rust`) — the original specification this EIMP is
  ported from.
- **FOOP-15** (`foolish-rust`) — secured interactive einmo review: the
  original notes that this FOOP-25/EIMP-1 supplies the session/state layer
  its perspectives/MCP phases would attach to. Not ported into this
  repository; cited for provenance only.
- **FOOP-64** (`foolish-rust`) — the einmo suite migration and
  `poor_einmo.sh`, whose review protocol (verbs, replace-not-stack,
  `u`-revisit, PROMOTE gate, `-d` default, top info panel) is the
  behavioral prototype this EIMP libraries-izes. Not ported into this
  repository; cited for provenance only.
- **FOOP-92** (`foolish-rust`, Complete) — einmo itself, before extraction
  into its own repository.
- Code: `src/{einmo_suite,transitions,signature,verify,format,compare}.rs`;
  `scripts/experimental_reviewer.sh`.
