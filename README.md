# Einmo

> Gold-standard snapshot testing with discrete, cryptographically signed promotion.

## What is Einmo?

Einmo is a standalone Rust crate for snapshot-style testing where every
generated output is a **signed artifact** and promotion between review stages
is a **deliberate, attributable act** -- never an automated accept.

The genre goes by several names (see below). Whatever you call it, the
universal lifecycle is the same: a machine generates output, a human reviews
the diff, and the reviewed output becomes the baseline that future runs are
checked against. Einmo's contribution is to make each step of that lifecycle
**cryptographically attested** and **separately inspectable**, so that an
auditor can tell who produced a baseline, who reviewed it, and whether the
reviewer was a human or a machine.

### What einmo adds

- **Three-stage promotion pipeline**: the output stage, the checked stage,
  and the verified stage. Each stage is a directory; each promotion appends
  a signed stamp. Each stage also carries its own nested `flagged/` sink for
  artifacts set aside pending reviewer action.
- **Ed25519 + Argon2id signing**: every `.einmo` file carries a tamper-evident
  stamp chain. No surveyed framework signs its snapshots.
- **Verify-on-inspect**: every read verifies all signatures first. A tampered
  file is refused, never operated on.
- **Directory-based hierarchical storage**: stage directories mirror the
  `input/` tree at any depth, decoupling test organisation from test code.
- **No automated accept**: there is no `--accept`, no `--update`, no
  `INSTA_UPDATE` equivalent. Promotion is always a CLI command.
- **Catastrophe crumb defense**: a signed "test in progress" file is written
  before evaluation. If the process crashes, the crumb survives as forensic
  evidence.
- **Duration and depth limits**: per-test timeouts, per-suite timeouts, and
  configurable recursion depth -- all env-var accessible for CI.

Einmo does not aim to replace any existing tool. It targets projects that need
stronger attestation than an unsigned `.snap` file provides -- VM reference
implementations, security-sensitive approval suites, and any codebase where
distinguishing "a machine generated this" from "a human signed off on this"
matters.

## The testing style

The genre has four names, one mechanism:

| Name | Origin | Core idea |
|---|---|---|
| **Characterization test** | Michael Feathers, *Working Effectively with Legacy Code* (2004) | Document actual behaviour, not desired behaviour. |
| **Approval test** | Llewellyn Falco, ApprovalTests (2008) | A machine generates output; a human approves or rejects it. |
| **Golden master** | Record industry metaphor | Capture the known-good output of a system you may not fully understand. |
| **Snapshot test** | Jest / web community | The most common name today; the "snapshot" is the approved baseline file. |

The philosophical core: a machine can only verify "the received output equals
the approved file." It can never verify "this output is the behaviour we
want." The genre trades the oracle problem for a review problem.

## Other implementations

Einmo is not the first tool in this space. The following table (adapted from
the FOOP-92 research appendices) surveys the landscape:

| Ecosystem | Tool | Storage | Review model |
|---|---|---|---|
| JS/TS | [Jest](https://jestjs.io/docs/snapshot-testing) `toMatchSnapshot` | `__snapshots__/*.snap` | `-u` overwrites; 1 state, no review gate |
| JS/TS | [Vitest](https://vitest.dev/guide/snapshot.html) | `__snapshots__/*.snap` | `-u`; refuses to write in CI |
| Python | [Syrupy](https://github.com/tophat/syrupy) | `__snapshots__/*.ambr` | `--snapshot-update` |
| Rust | [insta](https://crates.io/crates/insta) | `.snap` / `.snap.new` | `cargo insta review` TUI; 2 states, review == promotion |
| Rust | [expect-test](https://crates.io/crates/expect-test) | inline `expect!["..."]` | `UPDATE_EXPECT=1` |
| Rust | [trycmd](https://crates.io/crates/trycmd) | `.stdout` / `.stderr` | `TRYCMD=overwrite`; elision for volatile values |
| Ruby | [ApprovalTests.Ruby](https://github.com/approvals/ApprovalTests.Ruby) | `.received` / `.approved` | `approvals verify --ask` |
| Go | [cupaloy](https://github.com/bradleyjkemp/cupaloy) | `testdata/*.golden` | `-update`; updating *fails* the test to block CI auto-update |
| JVM | [ApprovalTests.Java](https://github.com/approvals/ApprovalTests.Java) | `.received` / `.approved` | Pluggable reporters; scrubbers |
| Swift | [SnapshotTesting](https://github.com/pointfreeco/swift-snapshot-testing) | `.snap` / images | `record` mode |

**None of these cryptographically signs snapshot files.** That is the gap
einmo fills. insta comes closest with its two-state `.snap` / `.snap.new`
model and `cargo insta review` TUI, and has even added [non-interactive review
for LLMs/CI](https://github.com/mitsuhiko/insta/pull/815) -- but there is no
signing, no human-vs-machine key distinction, and no separation between "I
reviewed this" and "I promoted this."

For projects where a signed, auditable promotion chain is not needed, insta
and expect-test remain excellent choices. Einmo is for when attestation
matters.

## The Four Stages

| Stage | Directory | Committed | What it holds | Who writes it |
|---|---|---|---|---|
| **The generation stage** | `generated/` | **no** — gitignored | Fresh results, signed by the runner. The work file. | `einmo generate` |
| **The output stage** | `output/` | yes | The accepted baseline: it ran, and the output looks reasonable | `einmo promote generated to output` |
| **The checked stage** | `checked/` | yes | Reviewed outputs (AI or human promoted) | `einmo promote output to checked` |
| **The verified stage** | `verified/` | yes | Human-signed (passphrase required) | `einmo promote checked to verified` |

`generated/` is a stage in every respect the others are — signed artifacts,
verify-on-inspect, its own nested `flagged/` sink, promotion into the next
stage. It differs in exactly two ways: **its directory is not committed**, and
it exists to be compared against `output/`. That is what lets you run the
evaluator any time you like, look at what came out, and decide — without
touching anything a reviewer has signed. See
[What Passing Means](#what-passing-means).

Each stage directory mirrors the `input/` tree at any depth. An input file
like `stage1/section3/specific.foo` produces
`output/stage1/section3/specific.foo.einmo`, and the same relative path is
used in every other stage directory.

**`flagged/` is not a stage of its own.** Each of the four stages above carries
its own nested `flagged/` sink (`generated/flagged/`, `output/flagged/`,
`checked/flagged/`, `verified/flagged/`) for artifacts set aside from that stage's ordinary flow
pending reviewer action (`einmo flag <stage>`). This keeps a flagged
artifact's origin stage always known, and preserves the intentional
input/output/checked/verified directory split -- a hand-authored suite stays
easy to browse and edit in place: type into `input/`, look at `generated/`,
commit `output/`.

## What Passing Means

**Your suite passes when all three gates pass.** That's the whole claim, and
it is worth understanding what it is built out of, because a red gate tells
you not just *that* something is wrong but *where*.

Results move through four stages, one step at a time:

```
generated  ──▶  output  ──▶  checked  ──▶  verified
  it ran       reasonable      reviewed      attested
```

Each arrow is a promotion, and each promotion adds a signature. Each gate
checks exactly one arrow:

| Gate | Checks | What a green gate tells you |
|---|---|---|
| `--level output` | `generated` ↔ `output` | the code still produces the baseline you committed |
| `--level checked` | `output` ↔ `checked` | that baseline is what somebody reviewed |
| `--level verified` | `checked` ↔ `verified` | what they reviewed is what a human signed |

Three gates instead of one, for two reasons. A red gate **names the link that
broke** — "the code changed" and "the review is stale" are different problems
with different fixes. And only the first gate needs to run your code at all;
the other two are file comparisons, so you can verify a signed release against
its reviewed baseline with nothing but the files and the keys.

### Generate passes when your code ran

```bash
einmo generate my_suite --command ./my-evaluator
```

Generation runs your evaluator over every input and writes the results to
`generated/`. It passes when:

- every input's evaluator returned **without an error**, and
- every file einmo wrote is a valid `.einmo` that **verifies against its own
  signatures**.

That is all. **Generation compares against nothing.** If what it produced
differs from your committed `output/`, that is *not* a generation failure — it
is the normal result of changing your code, and deciding what to do about it
belongs to the next gate.

`generated/` is gitignored. Generating never touches your committed baseline,
which is what makes it safe to run any time you want to look at fresh results.

### Output passes when the code still produces your baseline

```bash
einmo verify my_suite --level output --command ./my-evaluator
```

This is the only gate that runs your code — so it needs to be told how. It
generates first, then compares, and **refuses to run without `--command`**:
grading whatever happens to be sitting in `generated/` would let a leftover
from an earlier run pass the gate while asserting nothing about your code.
The other two levels neither need it nor accept it.
It passes when **all** of the following hold:

1. **Generation passed** — everything above is true.

2. **The two stages line up.** Every case in `generated/` has a counterpart in
   `output/`, and every case in `output/` has one in `generated/`. Neither side
   holds a case the other lacks.

3. **Every signature on both sides verifies.** Each artifact carries a chain of
   signatures, and all of them are checked:

   | Signature | What it attests |
   |---|---|
   | `compiled` | which einmo binary produced this artifact — signed with einmo's own built-in key |
   | `configured` | the suite configuration in force — signed with your suite's key |
   | `stage:generated` | this content came out of a generation run |
   | `stage:output` | someone accepted it as the baseline |

   A file whose signature does not check out is **refused, not compared**.
   Einmo will never tell you "these match" about bytes it could not verify.

   The two sides carry different stage stamps, and that is expected: the
   `generated/` artifact has `stage:generated`, and its `output/` counterpart
   has `stage:output` appended on promotion. A baseline promoted before
   `generated/` existed carries only `stage:output` — no `stage:generated` —
   and passes exactly the same. **Einmo does not require a stamp to prove
   where a baseline came from; the gate passing is the proof.**

4. **Every compared section is byte-identical.** The sections compared are:

   - `INPUT` — the source that was evaluated
   - `OUTPUT`, or `OUTPUT[0]`, `OUTPUT[1]`, … when a case produces several
   - `DIFF` — on cases that reference another case
   - `COMMENTS` — only if your suite is configured to require it

5. **Nothing is orphaned.** No file sits in `output/` whose `input/` file you
   deleted.

#### What is deliberately not compared

Two artifacts can match while differing in bytes, and this is intentional:

- **The `STAMPS` section itself.** The `output/` side legitimately carries a
  `stage:output` signature its `generated/` twin does not — that is what
  promotion *is*. Comparing signature blocks would make every promotion look
  like a change.
- **The metadata header** — the suite path, the producing commit, the einmo
  binary's hash, and the timestamp of the run.

So **two runs an hour apart produce different timestamps and still match.**
Einmo compares what your code produced, not when it produced it. This is the
single most common surprise, and it is the point: a baseline that churned on
every run would be worthless as a baseline.

### Checked passes when your baseline is what was reviewed

```bash
einmo verify my_suite --level checked
```

The same conjunction, one link along: `output/` ↔ `checked/`. The two stages
must line up, every signature on both sides must verify (now including
`stage:checked`), every compared section must be byte-identical, and nothing in
`checked/` may be orphaned. The same exclusions apply — stamps and metadata are
not compared.

**This gate runs nothing.** No evaluator, no build of the system under test, no
runner command at all. It reads two directories.

It also **does not re-check** the generated↔output link. That is the output
gate's job, and duplicating it here would only tell you twice about the same
failure.

### Verified passes when a human signed it

```bash
einmo verify my_suite --level verified
```

Again the same conjunction, for `checked/` ↔ `verified/`, with signatures now
including `stage:verified`. It runs nothing and writes nothing.

It adds two checks about **who** signed, not just whether the signature is
valid:

- The `stage:verified` signature must carry your **configured reviewer's key**.
  A perfectly valid signature from the wrong person fails.
- It must **not** carry the well-known empty-passphrase key.

That second check exists because an automated agent that pipes an empty
passphrase produces a predictable, well-known key. Einmo derives that key
itself and looks for it, so an agent cannot quietly stand in for the human
whose attestation the verified stage exists to record.

### When a gate goes red

| Red gate | What it means | What to do |
|---|---|---|
| generate | your evaluator errored, or produced an unsound artifact | fix the runner — nothing downstream can be trusted yet |
| `--level output` | your code no longer produces the committed baseline | either fix the code, or accept the new results: `einmo promote generated to output my_suite` |
| `--level checked` | the baseline moved past what was reviewed | review the difference, then `einmo promote output to checked my_suite` |
| `--level verified` | the reviewed content moved past what was attested, or the wrong key signed | `einmo promote checked to verified my_suite --interactive` |

The two promotions are **not** the same kind of act, and the difference
matters:

- `promote generated to output` is a **weak** claim: the run completed and the
  output looks reasonable. A sanity check — explicitly *not* a semantic or
  stylistic review.
- `promote output to checked` is the **real review**: the results are correct
  against the specification, justified statement by statement.

Retraction runs the chain backwards. You cannot retract from `generated/` — it
is rebuilt every run. Retracting from `output/`, `checked/`, or `verified/`
removes that artifact and cascades forward, so withdrawing a baseline you no
longer trust also withdraws everything promoted from it.

### Looking at results without committing anything

Because `generated/` holds ordinary signed `.einmo` files, every tool works on
it:

```bash
einmo generate my_suite --command ./my-evaluator   # produce fresh results
einmo compare generated output my_suite --root-cause   # what changed, and why
einmo show   generated/my_case.foo.einmo           # summary + signature chain
einmo body   generated/my_case.foo.einmo           # the signed sections
```

None of that touches your committed baseline. Generate, read, decide, and only
then promote.

## The `.einmo` File Format

A `.einmo` file is a header line, followed by sections separated by a
configurable separator, ending with the JSON STAMPS section and an optional
unsigned advisory line:

```
#einmo 1 encoding=utf-8 separator=①\n
test: <test-name>
suite: <suite-name>
producer: <git-sha>
producer-diff: <sha256-of-git-diff-or-omitted>
generated: <ISO8601>
status: normal
status-detail: 
sections: INPUT, OUTPUT, COMMENTS, STAMPS
①
<input content>
①
<output content>
①
<comments content>
①
<stamps JSON lines>
# flagged: <reason> <timestamp>   (optional, unsigned advisory)
```

**Header line**: declares the format version (`1`), the encoding (`utf-8`),
and the escaped separator string.

**Metadata section**: fixed key/value lines in a byte-stable order (`test`,
`suite`, `producer`, optional `producer-diff`, `generated`, `status`,
`status-detail`, optional `reference`, `sections`). The order never changes,
so signatures cover a stable byte sequence.

**Body sections**: separated by the configurable separator. Default is `①\n`
(U+2460 followed by LF). Foolish suites use `!!\n` (a Foolish line comment
followed by LF). The `sections:` metadata field declares the ordered list of
section names, and the number of declared body sections must match what
appears on disk.

**STAMPS section**: JSON-lines, one stamp object per line. Each stamp records
its role key, hex pubkey, what it signs, the base64 signature, producer
provenance, and a timestamp.

**Advisory line**: an optional `# flagged: <reason> <timestamp>` line may
appear after STAMPS. It is stripped before verification and is not part of
the signed bytes, so flagging a file does not invalidate its stamp chain.

**Separator collision rule**: serialization refuses if any section body or
the metadata contains the configured separator. This keeps parsing byte-exact
and unambiguous. When it happens, configure a different separator via
`TestConfig::with_separator`.

## The Three-Role Key Model

Every `.einmo` file carries a chain of Ed25519 stamps built from three key
roles:

- **Compiled Key**: embedded in the binary at compile time. In the stock
  open-source build it is a deterministic, publicly-known keypair derived from
  a fixed seed passphrase. Its stamp *certifies* the Configured Key's public
  key.
- **Configured Key**: set at configuration time (defaults to the
  empty-passphrase key). Its stamp *certifies* the Stage Key's public key.
- **Stage Keys**: one per stage, resolved through the passphrase cascade.
  Each `stage:<name>` stamp signs **all file bytes before its own line**,
  forming an append-only integrity chain.

The two certification stamps sign only small, constant-size public keys.
Content integrity comes solely from the stage-key stamps, which cover every
prior byte. A promotion appends exactly one destination-stage stamp and never
touches existing stamps.

**Emergent human-attestation**: the `verified` stage is deliberately
unconfigured by default (its passphrase is `None`), so the cascade falls
through to an interactive prompt. An AI that pipes `--passphrase ""` instead
gets the well-known computer key. Einmo detects this post-hoc: the promotion
report flags `non_human: true` whenever a `stage:verified` stamp is produced
by the empty-passphrase key, and the CLI prints a warning.
Additionally, when a human attests to a promotion to `verified`, a passphrase quality check is performed. The effectiveness of the passphrase is scored relative to the existing verified corpus, ensuring a minimum level of uniqueness.

Keys are derived deterministically from a passphrase via Argon2id. The
Argon2id parameters are pinned constants (m=19456 KiB, t=2, p=1, matching the
OWASP Password Storage Cheat Sheet minimum baseline), not crate defaults. A
dependency bump cannot silently change key derivation. The salt is
domain-separated (`einmo:stamp-key:v1`) from any other derivation. Changing
these parameters invalidates every previously-derived keypair, so they must
not change without a corpus re-sign.

## Verify-on-Inspect

Every operation that reads a `.einmo` file verifies ALL stamps first. A
tampered file is refused, never operated on. The single filesystem-touching
entry point is `EinmoFile::from_file`, which reads bytes and passes them
through `verify_bytes`.

The verification path checks the **actual raw file bytes**, not a recomputed
canonical form. This matters because `parse()` normalizes metadata (it trims
whitespace after `key:`). If someone adds an extra space to a metadata value
on disk, the parsed form looks identical, but the raw bytes differ. The stage
stamps cover the raw bytes, so `verify_bytes` reconstructs the exact byte
range from the file and checks against that. Metadata whitespace tampering is
caught.

The pure verification functions (`verify_bytes`, `verify_all`) touch no
filesystem, no tty, and no Argon2. They only re-check Ed25519 signatures
already present in a parsed file. This makes them WASM-targetable.

## Flagging

Flagging moves a file out of its stage's ordinary flow and into that SAME
stage's own nested `flagged/` sink -- from the output stage into
`output/flagged/`, from the checked stage into `checked/flagged/`, from the
verified stage into `verified/flagged/`. A flagged artifact's origin stage is
therefore always known just from which sink it's in; there is no single
top-level `flagged/` shared across stages. The move appends an unsigned
advisory line `# flagged: <reason> <timestamp>` after the STAMPS section. No
stamp is added -- flagging is not a promotion. The original file is removed
from its source stage (move semantics, not copy).

The advisory line is outside the signed bytes, so flagging does not
invalidate the existing stamp chain. A flagged file still verifies normally.

**Flagging the same path twice:** if `<stage>/flagged/<rel>` already exists
when a new flag operation targets the same relative path in that same stage,
the new file gets a timestamp suffix: `<stage>/flagged/<base>.<timestamp>.einmo`.
The previously flagged file is not overwritten -- both coexist, each with its
own advisory line recording when and why it was flagged. This preserves the
history of set-aside files.

## Development

For setup instructions, `just` recipes, the toolchain pin rationale, and
troubleshooting, see the **Developer Guide** section in
[`rust_instructions.md`](rust_instructions.md).

## Running specific tests

**The central reference for running ONE test or a SUBSET of tests** — the
fast-iteration loop while developing a feature. EIMP plan checkboxes link here
and name their tests; the command forms live ONLY in this section, so when the
tooling changes, this one section is what gets updated.

A subset run is a development-loop ANALYSIS tool — it never replaces the full
suite. At every sub-section and phase boundary, the canonical judgment is:

```bash
just                                     # fmt + lint + full suite (~6 min)
just test                                # all tests only
```

### Select by name filter

`just test <filter>` (and underlying `cargo nextest run`) selects tests by
name substring. nextest runs each test in its own process — no mutex-poison
cascade across tests (see `rust_instructions.md` §"Things that will bite you").

```bash
# One test (every test whose name contains "verify"):
just test verify

# Several filters in ONE invocation — a test matching ANY filter runs:
just test verify stamp roundtrip

# Exact test name (no substring matching):
just test -- --exact signature::stamp_chain::append_only_integrity

# Discover test names to filter on:
just test -- --list promotion
```

### Scoping by crate or module

einmo is a workspace with two crates (`einmo` and `zweimomo`). Scope to one
crate when you don't need the full workspace:

```bash
just test -p einmo verify                  # only einmo crate tests matching "verify"
just test -p zweimomo evaluator            # only zweimomo tests matching "evaluator"
```

### Combining filters with crate scope

```bash
# Batch — crate-scoped, several name filters (OR semantics):
just test -p einmo promote flag retract
```

Note: subset runs never replace the full suite. At sub-section and phase
boundaries, run `just` (the full gate) — not just the subset.

If a subset run reveals a failure, that is broken code — fix it; do not ignore
it because "the full suite might pass."

---

## Quick Start

```rust
use einmo::{EinmoSuite, Evaluator, TestConfig, Stage};

struct MyEvaluator;
impl Evaluator for MyEvaluator {
    fn evaluate(&self, source: &str) -> Result<Vec<String>, String> {
        Ok(vec![format!("result: {source}")])
    }
}

fn main() {
    let config = TestConfig::new("my-suite")
        .require_correspondence(Stage::Output, Stage::Checked);
    let suite = EinmoSuite::new(config);
    let results = suite.evaluate_all(&MyEvaluator).unwrap();
    assert!(results.all_output_written_and_verified());
}
```

The suite discovers every file under `input/`, evaluates each one, writes a
signed `.einmo` to **`generated/`**, and re-verifies what it just wrote. It
never writes `output/` — accepting results as the baseline is a separate,
deliberate act:

```bash
einmo compare generated output my-suite     # look at what changed
einmo promote generated to output my-suite  # accept it as the baseline
```

`all_output_written_and_verified()` folds in the suite's integrity at the
configured validation level, so on a suite with no baseline yet it reports
`false` — correctly: there is nothing for the output gate to affirm until you
promote once. If you only want to know whether every input evaluated, test
`results.files.iter().all(|f| f.written_and_verified || f.ignored)`.

If you configured `require_correspondence(Output, Checked)`, it also compares
those two stages and reports any files that exist only on one side or differ
in their INPUT/OUTPUT sections.

## The `Evaluator` Trait

```rust
pub trait Evaluator: Sync {
    fn evaluate(&self, source: &str) -> Result<Vec<String>, String>;
}
```

The trait is language-agnostic. Source text goes in, formatted output chunks
come out. Returning `Err(String)` signals the input could not be parsed or
accepted (recorded as `status: input-error`). A panic during `evaluate` is
caught by the suite and recorded as `status: output-error`. An expected error
*value* (a division-by-zero alarm, "infinite loop detected") is a normal `Ok`
output, marked `status: normal`.

The `Sync` bound lets `evaluate_all` share one evaluator across threads.
Adapters that wrap a `!Send` interpreter construct it *inside* `evaluate`,
per call. The reference implementations in zweimomo do exactly this:

- `UbcaEvaluatorAdapter` wraps the Foolish UBCa evaluator and formats each FIR
  via the humanizing sequencer.
- `RustPythonEvaluator` spins up a fresh sandboxed `rustpython-vm` interpreter
  (no stdlib) per call.
- `BoaEvaluator` spins up a fresh `boa_engine` context (no fs/network/Node
  APIs) per call.

Einmo never parses body content. It treats every section as opaque text, so
any language that can format its results as strings works as an evaluator.

## CLI

Einmo ships a single CLI app with two binary targets sharing the same parser:
`einmo` (canonical) and `cargo-einmo` (so `cargo einmo ...` also works).

| Subcommand | What it does |
|---|---|
| `einmo generate <work_dir> --command <evaluator>` | Run the evaluator over every input and write signed artifacts to `generated/`. Never writes `output/`. (Alias: `einmo evaluate`.) |
| `einmo promote <from> to <to> <work_dir>` | Append the destination stage's stamp to every matching file. `* to flagged` delegates to `flag`. Legal pairs: `generated to output`, `output to checked`, `output to verified`, `checked to verified`, and `verified to checked`. |
| `einmo retract <work_dir> <stage>` | Withdraw artifacts from `output`, `checked`, or `verified`, cascading forward through everything promoted from them. `generated` is refused — it is rebuilt every run. |
| `einmo flag <work_dir> <stage>` | Move matching files into that stage's own nested `flagged/` sink with an unsigned advisory line. No stamp. |
| `einmo compare <a> <b> <work_dir>` | Per-section comparison of two stages over the mirrored tree. |
| `einmo verify <work_dir>` | Verify signature integrity across one stage (`--stage`) or all stages (`--all`), and judge the suite at `--level output\|checked\|verified`. `--level output` requires `--command`: it generates first, then compares. |
| `einmo confirm-signatures <path> <prefix>` | Report which files carry a stamp whose pubkey starts with the prefix. |
| `einmo show <file>` | Print an envelope's metadata and stamp chain summary. |
| `einmo self-check` | Compute the SHA-256 of the running binary (self-attestation). |

Every subcommand accepts `--json` for machine-readable output. `promote` and
`flag` accept `--filter <glob>` to restrict the operation to matching input
paths (the glob supports `*` as a wildcard). `compare` accepts `--root-cause`
to descend the differing subtree and report only the deepest differing
descendants, and `--require-match` to exit non-zero when anything differs, is
one-sided, or is tampered.

### Targeting specific files

Most of the time you want to act on the whole suite or a glob-filtered
subset. But sometimes you need to target one or two files precisely.
`promote`, `flag`, `compare`, and `verify` all accept **positional file
arguments** after `work_dir`:

```bash
# Promote just one file:
einmo promote output to checked suite_dir alarm_division_by_zero.foo.einmo

# Promote two files:
einmo promote output to checked suite_dir a.foo.einmo b.foo.einmo

# Use -- to separate flags from file names (when a file name starts with -):
einmo promote output to checked suite_dir -- -weird-name.einmo

# Flag a single file with a reason:
einmo flag suite_dir output broken_test.einmo --reason "produces wrong output"

# Compare just one file between two stages:
einmo compare output checked suite_dir important.einmo

# Verify just one file:
einmo verify --all suite_dir critical.einmo
```

**Reading file paths from stdin** with `-`:

```bash
# Pipe a list of files to promote:
echo "a.foo.einmo\nb.foo.einmo" | einmo promote output to checked suite_dir -

# Promote everything that changed (find + einmo):
find suite_dir/output -name '*.einmo' -newer suite_dir/checked | \
  einmo promote output to checked suite_dir -

# Read from a file list:
cat changed-files.txt | einmo promote output to checked suite_dir -
```

**Path normalization:** the CLI accepts file paths in any of these forms and
normalizes them internally to mirror-relative paths:

| Input form | Example | Normalizes to |
|---|---|---|
| Mirror-relative | `test.einmo` | `test.einmo` |
| Nested mirror-relative | `subdir/test.einmo` | `subdir/test.einmo` |
| Stage-relative | `output/test.einmo` | `test.einmo` |
| Stage-relative (nested) | `checked/sub/test.einmo` | `sub/test.einmo` |
| Absolute path | `/home/user/suite/output/test.einmo` | `test.einmo` |
| Input name (no `.einmo`) | `test.foo` | `test.foo.einmo` |

When file arguments are provided, `--filter` is ignored. When no file
arguments are given, `--filter` (or all files if no filter) is used.

Promotion key resolution follows a cascade: `--passphrase` >
`--stdin-passphrase` > `EINMO_PASSPHRASE` env var > `einmo.toml
[signing]` > interactive prompt. The `--interactive` flag forces the
prompt, skipping all other tiers. An explicit empty string is "set to empty"
(the computer key), never "unset".

## Review Server

Einmo's review system has two layers:

- **`einmo-review-server`** — a background process that holds a live
  `EinmoReview` session (worklist, decisions, verified-body cache,
  execution). It exposes a JSON API over unix-domain sockets (default) or
  TCP (with bearer-token auth). The server supports three review modes as
  backend capabilities: **Full** (every case), **NewOrBroken** (only
  mismatches), and **Random** (shuffled order for sampling).
- **`einmo_review_client.sh`** — a vim-based TUI that launches its own
  private server, drives the review loop, and tears everything down on
  exit. This is the user-facing tool — it passes the mode to the server
  and presents the results.

### Reviewing with the TUI client

```bash
# Full review — every case in the suite (default):
./scripts/einmo_review_client.sh -p /path/to/suite

# New or broken — only cases with no baseline or a content mismatch:
./scripts/einmo_review_client.sh -p /path/to/suite -n

# Filter to cases matching a substring:
./scripts/einmo_review_client.sh -p /path/to/suite "alarm"
```

The `-p` flag launches a private server for the given suite directory.
The server binds an unpredictable socket in a mode-700 scratch dir and is
torn down on exit. To attach to an already-running standalone server
instead, use `-s /path/to/socket`.

Inside vim:

| Key | Action |
|---|---|
| `c` | Promote to checked |
| `v` | Promote to verified |
| `f` | Flag with a reason |
| `k` | Kick (retract from highest stage) |
| `u` | Undo decision |
| `\d` | Fetch server-side diff hunks (output vs checked) |
| `\D` | Toggle vim's built-in diff mode across all panes |
| `Enter` | Next case |
| `q` | Quit and show the execution plan |

At the end of the pass, the script shows the plan and asks you to type
`PROMOTE` to execute all pending promotions. If any promotion targets
`verified`, you'll be prompted for a passphrase.

### Session persistence and crash recovery

Every session writes an append-only JSONL journal. If the process crashes
mid-review, resume with `--session <id>` — the journal replays every
decision and the session picks up where it left off.

### Standalone server (for scripting or multi-client setups)

A long-lived server can be started separately and attached to by multiple
clients:

```bash
# Start a standalone server:
einmo-review-server serve /path/to/suite

# Attach the TUI client:
./scripts/einmo_review_client.sh -s .einmo-review.sock
```

## Catastrophe Crumb Defense

Before each evaluator call, einmo writes a **signed** `.einmo` "catastrophe
crumb" to the **`generated/`** path with `status: output-error` and
`status_detail:
"TEST IN PROGRESS -- if you see this file, the test harness crashed during
evaluation. Escalate to human or other agents for support."`.

If the process crashes during evaluation (stack overflow, OOM, abort, kill
signal), this signed file remains as forensic evidence. `catch_unwind` catches
ordinary panics and records them as `status: output-error` with the panic
message, but it cannot catch `abort()` or SIGSEGV. The catastrophe crumb
covers those cases.

Crumbs land in `generated/`, which is gitignored, so **a crash never dirties a
committed stage.** On success the real output overwrites the crumb.

The crumb itself is a valid signed `.einmo` file: it can be verified,
compared, and even promoted like any other artifact — there is no special
casing for it in the promotion machinery. The test suite proves this by
spawning a child process that calls `std::process::abort()`, then checking
that the crumb exists, verifies, and promotes the whole way to `checked`.

### Detecting stale crumbs

If a previous run crashed and left a catastrophe crumb on disk, the next run
will **not** silently overwrite it. Einmo detects stale crumbs and gates on
three flags (see Configuration Precedence below):

1. If the crumb's path is in `ignore_catastrophe_crumbs`, the test is skipped
   and marked `ignored`. The suite passes if all other tests pass. The crumb
   stays on disk and can be promoted as the accepted output.
2. If `rerun_catastrophes` is enabled, the crumb is overwritten and the test
   re-runs normally.
3. If neither applies, the suite **fails** with a message naming the crumb
   path and suggesting both flags.

## Configuration Precedence

Einmo resolves each configurable parameter from the first available source,
in decreasing priority:

1. **CLI flag** (e.g. `--walk-depth-limit 32`)
2. **Environment variable** (e.g. `EINMO_WALK_DEPTH_LIMIT=32`)
3. **Code configuration** (`TestConfig::with_walk_depth_limit(32)`)
4. **Per-suite `einmo.toml`** (in `work_dir/einmo.toml`)
5. **Crate-wise `einmo.toml`** (found by walking up from `work_dir`'s parent)
6. **Default**

Environment variables override code configuration. This is intentional: a CI
environment or operator can enforce a limit that test code cannot accidentally
disable.

### Parameters

| Parameter | Env var | Type | Default |
|---|---|---|---|
| Walk depth limit | `EINMO_WALK_DEPTH_LIMIT` | integer | 64 |
| Per-test duration limit | `EINMO_DURATION_LIMIT` | seconds | none |
| Per-suite duration limit | `EINMO_SUITE_DURATION_LIMIT` | seconds | none |
| Rerun catastrophes | `EINMO_RERUN_CATASTROPHES` | `1` / `true` / `yes` | false |
| Ignore catastrophe crumbs | `EINMO_IGNORE_CATASTROPHE_CRUMBS` | colon-separated paths | empty |

### `einmo.toml` `[signing]` section

```toml
[signing]
generated = ""        # the generation stage's key (default: empty = computer key)
output    = ""        # the baseline stage's key
checked   = "..."     # the review stage's key
verified                # DELIBERATELY UNSET -- falls through to an interactive prompt
reviewer_key_prefix = "a1b2c3"
```

One key per stage, resolved through the passphrase cascade. `generated`,
`output`, and `checked` conventionally default to the empty passphrase (the
well-known computer key), because an agent may legitimately produce, accept,
and review. **Leave `verified` unset**: that is what makes human attestation
emergent rather than enforced — see "The Three-Role Key Model" above.

Stage *directory names* are not configurable from `einmo.toml`; only the keys
are.

### `einmo.toml` `[suite]` section

```toml
[suite]
walk_depth_limit = 32
duration_limit = 30
suite_duration_limit = 300
rerun_catastrophes = false
ignore_catastrophe_crumbs = ["crash.foo.einmo", "overflow.foo.einmo"]
```

A per-suite `einmo.toml` in the work directory beats a crate-wise `einmo.toml`
found by walking up the directory tree. Per-suite values override crate-wise
values; unset keys fall through to the crate-wise file, then to defaults.

## Duration Limits

Two parameters control timeouts:

- `EINMO_DURATION_LIMIT` (seconds): per-test timeout. Tests exceeding this
  are marked `OutputError` with a detail line reporting the limit and actual
  elapsed time.
- `EINMO_SUITE_DURATION_LIMIT` (seconds): per-suite timeout. Aborts early,
  skipping remaining tests, and records a correspondence failure explaining
  how many were skipped.

You can also set these programmatically via `TestConfig::with_duration_limit`
and `TestConfig::with_suite_duration_limit`. The per-test limit is checked
after evaluation completes (it does not interrupt a running evaluator), while
the per-suite limit is checked before starting each new test.

## Configurable Depth Limit

`TestConfig::with_walk_depth_limit(n)` controls the maximum recursion depth
for directory walks (default 64). This prevents stack overflow on
pathologically deep trees and catches symlink cycles. When the walk exceeds
the limit, einmo returns an I/O error explaining the situation. Symlinks are
followed: a broken symlink (target missing) is skipped silently, but an
ELOOP from a symlink cycle propagates as an error.

## The `TestConfig` API

`TestConfig` is constructed via `new(work_dir)` or `default_for(work_dir)` and
refined with builder-style methods:

| Method | Effect |
|---|---|
| `new(work_dir)` / `default_for(work_dir)` | Create a config with all defaults for the given work directory. |
| `require_correspondence(a, b)` | Add a required stage pair checked after `evaluate_all`. |
| `with_separator(sep)` | Set a custom section separator. |
| `foolish_separator()` | Use `!!\n` (a Foolish line comment) as the separator. |
| `with_perspectives(vec)` | Register statically configured perspectives. |
| `with_parallel(threads)` | Run with `n` threads (`None` for serial). |
| `with_walk_depth_limit(n)` | Set the recursion depth limit for input-tree walks. |
| `with_duration_limit(duration)` | Set the per-test timeout. |
| `with_suite_duration_limit(duration)` | Set the per-suite timeout. |
| `with_rerun_catastrophes(bool)` | Allow overwriting stale catastrophe crumbs. |
| `with_ignore_catastrophe_crumbs(vec)` | Accept specific crumbs as expected (skip + pass). |
| `with_diff_limit(chars)` | Set the DIFF size limit for dependents (default 2000). |
| `with_dependent_separator(sep)` | Set the dependent-name separator (default `++`). |
| `with_match_sections(policy)` | Set which sections must match in `compare`. |
| `with_suite_name(name)` | Set the human-readable suite name for metadata. |

Accessors (`work_dir()`, `input_path()`, `stage_dir(stage)`, `separator()`,
`encoding()`, `suite_name()`, etc.) are all `#[must_use]`.

Defaults: `input/` input dir, standard stage dirs
(`output`/`checked`/`flagged`/`verified`), `①\n` separator, `InputOutput`
matching, empty-passphrase for `output` and `checked` stages, `verified`
unset (prompts interactively), `++` dependent separator, 2000-char diff
limit, 64-deep walk limit.

## Perspectives

A `Perspective` is a pure `fn(&str) -> String` transform applied to the INPUT
or a specific OUTPUT chunk, producing a derived section stored alongside the
body. Einmo stays language-agnostic: it never parses body content, it just
applies the function and stores the result.

```rust
use einmo::{Perspective, PerspectiveOf};

let shout = Perspective {
    name: "shout",
    of: PerspectiveOf::Input,
    extract: |s| s.to_uppercase(),
};
```

`PerspectiveOf::Input` derives from the INPUT body. `PerspectiveOf::Output(i)`
derives from the `i`-th OUTPUT chunk. Perspectives are registered via
`TestConfig::with_perspectives(vec![...])` and emitted automatically during
`evaluate` and `evaluate_all`.

The **Charmer** plugin (in zweimomo) is a reference perspective.
`zweimomo::aspects::aspects_perspective()` wraps `compute_aspects`, which
reports four metrics from the primary output chunk:

```
encoding: ascii
lines: 1
chars: 1
alnum: 1
```

The core `compute_aspects` function has zero einmo dependency. You can copy
it into any project and call it directly.

## Error Handling

Einmo uses a single `EinmoError` enum, marked `#[non_exhaustive]` and deriving
`thiserror::Error`. All fallible functions return `Result<T, EinmoError>`.
The variants:

- `Io { path, source }` carries the offending path.
- `Parse(String)` for malformed envelopes.
- `SeparatorCollision { section }` when a body contains the separator.
- `Verification(String)` when a stamp fails verify-on-inspect.
- `Stamp(String)` for invalid stamp JSON.
- `InvalidStageName(String)` for names failing `[A-Za-z0-9_-]+`.
- `IllegalTransition { from, to }` for disallowed stage pairs.
- `Config(String)` for invalid configuration values.
- `NoKey(String)` when no key material could be resolved.

## Standalone Scope

Einmo has **zero dependency on any Foolish crate**. It reimplements the
signing and format machinery from scratch: Ed25519 via `ed25519-dalek`,
Argon2id via `argon2`, SHA-256 via `sha2`. The crate is structured to be
promoted to its own repository as-is. The `Cargo.toml` explicitly documents
this: "Standalone crate: NO dependency on any workspace crate."

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `ed25519-dalek` | 2 | Ed25519 signing and verification |
| `argon2` | 0.5 | Argon2id passphrase-to-key derivation |
| `base64` | 0.22 | Base64 encoding of signatures |
| `hex` | 0.4 | Hex encoding of pubkeys and hashes |
| `clap` | 4 | CLI argument parsing (with `derive` and `env` features) |
| `serde` | 1 | Serialization (with `derive` feature) |
| `serde_json` | 1 | JSON stamp serialization |
| `toml` | 0.8 | Configuration file parsing |
| `time` | 0.3 | ISO-8601 timestamps (with `macros`, `formatting`, `parsing` features) |
| `thiserror` | 2 | Error enum derivation |
| `similar` | 2 | Unified diff generation for dependent DIFF sections |
| `sha2` | 0.10 | SHA-256 for git diff hashing and self-check |
| `rpassword` | 7 | Interactive passphrase prompt on the tty |

Dev dependency: `tempfile` 3 (for tests).

## License

`MIT`

---

## Appendix: Migrating an insta test to einmo

This appendix demonstrates, step by step, how to refactor an existing insta
snapshot test into an einmo suite. The example is based on a real test in the
Foolish project (`foolish-ubca/src/ubca_snapshot_tester.rs`).

### Before: the insta test

The original test uses insta's `Settings` API to bind a snapshot path, then
calls `assert_snapshot!` for each evaluated input:

```rust
// foolish-ubca/src/ubca_snapshot_tester.rs (BEFORE)

use std::path::PathBuf;
use crate::evaluator::UbcaEvaluator;

fn suite() -> foolish_core::SnapshotSuite {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    foolish_core::SnapshotSuite::new(
        base.join("snapshot_tests").join("input"),
        base.join("snapshot_tests").join("approved"),
    )
}

#[cfg(test)]
mod approval_tests {
    use super::*;

    #[test]
    fn approval_all() {
        let eval = UbcaEvaluator;
        let suite = suite();
        let evaluations = suite.evaluate_all(num_cpus::get(), &eval);
        let approved = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("snapshot_tests")
            .join("approved");
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_path(&approved);
        settings.set_prepend_module_to_snapshot(false);
        settings.set_omit_expression(true);
        settings.bind(|| {
            for (name, result) in evaluations {
                eprintln!("Evaluating: {}", name);
                match result {
                    Ok(output) => {
                        insta::assert_snapshot!(format!("{}.foo", name), output);
                    }
                    Err(msg) => {
                        eprintln!("  ERROR: {}", msg);
                    }
                }
            }
        });
    }
}
```

What this does:
- Evaluates every `.foo` file under `snapshot_tests/input/`.
- For each result, calls `insta::assert_snapshot!` which compares against
  `.snap` files in `snapshot_tests/approved/`.
- If the output differs, insta writes a `.snap.new` file. A human runs
  `cargo insta review` to accept or reject.
- There is no signing. The `.snap` files are unsigned text. Anyone can edit
  them, and there is no audit trail of who approved what.

The `.snap` file format is YAML frontmatter + content:

```
---
source: foolish-ubca/src/ubca_snapshot_tester.rs
assertion_line: 34
---
INPUT:
```foolish
{a = 10 / 2; b = 10 / 0; c = 20 / 4;}
```
[0] RESULT:
```hfssnap
{NK
...
```
```

### After: the einmo test

The refactored test creates an einmo `EinmoSuite` over the same input
directory, uses an `Evaluator` adapter around `UbcaEvaluator`, and enforces
the `output == checked` correspondence gate:

```rust
// foolish-ubca/src/ubca_snapshot_tester.rs (AFTER)

use std::path::PathBuf;
use einmo::{EinmoSuite, Evaluator, Stage, TestConfig};

fn suite_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("snapshot_tests")
}

#[cfg(test)]
mod approval_tests {
    use super::*;

    struct UbcaEinmoAdapter;
    impl Evaluator for UbcaEinmoAdapter {
        fn evaluate(&self, source: &str) -> Result<Vec<String>, String> {
            let inner = crate::evaluator::UbcaEvaluator;
            let firs = inner.evaluate(source)?;
            Ok(firs.iter().map(|fir| {
                let fir = foolish_core::clone_steppable(fir);
                foolish_core::FirSequencer::format(&fir)
            }).collect())
        }
    }

    #[test]
    fn approval_all() {
        let config = TestConfig::new(suite_dir())
            .require_correspondence(Stage::Output, Stage::Checked);
        let suite = EinmoSuite::new(config);
        let results = suite.evaluate_all(&UbcaEinmoAdapter)
            .expect("evaluate_all should not fail at the fs level");

        // Every input must have produced a written, self-verifying .einmo.
        assert!(!results.files.is_empty());
        for file in &results.files {
            assert!(
                file.written_and_verified,
                "{} was not written+verified: {:?}",
                file.rel_path.display(),
                file.detail
            );
        }

        // If a checked/ baseline exists, output must match it.
        assert!(
            results.all_output_written_and_verified(),
            "correspondence failures: {:?}",
            results.correspondence_failures
        );
    }
}
```

### What changed

| Aspect | insta (before) | einmo (after) |
|---|---|---|
| **Storage** | `approved/*.snap` (flat, module-coupled) | `output/*.einmo` + `checked/*.einmo` (hierarchical, mirrors `input/`) |
| **Format** | YAML frontmatter + content | Header + metadata + sections + signed JSON stamps |
| **Signing** | None | Ed25519 append chain (compiled + configured + stage stamps) |
| **Review** | `cargo insta review` (TUI, review == promotion) | `einmo promote output to checked` (CLI, review and promotion are separate acts) |
| **CI gate** | `INSTA_UPDATE=no` (env var, easy to miss) | `einmo compare output checked --require-match` (explicit, signed) |
| **Human attestation** | None | `einmo promote checked to verified --interactive` (passphrase, detects computer key) |
| **Crash safety** | None (crash leaves no trace) | Catastrophe crumb (signed "TEST IN PROGRESS" file survives) |
| **Error capture** | `eprintln!` (swallowed) | `status: input-error` / `status: output-error` in the signed `.einmo` |

### Step-by-step migration guide

**Step 1: Create the einmo suite directory structure.**

If your insta snapshots live in `snapshot_tests/input/` and
`snapshot_tests/approved/`, create the einmo stage directories alongside
`input/`:

```
snapshot_tests/
├── input/          # existing .foo inputs (unchanged)
├── generated/      # einmo generates here (new); GITIGNORED -- never commit it
├── output/         # the accepted baseline (new); output/flagged/ holds set-aside files
├── checked/        # promoted baselines (new, replaces approved/); checked/flagged/ holds set-aside files
└── verified/       # human-signed (new); verified/flagged/ holds set-aside files
```

Add `generated/` to your `.gitignore`. It is the work file: einmo rewrites it
on every run, and committing it would put churn in every diff.

(`flagged/` is not a top-level directory or a stage of its own -- each of the
four stages above carries its own nested `flagged/` sink, so a flagged
artifact's origin stage is always known. See "The Four Stages" above.)

**Step 2: Write the `Evaluator` adapter.**

Wrap your existing evaluator in an `impl Evaluator for ...` that returns
`Vec<String>` (one string per top-level result). The adapter formats the
internal type to a human-readable string. For Foolish, this is
`FirSequencer::format`.

**Step 3: Replace the test function.**

Replace the `insta::Settings` + `assert_snapshot!` loop with
`EinmoSuite::evaluate_all`. The suite handles discovery, evaluation, signing,
writing, and re-verification.

**Step 4: Run the test to generate `output/`.**

```bash
cargo test -p foolish-ubca -- approval_all
```

Every input produces `output/<name>.einmo` -- a signed file with the input,
the evaluator's output, and the stamp chain.

**Step 5: Review and promote to `checked/`.**

```bash
einmo compare output checked snapshot_tests/          # see what's new
einmo promote output to checked snapshot_tests/          # promote all

# Or promote just one file (no --filter needed):
einmo promote output to checked snapshot_tests/ alarm_division_by_zero_in_brane.foo.einmo

# Promote a few specific files:
einmo promote output to checked snapshot_tests/ a.foo.einmo b.foo.einmo c.foo.einmo

# Promote everything that changed (pipe from find):
find snapshot_tests/output -name '*.einmo' -newer snapshot_tests/checked | \
  einmo promote output to checked snapshot_tests/ -
```

The promoted files in `checked/` carry an additional `stage:checked` stamp.
The `output == checked` correspondence gate now passes on the next test run.

**Step 6: (Optional) Human-verify for release.**

```bash
einmo promote checked to verified snapshot_tests/ --interactive
# Enter a human passphrase (not empty) to produce a stage:verified stamp.
# An AI piping --passphrase "" gets the computer key -- post-hoc detectable.
einmo confirm-signatures snapshot_tests/verified <release-key-prefix> --require-all

# Verify just one critical file before signing off:
einmo verify --all snapshot_tests/ critical_path_test.einmo

# Show the stamp chain on one file to confirm the officer's key:
einmo show snapshot_tests/verified/critical_path_test.einmo

# Compare one specific file between checked and verified:
einmo compare checked verified snapshot_tests/ critical_path_test.einmo --require-match

# If a file needs to be pulled from the release:
einmo flag snapshot_tests/ verified broken_release_test.einmo --reason "regression found post-verify"
```

**Step 7: Delete the old `.snap` files.**

Once the einmo suite is green and the `checked/` baselines are committed,
remove the old `approved/*.snap` files and the `insta` dev-dependency. The
migration is complete.

### Notes for the migrating agent

- The `Evaluator` adapter must be `Sync`. If your interpreter is `!Send`
  (RustPython, Boa), construct it *inside* `evaluate` per call -- do not
  store it in the adapter struct.
- If your output contains the separator character (`①` by default), use
  `TestConfig::with_separator("!!\n")` or another string that does not appear
  in your output.
- The `checked/` directory replaces `approved/`. The `.snap` format and the
  `.einmo` format are not compatible -- migration is a one-way conversion
  (generate fresh `.einmo` from the evaluator, then promote).
- Catastrophe crumbs from crashed runs will block re-runs. Use
  `EINMO_RERUN_CATASTROPHES=1` to overwrite them, or
  `EINMO_IGNORE_CATASTROPHE_CRUMBS=crash.foo.einmo` to accept a known crash
  as expected.

---

## Appendix: Draconian Release Gate

This example shows a deployment test that enforces the strictest possible
attestation chain. It is appropriate for a release gate where every test
output must be human-verified under a specific release officer's key before
the release may ship.

The test is named `iron_grid_release_attestation` to convey its severity.

### What it enforces

1. Every input file produces a written, self-verifying `.einmo` in `output/`.
2. Every `output/` file has a corresponding, non-empty file in `verified/`.
3. Every `verified/` file carries a `stage:verified` stamp whose public key
   matches the release officer's key embedded in the test code.
4. No file in `verified/` was signed by the well-known computer (empty
   passphrase) key -- an AI bypass is detected and fails the gate.
5. The `output` and `verified` stages are byte-identical in their INPUT and
   OUTPUT sections (no unreviewed drift).

If any of these conditions fail, the test fails and the release is blocked.

### The code

```rust
use einmo::{
    confirm_signatures, compare, EinmoFile, EinmoSuite, Evaluator,
    MatchSections, Stage, TestConfig,
};
use std::path::Path;

/// The release officer's Ed25519 public key (hex).
///
/// This is NOT the empty-passphrase computer key. It is derived from the
/// release officer's private passphrase, which is never stored in code.
/// Only someone who knows the passphrase can produce a `stage:verified`
/// stamp whose pubkey matches this constant.
const RELEASE_OFFICER_PUBKEY: &str = "dc5f586c3a1b2e4f8a0c7d9e1f3a5b7c9d1e3f5a7b9c1d3e5f7a9b1c3d5e7f9a";

/// The well-known computer key prefix (first 8 hex chars of the
/// empty-passphrase key). Used to detect AI bypass.
const COMPUTER_KEY_PREFIX: &str = "a4c2e6b0";

#[test]
fn iron_grid_release_attestation() {
    let suite_dir = Path::new("release_suite");

    // --- Phase 1: generate output/ ---
    //
    // Every input is evaluated. The suite writes signed .einmo files to
    // output/. If any file fails to write or self-verify, the gate fails
    // immediately.
    let config = TestConfig::new(suite_dir);
    let suite = EinmoSuite::new(config);
    let results = suite
        .evaluate_all(&ProductionEvaluator)
        .expect("evaluation must not fail at the filesystem level");

    assert!(
        !results.files.is_empty(),
        "release suite must contain at least one test"
    );
    for file in &results.files {
        assert!(
            file.written_and_verified,
            "RELEASE BLOCKED: {} was not written and verified: {:?}",
            file.rel_path.display(),
            file.detail
        );
    }

    // --- Phase 2: every output must have a non-empty verified counterpart ---
    //
    // The verified/ directory must contain a file for every input. The file
    // must be non-empty (a zero-byte file means someone created the path
    // but never actually promoted and signed it).
    let inputs = einmo::verify(&TestConfig::new(suite_dir), Some(Stage::Output))
        .expect("verify output stage");
    for file_verif in &inputs.files {
        let verified_path = suite_dir
            .join("verified")
            .join(&file_verif.rel_path);

        assert!(
            verified_path.exists(),
            "RELEASE BLOCKED: no verified file for {}",
            file_verif.rel_path.display()
        );

        let metadata = std::fs::metadata(&verified_path)
            .expect("verified file must be stat-able");
        assert!(
            metadata.len() > 0,
            "RELEASE BLOCKED: verified file for {} is empty (was it actually promoted?)",
            file_verif.rel_path.display()
        );
    }

    // --- Phase 3: every verified file must carry the release officer's key ---
    //
    // confirm_signatures walks verified/ and reports which files carry a
    // stamp whose pubkey starts with RELEASE_OFFICER_PUBKEY. If any file
    // lacks the officer's signature, the gate fails.
    let sig_report = confirm_signatures(
        &suite_dir.join("verified"),
        RELEASE_OFFICER_PUBKEY,
    )
    .expect("confirm-signatures must not fail at the filesystem level");

    assert!(
        sig_report.all_matched(),
        "RELEASE BLOCKED: {} verified file(s) lack the release officer's signature ({:#x}): {:?}",
        sig_report.unmatched.len(),
        RELEASE_OFFICER_PUBKEY,
        sig_report.unmatched,
    );

    // --- Phase 4: no verified file may carry the computer key ---
    //
    // An AI that ran `einmo promote checked to verified --passphrase ""` would
    // produce a stage:verified stamp under the well-known empty-passphrase
    // key. This is post-hoc detectable: confirm_signatures with the computer
    // key prefix should match ZERO files.
    let computer_report = confirm_signatures(
        &suite_dir.join("verified"),
        COMPUTER_KEY_PREFIX,
    )
    .expect("computer-key scan must not fail");

    assert!(
        computer_report.matched.is_empty(),
        "RELEASE BLOCKED: {} verified file(s) were signed by the computer key (AI bypass detected): {:?}",
        computer_report.matched.len(),
        computer_report.matched,
    );

    // --- Phase 5: output and verified must be byte-identical in content ---
    //
    // The INPUT and OUTPUT sections of every file in output/ must match its
    // counterpart in verified/. This catches unreviewed drift: if someone
    // changed the code after the last verification, output/ will differ from
    // verified/ and the gate fails.
    let cmp = compare(
        &TestConfig::new(suite_dir),
        Stage::Output,
        Stage::Verified,
        MatchSections::InputOutput,
    )
    .expect("compare must not fail at the filesystem level");

    assert!(
        cmp.is_clean(),
        "RELEASE BLOCKED: output does not match verified \
         ({} differing, {} only-in-output, {} only-in-verified, {} tampered)",
        cmp.differing.len(),
        cmp.only_in_a.len(),
        cmp.only_in_b.len(),
        cmp.tampered.len(),
    );
}

// --- The evaluator (application-specific) ---

struct ProductionEvaluator;
impl Evaluator for ProductionEvaluator {
    fn evaluate(&self, source: &str) -> Result<Vec<String>, String> {
        // ... your production evaluator logic ...
        Ok(vec![source.to_uppercase()])
    }
}
```

### How to set up the release officer's key

The `RELEASE_OFFICER_PUBKEY` constant is the hex-encoded Ed25519 verifying
key derived from the officer's passphrase. To obtain it:

```bash
# The officer runs this once and pastes the output into the test code:
echo -n "my-secret-release-passphrase" | einmo derive-pubkey
# (hypothetical subcommand; or use a small Rust script calling
#  einmo::signature::derive_keypair -- note: derive_keypair is pub(crate),
#  so in practice you'd derive it via the einmo CLI or a helper crate.)
```

The passphrase itself is never stored in the repository. Only the public key
is embedded in the test. An attacker who steals the repository cannot produce
a valid `stage:verified` stamp without the passphrase.

### Why this is draconian

| Gate | What it catches |
|---|---|
| Phase 1 (written + verified) | Evaluator crash, signing failure, filesystem error |
| Phase 2 (non-empty verified) | Someone forgot to promote a test to verified/ |
| Phase 3 (officer key present) | Verified by the wrong person, or not verified at all |
| Phase 4 (no computer key) | An AI bypassed the human gate with `--passphrase ""` |
| Phase 5 (output == verified) | Code changed after the last verification (drift) |

If all five phases pass, every test output has been generated, human-reviewed,
signed by the release officer, and has not drifted since. The release may
ship.

---

## Appendix: Development Compliance Test

This is the everyday test a coding agent runs during development. It is
lightweight: it generates `output/`, then checks that `output` matches the
committed `checked/` baseline. No human passphrase, no verified stage, no
release officer key -- just "does my code still produce the same output as
the last reviewed baseline?"

The default signing key (empty passphrase, the well-known computer key) is
configured directly in the code. This is intentional for development: the
test runner signs with the computer key, and the `checked/` baseline was
promoted with the same key. A human reviews the diff before promoting, but
the signature is the computer key -- not a human attestation. That is fine
for development; the draconian gate (above) is what enforces human
attestation for releases.

### The code

```rust
use einmo::{EinmoSuite, Evaluator, Stage, TestConfig};

struct DevEvaluator;
impl Evaluator for DevEvaluator {
    fn evaluate(&self, source: &str) -> Result<Vec<String>, String> {
        // ... your evaluator ...
        Ok(vec![format!("processed: {source}")])
    }
}

#[test]
fn dev_compliance_output_matches_checked() {
    let config = TestConfig::new("dev_suite")
        // The default key is the empty-passphrase computer key.
        // TestConfig::new already sets this (output/checked passphrases
        // default to ""). This line makes it explicit for readability.
        .require_correspondence(Stage::Output, Stage::Checked);

    let suite = EinmoSuite::new(config);
    let results = suite.evaluate_all(&DevEvaluator)
        .expect("evaluate_all should not fail at the filesystem level");

    // Every test must have been written and self-verified.
    for file in &results.files {
        assert!(
            file.written_and_verified,
            "{} failed to write/verify: {:?}",
            file.rel_path.display(),
            file.detail
        );
    }

    // output must match checked. If checked/ is empty (first run, or
    // no baseline committed yet), this will report only-in-output for
    // every file -- which is the signal to review and promote.
    assert!(
        results.all_output_written_and_verified(),
        "output does not match checked baseline:\n  {}",
        results.correspondence_failures.join("\n  ")
    );
}
```

### What the coding agent does during development

The agent writes code, runs the test, and inspects the diff. If `output`
diverged from `checked/`, the agent decides: fix the code (so output matches
checked), or promote the new output (if the change is intentional).

```bash
# 1. Run the test (generates output/ and compares to checked/)
cargo test -- dev_compliance_output_matches_checked

# 2. If the test failed, see what changed:
einmo compare output checked dev_suite/

# Or check just the files that failed:
einmo compare output checked dev_suite/ failing_test.einmo

# 3a. If the change is a bug -- fix the code, re-run step 1.

# 3b. If the change is intentional -- review the diff, then promote:
einmo promote output to checked dev_suite/                         # promote all
einmo promote output to checked dev_suite/ fixed_test.einmo        # or just one
echo "a.einmo\nb.einmo" | einmo promote output to checked dev_suite/ -  # or from stdin

# 4. Verify signature integrity of the promoted files:
einmo verify dev_suite/ --stage checked                           # all files
einmo verify --all dev_suite/ important_test.einmo                # just one

# 5. Confirm the computer key signed everything (no human key leaked in):
einmo confirm-signatures dev_suite/checked a4c2e6b0 --require-all

# 6. Show a specific file's stamp chain to inspect provenance:
einmo show dev_suite/checked/important_test.einmo

# 7. If a test is wrong and needs to be set aside:
einmo flag dev_suite checked --filter broken_test.einmo --reason "needs rework"
# Or target the file directly:
einmo flag dev_suite checked broken_test.einmo --reason "needs rework"

# 8. Re-run the test to confirm the suite is clean:
cargo test -- dev_compliance_output_matches_checked
```

### Setting up a new suite from scratch

When starting a new einmo suite with no `checked/` baseline, every input
will appear as `only-in-output` on the first run. The agent reviews the
generated output, then promotes:

```bash
# First run -- generates output/, checked/ is empty:
cargo test -- dev_compliance_output_matches_checked
# Expected: failures for every file (only-in-output).

# Review the generated outputs:
einmo show dev_suite/output/first_test.einmo
einmo show dev_suite/output/second_test.einmo

# Promote everything to checked/ (the computer key is used by default):
einmo promote output to checked dev_suite/

# Commit the checked/ baseline:
git add dev_suite/checked/
git commit -m "Add einmo checked baseline for dev_suite"

# Now the test passes:
cargo test -- dev_compliance_output_matches_checked
```

### The default key in configuration

`TestConfig::new(work_dir)` sets the `generated`, `output`, and `checked`
stage passphrases to `""` (the empty string). This means the test runner signs
with the well-known computer key. An `einmo.toml` in the work directory can
override this:

```toml
# dev_suite/einmo.toml
[signing]
generated = ""
output    = ""
checked   = ""
```

Or, to use a shared team key for development (so team members can promote
without prompting):

```toml
[signing]
checked = "team-dev-shared-passphrase"
```

> **Corrected 2026-08-13.** Earlier revisions of this section documented a
> `[signing.<stage>]` table with a `passphrase = "..."` key. **That form does
> not work and never did** — einmo reads `[signing]` with one string value per
> stage name, and silently ignores a nested table. The failure is quiet and
> the wrong way round: a suite configured the documented way signs with the
> well-known **computer key** while its author believes a team key is in use.
> If you have an `einmo.toml` written against the old text, convert it and
> re-check the stamps with `einmo show`.

The passphrase is resolved through the cascade: CLI `--passphrase` > env
`EINMO_PASSPHRASE` > `einmo.toml [signing]` > interactive prompt. For
development, the empty passphrase is the default and requires no
configuration.

---

## Last Updated

**Date**: 2026-08-13 (2)
**Updated By**: Claude Code (Opus 5)
**Changes**: EIMP-01 landed, so the "What Passing Means" section's status
marker is **removed** — it now describes einmo as it ships. Every claim in it
was re-verified against the implementation first: the named signatures against
`stage.rs`/`signature.rs`, the compared sections against `compare.rs`, and
**every command executed** on a scratch suite (the two that should exit
non-zero did, and went green after their promotion).
Four commands were wrong as written and are fixed: `einmo verify --level
output` now needs `--command`, and `compare`/`promote` examples were missing
their `<work_dir>`.
"The Three Stages" becomes **The Four Stages**, with `generated/` and a
committed column; the `flagged/` note, the insta-migration directory tree, the
CLI table (`generate`, `retract`, the legal promotion pairs, `verify --level`),
Quick Start, and Catastrophe Crumb Defense all updated for the new model.
Quick Start also now explains that `all_output_written_and_verified()` folds in
the level's gate, so a suite with no baseline reports `false` correctly.
Added an `einmo.toml` `[signing]` section.
**Corrected a pre-existing defect** found while writing it: the README
documented `[signing.<stage>] passphrase = "..."`, which einmo does not parse.
Verified by experiment — a suite configured the documented way signs with the
well-known computer key (`5b846599`) while its author believes a team key is
in use, and the correct form (`[signing] generated = "..."`) yields a
different key. The old text is called out explicitly rather than quietly
replaced, since anyone who followed it has a suite signed by the wrong key.

**Date**: 2026-08-13
**Updated By**: Sisyphus (mimo-v2.5-pro)
**Changes**: Added **§"Running specific tests"** — the CENTRAL reference for
running one test or a subset of tests (what EIMP plan checkboxes link to):
test selection by name filter (`just test <filter>`, single filter, multi-filter
batch with OR semantics, `--exact`, `--list`), crate scoping (`-p einmo`), and
combined crate+filter batch examples. Command forms live ONLY in this section;
EIMP plans name tests, not commands — so tooling evolution touches one place.

**Date**: 2026-08-11
**Updated By**: Claude Code (Opus 5)
**Changes**: Added the end-user "What Passing Means" section — a cascading
explanation of what a passing suite asserts, from "all three gates pass" down
to the named signatures (`compiled`, `configured`, `stage:*`) and named
sections (`INPUT`, `OUTPUT[i]`, `DIFF`, `COMMENTS`) each gate compares, plus
what is deliberately excluded (the `STAMPS` block and the metadata header, so
two runs an hour apart still match). Also covers the remedy per red gate, the
weak-versus-real distinction between the two promotions, retraction's cascade,
and inspecting `generated/` without touching the baseline. **The section
carries a status marker: it describes the model specified in
`docs/eimp/EIMP-01.md`, which is not yet implemented.** The marker is removed,
and "The Three Stages" updated to four, as part of that EIMP's Phase 7.

**Date**: 2026-08-01
**Updated By**: Sisyphus (mimo-v2.5-pro)
**Changes**: Added "Development" section with cross-reference to the
Developer Guide in `rust_instructions.md`.

**Date**: 2026-07-31 (2)
**Updated By**: Claude Code (Sonnet 5)
**Changes**: `EIMP-7`'s documentation follow-up (§S.9): "The Four Stages"
→ "The Three Stages" — `flagged/` is not a fourth stage, it's a sink
nested inside each of the three real stages (`output/flagged/`,
`checked/flagged/`, `verified/flagged/`). Updated the stage table, the
Flagging section's prose, the same-path-twice rule, the CLI subcommand
table, and the insta-migration directory tree to match.

**Date**: 2026-07-31
**Updated By**: Sisyphus (mimo-v2.5-pro)
**Changes**: Added "Review Server" section documenting the review
architecture (server as backend, TUI as user-facing tool), the three
review modes as server capabilities, TUI client workflow, vim keybindings,
and crash recovery.
