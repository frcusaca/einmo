---
eimp: 2
title: einmo-review-server — a minimal HTTP prototype of the review/sign/promote/flag loop
author: Claude Code (Sonnet 5) <noreply@anthropic.com>
status: Implementing
type: Standards
created: 2026-07-29
supersedes: []
begun: [x]
---

# EIMP-2: einmo-review-server — a minimal HTTP prototype of the review/sign/promote/flag loop

## Abstract

Stand up a minimal, single-suite HTTP server — `einmo-review-server` — that
hosts one `EinmoReview` (per `EIMP-1`) for the current directory, started via
`cargo einmo-review-server` (mirroring the existing `cargo-einmo` alias-binary
pattern). A **new** script, `scripts/einmo_review_client.sh`, is built to
drive it — for everywhere `scripts/experimental_reviewer.sh` (today's direct-
CLI prototype) shells out to `einmo` directly (`einmo list`, `einmo body`,
`einmo promote`) or mutates the corpus itself (the raw `mv … flagged/` it
uses for flagging today), the new script instead makes an HTTP request to
this server. `experimental_reviewer.sh` is **left alone, untouched** — not
rewired, not deprecated in place — both as a working fallback and as
reference material: its vim invocation, pane layout, and review-loop UX are
proven and worth reusing verbatim in the new script; only the
state-tracking machinery (the bash arrays, §5) does not carry over, because
that state now lives server-side. This is explicitly a **prototype slice**
of `EIMP-1`'s full server design (§S.7) — not the dhtml frontend, not SSE,
not multi-verifier claims, not the journal, not `CorpusSigner`. It exists to
prove out the core review loop — list, inspect, decide (promote/flag),
execute — hosted behind HTTP before investing in the rest of `EIMP-1`'s
surface.

State ownership moves server-side (§5): the old script's parallel bash
arrays (`promote_checked`, `flag_stage`/`flag_rel`/`flag_reason`, etc.) are
exactly `EinmoReview`'s `DecisionBook` reimplemented in shell; the new
script sends decisions to the server as they're made and keeps only the one
array it needs to iterate (the ordered list of `EinmoId`s, §0), asking the
server for everything else. **This makes `einmo_review_client.sh`
fundamentally different from what it's modeled on**: it never holds review
state for the duration of a pass — it is a thin **TUI/bridge**: it drives
vim (the load-bearing review tool — panes, diffing, editing, the reviewer's
actual moment-to-moment interaction, reused from `experimental_reviewer.sh`)
and translates each pane-edit into an HTTP call against the server and
back, nothing more. All the state that makes a review session a *session* —
what's been decided, what's pending, what the plan will do — lives entirely
in the server's `EinmoReview`, not in the script's process. To exercise this
against real, non-trivial content rather than synthetic tempdir fixtures,
this EIMP also brings a JavaScript-only (Boa) copy of the `foolish-rust`
workspace's `zweimomo` test crate into this repository (§8) — its own
crate, its own real signed test suite, no cross-repo dependency.

**Known weakness, accepted for this prototype**: the `checked to verified`
passphrase travels from `einmo_review_client.sh` to `einmo-review-server`
as **plaintext** inside the `POST /einmo/<session>/execute` request body
(§4). Even over a unix-domain socket (not the network), this is weaker than
the direct-CLI path's `/dev/tty` prompt read directly by the process that
derives the key — an HTTP body is, in principle, observable by anything
that can read the request (e.g. logging middleware, a proxy, if one were
ever added). This is flagged explicitly as a design weakness to revisit
(§Open Questions), not a silent gap.

## Motivation

`EIMP-1` specifies the full `EinmoReview` design, but it is large: a
thread-safe session object, a journal, multi-verifier concurrency semantics, a
dhtml frontend, and (§S.11) a whole separate post-quantum attestation
subsystem. Building all of it before proving the *shape* of "a script talks
to a server instead of calling `einmo` directly" is a lot of investment before
the first feedback. This EIMP carves out the smallest useful slice:

- One resident process, one suite (the current directory) — no multi-suite
  routing, no auth beyond what a unix-domain socket's file permissions give
  for free.
- Just enough server surface for `einmo_review_client.sh`'s actual needs:
  list the worklist, fetch a verified body, record a decision
  (promote-to-checked, promote-to-verified, retract, or flag), and execute
  it.
- No journal yet — decisions live in memory only; a server restart loses
  in-flight (undecided/unexecuted) state. This is an explicit, acceptable
  prototype limitation (see Open Questions).

If this slice works and feels right, `EIMP-1`'s fuller design (journal,
concurrency, dhtml, `CorpusSigner`) is the natural next step, informed by
what this prototype teaches. If it doesn't, the cost of finding that out is
small.

## Specification

### 0. `EinmoId` — a formal case identifier (upstream refactor)

Before any server or script work, this EIMP builds a piece of core-library
plumbing every other section depends on: **`EinmoId`**, a validated newtype
identifying one reviewable case (what the script today calls a "test" and
what its worklist rows already key on informally as a mirror-relative path).
This is **not** new einmo *behavior** — `stage.rs::mirror_input_path` already
computes exactly this mapping (`stage1/section3/specific.foo` →
`stage1/section3/specific.foo.einmo`) — it is making that identity a
first-class, reusable, tested type instead of an ad-hoc `PathBuf` recomputed
at each call site. Landing this as an upstream refactor benefits `einmo`
itself (the CLI's `--filter`, `list`, `compare` already reason about the
same identity informally), not just this EIMP's server.

```rust
/// A validated identifier for one reviewable case: the input-relative path
/// with any input extension stripped, e.g. `foop/23/comprehensive` for an
/// input at `input/foop/23/comprehensive.foo`. Stable across stages — the
/// same EinmoId names the input, and every stage artifact
/// (`output/<id>.<ext>.einmo`, `checked/<id>.<ext>.einmo`, …).
///
/// Wait — see the worked example below: einmo's existing `mirror_input_path`
/// keeps the input's own extension in the mirror path (`specific.foo` ->
/// `specific.foo.einmo`, not `specific.einmo`), because two same-stem inputs
/// with different extensions (`x.foo` and `x.js`) must not collide. `EinmoId`
/// therefore preserves the input extension too: id `foop/23/comprehensive.foo`
/// (not `foop/23/comprehensive`), consistent with `mirror_input_path`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EinmoId(String);   // validated relative path, forward slashes, no ".."/absolute/NUL

impl EinmoId {
    /// Construct from an input-relative path (e.g. discovered by
    /// `walk_input_tree`). Validates and sanitizes — see §Validation below.
    pub fn from_input_rel(input_rel: &Path) -> Result<Self>;

    /// Derive from any stage artifact's path, given that stage's root
    /// directory — strips the stage root and the trailing `.einmo`, leaving
    /// the same id `from_input_rel` would have produced. Lets a case be
    /// identified even when its `input/` file is missing (deleted after
    /// being promoted) by reading back from `output/`/`checked/`/`verified/`/
    /// `flagged/` instead.
    pub fn from_stage_artifact_path(stage_root: &Path, artifact_path: &Path) -> Result<Self>;

    /// Construct the path to this case's artifact in a given stage, rooted
    /// at the suite directory — e.g. `output/foop/23/comprehensive.foo.einmo`.
    #[must_use]
    pub fn to_stage_path(&self, suite_root: &Path, stage: Stage) -> PathBuf;

    /// The validated id as a `&str` (forward-slash separated, no `.einmo`
    /// suffix) — used for URL path segments (percent-encode the caller's
    /// side, this method does not encode) and log lines.
    #[must_use]
    pub fn as_str(&self) -> &str;
}

impl std::fmt::Display for EinmoId { /* … */ }
impl TryFrom<&str> for EinmoId {
    // Parses an incoming (already percent-decoded) URL path segment: same
    // validation as from_input_rel, over an arbitrary caller-supplied string
    // rather than a filesystem-discovered path.
    type Error = EinmoError;
    fn try_from(s: &str) -> Result<Self> { /* … */ }
}
```

