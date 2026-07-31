---
eimp: 7
title: EinmoCase / EinmoSuite / EinmoDirectory — unify case access behind an EinmoStorage trait
author: Claude Code (Sonnet 5) <noreply@anthropic.com>
status: Implementing
type: Standards
created: 2026-07-31
supersedes: []
begun: [x]
---


# EIMP-7: EinmoCase / EinmoSuite / EinmoDirectory — unify case access behind an EinmoStorage trait

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

## Abstract

Introduce three explicit types — **`EinmoCase`** (one case's full cross-stage
bundle: `input`/`output`/`checked`/`verified`, plus every operation
performable on it as itself), **`EinmoSuite`** (an in-memory collection of
`EinmoCase`s, built by scanning a suite once) and **`EinmoDirectory`** (the
filesystem reader/writer a suite is built from) — behind a new
**`EinmoStorage`** trait, so `einmo test`'s FAE/FF runner and `einmo
review`'s worklist share one case-scanning, cross-stage-comparison, and
promotion implementation instead of two independently-maintained ones.
Renames the existing `einmo_suite::EinmoSuite` (the test-runner) to
`EinmoTestRunner` to free the name for the new suite-of-cases type, and
moves `flagged/` from a top-level directory into each stage
(`output/flagged/`, `checked/flagged/`, `verified/flagged/`) so a flag's
origin stage is visible to a reviewer (§S.2a). This is `EIMP-1`'s **P1**
finding, spun into its own EIMP per that finding's own recommendation,
given the blast radius across `einmo_suite.rs`, `review.rs`,
`transitions.rs`, `compare.rs`, `corpus_signer.rs`, and `cli.rs`.

## Motivation

`EIMP-1`'s maintainer-review pass (`EIMP-1.plan.md`, "Maintainer-found
defects", P1) found that `einmo_suite.rs`'s `scan_tests`/`TestRow` and
`review.rs`'s per-id operations each implement their own ad hoc version of
"walk the suite, match files across stage directories, compare bodies,
promote/flag" — and that this drift is not hypothetical: `transitions.rs`'s
`promote` (clobbers the destination on every promotion) and `review.rs`'s
private `promote_one_accumulating` (reads the existing destination first and
appends onto it, preserving prior signers' stamps) already disagree, and
only the review path has the correct multi-signer behavior. A closer read
during this EIMP's drafting found the drift is narrower than P1 first
suggested — `TestRow`/`scan_tests` is *already* the one function both
`einmo test` and `einmo review` call for listing/diffing (`review.rs:15`
imports it directly) — but the finding that motivated P1 stands: there is no
single owned type that carries a case's data *and* its operations, so
`flag`/`retract`/`promote` live as free functions taking `(&TestConfig,
&EinmoId)` rather than methods on the case they act on, and the one place
that *did* need different promote semantics (multi-signer accumulation)
grew its own private copy instead of a shared one.

A second, sharper problem P1 named directly: `TestRow::differing`
(`einmo_suite.rs`) is `true` if *any* stage is absent OR any two present
stages disagree, and `review.rs`'s `ReviewMode::NewOrBroken` reuses that
exact boolean, even though its own documentation promises "differs
**between output and checked**" specifically. On a fresh suite where
`verified/` simply hasn't been populated yet — the normal starting state —
every case reads `differing: true` regardless of whether output and checked
actually agree, making `-n`/`NewOrBroken` nearly useless. The fix the
maintainer directed is architectural, not a boolean patch: `einmo test`
(must *fail* when conditions aren't met) and `einmo review` (must *prompt
the reviewer to act*) are different consumers of the same underlying
stage-comparison, and should each derive their own, correctly-scoped
predicate from shared structured data instead of both reading one
overloaded `bool`.

**A third comparison implementation, found during this EIMP's drafting and
not named by P1 at all**, makes the case sharper still. `compare.rs`'s
`compare(config, a, b, MatchSections, files)` → `ComparisonResult` is a
*richer* pairwise stage comparison than `scan_tests`': it is
**policy-driven** (`MatchSections` — INPUT and every OUTPUT[i] always
required, DIFF on dependents, COMMENTS only under
`InputOutputComments`), it reports **which sections** diverged, and it
distinguishes **`tampered`** (verify-on-inspect failed) from `differing`
and from one-sided presence (`only_in_a`/`only_in_b`). `einmo test`
already uses it for both jobs that matter — `stage_pair_problems`
(`einmo_suite.rs:639`, which turns each diverging section into its own
`Problem::SectionDifference`) and `require_correspondence`
(`einmo_suite.rs:987`) — and `cli.rs`'s `einmo compare` verb exposes it
directly with `--require-match`/`--root-cause`.

So the split is not "two consumers, one shared scan" but **the good
comparison and the crude one, divided by consumer**: `einmo test` gets
section-aware, policy-driven, tamper-distinguishing results; `einmo
list` and `einmo review` get `scan_tests`'s single bool computed over
*every* non-STAMPS section with no policy at all. That difference is
directly observable as a bug: a case whose `COMMENTS` differ but whose
`INPUT`/`OUTPUT` agree is **clean** to `einmo test` (under the default
`MatchSections::InputOutput`) and **differing** to `einmo review` — the
same suite, two contradictory answers, neither obviously wrong from its
own side. This is P1's disease with a second symptom, and it settles what
the shared core must be: `compare.rs`'s section-aware comparison, not
`scan_tests`'s `body_sections` equality. §S.3's `StageAgreement` is
therefore specified as a per-case projection *of that core*, so unifying
the consumers raises `review`'s fidelity to `test`'s rather than
flattening `test`'s down to `review`'s.

A fourth motivation, added during this EIMP's drafting (`EIMP-1.plan.md` P1
entry, 2026-07-31 discussion): `EIMP-5` (Merkle-tree corpus signing, still
`Draft`, not yet begun) will need to fold per-case digests into a tree.
`CorpusSigner` (`EIMP-1` §S.11) already reads one file at a time via
`EinmoId::to_stage_path` — compatible with a leaf-per-file Merkle fold with
no redesign — but a further idea surfaced in that discussion: since
`EinmoId` is itself a nested path (`foop/23/sub_feature/test1`, mirroring
`input/foop/23/sub_feature/test1.<ext>`), a Merkle tree could hash **at
every directory level** — mirroring the suite's real section/subsection
structure — rather than folding a flat, sorted, arbitrarily-paired binary
tree over the leaf list as `EIMP-5` §S.2 currently sketches. This EIMP does
not implement that (`EIMP-5` is out of scope, still `Draft`), but its
`EinmoSuite`/`EinmoStorage` design must not foreclose it — see §S.5 below
and the corresponding note added to `EIMP-5`'s Open Questions.

## Specification

### S.0 Naming resolution (must land first)

`einmo_suite.rs` already defines `pub struct EinmoSuite` — the *test
runner* (`evaluate`/`evaluate_all`/`check_integrity`), not a case
collection. This EIMP's `EinmoSuite` is a different concept and needs the
name. The existing type is renamed **`EinmoTestRunner`**, in place, with no
behavior change — every call site (`cli.rs`, `review_server.rs`, tests)
updates its type name only. `EinmoTestRunner::new(config)`,
`::evaluate_all(...)`, `::check_integrity(...)` keep their existing
signatures.

### S.1 `EinmoStorage` — a byte-addressable trait, not a filesystem-shaped one

```rust
/// Where one case's stage artifacts and its input actually live. The
/// contract is byte-addressed by `(EinmoId, ArtifactLocation)` —
/// deliberately NOT path-shaped, so a non-filesystem implementation (a
/// database, EIMP-1 §P1's own suggestion) never has to fake directories.
pub trait EinmoStorage {
    /// Read one artifact's raw bytes, or `None` if it does not exist.
    fn read(&self, id: &EinmoId, at: ArtifactLocation) -> Result<Option<Vec<u8>>>;

