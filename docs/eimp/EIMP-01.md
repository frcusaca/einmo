---
eimp: D10
title: A separate generation phase writing an uncommitted generated/ stage, and validation levels that compare only against their predecessor
author: Claude Code (Opus 5) <noreply@anthropic.com>
status: Implementing
type: Standards
created: 2026-08-11
supersedes: [EIMP-3]
begun: [x]
---

# EIMP-01: A separate generation phase writing an uncommitted `generated/` stage, and validation levels that compare only against their predecessor

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root — **read it before creating or editing an EIMP.**

## Abstract

Einmo evaluation writes directly into `output/`, and its three validation
levels are cumulative (`Verified ⊃ Checked ⊃ Output`), so asserting a fact
about two committed files re-runs the evaluator over the whole suite first.
This EIMP separates **generation** from **gating**. A new, gitignored fourth
stage — `generated/` — becomes the only directory the evaluator writes to,
reached by a new `einmo generate` phase whose sole rule is *the evaluator ran
and the artifact it wrote is a self-verifying `.einmo`*. `output/` becomes a
committed baseline reached by an explicit `einmo promote generated to output`.
The three gates keep their names and stay the only gates, but stop escalating:
each compares against its immediate predecessor and nothing else.

```
generated  →  output  →  checked  →  verified
 (it ran)    (reasonable)  (reviewed)  (attested)
```

Generation is formally separate from the Output gate so it can be run on its
own — as a plain "does everything still run?" test, and as the way an agent or
human materializes fresh results **for direct inspection without disturbing the
committed `output/`** — while the Output gate invokes it and then compares.

## Motivation

### Generation and judgment are two different acts

Today they are one. `EinmoTestRunner::evaluate` runs the evaluator and, in the
same breath, decides whether what it produced is acceptable — `write_output`
compares the fresh result against the signed `output/` baseline and fails the
case on any difference (`src/einmo_suite.rs:1352`, EIMP 3). There is no way to
say "just run everything and show me what comes out." An agent that has
finished a change and wants to *look at* the new results has only two moves:
run the suite and watch it go red, or run `einmo regenerate-output` and
overwrite the committed baseline before anyone has seen the diff.

Separating the two gives the act a home. `einmo generate` materializes every
result into `generated/`, passing as long as the evaluator ran and produced a
sound artifact. Nothing committed moves. The results sit on disk as ordinary
`.einmo` files, so every tool einmo already has — `einmo show`, `einmo body`,
`einmo compare generated output`, the review server — works on them
unmodified. *Then* a human or agent decides whether to proceed, and records
that decision by promoting.

### `output/` is both a work file and a committed artifact

`EinmoCase::retract` refuses to retract from `output/` on the grounds that it
"is regenerated every run" (`src/case.rs`), yet the 8 artifacts under
`zweimomo/suites/python/output/` are tracked in git. Both statements are true
today, and that is the problem: the same directory is a scratch space the
runner overwrites and a baseline reviewers promote from. Every evaluation
dirties the working tree. **You never commit the work file.**

### Cumulative levels re-evaluate to assert facts about files on disk

`ValidationLevel::escalation()` (`src/einmo_suite.rs:115`) makes `Verified`
perform everything `Checked` performs, which performs everything `Output`
performs. So a gate that wants to assert `checked ≡ verified` — a comparison
of two committed files — first re-checks `output ↔ checked`, and any caller
that pairs a level with an evaluation run re-evaluates the whole suite. Worse,
a failure at the top level does not say *which* link broke.

Under this EIMP each level states exactly one link, and the three together
establish the whole claim by transitivity:

| Gate | Claim |
|------|-------|
| `output` | the code produces what is committed as the baseline |
| `checked` | the baseline matches what was reviewed |
| `verified` | what was reviewed matches what was attested |

A red gate names its own link. Only the `output` gate evaluates; `checked` and
`verified` become pure comparisons that need no evaluator, no runner command,
and no build of the system under test.

### EIMP 3's intent survives, relocated

