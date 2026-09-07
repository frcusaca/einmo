# How to Understand and Develop einmo

This document has two deliberately separate parts. Part I is a code-reading
and experimentation tutorial. Part II is a critical engineering review with
prioritized recommendations.

The review describes commit `9c8589a` on branch `jia`, inspected on
2026-08-25/26 and rechecked on 2026-09-01. It covers the `einmo`,
`einmo-tools`, and `zweimomo` workspace crates. It does not treat the EIMP
documents as proof that the implementation is correct: claims below were
checked against the current source and, where practical, against the CLI.

If you knew einmo before EIMP-01, translate the old workflow before reading
further:

| Before EIMP-01 | Current model |
|---|---|
| Evaluation overwrote committed `output/` | Generation writes only gitignored `generated/` |
| `einmo evaluate` | `einmo generate` (`evaluate` remains an alias) |
| `einmo regenerate-output` | Inspect, then `einmo promote generated to output` |
| Validation levels accumulated lower checks | Each level checks exactly one adjacent pair |
| `output/` was both work file and baseline | `generated/` is the work file; `output/` is the committed baseline |

The practical consequence is important: running the evaluator is no longer an
acceptance decision. Fresh results can be generated and inspected without
changing anything already reviewed.

## Part I — A Practical Tutorial for Reading einmo

### 1. Begin with the invariant, not with `main`

The most useful one-sentence model is:

> einmo evaluates input into an uncommitted signed work artifact, then records increasingly strong claims by copying that content forward and appending signatures.

The intended flow is:

```text
input source
    │ generate (invokes the evaluator)
    ▼
generated ── sanity acceptance ──▶ output ── substantive review ──▶ checked ── human attestation ──▶ verified
    │                                  │                              │                              │
    └─ generated/flagged/              └─ output/flagged/             └─ checked/flagged/            └─ verified/flagged/
```

The four directories are stages; `flagged` is not a fifth stage. It is a sink nested under each stage. The three gates each compare one adjacent link:

| Gate | Pair checked | Claim |
|---|---|---|
| output | `generated ↔ output` | the current evaluator still produces the accepted baseline |
| checked | `output ↔ checked` | the accepted baseline is what was reviewed |
| verified | `checked ↔ verified` | the reviewed content is what a human attested |

Keep three kinds of equality separate while reading:

1. File bytes are not expected to match across stages because timestamps and stamp chains differ.
2. Required body sections (`INPUT`, `OUTPUT[*]`, `DIFF`, optionally `COMMENTS`) are expected to match at a gate.
3. Every individual file must independently pass signature verification before its bodies may be compared.

That distinction explains most of the code.

### 2. Read the repository in this order

Do not start with the 1,500-line CLI or the 3,600-line runner. Use this route:

1. `src/stage.rs`
   - Learn `Stage`, its lifecycle ordering, `EinmoId`, path mirroring, and tree walking.
   - Notice that declaration order is semantically significant because `Ord` is derived.
2. `src/format.rs`
   - Read `Status`, `Section`, `Metadata`, and `EinmoFile`.
   - Trace `EinmoFile::signed_prefix`, `serialize`, and `parse` together.
   - The separator-collision rule is the key format simplification.
3. `src/signature.rs` and then `src/verify.rs`
   - Follow the certification stamps and stage stamps.
   - The crucial verification detail is that `verify_bytes` verifies the raw bytes from disk, not a reserialized canonical form. This catches whitespace changes inside the signed prefix.
4. `src/storage.rs`
   - Learn the `(EinmoId, ArtifactLocation)` abstraction. It prevents higher layers from hard-coding directory operations and gives tests an in-memory backend.
5. `src/case.rs`
   - This is the best domain-level entry point. One `EinmoCase` owns read, agreement, promotion, flagging, and retraction behavior.
6. `src/suite.rs`
   - See how a suite scans and deduplicates case IDs, selects subsets, derives one expensive key per batch, and applies case operations.
7. `src/compare.rs` and the integrity portion of `src/einmo_suite.rs`
   - Read `compare_sections`, `EinmoCase::agreement`, and `check_suite_integrity` together.
   - Then read `ValidationLevel::stage`/`compares_against` to see why each gate checks only one link.
8. The generation portion of `src/einmo_suite.rs`
   - Trace the public `evaluate`/`evaluate_all` methods through crash-crumb creation, evaluator capture, dependent-case ordering, writes to `generated/`, and self-verification. The method names describe invoking an `Evaluator`; the stage-producing CLI operation is named `generate`.