    /// Write (create or overwrite) one artifact's raw bytes.
    fn write(&self, id: &EinmoId, at: ArtifactLocation, bytes: &[u8]) -> Result<()>;

    /// Remove one artifact. A no-op (not an error) if it does not exist.
    fn remove(&self, id: &EinmoId, at: ArtifactLocation) -> Result<()>;

    /// Every case id with SOMETHING at `at` (an input file, or a stage
    /// artifact) — the union this trait can enumerate at all. Building the
    /// full cross-stage union (today's `scan_tests`) means calling this
    /// once per `ArtifactLocation` and unioning the results; that stays
    /// `EinmoSuite`'s job, not this trait's (see §S.5).
    fn list_ids(&self, at: ArtifactLocation) -> Result<Vec<EinmoId>>;
}

/// One place an artifact can live. `Stage` alone (`stage.rs`) cannot
/// name the input tree, nor the flagged sink within a stage, which is
/// why this is its own enum rather than a reuse of `Stage`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactLocation {
    /// The `input/` tree — the source a case is generated from.
    Input,
    /// A stage's own directory: `output/`, `checked/`, `verified/`.
    Stage(Stage),
    /// The flagged sink *within* a stage: `output/flagged/`,
    /// `checked/flagged/`, `verified/flagged/` (§S.2a). Carrying the
    /// stage means the origin of a flag is recoverable from where the
    /// artifact sits, which is exactly what today's single top-level
    /// `flagged/` discards.
    Flagged(Stage),
}
```

`EinmoStorage` intentionally has **no notion of directories, sections, or
nesting** — that structure lives in `EinmoId` (already a validated,
nested, forward-slash path — `foop/23/sub_feature/test1`, confirmed against
real fixture paths during drafting) and in `EinmoSuite` (§S.5), which
derives the directory tree *from* the flat id list on demand rather than
having `EinmoStorage` model it. This is what keeps a future database-backed
`EinmoStorage` honest: it stores bytes keyed by id and location, full stop.

### S.2 `EinmoDirectory` — the filesystem `EinmoStorage`

```rust
/// The filesystem `EinmoStorage`: one suite root directory, its `input/`
/// tree, and its four stage directories, addressed exactly as
/// `EinmoId::to_stage_path` / `mirror_input_path` (`stage.rs`) already do.
/// Owns no cache and no in-memory state — every call touches disk, same as
/// today's free functions it replaces.
pub struct EinmoDirectory {
    config: TestConfig,
}

impl EinmoDirectory {
    #[must_use]
    pub fn new(config: TestConfig) -> Self { ... }
}