EIMP 3 made silent baseline drift a failure. That was right, and this EIMP
keeps it — it just moves where the fact is discovered. "The evaluator produced
something different from the baseline" is no longer a per-file verdict buried
inside `write_output`; it is exactly what the `output` gate compares, reported
as a `SectionDifference { left: Generated, right: Output }` naming the file and
the section. EIMP 3's escape hatch (`einmo regenerate-output`) becomes the
promotion, which — unlike the old verb — signs.

## Specification

### S.0 — Product specification

**This section is normative.** It states what einmo does. S.1 through S.8
state how the implementation changes to do it; where they appear to disagree
with this section, this section governs and the later section is a defect.

Einmo holds a suite's results in four stages. Three of them are committed;
`generated/` is not. Content moves forward one stage at a time by promotion,
and each promotion appends a signature. Nothing skips a stage.

```
generated  →  output  →  checked  →  verified
 (it ran)    (reasonable)  (reviewed)  (attested)
```

Four phases act on those stages: one that produces, and three that judge. The
three that judge are gates — the things a merge is gated on. Each gate
compares exactly one adjacent pair, and no gate re-checks another gate's pair.
Their conjunction establishes, by transitivity, that the code produces
human-attested results.

Two rules hold across all four phases:

- **A malformed tree is a failure.** Every phase fails on an extraneous
  (dot-prefixed) file under `input/`, and on a suite that discovered no inputs
  at all. Einmo never silently skips a file in its own tree.
- **Every comparison verifies both sides.** An artifact that fails
  verify-on-inspect is refused, not compared, so einmo never reports "the
  sections match" about bytes whose signatures do not check out. The
  comparison itself covers the configured sections only: STAMPS and metadata
  are excluded, because a later stage legitimately carries a stamp the earlier
  one does not, and metadata carries per-run timestamps.

---

#### Generation — `einmo generate <suite> --command <evaluator>`

Generation produces results. It is the only phase that runs the evaluator and
the only phase that writes.

- **It runs** the evaluator over every discovered input.
- **It writes** `generated/<id>.einmo` for each input, signed with the
  generation key. If the run dies, its catastrophe crumbs land in `generated/`
  too. It writes nothing else, and it never touches `output/`.
- **It compares against nothing.**
- **It passes** when every input's evaluator returned without error and every
  artifact einmo wrote verifies against its own stamps.
- **It fails** when an evaluator returns an error for an input, or an artifact
  einmo just wrote does not self-verify.
- **It does not fail** when `generated/` differs from `output/`. A difference
  there is the normal outcome of a change; judging it is the Output gate's job.

Generation serves two purposes, both first-class. It is a cheap test that
everything still runs — no baseline needed, no committed state consulted. And
it is how an agent or human materializes fresh results **for direct inspection
without disturbing the committed `output/`**: the results are ordinary signed
`.einmo` files, so `einmo compare generated output`, `einmo show`, `einmo
body`, and the review server all read them unmodified. The decision to accept
follows from what the reader sees.

#### The Output gate — `einmo verify --level output`

The Output gate establishes that the code produces what is committed as the
baseline.

- **It runs** generation first — its predecessor stage is not committed, so it
  must be materialized before it can be compared — and then compares.
- **It writes** `generated/`, by way of generation. It never writes `output/`.
- **It compares** `generated/` against `output/`.
- **It passes** when generation passed, every case present in one stage is
  present in the other, both sides verify, every configured section is
  byte-identical, and no `output/` artifact has lost its input.
- **It fails** when generation failed, a case is present on one side and
  missing from the other, either side fails verify-on-inspect, any configured
  section differs, or an `output/` artifact is orphaned.

A section difference is answered one of two ways: fix the code, or accept the
new baseline with `einmo promote generated to output`. That promotion makes a
deliberately weak claim — the run completed and the output looks reasonable —
and is not a semantic or stylistic review.

#### The Checked gate — `einmo verify --level checked`

The Checked gate establishes that the baseline matches what was reviewed.

- **It runs nothing.** No evaluator, no runner command, no build of the system
  under test.
- **It writes nothing.**
- **It compares** `output/` against `checked/`.
- **It passes** when every case present in one stage is present in the other,
  both sides verify, every configured section is byte-identical, and no
  `checked/` artifact has lost its input.