9. `src/config.rs`
   - Study stage directories, matching policy, parallelism and limits, and the signing-key precedence cascade.
10. `src/cli.rs`
    - Only now map commands onto the library operations. Start at `dispatch`, then inspect one `cmd_*` function at a time.
11. `src/review.rs`, `src/review_server.rs`, and the review-server binary
    - Treat these as a second application layered over the same case/suite operations: decisions become a plan; the plan becomes signed transitions; a journal supports recovery.
12. `src/corpus_signer.rs` and `einmo-tools`
    - Read last. They add section-level post-quantum corpus signatures and a passphrase heuristic, but are not needed to understand the basic snapshot loop.

After each module, state its invariant in your own words. Good checkpoints are:

- `EinmoId`: “validated, stage-independent case identity.”
- `EinmoFile`: “ordered opaque sections plus a stamp chain; layout is private.”
- `EinmoStorage`: “artifact bytes addressed by identity and role, not by path.”
- `EinmoCase`: “all operations on one case.”
- `EinmoSuite`: “selection and batch policy over cases.”
- `EinmoTestRunner`: “evaluation lifecycle and gate aggregation.”

### 3. Trace one artifact end to end

For one input such as `integer_arithmetic.py`, follow these calls:

```text
CLI `generate`
  → CommandEvaluator::evaluate(source)
  → EinmoTestRunner::evaluate/evaluate_all
  → write crash crumb in generated/
  → evaluator outcome becomes Status + OUTPUT section(s)
  → build Metadata + Section list
  → Stamps::generate(..., stage:generated)
  → EinmoFile::serialize
  → write generated/integer_arithmetic.py.einmo
  → EinmoFile::from_file
  → verify_bytes against raw signed bytes
```

Promotion follows a different path:

```text
CLI `promote generated to output`
  → EinmoSuite::scan
  → derive StageKeypair once for the batch
  → EinmoCase::promote
  → verify source
  → compare source body with any destination body
  → append stage:output stamp (or co-sign existing matching content)
  → serialize and write destination
```

Comparison is deliberately signature-aware but stamp-insensitive:

```text
compare(a, b)
  → EinmoSuite::scan
  → EinmoCase::agreement([a, b], MatchSections)
  → verify both artifacts
  → compare required body sections
  → Agree | Differ | OneSided | BothAbsent | Tampered
```

### 4. Use zweimomo as the laboratory

`zweimomo` is both a demo and an integration fixture. It supplies:

- `BoaEvaluator`, which evaluates JavaScript in-process and converts the resulting value to a string;
- `Pyo3Evaluator`, which evaluates a Python expression in-process and returns `str(result)`;
- committed `output/` and `checked/` artifacts for real JavaScript and Python cases;
- comprehensive tests that copy fixtures to a temporary directory before perturbing them.

The source fixtures are small on purpose. Read these pairs first:

- `input/integer_arithmetic.*` and its `.einmo` artifacts;
- `division_by_zero.*`, because JavaScript returns `Infinity` while Python produces an evaluator error;
- `nested_expressions++divisionByZero.*`, because `++` identifies a dependent case and produces a `DIFF` section;
- `search_query.*` and `data_structures.*`, because serialization choices become visible.

Important environment caveat: the current `zweimomo` README says a system Python is enough, but linking its tests requires the matching Python development/shared library. In the review environment Python 3.14 existed, while `python3-config` and `libpython3.14` did not; `zweimomo` compiled under clippy but its test binary could not link. Verify both before blaming Rust code:

```bash
python3 --version
python3-config --ldflags
```

### 5. Build a safe scratch lab

Never perturb the committed signed fixtures. Copy a suite to a temporary directory:

```bash
scratch=$(mktemp -d /tmp/einmo-lab.XXXXXX)
cp -a zweimomo/suites/python "$scratch/python"

# Build the CLI once. If your environment shares CARGO_TARGET_DIR across
# checkouts, override it with a private ignored directory.
CARGO_TARGET_DIR="$PWD/target/einmo-lab" cargo build -p einmo --bin einmo
einmo="$PWD/target/einmo-lab/debug/einmo"
```

If the project tooling is installed, prefer its isolated runner for tests:

```bash
just test integer_arithmetic
```

Do not run multiple Cargo commands concurrently in this repository. Do not use `cargo test` as a substitute for the full `nextest` gate: tests that share mutexes can poison later tests after a panic, creating misleading cascades.

### 6. First experiment: generate, inspect, and compare

The generic CLI evaluator receives the source on stdin and records stdout byte-for-byte:

```bash
"$einmo" generate "$scratch/python" \
  --command 'python3 -c "import sys; print(eval(sys.stdin.read()))"' \
  --filter integer_arithmetic

"$einmo" show "$scratch/python/generated/integer_arithmetic.py.einmo"
"$einmo" body "$scratch/python/generated/integer_arithmetic.py.einmo"
"$einmo" compare generated output "$scratch/python" \
  integer_arithmetic.py.einmo
```

Expected observations:

- `show` exposes metadata and the `compiled → configured → stage:generated` chain.
- `body` exposes signed `INPUT`, `OUTPUT`, and `COMMENTS` sections.
- The CLI command above may differ from the committed `Pyo3Evaluator` fixture even though both display `9`: Python `print()` adds `\n`, while `Pyo3Evaluator` returns exactly `"9"`. This is a valuable demonstration that einmo compares bytes, not terminal appearance.

To reproduce the in-process evaluator exactly from a subprocess, suppress the newline:

```bash
"$einmo" generate "$scratch/python" \
  --command 'python3 -c "import sys; sys.stdout.write(str(eval(sys.stdin.read())))"' \
  --filter integer_arithmetic
```

### 7. Perturbations that teach the design

#### A. Change a result without touching a baseline

```bash
sed -i 's/2 + 3 \* 4 - 5/2 + 2/' \
  "$scratch/python/input/integer_arithmetic.py"

"$einmo" generate "$scratch/python" \
  --command 'python3 -c "import sys; sys.stdout.write(str(eval(sys.stdin.read())))"' \
  --filter integer_arithmetic

"$einmo" compare --require-match generated output "$scratch/python" \
  integer_arithmetic.py.einmo
```

Generation should succeed; comparison should report `INPUT` and `OUTPUT` differences and exit 1. This is the central design lesson: generation is indifferent to the baseline; the output gate objects.

If the change is intentional, accept it only in the scratch copy:

```bash
"$einmo" promote generated to output "$scratch/python" \
  integer_arithmetic.py.einmo
"$einmo" compare --require-match generated output "$scratch/python" \
  integer_arithmetic.py.einmo
```

The second comparison should be clean. `show` on the new output should reveal that `stage:output` was appended after `stage:generated`.

#### B. Demonstrate the CLI option-ordering defect

With a known mismatch, compare these commands:

```bash
# Correct today: option before the positional stages.
"$einmo" compare --require-match generated output "$scratch/python" \
  integer_arithmetic.py.einmo
echo "$?"   # 1

# Defective today: the trailing positional list consumes the option as a path.
"$einmo" compare generated output "$scratch/python" \
  integer_arithmetic.py.einmo --require-match
echo "$?"   # 0, despite reporting a difference
```

Until fixed, place every option before the first positional argument whenever a subcommand also accepts `[FILES]...`.

#### C. Change formatting only

Change a Python expression from `2 + 2` to `(2+2)` while preserving the result. The comparison should report `INPUT` but not `OUTPUT`. This demonstrates that einmo snapshots both what ran and what it returned.

#### D. Add output whitespace

Use `print` rather than `sys.stdout.write`, or append two spaces. The gate should report an `OUTPUT` difference. Then use `body` and a byte-oriented tool to make invisible whitespace visible:

```bash
"$einmo" body "$scratch/python/generated/integer_arithmetic.py.einmo" | sed -n l
```

#### E. Produce an evaluator error

Put `1 / 0` in a Python input and regenerate. The CLI subprocess exits nonzero only if your evaluator command propagates that failure. A naive `eval` command does; inspect the resulting status/detail behavior. Compare this with JavaScript’s `10 / 0`, which is a normal value (`Infinity`) under `BoaEvaluator`.

This teaches that `Status` describes harness abnormality, not whether the tested language regards a value as alarming.

#### F. Tamper with a signed artifact

In the scratch copy only:

```bash
cp "$scratch/python/output/integer_arithmetic.py.einmo" \
   "$scratch/original.einmo"
sed -i 's/^9$/8/' "$scratch/python/output/integer_arithmetic.py.einmo"
"$einmo" verify --all "$scratch/python" integer_arithmetic.py.einmo
"$einmo" show "$scratch/python/output/integer_arithmetic.py.einmo"
```

Both inspection paths should refuse the file. Restore it from `original.einmo`; do not “repair” a signed envelope by editing it.

#### G. Tamper with insignificant-looking metadata whitespace

Add a space after a signed metadata colon in a scratch artifact. Parsing alone normalizes some fields, but `verify_bytes` verifies the actual raw prefix and should reject the change. This is the clearest way to understand why `raw_signed_prefix` exists.