impl EinmoStorage for EinmoDirectory {
    // read/write/remove delegate to EinmoId::to_stage_path (stage
    // artifacts) or config.input_path().join(...) (Input), reusing
    // stage.rs's existing ensure_parent_dir/mirror_input_path — no new
    // path-construction logic, just a trait wrapper over what stage.rs and
    // config.rs already do.
    // list_ids(Input) wraps walk_input_tree + EinmoId::from_input_rel;
    // list_ids(Stage(s)) wraps walk_input_tree(config.stage_dir(s), ...)
    // + EinmoId::from_stage_artifact_path — exactly scan_tests's existing
    // per-stage walk, factored out.
}
```

**The `input/` + per-stage directory split is NOT collapsed by this EIMP
and `EinmoDirectory` does not hide it.** This is intentional,
pre-existing design (`stage.rs`'s `Stage::dir_name`, `config.rs`'s
`stage_dir`) that this EIMP preserves: a human authoring a test types
into `input/`, looks at `output/` to see what the harness produced, and
each stage's directory is independently browsable and hand-editable on
disk. `EinmoStorage`/`EinmoDirectory` exist to give that SAME layout a
byte-addressed trait interface — one `EinmoDirectory` per suite root,
internally resolving every `(EinmoId, ArtifactLocation)` to a real,
inspectable path — not to merge, hide, or reshape the directories
themselves. A non-filesystem `EinmoStorage` (§C, Rejected Alternatives)
is free to organize its backing store however it likes; `EinmoDirectory`
specifically is defined as "the directories a human reads, unchanged."

The one layout change this EIMP *does* make is §S.2a, and it is made for
the same reason: to serve manual inspection and the reviewer.

### S.2a `flagged/` moves inside each stage

**Today**: one top-level `flagged/`, a sibling of `output/`/`checked/`/
`verified/`, and `Stage::Flagged` is a fourth variant of the `Stage` enum.
Every flag from every stage lands in the same directory.

**After this EIMP**: each stage owns a `flagged/` child —
`output/flagged/`, `checked/flagged/`, `verified/flagged/` — and
`Stage` drops to three variants: `Output`, `Checked`, `Verified`.
"Flagged" stops being a stage and becomes a *location modifier on* a
stage (`ArtifactLocation::Flagged(Stage)`, §S.1).

**Why.** This is fine-grained state **the reviewer needs**. A flag raised
against an `output`-stage artifact and a flag raised against a
`verified`-stage one mean different things — the first says "the harness
produced something wrong", the second says "something already attested by
a human is wrong" — and a reviewer triaging a suite must be able to tell
them apart. `transitions.rs:68`'s legal-transition table already
distinguishes `(Output, Flagged)`, `(Checked, Flagged)` and
`(Verified, Flagged)` as three separate transitions, so the *model*
has always carried this distinction; only the filesystem discarded it,
which meant recovering a flag's origin required reading the advisory
text, if it was recoverable at all. Putting the flag next to the stage it
came from makes the distinction visible to `ls`, which is the same
standard §S.2 holds the rest of the layout to.

**A welcome side effect**: `Stage` (3) and `ValidationLevel` (3) now have
matching variants, removing the cardinality mismatch §S.9 previously had
to explain away. They remain distinct concepts — *where an artifact is*
vs. *how strict the gate is* — but a reader no longer has to hold "except
`Flagged`, which is a stage but never a level" in their head.

**Migration.** This breaks the on-disk layout of every existing suite,
including the live fixture at `zweimomo/suites/javascript/day.1/flagged`.
Because `flagged/` is a terminal sink that nothing reads back into the
promotion flow, migration is a **move, not a conversion**: artifacts in
the old top-level `flagged/` have no recorded origin stage, so they
cannot be automatically distributed among the three new directories. The
plan therefore migrates them to `output/flagged/` (the most conservative
reading — "flagged at some point, origin unknown") and says so in the
commit, rather than guessing per-file. `StageDirs::flagged` remains a
configurable *name*; only its parent changes.

### S.3 `EinmoCase` — one case, its data, and its operations

```rust
/// One case's full cross-stage bundle: its id, and what's at each of
/// input/output/checked/verified. Replaces `TestRow` (identical shape:
/// `id` for `rel`, `presence` folds `stages: Vec<(Stage, Option<String>)>`
/// into something P1's fix can build on — see StageAgreement below) and
/// carries the operations `transitions.rs`'s free functions and
/// `review.rs`'s private `promote_one_accumulating` currently perform
/// on a case from outside it.
pub struct EinmoCase<'s, S: EinmoStorage> {
    id: EinmoId,
    storage: &'s S,
}

impl<'s, S: EinmoStorage> EinmoCase<'s, S> {
    /// Read one location's artifact, verify-on-inspect
    /// (`EinmoFile::from_file`'s in-memory equivalent), `None` if absent.
    ///
    /// # Errors
    /// Returns [`EinmoError::Verification`] if the artifact exists but its
    /// stamp chain does not verify — never returned silently as absent.
    pub fn read(&self, at: ArtifactLocation) -> Result<Option<EinmoFile>> { ... }

    /// The per-location presence/status facts `scan_tests` computes today —
    /// unchanged shape, still needed for `einmo list`'s existing display.
    pub fn stages(&self) -> Result<Vec<(Stage, Option<String>)>> { ... }

    /// The P1 fix: structured, section-aware, policy-driven stage-agreement
    /// facts — not one bool. Each consumer derives its OWN predicate from
    /// this instead of sharing `differing`. Computes every ordered pair
    /// drawn from `stages`, under `policy` (which is recorded in the
    /// result).
    ///
    /// Internally this is `compare.rs`'s `compare_sections` applied
    /// per-pair to ONE case, rather than `compare`'s whole-tree walk —
    /// same policy, same required-section rules, same verify-on-inspect
    /// refusal, one case at a time. §S.7 folds `compare::compare` itself
    /// onto this so there is exactly one implementation.
    pub fn agreement(&self, stages: &[Stage], policy: MatchSections) -> Result<StageAgreement> { ... }

    /// Promote from `from` to `to`, accumulating onto whatever already
    /// exists at `to` if its content matches (multi-signer safe) — this
    /// is `review.rs`'s `promote_one_accumulating` logic, now the ONLY
    /// promote implementation (see S.4: `transitions::promote` becomes a
    /// thin per-case loop over this).
    ///
    /// **The destination-match test here is deliberately NOT
    /// `MatchSections`-policy-driven**, unlike [`Self::agreement`]: it
    /// compares every non-STAMPS section (today's
    /// `body_sections(&existing, None)` equality, carried over
    /// unchanged). Promotion writes bytes and appends an attestation, so
    /// "the destination already holds exactly this content" must mean
    /// *exactly*, including sections a comparison policy is willing to
    /// overlook. Co-signing a file whose COMMENTS differ from what the
    /// signer actually reviewed would make the stamp attest to content
    /// that was never inspected. The asymmetry is intentional and is
    /// tested (§Test Plan).
    ///
    /// # Errors
    /// [`EinmoError::IllegalTransition`] for a disallowed pair,
    /// [`EinmoError::Verification`] if the source fails verify-on-inspect.
    pub fn promote(&self, from: Stage, to: Stage, key: &StageKeypair) -> Result<PromoteOutcome> { ... }

    /// Move this case's `stage` artifact into `flagged/`, concatenating
    /// onto any existing advisory block. Delegates to `transitions::flag`'s
    /// existing (already-correct, already-shared) logic, scoped to one id.
    pub fn flag(&self, stage: Stage, reason: &str) -> Result<()> { ... }