- **It fails** when a case is present on one side and missing from the other,
  either side fails verify-on-inspect, any configured section differs, or a
  `checked/` artifact is orphaned.
- **It does not check** `generated ↔ output`. That link belongs to the Output
  gate.

A divergence is answered by reviewing it statement by statement against the
in-force specification, then `einmo promote output to checked`.

#### The Verified gate — `einmo verify --level verified`

The Verified gate establishes that what was reviewed matches what was attested,
and that a human attested it.

- **It runs nothing. It writes nothing.**
- **It compares** `checked/` against `verified/`.
- **It passes** when every case present in one stage is present in the other,
  both sides verify, every configured section is byte-identical, no `verified/`
  artifact has lost its input, and every `stage:verified` stamp was signed by
  the configured reviewer's key and not by the well-known empty-passphrase
  computer key.
- **It fails** when any of those does not hold — including on a signature that
  verifies perfectly but belongs to the wrong signer, or to an agent that
  passed `--passphrase ""`.
- **It does not check** `generated ↔ output` or `output ↔ checked`. Those links
  belong to their own gates.

A divergence is answered by `einmo promote checked to verified --interactive`.

---

Generation is separately runnable but not separately gated. The Output gate
depends on it and fails if it fails, so a green Output gate already carries
"everything ran," and a fourth gate would restate it.

Retraction runs the chain backwards. `generated/` cannot be retracted from —
it is regenerated every run. Retracting from `output/`, `checked/`, or
`verified/` removes that stage's artifact and cascades forward, so withdrawing
a baseline correctly invalidates everything promoted from it.

---

*S.1 through S.8 are the implementation: what changes in einmo's code to make
S.0 true. They add no product behavior S.0 does not state.*

### S.1 — `Stage::Generated`, a fourth stage uniform with the other three

```rust
pub enum Stage {
    /// Written by `einmo generate`. Carries `compiled` + `configured` +
    /// `stage:generated` stamps. **Never committed** — this is the work file.
    Generated,
    Output,
    Checked,
    Verified,
}

impl Stage {
    pub const ALL: [Stage; 4] =
        [Stage::Generated, Stage::Output, Stage::Checked, Stage::Verified];
}
```

- Directory name: `generated`. Stamp key: `stage:generated`.
- `Stage::parse("generated")` accepts it; `validate_stage_name` is unchanged
  (no dot in the name, so its `[A-Za-z0-9_-]+` rule already admits it).
- `Generated` is declared **first**, so the derived `Ord` keeps the lifecycle
  ordering `Generated < Output < Checked < Verified`. Nothing may persist a
  `Stage`'s ordinal; `Stage`'s `Deserialize` already routes through
  `Stage::parse`, which is by name.
- `StageDirs` gains a `generated: String` field defaulting to `"generated"`,
  validated by the same `[A-Za-z0-9_-]+` rule as the other three.
  **Correction, recorded during implementation**: an earlier draft of this
  section said the name is "configurable from `einmo.toml` exactly as the
  other three are." That is false — **no** stage directory name is settable
  from `einmo.toml` today; `TestConfig` always constructs
  `StageDirs::default()`. This EIMP does not add that plumbing. The claim is
  corrected rather than implemented, because nothing here needs it.
- `StagePassphrases` gains a `generated` field, fed by `[signing] generated`
  in `einmo.toml`. Stage *passphrases* genuinely are per-stage configurable,
  so this one really is "exactly as the other three are."

`Generated` is a stage in the full sense: it has outputs, it has signatures,
its artifacts are `.einmo` files. **The only respects in which it differs from
the other three are that its directory is not committed, and that it exists to
be compared against `output/`.** Every existing `Stage::ALL` loop —
`config::ensure_stage_dirs`, `verify`, `count_flagged`, `EinmoSuite::scan`,
`EinmoCase::stages`, `normalize_file_path`, `EinmoDirectory` listing — picks it
up with no carve-out and no special case. It has a nested `generated/flagged/`
sink like any other stage. It is listed by `einmo list`, shown by `einmo show`,
and verified by `einmo verify`.