#### H. Exercise one-sided and orphan cases

- Delete a scratch `generated/*.einmo`: comparison reports only-in-output.
- Add a new input, generate it, and do not promote: comparison reports only-in-generated.
- Delete an input but retain its committed output: the integrity gate reports an orphan.

These are different conditions and intentionally receive different diagnostics.

#### I. Exercise dependent cases

Change `division_by_zero.py`, regenerate it and `nested_expressions++divisionByZero.py`, and inspect the dependent artifact’s `DIFF` section. Then use `--root-cause`. Read `topological_order`, `reference_of`, and `deterministic_unified_diff` while the artifacts are open; the code becomes much easier to understand when tied to the two files.

#### J. Exercise flagging and retraction

In scratch data:

```bash
"$einmo" flag "$scratch/python" output integer_arithmetic.py.einmo \
  --reason 'tutorial perturbation'
"$einmo" verify --all "$scratch/python"
```

The file moves to `output/flagged/`; its signed body still verifies, while the advisory is unsigned. Then study `EinmoCase::flag` and `ArtifactLocation::Flagged`.

For retraction, promote through output and checked in scratch, then retract output. Observe that output, checked, and verified are removed highest-first. Generated is refused because it is regenerated rather than demoted.

#### K. Try configuration failure explicitly

Place malformed TOML or a wrong-typed key in the scratch suite and run a command. Today some configuration errors are silently ignored; this is a finding in Part II. The experiment is useful because it distinguishes “configuration was accepted” from “configuration was not read and defaults happened to work.”

### 8. Read tests as executable design documents

The test suite is unusually rich. Start with behavior-level tests, not every small parser case. There are two complementary EIMP-01 narratives: the library-level `eimp01_comprehensive_the_whole_chain` and the real-evaluator `zweimomo` test `eimp01_generate_promote_comprehensive`.

- `eimp01_comprehensive_the_whole_chain`
- `eimp01_generate_promote_comprehensive`
- `eimp01_output_gate_goes_red_on_divergence_and_green_after_promotion`
- `metadata_whitespace_tamper_detected`
- `promote_refuses_tampered_source`
- `agreement_tampered_is_never_folded_into_differ`
- `crash_crumb_survives_stack_overflow`
- parallel-versus-serial agreement tests
- review journal replay and execution-plan tests

The comprehensive `zweimomo` tests are especially good reading because they narrate intent step by step and operate on temporary copies of real signed fixtures.

When changing behavior, use the repository’s process:

1. Write a focused failing test first.
2. Run its small subset with `just test <filter>`.
3. Implement the smallest correction.
4. Re-run the subset.
5. Run formatting, clippy, and the complete `nextest` gate before completion.
6. Use scoped mutation testing for security- or gate-sensitive modules.
7. Generate into `generated/`, inspect, and promote only after making the appropriate claim.
8. Never promote `checked` to `verified` as an agent; that is a human attestation.

### 9. A concise mental map of the review subsystem

The review system is easier to understand as command/query separation:

```text
scan cases → present ReviewItems → record Decisions in DecisionBook + Journal
                                      │
                                      ▼
                                ExecutionPlan
                                      │ confirmed
                                      ▼
                         EinmoSuite/EinmoCase transitions
```

`VerifiedCache` avoids repeatedly parsing and verifying the same artifact. `Journal` is append-only recovery state outside the signed corpus. `AppState` manages sessions and broadcasts events. The HTTP layer converts typed path/body values into calls on `EinmoReview`; the standalone binary adds CLI transport and passphrase handling.

Read one endpoint from route to mutation, for example decision submission:

```text
PUT decision route → typed SessionId + EinmoId + DecisionRequest
  → Decision conversion
  → EinmoReview decision method
  → journal event
  → plan changes
  → SSE notification
```

That path displays the architecture better than reading the 2,500-line server module linearly.

## Part II — Critical Engineering Review and Recommendations

### 1. Executive assessment

einmo has a strong and distinctive core: it separates fresh generation, baseline acceptance, semantic review, and human attestation; signs artifacts; refuses tampered inputs on most read paths; uses typed stage/case abstractions; and has extensive behavior-focused tests. The code shows unusually explicit design intent.

It is not ready to be treated as a high-assurance security boundary without further work. The largest risks are not cryptographic primitives but state-transition semantics, fail-open configuration, non-atomic storage, and inconsistencies between stated invariants and actual code. The project’s process documentation is more mature than its automated enforcement.

### 2. Prioritized findings

#### P0 — Promotion can silently overwrite a tampered destination