    /// Delegates to `transitions::retract`'s existing logic, scoped to one
    /// id (cascades `checked` → `verified`, same as today).
    pub fn retract(&self, stage: Stage) -> Result<()> { ... }
}

/// The outcome of one `EinmoCase::promote` call — replaces the
/// `(String, bool)` tuple `promote_one_accumulating` returns today with a
/// named enum, per this repo's state/status/error-is-an-enum convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromoteOutcome {
    /// A fresh baseline was written (absent, corrupt, or genuinely
    /// different content at the destination).
    Promoted { non_human: bool },
    /// Content at the destination matched; this signer's stamp was
    /// appended onto the existing file, preserving prior signers'.
    CoSigned { non_human: bool },
    /// The destination already carried this exact content, already signed
    /// by this exact key. Nothing written.
    AlreadySigned,
}

/// How ONE pair of stages stands for ONE case — the per-case projection
/// of `compare.rs`'s `ComparisonResult` (§Motivation, third finding).
/// This is the shared core both consumers derive from: `einmo test`'s
/// `Problem::SectionDifference` generation and `einmo review`'s
/// `NewOrBroken` predicate are two readings of the SAME value, computed
/// under the SAME `MatchSections` policy.
///
/// Deliberately an enum, not a struct with a bool (this repo's
/// state/status convention): the four outcomes are mutually exclusive,
/// and `Tampered` in particular must never be collapsed into
/// "differing" — today `scan_tests` does exactly that, losing the
/// distinction `compare.rs` was careful to make.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagePairAgreement {
    /// Both present; every section required by the policy is
    /// byte-identical.
    Agree,
    /// Both present; these policy-required sections diverged. Never
    /// empty — an empty divergence list is `Agree`.
    Differ { sections: Vec<String> },
    /// Exactly one side is present. Carries which one, so a caller can
    /// tell "not promoted yet" (`present: Output`) from "input deleted
    /// after promotion" (`present: Checked`) without a second lookup.
    OneSided { present: Stage, absent: Stage },
    /// Neither side is present.
    BothAbsent,
    /// At least one side failed verify-on-inspect. The case is refused,
    /// never compared — matching `compare.rs`'s `tampered` bucket, and
    /// matching this crate's standing verify-on-inspect rule.
    Tampered { stages: Vec<Stage> },
}

/// Every pairwise agreement a case needs, computed in one pass over the
/// stages the caller asked about. `EinmoCase::agreement` returns this;
/// each consumer reads the pairs it actually cares about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageAgreement {
    /// Which of the requested stages are present at all.
    pub present: Vec<Stage>,
    /// Which of the requested stages are missing.
    pub missing: Vec<Stage>,
    /// The policy this agreement was computed under — recorded, not
    /// assumed, so a caller can never compare two `StageAgreement`s
    /// derived under different `MatchSections` and think they disagree
    /// about the suite when they disagree about the question.
    pub policy: MatchSections,
    /// One entry per requested ordered stage pair.
    pairs: BTreeMap<(Stage, Stage), StagePairAgreement>,
}

impl StageAgreement {
    /// How `left` and `right` stand. `None` if that pair was not among
    /// the stages requested.
    #[must_use]
    pub fn pair(&self, left: Stage, right: Stage) -> Option<&StagePairAgreement> { ... }
}
```

**Why an enum per pair rather than one `bool` over all stages.** P1's bug
is exactly the loss of information that flattening causes: today
`differing` answers "is anything at all unusual about this case?", which
is a fine question for `einmo list`'s display and a useless one for
`NewOrBroken`. With per-pair results each consumer asks its own question
of the same data — `einmo test` asks about every pair its
`ValidationLevel` requires and emits one `Problem::SectionDifference` per
diverging section (exactly what `stage_pair_problems` already produces via
`compare.rs`, so its behavior is preserved *by construction*); `einmo
review` asks only about `(Output, Checked)` and treats `Differ` and the
`OneSided { present: Output, .. }` case as "needs review", ignoring
whether `verified/` is populated at all. Neither consumer reads a field
computed for the other's benefit.

`EinmoCase` borrows its `EinmoStorage` rather than owning it — cheap to
construct per-id inside a loop (as `scan_tests` does today), and generic
over `S` so tests can plug in an in-memory `EinmoStorage` fake instead of
touching a real tempdir for the module's own unit tests (see Test Plan).

### S.4 `transitions.rs` after this EIMP

`transitions::promote`'s current body (clobber the destination) is
replaced by a loop over `EinmoCase::promote` (§S.3, the accumulating
semantics) — the ONE promote implementation, used by both `einmo promote`
(CLI, direct, no review session) and `einmo review`'s execute path. This is
a **behavior change** for the plain CLI path: promoting the same content
twice with two different keys now co-signs instead of the second call
overwriting stamp history. `flag`/`retract` are unchanged in behavior
(already correct, already shared) — only their internals move to be called
through `EinmoCase` for API consistency; `PromotionReport` keeps its
existing public shape (`Promoted { rel_path, stamp_pubkey, non_human }`) so
`cli.rs`'s existing formatting code does not need to change.

### S.5 `EinmoSuite` — the in-memory case collection

```rust
/// An in-memory snapshot of one suite: every `EinmoCase` with something at
/// input or any stage, built by one scan. Replaces `scan_tests`/`TestRow`
/// as the shared listing/diffing implementation `einmo test` and `einmo
/// review` both already call through today (`review.rs:15`) — this EIMP
/// gives that sharing an owned type instead of a free function + struct
/// pair.
pub struct EinmoSuite<S: EinmoStorage> {
    storage: S,
    ids: Vec<EinmoId>, // sorted, deduplicated; see below
}

impl<S: EinmoStorage> EinmoSuite<S> {
    /// Scan `storage`: union every id found at `Input` and every `Stage`,
    /// sorted and deduplicated — identical union `scan_tests` computes
    /// today, now storage-backed instead of walking the filesystem
    /// directly.
    ///
    /// # Errors
    /// Propagates any [`EinmoStorage::list_ids`] failure.
    pub fn scan(storage: S, filter: Option<&str>) -> Result<Self> { ... }