`.gitignore` gains one line:

```gitignore
generated/
```

This also ignores any `input/` subdirectory literally named `generated`. The
collision is noted rather than guarded against in code, and — since stage
directory names are not `einmo.toml`-settable (above) — a suite that hits it
has no configuration escape today. It would need either that plumbing or a
narrower ignore pattern. Accepted as a known, recorded limitation; no suite
in this repository has such an input directory.

### S.2 — The generation phase: `einmo generate`

The existing `Evaluate` verb is renamed `Generate`, retaining `evaluate` as a
command alias, and retargeted at `generated/`:

```
einmo generate <work_dir> --command <evaluator> [--filter <substr>] [--json]
```

It runs the evaluator over every discovered input and writes each result to
`generated/<id>.einmo`, signed with the generation stage's key
(`compiled` + `configured` + `stage:generated`) exactly as `output/` is signed
today. It **never writes `output/`**. Catastrophe crumbs are dropped in
`generated/`, so a crash leaves no committed stage dirty.

**What makes generation fail.** Per artifact, the rule is narrow — the
evaluator ran, and what einmo wrote is a sound `.einmo`:

| Fails | Why |
|-------|-----|
| the evaluator returned an error for an input | the runner did not run |
| the artifact just written does not verify against its own stamps | einmo produced an unsound artifact |

Two suite-shape preconditions also apply, because generating into a
malformed tree produces a malformed result: an extraneous (dot-prefixed) file
under `input/` (O1), and a suite that discovered no inputs at all (O2).

**What does not make generation fail.** Generation does not compare against
anything. A `generated/` artifact that differs from `output/` is not a
generation failure — it is the normal outcome of a change, and judging it is
the Output gate's job.

**Pruning.** `einmo generate` owns `generated/` outright. It removes any
artifact there whose `input/` file no longer exists, skipping the nested
flagged sink (`is_in_flagged_sink`), so a deleted input cannot leave a stale
artifact behind to distort the Output gate's comparison.

**The two uses.** Both are first-class:

1. **As a test that things run.** `einmo generate` alone is the cheapest
   possible signal — no baseline needed, no comparison, no committed state
   consulted. It answers "does every input still evaluate?"
2. **As the way to materialize results for inspection.** An agent or human who
   has finished a change runs `einmo generate` and then reads the results
   directly — `einmo compare generated output --root-cause`, `einmo body`,
   `einmo show`, the review server — **without disturbing the committed
   `output/`**. The decision to accept follows from what they see.

### S.3 — Promotion: `einmo promote generated to output`

```bash
einmo promote generated to output zweimomo/suites/python
```

`is_legal_transition` gains exactly one pair, `(Generated, Output)`.
`(Generated, Checked)` and `(Generated, Verified)` remain **illegal**:
generated content reaches a reviewed stage only by passing through the
baseline.

The promotion needs no new code path — it is a stage pair through
`EinmoSuite::promote`, appending a signed `stage:output` stamp to a body that
does not change. A second signer promoting the same content co-signs the
existing `output/` artifact (`PromoteOutcome::CoSigned`), which is where EIMP
3's multi-signer output stamps now live.

The claim this promotion makes is **deliberately weak**, and its help text
says so:

> the run completed and the output looks **reasonable** — a sanity check, not
> a semantic or stylistic review

`output → checked` is unchanged and remains the strong, statement-by-statement
review. Introducing a weaker-sounding sibling is precisely why the two must
stay verbally distinct wherever they are documented.

### S.4 — The three gates compare only against their predecessor

`ValidationLevel` keeps its three variants — **generation is a phase, not a
gate**, so nothing is added here — and stops escalating:

```rust
impl ValidationLevel {
    /// The stage this level judges.
    pub fn stage(self) -> Stage;

    /// The stage this level compares that stage AGAINST — its immediate
    /// predecessor. Replaces `escalation()`; the levels are independent,
    /// not cumulative.
    pub fn compares_against(self) -> Stage {
        match self {
            ValidationLevel::Output => Stage::Generated,
            ValidationLevel::Checked => Stage::Output,
            ValidationLevel::Verified => Stage::Checked,
        }
    }

    /// Whether this level must run the generation phase first. True only
    /// for `Output`, whose predecessor stage is not committed and so must
    /// be materialized before it can be compared.
    pub fn generates(self) -> bool {
        matches!(self, ValidationLevel::Output)
    }
}
```

`escalation()` and `escalates_from()` are removed.

| Gate | generates | compares | also checks |
|------|-----------|----------|-------------|
| `output` | **yes** → `generated/` | `generated ↔ output` | O1, O2, orphans in `output/` |
| `checked` | no | `output ↔ checked` | O1, O2, orphans in `checked/` |
| `verified` | no | `checked ↔ verified` | O1, O2, orphans in `verified/`, attestation (V6, V7) |

`generated/` is deliberately absent from the orphan column: S.2's pruning
makes an input-less artifact there impossible by construction, and the gate
would only be re-asserting the phase's own post-condition.

Every gate verifies signatures on **both sides** of its pair — that is already
what `stage_pair_problems` → `compare` does, refusing rather than comparing an
artifact that fails verify-on-inspect. "Signatures and all" is the existing
`compare` contract: the two sides' configured sections must be byte-identical
*and* both artifacts must verify against their own stamps. STAMPS and metadata
themselves are not compared — `output/` legitimately carries a `stage:output`
stamp its `generated/` twin does not, and metadata carries per-run timestamps.

`Problem` needs no new variants. Divergence at the Output gate reports as
`SectionDifference { left: Generated, right: Output, path, section }`, and a
generation runtime error reports as the existing `ArtifactUnsound` — the two
failure modes are already distinct in the report and already name the file.

`Verified` no longer reports `output ↔ checked` problems, and no level above
`output` invokes the evaluator. `einmo verify --level checked` and
`--level verified` require no runner command and no build of the system under
test — the capability gap this EIMP closes.

### S.5 — `write_output` simplifies

With `output/` no longer written by the runner, `write_output` (retargeted to
`generated/` and renamed to match) reduces to: serialize, sign with the
generation key, write, verify.

- The **byte-identical no-op** fast path is retained. `generated/` is
  gitignored so churn costs nothing in git, but the fast path still avoids
  needless rewrites and timestamp churn on an unchanged run.
- The **drift** path is removed. `FileResult::drifted` and its `detail`
  message retire, along with the `einmo regenerate-output` verb and
  `EinmoTestRunner::regenerate_output`. Drift is now the Output gate's
  comparison, and accepting it is the promotion.
- The **multi-signer co-sign** path is removed from the runner. Co-signing is
  what `promote` does, and `generated/` is a per-machine work directory where
  accumulating signers has no meaning.

### S.6 — Retraction inverts

`EinmoCase::retract` currently refuses `output/` because it "is regenerated
every run." After this EIMP that sentence is true of `generated/` and false of
`output/`, so the rule follows the fact:

| retract from | result |
|--------------|--------|
| `generated` | **refused** — it is regenerated every run |
| `output` | legal; cascades `output` → `checked` → `verified` |
| `checked` | legal; cascades `checked` → `verified` |
| `verified` | legal; itself only |

Retracting a baseline that turned out to be wrong is now expressible, and
correctly invalidates everything promoted from it.

### S.7 — Concurrency

`checked` and `verified` neither evaluate nor write, so they may run
concurrently with each other and with anything else. Only the generation phase
writes, and only `output` invokes it, so two concurrent writers can arise only
from two concurrent generations — which `suite_lock` already serializes.

### S.8 — No migration

An `output/` artifact is **not** required to carry a `stage:generated` stamp.
The Output gate compares sections and requires both sides to verify; provenance
is asserted by the gate passing, not by a stamp. The 8 committed artifacts
under `zweimomo/suites/python/output/` therefore stay valid untouched, and pick
up a `stage:generated` stamp naturally the next time they are re-promoted.

A green `--level output` run after implementation is the acceptance test that
this EIMP did not change what the evaluator produces. Any *section* difference
observed during implementation is a bug introduced by the implementation, not a
diff to promote.