`EinmoCase::promote` verifies the source, but reads the destination with `verify_bytes(&bytes).ok()`. A parse or signature failure becomes `None`, after which the destination is treated like an absent baseline and overwritten.

This contradicts the crate-level/error documentation that a tampered file is “refused, never operated on.” It can erase evidence of corruption and turn an alarming state into a normal promotion result.

Recommendation:

- Distinguish `Absent`, `Verified`, and `Tampered` for promotion destinations, as agreement already does.
- Return a verification error on `Tampered`; require an explicit recovery/quarantine operation to replace it.
- Add regression tests for tampered output, checked, and verified destinations.
- Apply the same policy to the existing flagged-destination read in `EinmoCase::flag`, which also uses `.ok()` and can discard an invalid prior advisory artifact.

#### P0 — Verified attestation checks only the first verified stamp

`attestation_problems` uses `.find(|s| s.key() == "stage:verified")`. In a multi-signer chain, a human verified stamp followed by a computer-key co-sign causes the first human stamp to be examined and the later computer stamp to be ignored. This violates the documented V7 rule that verified artifacts must not carry the well-known empty-passphrase key.

The same first-stamp logic can also give misleading reviewer-key results when several reviewers accumulate.

Recommendation:

- Inspect all `stage:verified` stamps.
- Define policy explicitly: usually “at least one expected human reviewer and zero computer-key verified stamps.”
- Return diagnostics per offending stamp/key.
- Add order-sensitive tests: human→computer, computer→human, two humans including unexpected key, and duplicate same-key stamps.

#### P0 — State-machine implementation permits bypassing checked review

The documented primary pipeline and gate model is `generated → output → checked → verified`, and the supplied instructions say the three promotions make distinct claims. Yet `is_legal_transition` allows `Output → Verified`, `Stage::Verified` documents promotion from checked “or output,” and the CLI advertises the shortcut.

This lets an artifact acquire a verified-stage signature without ever acquiring or passing through the checked-stage claim. The verified gate still compares `checked ↔ verified`, so the shortcut can produce a verified artifact that cannot make its own gate green. At best this is incoherent; at worst it bypasses the substantive review claim.

Recommendation:

- Remove `Output → Verified` unless a current EIMP explicitly justifies a separate semantics for it.
- If retained, redesign the gate and documentation so the claim is coherent and cannot be mistaken for the adjacent-stage pipeline.
- Add a state-machine matrix test generated from one authoritative transition table.

#### P1 — Configuration parsing fails open

`parse_einmo_toml` returns a value rather than `Result`. It silently drops file-read errors, malformed TOML, wrong-typed values, unknown fields, and invalid per-suite config, falling back to crate-wide config or defaults. Negative signed integer values are also cast to unsigned for several duration/depth settings; `walk_depth_limit = -1` becomes a huge `usize`.

For signing and reviewer identity, “ignored config” can mean using a different key or omitting a required reviewer check. This is a correctness and security problem, not merely poor diagnostics.

Recommendation:

- Make configuration loading return `Result<TestConfigFragment, EinmoError>`.
- Use typed serde structs with `deny_unknown_fields` and validation.
- Reject negative/out-of-range numbers before conversion.
- Include the exact config path and field in errors.
- Make all signing, verification, generation, and server entry points fail closed on config errors.

#### P1 — Stage writes are not atomic or durable

`EinmoDirectory::write` calls `std::fs::write` directly on the destination. A crash, disk-full condition, or concurrent observer can see a truncated file. Promotions and co-signs overwrite committed signed state in place. The crash-crumb design protects evaluator crashes during generation, but it does not protect promotion writes.

Recommendation:

- Write a fully serialized artifact to a same-directory temporary file.
- Flush, optionally `sync_all` according to the durability contract, then atomically rename.
- Preserve permissions and fsync the parent directory where durable rename semantics matter.
- Add fault-injection tests ensuring the old valid artifact survives failures before rename.

#### P1 — CLI trailing file arguments swallow options and can neutralize gates

Subcommands use `#[arg(num_args = 0.., trailing_var_arg = true)] files`. Consequently, options after a positional file are interpreted as additional filenames. In a live reproduction:

- `compare --require-match generated output SUITE case.einmo` exited 1 on divergence;
- `compare generated output SUITE case.einmo --require-match` printed the same divergence but exited 0.

This is especially dangerous because the CLI help presents options normally, and users commonly put options last. Similar ambiguity affects other subcommands with `[FILES]...`.

Recommendation:

- Remove `trailing_var_arg` unless literal option-looking filenames after `--` are the actual requirement.
- Let clap parse options anywhere; require `--` explicitly for filenames beginning with `-`.
- Add integration tests for every boolean gate option before and after positionals.
- Correct examples and document the temporary ordering constraint until released.

#### P1 — The parallel default ignores the suite duration limit

`evaluate_all` contains a dated `KNOWN GAP`: `suite_duration_limit` is enforced only in serial execution, while parallel execution is the default. A configured operational limit therefore does nothing in the common mode.

Recommendation:

- Either reject `suite_duration_limit` with parallel mode until implemented, or enforce a shared deadline and stop scheduling new work after expiration.
- Clearly distinguish cooperative “do not start another case” limits from hard cancellation; subprocess evaluators also need termination semantics.
- Convert the source comment into a tracked issue/test rather than shipping a silently ineffective option.

#### P1 — No visible CI workflow enforces the documented gates

There is no checked-in `.github` workflow in the inspected tree. The `justfile` has useful recipes, but `ci-test` omits `--workspace`, and `pr` scopes mutation testing using `main...HEAD` even though the primary branch is `jia` and work lands directly on it. These problems are documented in prose rather than fixed.

Recommendation:

- Add CI for fmt, workspace clippy, workspace nextest, doctests, and platform/MSRV coverage.
- Fix `ci-test` to include the workspace and prevent fail-fast JUnit reports from masquerading as complete.
- Replace branch-diff mutation scoping with an explicit file list or a correct merge-base strategy for `jia`.
- Add dependency audit/deny checks suitable for a crypto-touching crate.

#### P2 — Documentation disagrees with the model and with itself

Examples:

- `src/lib.rs` still describes `flagged` as a stage and lists four directories as `output/checked/flagged/verified`, omitting `generated`.
- `stage.rs` module docs call the lifecycle “three-stage” while `Stage::ALL` contains four; the `Stage` item says “one of four.”
- `zweimomo/README.md` teaches `cargo test`, contrary to the mandatory nextest guidance, and uses `suites/...` paths without clearly saying to run from `zweimomo/`.
- The root README says CI uses `cargo test`, while the developer guide says that can produce poisoned-mutex cascades and must not be used as the project gate.
- The README’s transition list advertises `output to verified`, while the central diagram says results move one step at a time.
- `zweimomo` describes its evaluators as “pure-Rust,” although `Pyo3Evaluator` depends on a system CPython runtime and development library.

Recommendation:

- Establish one terminology/glossary source and test selected documentation snippets against CLI help.
- Make all command examples explicit about working directory.
- Replace historical FOOP references in public crate docs with current EIMP references where applicable.
- Use a small docs test that detects stale stage lists and command names.

#### P2 — Parser strictness falls short of the stated security policy

The project instructions require rejecting ambiguous, non-canonical, unknown, or trailing data. Current parsing is lenient in several ways:

- metadata lines without `:` are ignored;
- unknown metadata keys are ignored;
- duplicate metadata/header keys use the last value;
- unknown header tokens are ignored;
- the declared encoding is stored but not constrained to `utf-8`, even though bytes must decode as UTF-8;
- section-name invariants such as exactly one trailing `STAMPS`, uniqueness, and expected ordering are not all enforced at construction time;
- `Stamps::parse` accepts an empty stamp list, leaving later verification to fail indirectly.

Raw-byte verification prevents many tampering attacks on already signed files, which is good, but strict parsing still matters for canonical production, diagnostics, fuzz resistance, and any future trust boundary.

Recommendation:

- Define a strict grammar and reject unknown/duplicate fields and unsupported encodings.
- Validate separator non-emptiness and section/stamp-chain structure during parse.
- Separate raw wire DTOs from validated domain types more consistently.
- Add property tests and a fuzz target for parse/serialize/verify.

#### P2 — Lock poisoning can panic production review services

The review and server paths contain many `.expect("... lock poisoned")` calls. A panic while a lock is held can make subsequent requests panic, turning a local defect into a persistent denial of service for the process. This conflicts with the project rule against panics in production and handler paths.

Recommendation:

- Decide whether poisoned data can be recovered, discarded, or must shut down cleanly.
- Convert poison errors into `EinmoError`/`ApiError` or explicitly recover the inner value where invariants permit.
- Add a test that poisons each shared state lock and checks the service response/shutdown behavior.

#### P2 — Passphrase “effectiveness” is not an identity or entropy proof

`einmo-tools` accepts a human verified passphrase when appending it to the compressed verified corpus increases compressed size. This heuristic can accept weak or predictable strings; it is corpus- and compressor-dependent and does not establish reviewer identity. Its error type is also `String`, outside the central typed-error discipline.