    /// Every case, in `EinmoId`'s `Ord` (plain id-string order — `EIMP-1`
    /// §S.11a's `Collation` is a `CorpusSigner`-specific ordering over
    /// SIGNED manifests, not a general suite-iteration order; this EIMP
    /// keeps `EinmoSuite`'s own order as today's `rels.sort()` already
    /// does).
    pub fn cases(&self) -> impl Iterator<Item = EinmoCase<'_, S>> { ... }

    pub fn case(&self, id: &EinmoId) -> Option<EinmoCase<'_, S>> { ... }

    /// Group this suite's cases by their `EinmoId`'s path components —
    /// `foop/23/sub_feature/test1` nests under `foop` → `foop/23` →
    /// `foop/23/sub_feature`. Pure and on-demand: no separate tree state is
    /// stored or kept in sync — this is a view computed from `ids` each
    /// call.
    ///
    /// Exists for `EIMP-5` (Merkle-tree corpus signing, still `Draft`):
    /// that EIMP's own drafting raised hashing at every directory level
    /// (mirroring the suite's real section structure) as an alternative
    /// to a flat sorted-leaf binary fold — this method is what such a
    /// signer would walk. Not consumed by anything in THIS EIMP; exists
    /// so `EIMP-5` does not need a second, competing grouping function
    /// once it begins. See the note added to `EIMP-5`'s Open Questions.
    pub fn directory_tree(&self) -> DirectoryNode<'_, S> { ... }
}

/// One node of `EinmoSuite::directory_tree`'s output: a path component,
/// the cases directly at this level (a case can sit at any depth — a bare
/// `input/test1.foo` is a case at the root), and child nodes for deeper
/// path components.
pub struct DirectoryNode<'a, S: EinmoStorage> {
    pub component: &'a str,
    pub cases: Vec<EinmoCase<'a, S>>,
    pub children: BTreeMap<&'a str, DirectoryNode<'a, S>>,
}
```

### S.6 `EinmoTestRunner` and `EinmoReview` after this EIMP

`EinmoTestRunner`'s FAE/FF `Problem` generation and `EinmoReview::items`
both build on `EinmoSuite::cases()` instead of calling `scan_tests`
directly. Each derives its own predicate from `EinmoCase::agreement`
instead of reading a shared `differing` bool:

- `EinmoTestRunner::stage_pair_problems` asks `agreement(...)` for the
  stage pairs its `ValidationLevel` requires, under
  `config.match_sections()`, and maps each `StagePairAgreement` variant
  onto the `Problem` variant it already emits today:
  `Differ { sections }` → one `Problem::SectionDifference` per section;
  `OneSided` → `Problem::RightMissingEntirely`/`LeftMissingEntirely`
  (chosen by which side is present); `Tampered` → the tampered handling
  `compare.rs` already feeds it. **Behavior is preserved by
  construction**, because the underlying comparison is the same
  `compare_sections` call it makes today — only the walk that reaches it
  changes (§S.7).
- `ReviewItem`'s `differing` field is recomputed from
  `agreement(&[Stage::Output, Stage::Checked], config.match_sections())`
  — scoped EXACTLY to what `ReviewMode::NewOrBroken`'s doc comment
  already promises ("differs between output and checked"), and now under
  the same section policy `einmo test` uses. This fixes **both** bugs at
  once: the fresh-suite false positive P1 found (unpopulated `verified/`
  no longer makes every case read as differing) and the
  COMMENTS-disagreement found while drafting this EIMP (`einmo review`
  and `einmo test` can no longer give contradictory answers about the
  same pair).

### S.7 One pairwise-comparison implementation

After §S.3, `compare::compare`'s per-file body — presence check,
verify-on-inspect both sides, `compare_sections` under the policy — is
exactly `EinmoCase::agreement` for a single pair. `compare::compare`
therefore becomes a **thin fold over `EinmoSuite`**: iterate
`suite.cases()`, call `agreement(&[a, b], policy)` on each, and bucket the
resulting `StagePairAgreement` into `ComparisonResult`'s existing
`matching`/`differing`/`only_in_a`/`only_in_b`/`tampered` vectors.

This retires the independent input-tree walk at `compare.rs:104` — the
fourth such walk in the crate — so `einmo compare`, `einmo list`, `einmo
test`, `einmo review`, and `CorpusSigner` (§S.8) all enumerate a suite
exactly once, the same way. `ComparisonResult`, `DiffEntry`,
`compare::compare`'s signature, and `root_causes` all keep their existing
public shapes: `cli.rs`'s `einmo compare` verb (including
`--require-match` and `--root-cause`) needs no changes, and
`einmo_suite.rs`'s two call sites keep calling `compare` exactly as they
do today. The `files: Option<&[PathBuf]>` argument stays too — it becomes
a filter over `suite.cases()` rather than a substitute id list.

### S.8 `EinmoSuite` drives `CorpusSigner`

`CorpusSigner::manifest_under` currently calls `walk_input_tree` and
`EinmoId::from_stage_artifact_path` directly (`corpus_signer.rs:100-115`)
— a third independent walk, and a caller (`cli.rs`) that has to assemble
the signer, hand it a config and a stage, and know when re-signing is
needed.

**The suite owns that instead.** `EinmoSuite` gains the driving method;
`CorpusSigner` stays exactly what it is (the construction: manifest →
digest → sign/verify) and gains no knowledge of suites:

```rust
impl<S: EinmoStorage> EinmoSuite<S> {
    /// Bring this suite's section signature up to date. Builds the
    /// manifest from the cases ALREADY scanned into this suite (no
    /// fourth directory walk), constructs a `CorpusSigner` over it,
    /// reads each artifact's bytes through `EinmoStorage`, and writes
    /// the `.section.sig` only where it is actually absent or stale.
    ///
    /// Returns what it did, per section — an unchanged section is
    /// reported as such, never silently re-signed.
    ///
    /// # Errors
    /// [`EinmoError::CorpusSignature`] on an unrecognized collation or a
    /// manifest/digest inconsistency; [`EinmoError::NoKey`] if no key
    /// material resolves.
    pub fn update_corpus_signature(
        &self,
        stage: Stage,
        key: &KeySource,
    ) -> Result<CorpusSignatureUpdate> { ... }
}