## Test Plan

- **`src/stage.rs`** — `Stage::ALL` has four entries in lifecycle order;
  `Stage::parse("generated")`; `dir_name`/`stamp_key` for `Generated`;
  `Generated < Output < Checked < Verified`; `EinmoId::to_stage_path` round-trips
  through `Generated`.
- **`src/config.rs`** — `StageDirs::default` includes `generated`; `validate`
  and `ensure_stage_dirs` cover all four; a suite may rename the generation
  stage in `einmo.toml`.
- **`src/einmo_suite.rs`** — generation writes only `generated/` and leaves
  `output/` byte-untouched (assert on `output/` mtime and bytes); the crash
  crumb lands in `generated/`; generation fails on an evaluator error; fails on
  an unsound written artifact; fails on an extraneous `input/` file; fails on
  an empty suite; **does not** fail when `generated` differs from `output`;
  pruning removes a `generated/` artifact whose input was deleted but leaves
  `generated/flagged/` alone.
- **Level tests** — `output` reports `SectionDifference { Generated, Output }`
  on divergence; `checked` and `verified` invoke the evaluator **zero** times
  (assert via a call-counting `Evaluator`) and write nothing (assert on
  directory mtimes); `verified` no longer reports `output ↔ checked` problems;
  each level still reports a signature failure on either side of its own pair.
- **`src/transitions.rs`** — `is_legal_transition(Generated, Output)` is true;
  `(Generated, Checked)` and `(Generated, Verified)` are false; retract from
  `generated` is refused; retract from `output` cascades through `verified`.
- **Promotion** — `promote generated to output` writes an `output/` artifact
  whose configured sections are byte-identical to its `generated/` source and
  whose stamps are the source's plus `stage:output`; a second signer co-signs
  rather than rewriting.
- **Legacy compatibility** — an `output/` artifact carrying only
  `compiled`/`configured`/`stage:output` (no `stage:generated`) passes the
  `output` gate.
- **Retired surface** — `FileResult::drifted` and `einmo regenerate-output` are
  gone; EIMP 3's tests covering them are removed with a comment naming this
  EIMP, and the tests covering EIMP 3's *intent* (drift must not be silently
  accepted) are rewritten against the Output gate.
- **Comprehensive test** — a single test walking the whole chain: generate →
  inspect → promote to output → gate green → mutate the evaluator → gate red
  naming the section → promote → green again → promote to checked → retract
  output → checked and verified cascade away.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.

## Rejected Alternatives

### A. Do nothing — keep EIMP 3's drift detection and cumulative levels

Works today. But it leaves `output/` a committed work file that dirties the
tree on every run, leaves an agent no way to see fresh results without either
a red suite or an overwrite of the baseline, keeps `verified` re-checking
`output ↔ checked`, and keeps attestation unverifiable without a working
evaluator. EIMP 3 treated the symptom (silent overwrite) without separating
the two acts that were conflated.

### B. Make `output/` gitignored and treat `checked/` as the first committed stage

Removes the churn without adding a stage. Rejected because it discards the
cheap "did behavior move at all?" signal entirely: every behavior change would
surface only as a `checked/` divergence, fused with the much stronger question
of whether the change is correct.

### C. Keep levels cumulative, add a `--no-evaluate` flag

A narrower change: skip evaluation when the caller knows it is redundant.
Rejected because it leaves two ways to express each level and admits
nonsensical combinations ("no-evaluate *and* require `generated ↔ output`").
Naming the chain explicitly is simpler and maps one-to-one onto the gates.

### D. `generated/` as a concept outside the `Stage` enum

Keeps `Stage::ALL` at three and leaves corpus signing, storage, and the review
server untouched. Rejected because every operation an inspecting agent wants —
compare, show, body, verify, list, review — would need a parallel
directory-taking implementation beside its `Stage`-taking one. Making it a
stage is what makes "generate, then look at it with the tools you already
have" free.

### E. Leave `generated/` artifacts unsigned until promotion