Recommendation:

- Treat it only as an advisory UX check, never as evidence of human identity or adequate entropy.
- Prefer explicit reviewer public-key configuration plus secure secret acquisition.
- If password quality is enforced, use a documented threat model and a mature estimator, while recognizing that identity comes from possession of the expected key.
- Return a typed error.

#### P2 — Public metadata is mutable plain data despite security-critical invariants

`Metadata` exposes all fields publicly, including status, section list, producer identity, and timestamps. `EinmoFile::new` accepts metadata whose declared section list is not derived from the actual sections and postpones validation to serialization. This permits invalid in-memory states and weakens encapsulation compared with the otherwise careful `Stamp` and `EinmoFile` APIs.

Recommendation:

- Make metadata fields private with accessors and checked constructors/builders.
- Derive the section declaration from the actual ordered section vector.
- Introduce validated timestamp/provenance types where security decisions use them.

#### P2 — Large modules and duplicated application surfaces impede auditability

`einmo_suite.rs` (~3,650 lines), `review.rs` (~3,440), `review_server.rs` (~2,500), `cli.rs` (~1,500), and the review-server binary (~1,650) are difficult to audit. The code often has good local names, but concerns such as evaluation scheduling, crash defense, integrity reporting, HTTP DTOs, server lifecycle, and CLI rendering are interleaved in very large files.

Recommendation:

- Split by responsibility while keeping the curated `lib.rs` surface.
- Good candidates are `runner/{evaluation,parallel,crash_crumb,integrity}.rs`, `review/{decision,plan,execution,cache}.rs`, and `review_http/{dto,handlers,auth,serve}.rs`.
- Keep tests next to the extracted behavior and avoid creating a generic `utils` module.

### 3. Code quality and style

Strengths:

- Strong newtypes/enums: `EinmoId`, `Stage`, `ArtifactLocation`, `ValidationLevel`, `Problem`, `StagePairAgreement`, and `PromoteOutcome` make many invalid combinations harder to express.
- Curated public API through private modules and `pub use` in `lib.rs`.
- Central non-exhaustive `EinmoError` with path-bearing I/O errors.
- Clear separation between storage, one-case behavior, and batch suite behavior.
- Batch key derivation avoids repeating expensive Argon2 work.
- Secret-bearing types have redacted `Debug`; stage seeds are zeroized and wrapped between signing operations.
- Serialization uses ordered vectors rather than unordered maps.
- Comments frequently explain why, particularly around raw-byte verification, crash crumbs, and multi-signer behavior.
- Clippy passed for the entire workspace with `-D warnings` in this review.

Weaknesses:

- The volume of historical commentary and EIMP archaeology inside production modules sometimes obscures the current contract.
- Several comments confidently state invariants that nearby code violates, especially verify-on-inspect at a destination and the stage sequence.
- `expect` is used for locks in production paths, and central error handling is bypassed in `einmo-tools`.
- Some APIs remain stringly typed at boundaries (`stage_key: &str`, provenance strings, free-form timestamps), although domain enums are used well elsewhere.
- Direct filesystem functions remain outside the storage abstraction for notes, verification walks, config, and journals, producing inconsistent semantics and test seams.

### 4. Test coverage assessment

The tree contains roughly 424 `#[test]`/`#[tokio::test]` declarations. Tests cover format round trips, separator collision, signature chains, tampering, stage agreement, transitions, crash crumbs, config precedence, server endpoints, review journaling, parallel evaluation, real JavaScript/Python fixtures, and comprehensive red→promote→green behavior.

That is excellent behavioral breadth. Particularly strong practices are temporary-directory isolation, an in-memory storage contract fake, explicit tamper tests, and narrative comprehensive tests.

Coverage gaps and limitations:

- No checked-in fuzz target or property-testing framework was found despite the project’s parser guidance.
- No checked-in CI workflow was found.
- Mutation and coverage recipes exist, but the mutation recipe is incorrectly scoped and no current reports are committed.
- The critical destination-tamper overwrite path lacks a refusal test.
- Multi-signer verified-attestation order is not covered.
- CLI option placement after file positionals is not covered.
- Malformed/wrong-typed/unknown configuration is largely tested as permissive behavior rather than fail-closed behavior.
- Parallel suite-duration enforcement is explicitly unimplemented.
- The full test gate could not be run in the review environment because `cargo-nextest` was absent. Focused `zweimomo` tests also could not link because the Python 3.14 development library was absent. These are environment limitations, not passing evidence.