/// What `update_corpus_signature` did. An enum per this repo's
/// state/status convention — "signed" and "already current" are
/// different outcomes and a caller (or a test) must be able to tell
/// them apart without diffing files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorpusSignatureUpdate {
    /// No `.section.sig` existed; one was written.
    Created { manifest_len: usize },
    /// The recorded digest no longer matched the corpus; re-signed.
    Updated { manifest_len: usize },
    /// The existing signature already matches. Nothing written.
    AlreadyCurrent,
}
```

Three consequences worth stating, because they are the point:

1. **The manifest comes from in-memory suite state**, not a fresh walk —
   `cases()` has already enumerated exactly the ids the manifest needs,
   in a deterministic order.
2. **Bytes come through `EinmoStorage`**, so a non-filesystem backend is
   signable with no change to `CorpusSigner`. Today's `digest_for` reads
   via `id.to_stage_path` and therefore silently assumes a filesystem.
3. **"Update where needed" is the method's job**, not the caller's. The
   `AlreadyCurrent` arm is what makes re-signing cheap enough to run
   routinely — and `EIMP-5` §S.4's incremental re-signing becomes an
   optimization *inside* this method, with the same signature and the
   same three outcomes, rather than a new API.

Per-artifact digest computation inside `CorpusSigner` is otherwise
unchanged — the byte-join construction and its digest are **byte-identical
to before this EIMP** for the same corpus. Only the manifest's source and
the byte-reading path change. This is also what §S.5's `directory_tree`
exists for: `EIMP-5`'s future Merkle signer becomes a different fold over
the same `EinmoSuite`, reached through the same `update_corpus_signature`
entry point.

### S.9 Vocabulary — `stage`, `level`, `section`, and the words this EIMP retires

Three near-synonyms circulate in this codebase's design conversations, and
one of them is not a concept at all. Pinning them here because the
ambiguity has already cost one round-trip during this EIMP's drafting, and
because every new type in §S.1–§S.8 is parameterized by one of them.

| Term | Type | Cardinality | What it answers |
|------|------|-------------|-----------------|
| **stage** | `Stage` (`stage.rs`) | 3: `Output`, `Checked`, `Verified` (§S.2a drops `Flagged`) | *Where does this artifact live, and what step of the promotion lifecycle is it at?* |
| **level** | `ValidationLevel` (`einmo_suite.rs`) | 3, **ordered**: `Output` < `Checked` < `Verified` | *How strict is `einmo test`'s gate — how much must hold for this suite to pass?* |
| **section** | section names within an `EinmoFile`, selected by `MatchSections` (`config.rs`) | per file: `INPUT`, `OUTPUT[i]`, `DIFF`, `COMMENTS`, `STAMPS` | *Which parts of two artifacts must be byte-identical for them to count as equal?* |

**`stage` and `level` are genuinely different and both names are correct.**
They share three variant names (`Output`/`Checked`/`Verified`) on purpose:
level `Checked` is *defined* as "everything through stage `Checked` holds."
The codebase already bridges them explicitly — `ValidationLevel::stage()`
(`einmo_suite.rs:117`) maps a level to the stage it judges, and
`ValidationLevel::escalation()` gives the levels it subsumes. No rename is
proposed; the distinction is real and already well-modeled.

After §S.2a the two have **matching variants** (`Output`, `Checked`,
`Verified`), which makes the pairing easier to hold in the head, not
harder — but they must not be merged. A stage is a *place*; a level is a
*standard*. `verified/` the directory exists whether or not anyone
demands `ValidationLevel::Verified` of the suite, and a suite validated at
level `Checked` still has a `verified/` stage directory it simply does
not judge.

**The prose form is the preferred one in documentation**: write "the
output stage", "the checked stage", "the verified stage" — not "output"
bare (ambiguous with the `OUTPUT` section of an envelope) and not
"the output directory" (which names the storage, not the lifecycle step).
`AGENTS.md`, `README.md`, and the doc comments should be brought onto this
phrasing (tracked in the plan's follow-ups).

**"slice" is retired.** It appeared in this spec's earlier Open Questions
naming the Rust type `&[Stage]`, sitting in a list beside two domain
concepts as though it were a third. It is not — it is a representation of
"some stages." Wherever a set of stages is meant, this spec says **"the
stages to compare"** and types it `&[Stage]`; the word "slice" carries no
design meaning and is not used again.

**Applying the distinction** (this resolves the second Open Question):
`EinmoCase::agreement` takes **stages**, never a level. Comparison is
mechanical — "compare these places" — and belongs to the case. A *level*
is `einmo test`'s policy about how strict to be, so `EinmoTestRunner` is
what translates its `ValidationLevel` into the stage pairs to ask about
(via `escalation()` + `stage()`, both of which already exist). `einmo
review` has no `ValidationLevel` at all and simply asks about
`[Output, Checked]`. Pushing levels down into `EinmoCase` would give the
case a concept only one of its two consumers has — which is precisely the
overloading P1 was about.

### S.10 Module layout, and who receives an instruction

**Module layout** (resolves the first original Open Question): three new
files, one type each — `src/storage.rs` (`EinmoStorage`,
`ArtifactLocation`, `EinmoDirectory`), `src/case.rs` (`EinmoCase`,
`StageAgreement`, `StagePairAgreement`, `PromoteOutcome`), `src/suite.rs`
(`EinmoSuite`, `DirectoryNode`, `CorpusSignatureUpdate`).
`einmo_suite.rs` keeps only the renamed `EinmoTestRunner` and its
`Problem`/`ValidationLevel`/`SuiteIntegrity` machinery — it is already
2928 lines, and folding two more core types into it would produce a file
no one can hold in their head.

**Who receives an instruction** (resolves the third original Open
Question): **the suite is told; the suite tells the cases.** At einmo's
core level, `promote`/`flag`/`retract` are given to `EinmoSuite`, which
resolves which cases the instruction applies to and delegates the
per-case work to `EinmoCase`:

```rust
impl<S: EinmoStorage> EinmoSuite<S> {
    /// Promote every case matching `filter` from `from` to `to`.
    /// Derives the `StageKeypair` ONCE and lends it to each case —
    /// Argon2id derivation is ~1.8s, so per-case derivation would make a
    /// 161-case promotion take ~5 minutes of pure CPU for ~0.2ms of
    /// signing (the discipline `transitions::promote` established and
    /// this EIMP must not lose).
    pub fn promote(&self, from: Stage, to: Stage, key: &KeySource, filter: Option<&str>)
        -> Result<PromotionReport> { ... }