**Validation/sanitization (webserver-facing, since URL segments are
untrusted input, unlike `walk_input_tree`'s filesystem-discovered paths):**
reject `..` components, absolute paths, NUL bytes, empty segments, and any
path that would escape the suite root once joined to a stage directory —
the same discipline `EIMP-1`'s Test Plan already expects of parsed content
generally (`rust_instructions.md` §7 "Serialization & parsing" — validate
before trust).

**Worked example**, matching the concrete case discussed while scoping this
EIMP: input `foolish-ubca/einmo_suite/input/foop/23/comprehensive.foo` (a
`foolish-rust`-workspace path shown only as an illustration of the shape —
this EIMP's own suites are `zweimomo`'s, §8) has `EinmoId` `"foop/23/comprehensive.foo"`,
and `to_stage_path(suite_root, Stage::Output)` reconstructs
`<suite_root>/output/foop/23/comprehensive.foo.einmo`.

`EinmoId` replaces the placeholder `MirrorPath` type used informally in
`EIMP-1`'s sketches; every `EIMP-1`/`EIMP-2` signature written as
`m: &MirrorPath` should be read as `id: &EinmoId` from this EIMP onward.

### 1. What the old script calls today, and what the new one calls instead

`scripts/experimental_reviewer.sh` (left untouched, §6) currently touches
einmo in exactly these ways; `scripts/einmo_review_client.sh` (the new
script this EIMP builds) does the equivalent via HTTP. `<id>` below is a
case's `EinmoId` (§0), percent-encoded in the actual
URL, and `<session>` is the review session id (§2) — a session is created
once per server run and reused for every request in that run, so in
practice the script fetches it once at startup and treats it as a constant.

| `experimental_reviewer.sh` does today | `einmo_review_client.sh` does instead |
|---|---|
| `"$EINMO" list "$SUITE" [--filter …] [--differing]` | `GET /einmo/<session>/cases` |
| `"$EINMO" body "$f"` (verified body of a stage file) | `GET /einmo/<session>/cases/<id>/body/<stage>` |
| raw `mv "$SUITE/$stage/$rel" "$SUITE/$stage/flagged/$rel"` (writes the plaintext advisory note itself) | `POST /einmo/<session>/cases/<id>/flag` `{"reason":…}` — one call, atomic (§3) |
| `"$EINMO" promote output to checked "$SUITE" -- <files>` | `PUT /einmo/<session>/cases/<id>/decision` `{"kind":"promote","to":"checked"}` per case, then `POST /einmo/<session>/execute` |
| `"$EINMO" promote checked to verified "$SUITE" --interactive -- <files>` | same shape, `{"to":"verified"}`, passphrase carried in the execute call body (§4) |
| `\K` (kick) — accumulated locally as `retract_checked`/`retract_verified`; the existing script does not appear to actually invoke `einmo retract` to execute these today (a pre-existing gap, not introduced by this EIMP) | `POST /einmo/<session>/cases/<id>/retract` `{"from":"checked"\|"verified"}` — one call, atomic (§3); this EIMP closes the gap by actually executing kicks, cascade included (`einmo retract`'s existing checked→verified cascade, `transitions.rs`) |
| `u` (revisit) — local array surgery (`drop_from`, `answer_of`) | `GET /einmo/<session>/cases/<id>` to read the current decision, then `PUT … /decision` to replace it (§5) — or `DELETE … /decision` (`undecide`) if the reviewer backs out to "no decision yet" rather than replacing with a new one |

This table is the entire scope of this EIMP's HTTP surface. Beyond adding
`retract`/`undecide` support (closing the pre-existing kick-execution gap)
and the session-id URL shape (§2), nothing in `EIMP-1`'s §S.7 table beyond
these rows is built here.

### 2. `EinmoReview` — the minimum viable slice, and the session-scoped URL shape

Only the parts of `EIMP-1` §S.2's `EinmoReview` needed to back the rows
in §1's table above:

```rust
pub struct EinmoReview {
    suite: EinmoSuite,                       // immutable after open()
    worklist: RwLock<Worklist>,              // read-mostly; refresh() takes the write lock
    cache: VerifiedCache,                    // fingerprint -> verified body; single-flight verification
    decisions: RwLock<DecisionBook>,         // keyed by EinmoId; single implicit reviewer — see Open Questions
    exec: Mutex<()>,                         // execution (disk mutation + signing) is exclusive
}

impl EinmoReview {
    pub fn open(suite: &Path, opts: ReviewOpts) -> Result<Self>;
    pub fn items(&self) -> Vec<ReviewItem>;                                       // ReviewItem carries its EinmoId
    pub fn body(&self, id: &EinmoId, s: Stage) -> Result<Arc<VerifiedBody>>;
    pub fn decide(&self, id: &EinmoId, d: Decision) -> Result<Option<Decision>>;
    pub fn undecide(&self, id: &EinmoId) -> Option<Decision>;
    pub fn plan(&self) -> ExecutionPlan;
    pub fn execute(&self, plan: &ExecutionPlan, keys: &SignerSet) -> ExecutionReport;
}
```

**Every server-hosted `EinmoReview` is addressed by a session id in the URL
path — `/einmo/<session-id>/…` — from the start**, JSON-RPC-flavored, even
though this EIMP's server holds exactly **one** `EinmoReview` (one process,
one suite, one implicit reviewer — unchanged from the original scope). The
reason to shape the URLs this way now rather than later: `EIMP-1` §S.5's
eventual multi-verifier support means multiple sessions will exist someday,
and baking the session id into every path today means that future work is
additive (route to the right `EinmoReview` by id) rather than a breaking
URL-shape change across every endpoint. `POST /einmo/sessions` creates a
session (one call, at server startup, opening the `EinmoReview` for the
suite passed on the command line — see §7); every other endpoint is nested
under `/einmo/<session-id>/…`. A server that only ever creates the one
startup session does not need session listing, expiry, or cleanup in this
EIMP — those are `EIMP-1`-territory once multiple sessions are real.

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
live-mutation detection yet), and per-request `ReviewerId` (single implicit
reviewer per session — see Open Questions; distinct from the session id,
which addresses the `EinmoReview` itself, not who is deciding). `undecide`
**is** kept — see §5 — as the server-side mechanism a revisit (`u`) resolves
through, alongside `decide`'s own replace-not-stack behavior.
`Signer`/`SignerSet` (`EIMP-1` §S.4) are unchanged and still used exactly as
specified — key custody stays out of `EinmoReview` here too.