Quality-gate results from this review:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | passed |
| workspace clippy, all targets, warnings denied | passed after fetching locked dependencies and disabling an unusable sandbox `sccache` wrapper |
| full workspace nextest | not run: `cargo-nextest` not installed |
| focused zweimomo tests | build reached link step; failed because `libpython3.14` was unavailable |
| CLI generate/show/body/compare/promote scratch experiment | ran successfully and exposed the newline and option-ordering behaviors described above |

### 5. Dependency and supply-chain assessment

The dependency choices are generally mature for the jobs involved: `ed25519-dalek`, RustCrypto Argon2/AEAD/hash crates, serde, clap, tokio/axum, and tempfile. Pinning `boa_engine` exactly is justified because interpreter output becomes signed baseline data. Cryptographic KDF parameters and salts are explicitly pinned and domain-separated.

Risks and improvements:

- The root crate has a large dependency surface because the library, CLI, review server, post-quantum signer, and TUI backend ship together. Feature-gating or separate crates could reduce install/build/audit surface for users who only need core snapshots.
- `fips205`, axum/tokio, and review dependencies are unconditional for the published crate.
- There is no visible `cargo-deny`, `cargo-audit`, SBOM, provenance, or dependency policy automation.
- Version ranges are broad for most dependencies. `Cargo.lock` protects repository builds, but library consumers resolve compatible versions independently.
- The system-Python dependency makes `zweimomo` less portable than its “pure-Rust evaluator” description suggests.

Recommendation: define features such as `cli`, `review-server`, and `post-quantum-corpus`, or split application binaries from the minimal format/signature/testing library after measuring complexity. Add automated advisory/license/source checks.

### 6. Security-model observations

Positive:

- Actual raw signed bytes are verified.
- Signing keys are derived with explicit Argon2id parameters.
- The empty computer key is intentionally identifiable.
- The generated work area avoids mutating reviewed baselines merely to inspect new results.
- Stage stamps append to prior bytes, preserving an audit chain.
- Reviewer key prefixes can bind verification to an expected key.

Important limits to communicate more prominently:

- The stock compiled key and empty computer key are public; they provide provenance labeling, not secrecy or independent trust.
- A signature proves possession of a key, not that a thoughtful review occurred.
- A non-empty passphrase or compression score does not prove a human was present.
- Unsigned flag advisories are intentionally mutable and must never be interpreted as authenticated review statements.
- Local file permissions, atomic persistence, key acquisition, process compromise, and repository history remain part of the trust boundary.
- The binary self-hash records provenance but does not by itself establish that the binary was built from reviewed source.

### 7. Recommended improvement sequence

1. Fix the three P0 issues: refuse tampered destinations, inspect all verified stamps, and settle/remove `output → verified`.
2. Make config parsing strict and fallible; add regression tests before changing defaults.
3. Make artifact writes atomic and add fault-injection tests.
4. Fix clap positional parsing and add end-to-end exit-code tests.
5. Repair and automate the quality gates on `jia`: workspace nextest/JUnit, doctests, clippy, fmt, dependency audit, and correctly scoped mutation tests.
6. Enforce or reject duration limits consistently in parallel mode.
7. Reconcile documentation and crate docs with the four-stage/nested-flag model.
8. Add parser/config fuzzing and property tests.
9. Decompose the largest modules along existing responsibility boundaries.
10. Reduce the published dependency surface through features or crate boundaries.

### 8. Bottom line

einmo’s conceptual separation of “ran,” “accepted as a baseline,” “reviewed,” and “human-attested” is excellent, and the codebase contains many strong implementation choices. The most valuable way to learn it is to keep a scratch `zweimomo` suite open beside `Stage`, `EinmoFile`, `verify_bytes`, `EinmoCase`, and `check_suite_integrity`, then perturb one invariant at a time.

The project should currently be regarded as a promising, well-tested security-conscious tool rather than a completed high-assurance signing system. Fixing fail-open state/configuration behavior and making the automated engineering gates match the written process would yield the largest increase in trust.

## Last Updated

**Date**: 2026-09-01  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Rechecked the tutorial against the post-EIMP-01 source; added a
pre-/post-refactor migration map, clarified the distinction between CLI
generation and the library's evaluator methods, added the authoritative
library comprehensive test, and made the scratch tampering example target the
OUTPUT line rather than an arbitrary digit.

**Date**: 2026-08-25  
**Updated By**: OpenAI Codex (GPT-5)  
**Changes**: Created a code-reading and `zweimomo` experimentation tutorial, followed by a critical engineering review with verified quality-gate results and prioritized recommendations.