    pub fn flag(&self, stage: Stage, reason: &str, filter: Option<&str>) -> Result<FlagReport> { ... }
    pub fn retract(&self, stage: Stage, filter: Option<&str>) -> Result<RetractReport> { ... }
}
```

This is the layering the whole EIMP is for: **selection is a suite
concern** (which cases match this filter, in what order, under what
policy) and **application is a case concern** (read this artifact, verify
it, append a stamp, write it). Today both halves live tangled together in
`transitions.rs`'s free functions, which is why `review.rs` — needing the
same application with different selection — grew a private copy of the
application half.

`transitions.rs` accordingly shrinks to the pieces that are genuinely
neither selection nor per-case application: the legal-transition table
(`is_legal_transition`), the report types (`PromotionReport`,
`FlagReport`, `RetractReport`, `NoteReport`), and `flag`'s advisory-block
concatenation. Its `promote`/`flag`/`retract` free functions are removed;
`cli.rs` constructs an `EinmoSuite` and calls the methods above, which is
one line more at each of its call sites and removes the duplication
entirely.

**Blast radius of §S.2a's `flagged/` move**, recorded here so the plan
can phase it: `Stage::Flagged` is referenced 27 times across 6 files —
`transitions.rs` (15), `stage.rs` (4), `einmo_suite.rs` (3),
`verify.rs` (2), `config.rs` (2), `bin/einmo_review_server.rs` (1) — and
removing the variant touches `Stage::ALL`, `dir_name()`, `stamp_key()`
(dead for `Flagged`: flagging appends an advisory, never a stamp),
`is_legal_transition`'s three `* → Flagged` rows, `promote`'s
`to == Stage::Flagged` delegation, `count_flagged`, the R2 orphan
exemption, and the review server's flag/retract endpoints. It also breaks
every existing suite's on-disk layout (migration in §S.2a). This is why
§S.1's `ArtifactLocation` is addressed by location rather than by path:
`Flagged(Stage)` slots in as a third variant with no change to the trait's
shape.

## Test Plan

- `src/storage.rs` (new): `EinmoStorage`/`EinmoDirectory`/`ArtifactLocation`
  unit tests — read/write/remove round-trip, `list_ids` for each location,
  absent-artifact returns `None` not an error. An in-memory `EinmoStorage`
  test fake (e.g. `HashMap<(EinmoId, ArtifactLocation), Vec<u8>>`) is added
  here too, gated `#[cfg(test)]` but exported `pub(crate)` so
  `case.rs`/`suite.rs`'s own unit tests can use it without a tempdir.
- `src/case.rs` (new, or added to `stage.rs`): `EinmoCase` unit tests —
  `stages()` matches `scan_tests`'s existing output shape; `agreement()`
  covering **every `StagePairAgreement` variant**: `Agree`; `Differ`
  (asserting the *section names*, not merely that it differs);
  `OneSided` in both directions (asserting which stage is `present`);
  `BothAbsent`; and `Tampered` — the last asserting a tampered artifact
  is reported as `Tampered` and **never folded into `Differ`**, the
  distinction `scan_tests` loses today. Two named regression tests carry
  the bugs this EIMP fixes:
  - **the P1 repro** — fresh suite, `output` and `checked` agree,
    `verified/` empty → `pair(Output, Checked)` is `Agree` (today's
    `differing` bool is `true` here);
  - **the COMMENTS repro** (§Motivation, third finding) — two stages
    agreeing on INPUT/OUTPUT but differing in COMMENTS → `Agree` under
    `MatchSections::InputOutput`, `Differ { sections: ["COMMENTS"] }`
    under `InputOutputComments`, proving the policy is honored and that
    `einmo test` and `einmo review` can no longer disagree.

  Plus `promote()`'s three `PromoteOutcome` variants, each asserted
  against a byte-for-byte comparison with today's
  `promote_one_accumulating` (which this replaces) to confirm the
  behavior carried over exactly; a test pinning the **deliberate
  asymmetry** of §S.3 (a destination differing only in COMMENTS is *not*
  treated as matching by `promote`, even under a policy that would call
  it `Agree` — promotion attests to exact bytes); and `flag`/`retract`
  delegating correctly.
- `src/compare.rs` (§S.7): the existing `compare::compare` test suite
  must pass **unedited** after its body is refolded onto
  `EinmoCase::agreement` — that is the phase's whole acceptance
  criterion. Add a baseline test capturing `ComparisonResult` for each
  existing fixture *before* the refold, so behavior preservation is
  proven rather than assumed, and a test that the `files:
  Option<&[PathBuf]>` argument still selects the same subset.
- `src/suite.rs` (new, or added to `einmo_suite.rs`): `EinmoSuite::scan`
  against the in-memory fake AND a real `EinmoDirectory` fixture (parity
  test: same suite scanned both ways yields the same case set); `cases()`
  ordering; `directory_tree()` grouping against a multi-level fixture
  (`foop/23/sub_feature/test1`-shaped ids) — assert every case appears in
  exactly one node, at the right depth, and the tree contains no node for
  a path component with neither cases nor further children.
- `einmo_suite.rs`: `EinmoTestRunner` (renamed) existing test suite passes
  unchanged (behavior-preserving rename, S.0) plus new tests confirming
  `Problem` generation now sourced from `StageAgreement` produces identical
  `Problem` variants/paths as before, for every existing fixture.