### 3. The HTTP surface

Binds a **unix-domain socket** by default, same rationale as `EIMP-1` §S.7
(inherits directory permissions; no token machinery needed for a
localhost-only prototype). TCP + bearer token is **out of scope** for this
EIMP (`EIMP-1` §S.7 already specs it; add it there when a browser frontend
actually needs it).

| Method | Path | Body | Meaning |
|--------|------|------|---------|
| POST   | `/einmo/sessions` | `{"suite":"<path>"}` | open an `EinmoReview` for `suite`, return its session id — called once, at server startup, for the one suite given on the command line (§7) |
| GET    | `/einmo/<session>/cases` | — (query: `filter`, `differing`) | worklist rows (each carrying its `EinmoId`), mirrors `einmo list` |
| GET    | `/einmo/<session>/cases/<id>` | — | one case's detail, incl. its current decision (if any) — read before a revisit (§5) |
| GET    | `/einmo/<session>/cases/<id>/body/<stage>` | — | verified body content, mirrors `einmo body` |
| PUT    | `/einmo/<session>/cases/<id>/decision` | `{"kind":"promote","to":"checked"\|"verified"} \| {"kind":"retract","from":"checked"\|"verified"} \| {"kind":"flag","reason":string} \| {"kind":"skip"}` | record (or replace — replace-not-stack, `EIMP-1` §S.3) a decision — all four `Decision` variants. Promotions and `skip` only go through this route; flag/retract normally use the convenience endpoints below instead |
| DELETE | `/einmo/<session>/cases/<id>/decision` | — | `undecide` — clear a decision back to "untouched" (§5's revisit path, when the reviewer backs out rather than replaces) |
| POST   | `/einmo/<session>/cases/<id>/flag` | `{"reason":string}` | convenience: record `Decision::Flag` AND execute it in one call — no gate, matches the old script's single `mv` call site (resolved Open Question) |
| POST   | `/einmo/<session>/cases/<id>/retract` | `{"from":"checked"\|"verified"}` | convenience: record `Decision::Retract` AND execute it in one call, cascade included — no gate |
| GET    | `/einmo/<session>/plan` | — | structured plan preview (what execute would do) — also doubles as the end-of-pass summary the script renders (§5) |
| POST   | `/einmo/<session>/execute` | `{"confirm":"PROMOTE","passphrase"?:string}` | apply all pending `PUT`-recorded decisions (promotions and any `skip`s); requires the `confirm` token for promotions |

`<id>` is a case's `EinmoId` (§0) as a percent-encoded URL path segment.
`<session>` is the session id `POST /einmo/sessions` returned; every other
route 404s on an unknown session id rather than silently creating one. This
EIMP creates exactly one session per server run (§7) — the routes are
session-scoped in *shape* for `EIMP-1`'s future multi-session support (§2),
not exercised with multiple concurrent sessions *in* this EIMP.

**Decisions are sent the moment the reviewer decides, not batched
client-side** — see §5: the server's `DecisionBook` is the single
accumulating store, so the script never needs a local copy to dump at the
end. Flag and retract go through their own convenience endpoints and
execute immediately, no `confirm` gate — neither produces a new signature
the way a promotion does (flag is a plaintext advisory move; retract is a
local demotion). Promotions (and any `skip`) go through `PUT … /decision`
and accumulate as pending decisions across the whole pass, applied together
by one gated `POST /execute` at the end.

No session-summary endpoint beyond `GET .../plan`, no SSE — not needed by
the script's current flow. Every response is JSON; the script parses it
with `jq` (§6 — `curl` + `jq` is sufficient for JSON-RPC-shaped calls in
bash; a parallel client in another language was considered and dropped, see
Rejected Alternative H).

### 4. Signing stays exactly as `EIMP-1` §S.4 specifies

No change from `EIMP-1`: `Signer`/`SignerSet` is a separate object from
`EinmoReview`; the passphrase for a `checked to verified` execute arrives
only inside the `POST /einmo/<session>/execute` request body, is derived
into a key, used under the `exec` mutex, and dropped. `output to checked`
promotions use the computer/empty-passphrase key, same as today's
`--passphrase ""`-style default (see `Cargo.toml`/`einmo.toml` conventions
already in this crate).

### 5. State ownership moves server-side — the new script tracks indices, not decisions

`experimental_reviewer.sh` re-implements a decision store in bash: the
parallel arrays `promote_checked`, `promote_verified`, `retract_checked`,
`retract_verified`, `flag_stage`/`flag_rel`/`flag_reason`,
`send_to_agent_list`, `skip_list`, and `noop_list` are, collectively, exactly
what `EinmoReview`'s `DecisionBook` already is (`EIMP-1` §S.3) — a map from
test to the reviewer's current decision. `einmo_review_client.sh` does not
carry this pattern over: decisions are recorded server-side via
`PUT … /decision` the moment the reviewer makes each one (not accumulated
locally and dumped in a batch at the end), so the new script never holds or
replays its own copy. It is a thin loop:

- The one array the new script needs is `ids` — the ordered list of
  `EinmoId`s (§0) from `GET /einmo/<session>/cases` — because the script
  drives a `for i in "${!ids[@]}"` loop. It is a list of **identifiers**,
  not decision state: every question about "what has been decided for this
  case so far" is answered by asking the server (`GET
  /einmo/<session>/cases/<id>`), never by anything the script itself
  tracked. This is the concrete form of "access/modify through the einmo
  case id" — the script indexes into `ids[]`, then does everything else by
  id against the server.
- The results/stats summary at the end of a review pass (today computed by
  iterating the local arrays) becomes a `GET /einmo/<session>/plan` call —
  the structured plan **is** the summary; the script renders it rather than
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
  A revisit becomes: `GET /einmo/<session>/cases/<id>` to read the current
  decision (if any, for display), then either a fresh `PUT … /decision`
  that replaces it (`EinmoReview::decide`'s replace-not-stack semantics,
  `EIMP-1` §S.3) or, if the reviewer backs out to "no decision yet" rather
  than choosing a new one, a `DELETE … /decision` (`undecide`, kept in
  this EIMP's slice — §2).

### 6. `scripts/einmo_review_client.sh` — a new script, `experimental_reviewer.sh` untouched

**`experimental_reviewer.sh` is not modified by this EIMP.** It keeps
working exactly as it does today — a direct-CLI reference implementation
and a fallback if the new script has a bug. `scripts/einmo_review_client.sh`
is a **new file**, built by:

- **Reusing verbatim** from `experimental_reviewer.sh`: the vim invocation
  (top info panel + panes, `\d`/`\D`/`\i`/`\I`, statusline), the pane-verb
  reading convention (a pane containing one word = a decision), the
  PROMOTE/RETRACT/SKIP word-synonym lists, and the overall between-tests
  loop shape. This is proven UX; there is no reason to redesign it.
- **Not reusing**: the decision arrays and everything that manipulates them
  (`undo_last_decision`, `answer_of`, `drop_from`) — §5 explains why: that
  state now lives server-side, so the new script has nothing analogous to
  carry over. `einmo_review_client.sh` is expected to be **substantially
  shorter** than `experimental_reviewer.sh` as a result (see §7's line-count
  measurement).

New behavior specific to `einmo_review_client.sh`:

- A startup check: does a `einmo-review-server` UDS socket (and its session
  file) exist for this suite? If not, **fail with a clear message telling
  the user to start it** (`cargo einmo-review-server <suite>` or
  `einmo-review-server <suite>`) — no fallback to `experimental_reviewer.sh`
  or to direct `einmo` calls. The whole point of this EIMP is to prove the
  HTTP-only shape; a silent fallback would hide whether it actually works
  end to end. The script reads the session id from the session file (§7)
  once at startup and holds it as one constant for the rest of the run.
- **New dependency: `jq`**, alongside `curl` — `curl --unix-socket` + `jq`
  is sufficient to build and parse JSON-RPC-shaped requests from bash; a
  parallel client in another language was considered and dropped (Rejected
  Alternative H).
- `"$EINMO" list …` becomes a `curl --unix-socket` GET against
  `/einmo/<session>/cases`, parsed with `jq` into the one local array
  (`ids`) per §5.
- `"$EINMO" body "$f"` becomes a GET against
  `/einmo/<session>/cases/<id>/body/<stage>`.
- The raw `mv … flagged/` (from `experimental_reviewer.sh`) becomes one
  `POST /einmo/<session>/cases/<id>/flag` call, sent the moment the
  reviewer flags a test — no gate, matching current behavior (§3).
- `\K` (kick) — dead state in the old script (§1/§5) — becomes one
  `POST /einmo/<session>/cases/<id>/retract` call: kicks now actually
  execute, closing the pre-existing gap.
- `"$EINMO" promote …` becomes a `PUT … /decision` sent per-case as each
  decision is made (§5); at the end of the pass, one gated `POST /execute`
  (reading the plan the server already holds) carries the typed `PROMOTE`
  confirmation and, when any pending decision promotes to `verified`, the
  passphrase (read from `/dev/tty`, same UX as `experimental_reviewer.sh`
  today — see the Abstract's plaintext-transport caveat).
- Revisits (`u`) become a `GET … /cases/<id>` followed by either a re-`PUT`
  (replace) or a `DELETE … /decision` (`undecide`, to back out entirely) on
  the same case id, per §5 — no local array surgery, because there is no
  local array.

**Feature scope stays fluid during this prototyping phase.** Most of
`einmo_review_client.sh`'s v0 feature surface is already demonstrated by
`experimental_reviewer.sh`'s existing verb set (promote/retract/flag/skip/
revisit) — this EIMP is primarily a *plumbing* change (where the state and
logic live), not new review functionality. Even so, treat the endpoint
list (§3) and the phase-by-phase plan as a starting sequence, not a locked
contract: if building one phase surfaces a better shape for the next, adjust
the spec rather than forcing the original sketch.

### 7. Binary, socket location, and installation

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

`cargo einmo-review-server [--socket <path>] <suite>` runs the resident
process in the foreground (Ctrl-C to stop); it is the reviewer's
responsibility to run it in a second terminal/tmux pane/background job — no
daemonization, no service-manager integration in this EIMP.

**Socket location is configurable, defaulting to the current directory.**
`--socket <path>` overrides it; unset, the server binds
`./.einmo-review.sock` (dot-named, so einmo's own walkers skip it, matching
the convention `.section.sig` uses in `EIMP-1` §S.11). At startup, once the
session is created (§2 — `POST /einmo/sessions`, called once against
itself), the server also writes `<socket-path>.session` next to the socket,
containing the session id, so `einmo_review_client.sh` (§6) can discover
both the socket and the session id from one well-known location without a
separate API round-trip. **Both the socket and the session-id file are
removed on exit** (normal exit and signal handlers — `SIGINT`/`SIGTERM`),
so a clean shutdown leaves the directory as it found it; a crash may leave
a stale socket file behind, and the server refuses to start if one already
exists at the target path without being able to connect to it (same
stale-lockfile discipline `EIMP-1` §S.5 describes for the suite lockfile).

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

- **Unit — `EinmoId` (§0)**: `from_input_rel` round-trips through
  `to_stage_path` for every `Stage`; `from_stage_artifact_path` recovers the
  same id `from_input_rel` would have produced, for each stage; rejects `..`
  components, absolute paths, NUL bytes, and empty segments; `TryFrom<&str>`
  parses a percent-decoded URL segment identically to `from_input_rel` on
  the same logical path; two inputs with the same stem but different
  extensions (`x.foo` vs `x.js`) produce distinct ids (matches
  `mirror_input_path`'s existing collision-avoidance behavior).
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
  run of `einmo_review_client.sh` (reusing the stub-vim technique referenced
  in `EIMP-1`'s test plan) against a real
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
- **Integration — no-server behavior**: starting `einmo_review_client.sh`
  with no server running fails fast with the documented message; it does
  NOT silently fall back to `experimental_reviewer.sh` or direct `einmo`
  calls (§6).
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

### H. A parallel client in Python/Ruby/Rust instead of bash + `jq`

Considered while resolving this EIMP's Open Questions: write the new
HTTP-calling script in a more JSON-ergonomic language instead of bash.
Rejected once the actual blocker was named: the concern was JSON
parsing/construction inside bash, and `curl` + `jq` is a completely
standard, well-supported way to do exactly that — a JSON-RPC-shaped API
does not, by itself, require leaving bash. `jq` is added as a new
dependency (§6) instead. The new script's role also shrank enough during
this design pass (from "holds review state" to "a thin TUI/bridge between
vim and the server," see Abstract) that the case for a heavier client
language weakened further — there's very little logic left
in the script for a different language to do better.

### I. Server-minted opaque case IDs instead of the mirror-relative path

Considered: a case ID could be a server-assigned hash or sequential integer,
with an internal id↔path lookup table, giving cleaner URLs. Rejected: einmo
already has a real, existing identity for a case — the mirror-relative
input path (`stage.rs::mirror_input_path`) — and every stage artifact is
reachable from it deterministically (input extension + `.einmo` suffix +
stage directory). An opaque id would need that same lookup table anyway
(to answer "what is case 17") while also being less self-describing in a
`curl` command or a log line. `EinmoId` (§0) formalizes the existing path
identity instead of inventing a second one.

## Open Questions

Resolved during scoping (kept here as a record of the decision, per
`EIMP-0`'s Open-Questions-emptied-when-frozen convention — remove once this
EIMP reaches `Implementing`):

- ~~State loss on restart~~ — **acceptable for this prototype.** No journal;
  a server restart loses undecided/unexecuted decisions. A real journal is
  `EIMP-1`'s job.
- ~~Script JSON parsing~~ — **`jq`**, not a script-friendly plain-text mode
  and not a parallel client in another language (Rejected Alternative H).
- ~~Session identity / URL shape~~ — **session id in the URL path**
  (`/einmo/<session-id>/…`), one session per server run for this EIMP, JSON-
  RPC-flavored, future-proofing for `EIMP-1`'s multi-session support (§2).
- ~~Case identity~~ — **`EinmoId`**, the existing mirror-relative-path
  identity formalized as a validated, tested type (§0), not a server-minted
  opaque id (Rejected Alternative I).
- ~~Socket location~~ — **configurable, default `.`** (current directory),
  removed on exit (§7).
- ~~Immediate-execute for flags and retracts~~ — **one convenience endpoint
  each**: `POST /einmo/<session>/cases/<id>/flag` and `POST
  /einmo/<session>/cases/<id>/retract`, each atomic (record the decision
  and execute it in one call), rather than the two-call `PUT decision` +
  `POST execute` shape used for promotions. Closest match to
  `experimental_reviewer.sh`'s current single `mv` call site.

Still open — not blocking, revisit later:

- **Plaintext passphrase transport (documented weakness, not blocking this
  EIMP).** The `checked to verified` passphrase travels as plaintext inside
  `POST /einmo/<session>/execute`'s request body (§4, Abstract). Accepted
  for this prototype — the socket is unix-domain, local-only, mode-700
  discipline applies — but it is a real regression from the direct-CLI
  path's `/dev/tty` prompt read by the process that derives the key. Worth
  a real design pass (e.g. deriving the key client-side and never
  transmitting the passphrase at all, or a challenge/response scheme) before
  `EIMP-1`'s TCP+bearer-token mode (§S.7) is ever built — plaintext-over-UDS
  is a materially different risk than plaintext-over-TCP would be, so this
  is not an emergency, but it should not be silently carried forward either.

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
- Code: `src/{einmo_suite,transitions,signature,verify,format,compare,stage}.rs`
  (`stage::mirror_input_path` is what `EinmoId`, §0, formalizes);
  `scripts/experimental_reviewer.sh` (reference material, untouched, §6) and
  the new `scripts/einmo_review_client.sh`; `Cargo.toml` (existing
  `einmo`/`cargo-einmo` `[[bin]]` pattern this EIMP's binaries follow);
  `foolish-rust`'s `zweimomo/src/evaluators.rs` (`BoaEvaluator`, ported by
  §8) and `zweimomo/suites/javascript/` (ported test fixtures, §8).