Makes "the work file is never trusted" literal, and reads the phrase
"promotion signs the generated output" most directly. Rejected because
`compare`'s verify-on-inspect would have to become optional on one side, and
an unsigned `.einmo` is not a self-verifying artifact — the property the
format exists to provide. Signing at generation keeps promotion an ordinary
stamp append, which is how every other promotion already works.

### F. Make generation a fourth `ValidationLevel`

Symmetrical, and gives a fourth badge. Rejected because a generation gate
would assert nothing the Output gate does not already assert: **`output`
explicitly depends on generation and fails if generation fails.** A green
`output` gate therefore already carries "everything ran," and a fourth badge
would only restate it. Generation remains separately *invocable* — that is the
point of S.2 — without being separately *gated on*.

## Open Questions

- **Does `generated/` appear in the review worklist?** S.1 makes it a peer
  stage, so it appears by default. Against: a reviewer promoting
  `output → checked` has no business in an uncommitted work file. For: it is a
  full stage with signed artifacts, and `einmo compare generated output` is
  exactly a review activity.

  **Scheduled for resolution in `EIMP-01.plan.md` §Phase 6**, before that
  phase's implementation and recorded back into §S.1. It is deliberately left
  open while `status: Implementing`, contrary to the usual "design frozen"
  rule, because **it does not block Phases 1–5**: the stage enum, the
  generation phase, the CLI verb, the gates, and the promotion rules are all
  settled and none of them depend on the answer. Freezing it early would mean
  guessing at a review-surface question best answered with the review surface
  in front of us. The dhtml frontend is out of scope either way
  (`EIMP-1.md` §S.9, backburnered).

## References

- **EIMP 3** — *Output-stage drift fails the run; explicit regenerate;
  multi-signer output stamps* (`status: complete`). **Superseded by this
  EIMP**: its drift verdict moves into the Output gate's comparison, its
  `regenerate-output` verb becomes `promote generated to output`, and its
  multi-signer accumulation moves into `promote`'s existing co-sign path.
- **EIMP 7** — *`EinmoCase`/`EinmoSuite`/`EinmoDirectory` behind an
  `EinmoStorage` trait* (`status: complete`). §S.2a nested `flagged/` inside
  each stage and §S.9 pinned the stage-versus-level distinction this EIMP
  relies on: a stage is a place, a level is a policy.
- **FOOP-06** (`~/foolish/docs/foop/FOOP-06.md`) — the sibling proposal in the
  Foolish project that this EIMP is adapted from. Differences: FOOP-06 names
  the directory `output.gen/`, folds generation into a `Generation`
  *validation level*, and drops all three levels to non-cumulative including
  the generation one. This EIMP names the directory `generated/`, keeps
  gating on three levels only, and makes generation a separately invocable
  phase whose primary use is inspection before promotion.
- Code: `src/stage.rs` (`Stage`, `validate_stage_name`), `src/config.rs`
  (`StageDirs`, `TestConfig::stage_dir`), `src/einmo_suite.rs`
  (`ValidationLevel`, `check_integrity`, `evaluate_all`, `write_output`),
  `src/compare.rs` (`compare`, already non-evaluating), `src/transitions.rs`
  (`is_legal_transition`), `src/case.rs` (`retract`), `src/suite.rs`
  (`promote`, `retract`), `src/cli.rs` (`Evaluate`, `RegenerateOutput`,
  `Promote`, `Retract`).

## Last Updated

**Date**: 2026-08-11
**Updated By**: Claude Code / claude-opus-5
**Changes**: Added §S.0 as the **normative product specification** — what
einmo does, stated declaratively: the four stages, the four phases, what each
runs, writes, and compares, and the exact conditions that make each pass or
fail. S.1–S.8 are now explicitly the implementation that makes S.0 true, and
S.0 governs where they disagree. Initial draft. Separates generation from
gating: a new
`einmo generate` phase writes the gitignored `generated/` stage for direct
inspection without disturbing committed `output/`; `output/` becomes a
committed baseline reached by an explicitly weak promotion; and the three
gates stop escalating, each comparing only against its immediate predecessor
with signature verification on both sides.