- `review.rs`: `EinmoReview::items()` tests updated to assert the FIXED
  `differing` semantics — the P1 repro (fresh suite, `verified/` empty,
  `output`==`checked`) now reads `differing: false`. Existing
  `promote`/`flag`/`retract`/execute tests continue to pass unchanged
  (behavior-preserving except where S.4 explicitly changes CLI promote
  semantics — the tests exercising THAT change move from asserting
  clobber to asserting co-sign, with the old assertion kept as a comment
  pointing at this EIMP for anyone auditing the behavior change).
- `transitions.rs`: `promote`'s test suite updated for the new
  co-sign-on-matching-content behavior (S.4); a regression test added for
  "promote same content with two different keys → both stamps present"
  (today this only has coverage inside `review.rs`'s own tests).
- `corpus_signer.rs`: `manifest_under`/signing tests updated to build
  their `EinmoSuite` first and pass it in; digest values must be BYTE-
  IDENTICAL to before this EIMP for the same fixture (the manifest source
  changed, the manifest content must not).
- Comprehensive test (added to whichever of the above hosts it, likely
  `suite.rs`): scan a realistic multi-section, multi-depth fixture suite
  through `EinmoSuite`, exercise `einmo test`-shaped FAE/FF validation and
  `einmo review`-shaped promote/flag/retract/list through the SAME
  `EinmoSuite` instance, and assert both consumers' view of the suite
  agrees on every case's presence and agreement facts — the property this
  whole EIMP exists to guarantee. The fixture includes a case differing
  only in COMMENTS (asserted consistent across both consumers under one
  policy) and a tampered artifact (asserted reported as tampered, not as
  differing, by both).
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
  --check` all clean before completion, per this repo's standing rule.

## Rejected Alternatives

### A. Patch `TestRow::differing` into two booleans and stop there

Split `differing` into `differing_for_test` and `differing_for_review`
without introducing `EinmoCase`/`EinmoStorage`. Rejected: this fixes P1's
symptom but not its cause — the promote-path drift (`transitions::promote`
vs `promote_one_accumulating`) is a SEPARATE instance of the same "no
shared owned type" problem, and a second overloaded-field bug of this shape
is only a matter of time without one. The maintainer's own direction
(`EIMP-1.plan.md` P1, 2026-07-31) explicitly asked for the layered core,
not a boolean patch.

### B. Do nothing — leave both consumers independent

`einmo test` and `einmo review` already share `scan_tests`/`TestRow` for
reading; only `promote` has actually drifted so far. Rejected: the drift
that exists is a real, already-shipped-and-found bug
(multi-signer-accumulation silently missing from the CLI promote path),
and the read-path sharing that DOES exist today is via a free function +
a `pub(crate)` struct with no operations on it — every future addition
(this EIMP's own `EinmoStorage`/`EIMP-5`'s Merkle signer) would otherwise
need to choose between extending that struct ad hoc or, again, writing a
parallel implementation.

### C. Make `EinmoStorage` path-shaped (`fn read(&self, path: &Path)`) instead of id+location-shaped

Simpler to implement for `EinmoDirectory` (closer to what `stage.rs`
already does), but forecloses a non-filesystem backend from day one — a
database implementation would have to invent fake paths. Rejected per the
suggestion that motivated this EIMP (`EinmoStorage` should "leave open the
ability to store the tests in databases and other places as well"):
`(EinmoId, ArtifactLocation)` is the actual addressing scheme every
backend must support; a `Path` is just `EinmoDirectory`'s encoding of it.

### D. Build `EinmoSuite`'s directory grouping (§S.5 `directory_tree`) eagerly, stored alongside `ids`

Rejected for now: nothing in this EIMP consumes it (`EIMP-5` does, and
`EIMP-5` is still `Draft`, unbegun). Computing it on demand from the flat,
already-validated `ids` list is cheap (`EinmoId`'s path is already
parsed/validated once at scan time) and has no staleness-vs-`ids` state to
keep in sync. Revisit if `EIMP-5`'s begun-time design actually needs the
tree persisted (e.g. for incremental re-signing, `EIMP-5` §S.4) rather than
recomputed per signing run.

## Open Questions

*(Empty. All four of this EIMP's Open Questions were resolved on
2026-07-31 in conversation with the maintainer and are recorded in the
spec body: module layout and promote/flag/retract ownership in §S.10, the
stage-vs-level argument in §S.9, and `flagged/`'s location in §S.2a. Per
`EIMP-0`, an empty Open Questions section on an `Implementing` EIMP means
the design is frozen — reopen it explicitly rather than deciding
otherwise mid-implementation.)*

## References

- `EIMP-1` (`docs/eimp/EIMP-1.md`) — the review-session design this EIMP's
  `review.rs` changes build on; `EIMP-1.plan.md` "Maintainer-found defects,
  P1" — the finding this EIMP implements, including the maintainer's
  2026-07-31 architectural direction and the layered-core sketch this
  spec's naming is drawn from almost verbatim.
- `EIMP-5` (`docs/eimp/EIMP-5.md`) — Merkle-tree corpus signing, `Draft`,
  unbegun; §S.5's `directory_tree` and §S.8's `CorpusSigner` association
  exist specifically so that EIMP's future signer builds on this one's
  `EinmoSuite` rather than a fourth independent directory walk. A note
  recording the directory-mirrored-hash-levels idea from this EIMP's
  drafting was added to `EIMP-5`'s Open Questions on 2026-07-31.
- Code: `src/stage.rs` (`EinmoId`, `Stage`, `walk_input_tree`,
  `mirror_input_path`) — the addressing primitives `EinmoDirectory` wraps,
  unchanged by this EIMP; `src/einmo_suite.rs` (`TestRow`, `scan_tests`,
  `EinmoSuite`/to-be-`EinmoTestRunner`); `src/review.rs`
  (`promote_one_accumulating`, `ReviewItem`, `EinmoReview::items`);
  `src/transitions.rs` (`promote`, `flag`, `retract`);
  `src/corpus_signer.rs` (`CorpusSigner::manifest_under`).
