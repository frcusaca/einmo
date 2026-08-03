---
eimp: 9
title: The test-tooling contract — one reliable way to run einmo's tests and read the results
author: Claude Code (Opus 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-08-01
supersedes: []
begun: [x]
---

# EIMP-9: The test-tooling contract — one reliable way to run einmo's tests and read the results

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root — **read it before creating or editing an EIMP.**

## Abstract

The `just`/nextest/mutants toolchain landed in `ac873c3` together with a
Developer Guide in `rust_instructions.md`. Driving that toolchain end-to-end
as a first-time reader surfaced twelve defects: two recipes that cannot do
what they claim (`just pr` mutates the whole codebase; `just ci-test` tests
one package), a documented toolchain pin whose file does not exist, a
documented `just` feature that does not exist, a JUnit report that silently
truncates, a test four seconds away from being killed by its own timeout, and
a failure mode in which the suite reports 158 failures on source that is
green. This EIMP records each finding with the evidence that established it,
and specifies the fix: a **two-tier contract** — a fast inner loop that stays
fast, and a strict merge gate that is actually runnable — plus the
`rust_instructions.md` corrections that make the contract legible.

Scope is the *test tooling and its documentation only*. No library behaviour
changes. Findings about einmo's own code (`src/`) belong to EIMP 8.

## Motivation

The suite is green — 394 tests pass. That is not the problem.

The problem is that a reader following `rust_instructions.md` cannot reach
that conclusion reliably. Of the eight commands the Developer Guide lists,
one is broken (`just pr`), one is silently narrower than its sibling
(`just ci-test`), and three of the guide's factual claims are false on this
tree. The one artifact the guide explicitly directs AI agents to consume —
`target/nextest/ci/junit.xml` — was, on its first real run, a report of 55
tests when the workspace has 394, with nothing in the file to say so.

Worse, the suite is *not reproducible* under ordinary use. This session
observed 158 of 356 library tests failing with
`Verification("stamp(s) failed: stage:output")`, on source identical to a
tree that passes. The cause was build state in a shared `CARGO_TARGET_DIR`,
not code. An agent that trusted that run would have opened a bug report
against `src/case.rs`. Nothing in the guide says to suspect this, and 96 of
those 158 failures were a mutex-poisoning cascade that made the real count
unrecoverable from the output.

After this EIMP: `just test` is the inner loop and stays under a minute for a
focused run; `just pr` is a gate a human or agent can actually run to
completion before merging to `jia`; the JUnit report is complete or says that
it is not; and every claim in the Developer Guide is true of this repository.

## Specification

### S.0 — Verification method

Everything in S.1 was established against the working tree at `ac873c3` on
`jia`, on 2026-08-01, with `cargo 1.97.1` / `clippy 0.1.97` /
`just 1.57.0` / `cargo-nextest 0.9.88`. Neither `just` nor `cargo-nextest`
was installed before this session; both were installed per the guide's own
Setup block.

Baseline, established last and reported first because it governs how to read
everything else:

```
$ just test --no-fail-fast
     Summary [ 345.175s] 394 tests run: 394 passed (18 slow), 0 skipped
```

**The tree is green.** Every failure discussed below is a defect in the
tooling or the documentation, not in `src/`.

### S.1 — Findings

Severity: **High** = the command cannot do what it says; **Medium** = correct
but misleading or fragile; **Low** = hygiene.

---

#### T1 — `just pr` mutation-tests the entire codebase (High)

`justfile:22`:

```make
cargo mutants --in-diff <(git diff main...HEAD) --test-tool nextest -j `nproc`
```

`main` exists but is 24 commits behind `jia`, which is this repository's
primary branch (`origin/HEAD -> origin/jia`; `AGENTS.md`, `eimp.md`, and the
EIMP skills all say plans execute directly on `jia`).

```
$ git rev-list --count main..jia
24
$ git diff main...HEAD --stat -- src/ zweimomo/src | tail -1
 17 files changed, 5421 insertions(+), 1129 deletions(-)
```

So `--in-diff` scopes the mutation run to 5,421 inserted lines across 17
source files — effectively the whole library. With a 345-second baseline
suite, `just pr` does not terminate in any useful time. The guide's "`just pr`
already scopes to your diff" is the opposite of what happens.

**Fix:** the merge base must be the branch point against `jia`, and it must
not depend on a stale local `main`:

```make
pr:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace --no-fail-fast
    cargo mutants --in-diff <(git diff $(git merge-base jia HEAD)..HEAD) --test-tool nextest -j `nproc`
```

On `jia` itself `merge-base jia HEAD` is `HEAD`, so the diff is empty and the
mutation stage is a no-op. That is correct for this repository — EIMP work
commits directly to `jia`, so there is no branch diff to mutate, and mutation
testing must be requested explicitly and scoped by hand (T2).

---

#### T2 — mutation testing has no runnable scope on `jia` (High)

Because EIMP plans commit directly to `jia` (`eimp.md`, "Plan execution"),
there is never a feature branch to diff against. T1's fix makes `just pr`
terminate, but it also means `just pr` never mutation-tests anything. The
mutation gate therefore needs an explicit, human-scoped entry point, and the
plan file is where it gets scheduled:

```make
# Mutation-test an explicit scope. Always scope it; there is no useful
# unscoped run.
mutants *args:
    cargo mutants --test-tool nextest -j `nproc` "$@"

# The uncommitted working tree — the everyday scope while a phase is in flight.
mutants-wip:
    cargo mutants --in-diff <(git diff HEAD) --test-tool nextest -j `nproc`
```

This is the mechanism behind the maintainer's proposal (see References):
cheap tests run continuously, expensive tests are **scheduled as their own
plan checkboxes** and run once, after a feature is complete. S.2 specifies
the checkboxes.

---

#### T3 — `just ci-test` tests one package; `just test` tests the workspace (High)

```make
test *args:
    cargo nextest run --workspace "$@"

ci-test *args:
    cargo nextest run --profile ci "$@"      # <- no --workspace
```

`just ci-test` therefore skips `zweimomo` entirely. The two gates disagree
about what "the tests" are, and the narrower one is the one designated for
CI and for AI agents. Observed: the JUnit report from `just ci-test` contains
a single `<testsuite name="einmo">`.

**Fix:** add `--workspace` to `ci-test`.

---

#### T4 — the JUnit report silently reports a truncated run (High)

`.config/nextest.toml` sets `fail-fast = true` on `[profile.default]`, and
`[profile.ci]` inherits it. The first real `just ci-test` run produced:

```xml
<testsuites name="nextest-run" tests="55" failures="18" errors="0" time="20.553">
```

394 tests exist. 55 ran. The console said
`warning: 332/387 tests were not run due to test failure`; **the XML did
not** — there is no attribute distinguishing "55 tests, 37 passed" from
"55 tests, and 332 we never attempted." The guide tells agents to read the
XML *instead of* the console, which is exactly the channel that drops the
warning.

**Fix:** `[profile.ci]` sets `fail-fast = false`. A report generated for
consumption must describe the whole suite; fail-fast belongs to the
interactive loop, not the artifact.

---

#### T5 — a test runs 116s against a 120s hard kill (High)

`[profile.default] slow-timeout = { period = "30s", terminate-after = 4 }`
terminates any test at 120 seconds. Measured in the green baseline run:

```
SLOW [> 90.000s] einmo suite::tests::update_corpus_signature_re_signs_when_stale
PASS [ 116.053s] einmo suite::tests::update_corpus_signature_re_signs_when_stale
```

3.3% of margin. Under any load — a parallel `cargo build`, a busier CI
runner, `cargo mutants` running `-j nproc` — this test is killed and the
suite fails for reasons unrelated to the code. Two further tests sit at
~66s. 18 of 394 tests are flagged SLOW.

**Fix (two parts):**
1. Raise `terminate-after` to `8` (240s) so the kill threshold is a runaway
   detector rather than a coin flip on the slowest legitimate test.
2. Record the cause. These three tests are `update_corpus_signature*`, and
   the cost is Argon2id key derivation (`signature.rs:35-37`, m=19456 KiB,
   t=2, p=1) repeated per corpus signature. EIMP 8 measured a single
   derivation at ~515ms in a debug build; 116 seconds is ~200 of them. The
   structural fix — deriving once per suite rather than once per signature —
   is library work and belongs to a follow-up, not here. This EIMP raises
   the ceiling and names the debt.

---

#### T6 — `rust-toolchain.toml` does not exist (Medium)

The guide's "Things that will bite you" says:

> **Don't change the toolchain.** `rust-toolchain.toml` pins it.

```
$ ls rust-toolchain.toml
ls: cannot access 'rust-toolchain.toml': No such file or directory
$ rustup show active-toolchain
stable-x86_64-unknown-linux-gnu (default)
```

There is no pin. The active toolchain is whatever `rustup default` happens to
be, which is precisely the condition that produced the 1.95-vs-1.97 clippy
divergence the guide's "Why the toolchain is pinned" section narrates. The
`[workspace.lints.clippy]` half of that remediation *does* exist
(`Cargo.toml:95-97`, added in `78111d7`) and is the more durable half — but
the guide asserts a file-level guarantee the repository does not provide.

**Fix:** add `rust-toolchain.toml` pinning the channel the tree is verified
against, or delete the claim. This EIMP specifies adding the file, because
the guide's troubleshooting table (`rustup show active-toolchain`,
"toolchain precedence, highest first") is written for a repository that has
one.

---

#### T7 — `just` does not accept recipe prefixes (Medium)

> `just` accepts unambiguous prefixes, so `just cov`, `just mut`, and
> `just ci` all work.

```
$ just --dry-run cov
error: justfile does not contain recipe `cov`
```

`just 1.57.0` has no prefix matching. All three examples fail.

**Fix:** delete the paragraph. Optionally add real aliases (`alias cov :=
coverage`), which `just` does support — but the claim as written must go
either way.

---

#### T8 — test results are not reproducible under a shared `CARGO_TARGET_DIR` (High)

`CARGO_TARGET_DIR=/yolo/target` is set in this environment and shared across
checkouts. During this session, with concurrent `cargo` invocations against
it (including `cargo install`, which honours the variable), the suite entered
a state where 158 of 356 library tests failed:

```
$ cargo test --lib
test result: FAILED. 198 passed; 158 failed; finished in 163.46s

thread 'case::tests::retract_verified_leaves_checked' panicked at src/case.rs:1098:
  called `Result::unwrap()` on an `Err` value:
  Verification("stamp(s) failed: stage:output")
```

The same single test then **passed** at `f87a97b` in a fresh worktree, and
then **passed again at HEAD** with no source change — only a rebuild between
them. Two consecutive full runs disagreed with each other (159 vs 158
failures), which is itself the signature: a deterministic suite does not
drift by one.

This is the most dangerous finding in the list, because the failure is
plausible. `stamp(s) failed: stage:output` reads exactly like a real
signing regression, and an agent that trusted it would file a bug against
code that is correct. The guide's troubleshooting table has the right cure
(`cargo clean -p einmo`) but does not say when to reach for it.

**Fix:** documentation, not code. The guide gains a rule with a trigger
condition — see S.3.

---

#### T9 — one panic poisons the rest of the run under `cargo test` (Medium)

`src/review.rs:1244` and `src/review_server.rs:1142`, both in `test_context()`:

```rust
let guard = JOURNAL_ENV_LOCK.lock().unwrap();
```

A panic while the guard is held poisons the mutex for the remainder of the
process. In the 158-failure run, **96 of the failures were `PoisonError`** —
not distinct defects, but the cascade from a handful of real ones. The
true failure count was unrecoverable from the output.

`nextest` is immune by construction: one process per test, so poison cannot
propagate. That is an argument for the guide's "use `just test`, not
`cargo test`" rule, and the guide should make it, because the rule currently
reads as a tooling preference rather than a correctness property.

**Fix:** `unwrap_or_else(std::sync::PoisonError::into_inner)` at both sites —
the guarded value is `()`, so there is no invariant a poisoning panic could
have broken. Cheap, and it makes `cargo test` output honest for anyone who
runs it anyway.

---

#### T10 — JUnit failure detail is in `<system-err>`, not `<failure>` (Medium)

The guide points agents at the XML without saying how to read it. nextest
emits:

```xml
<failure type="test failure"/>
<system-out>running 1 test ... test result: FAILED.</system-out>
<system-err>thread '...' panicked at src/case.rs:1098:18:
called `Result::unwrap()` on an `Err` value: Verification("stamp(s) failed: stage:output")</system-err>
```

`<failure>` carries no `message` attribute and no body. A consumer that reads
`failure/@message` — the JUnit convention — gets nothing. This cost real time
in this session.

**Fix:** document the shape, with a copy-pasteable extractor (S.3).

---

#### T11 — two divergent bootstrap paths (Low)

`just setup` installs pinned versions (`cargo-mutants 25.0.0`,
`cargo-llvm-cov 0.6.16`, `cargo-nextest 0.9.88`). `install-dev-tools.sh`
installs the same three unpinned, plus `just`. Neither references the other;
the guide's Setup block calls `just setup` and never mentions the script.

**Fix:** `install-dev-tools.sh` becomes the bootstrap (it can install `just`,
which `just setup` cannot), and delegates to `just setup` for the pinned
tools. One list of versions.

---

#### T12 — `mutants.out/` is tracked; `.gitignore` covers only `mutants.out.*` (Low)

```
$ git ls-files mutants.out | head -1
mutants.out/caught.txt
```

`587d22b` ("ignore mutant outputs") changed `mutants.out` to `mutants.out.*`,
so the bare directory is no longer ignored — and it is committed
(`78111d7`). The guide sends readers to `mutants.out/missed.txt` for "the
useful artifacts" while a stale committed copy sits at that exact path.

**Fix:** `git rm -r --cached mutants.out mutants.out.old`; `.gitignore` gets
both `mutants.out` and `mutants.out.*`.

---

### S.2 — The two-tier contract

The maintainer's framing, adopted here: *normal unit tests during
development; coverage and mutation testing queued as separate plan
checkboxes, run after feature development is complete.*

| Tier | When | Command | Cost | Blocking? |
|---|---|---|---|---|
| **Inner loop** | every edit | `just test <filter>` | seconds–1 min | yes, always |
| **Phase gate** | end of each plan phase, before commit | `just` (fmt + lint + test) | ~6 min | yes |
| **Merge gate** | once, before an EIMP is marked complete | `just pr` + the scheduled expensive checkboxes | tens of minutes | yes |

The inner loop is `just test <substring>`. Verified:

```
$ just test crash_crumb_survives_stack_overflow
    Starting 2 tests across 7 binaries (392 tests skipped)
        PASS [   9.587s] einmo einmo_suite::tests::crash_crumb_survives_stack_overflow
        PASS [  11.578s] zweimomo::suites crash_crumb_survives_stack_overflow
     Summary [  11.580s] 2 tests run: 2 passed, 392 skipped
```

This works today and needs no change; it needs to be *documented*, because
the guide shows `just test verify` only in passing and never says that this
is the intended development loop.

**Plan-file consequence.** Every EIMP plan gains an explicit pre-completion
block, so the expensive gates are scheduled rather than skipped:

```markdown
- [ ] Write and verify the EIMP-<NUMBER> comprehensive test(s)
- [ ] Fast gate green: `just` (fmt + clippy + `cargo nextest run --workspace`)
- [ ] Full gate green: `just ci-test`, and `target/nextest/ci/junit.xml`
      reports 0 failures over the whole workspace
- [ ] Mutation gate: `just mutants --file <the files this EIMP touched>`;
      record survivors in the plan and either kill them or justify each
- [ ] Coverage checked: `just coverage`; note any line this EIMP added that
      no test reaches
- [ ] Update EIMP-<NUMBER>.md frontmatter `status: complete`
```

The four gate lines replace the current skeleton's single
"All tests pass: `cargo test`, `cargo clippy …`, `cargo fmt --check`" —
which names `cargo test`, the runner the guide tells readers not to use.

### S.3 — `rust_instructions.md` changes

1. **Delete** the prefix-matching paragraph (T7).
2. **Correct** the toolchain claim (T6) — either after `rust-toolchain.toml`
   is added, or by replacing "`rust-toolchain.toml` pins it" with the truth:
   the lints are pinned in `Cargo.toml`, the toolchain is not.
3. **Rewrite** "Use `just test`, not `cargo test`" to give the reason (T9):
   nextest runs one process per test, so a panic cannot poison a shared
   mutex and cascade into dozens of false failures — which is what
   `cargo test` does on this suite.
4. **Add** a "When results look impossible" rule (T8):

   > A cargo target directory shared between checkouts or written by
   > concurrent `cargo` invocations produces failures that look like real
   > bugs — this suite has produced 158 signature-verification failures on
   > source that is green. Before believing a surprising failure: run it
   > alone (`just test <name>`); if it disagrees with the full run, or two
   > full runs disagree with each other, `cargo clean -p einmo` and re-run
   > before reporting anything. Never run two `cargo` commands against the
   > same `CARGO_TARGET_DIR` at once — including `cargo install`, which
   > honours it.

5. **Add** to "Machine-readable output" (T10, T4):

   > `<failure>` carries no message; the panic is in `<system-err>`. And
   > check the counts against the workspace total before reading anything
   > else — a fail-fast run produces a well-formed XML file describing only
   > the tests that ran.

   ```bash
   python3 - <<'EOF'
   import xml.etree.ElementTree as ET
   r = ET.parse('target/nextest/ci/junit.xml').getroot()
   print(f"{r.get('tests')} run, {r.get('failures')} failed")   # expect 394 run
   for tc in r.iter('testcase'):
       if tc.find('failure') is not None:
           err = tc.findtext('system-err', '').strip()
           print(f"\n=== {tc.get('classname')}::{tc.get('name')}\n{err}")
   EOF
   ```

6. **Add** the two-tier table from S.2, and state the inner loop explicitly.
7. **Correct** "`just pr` already scopes to your diff" (T1/T2).
8. **State the suite's cost** — 394 tests, ~345s, 18 SLOW, dominated by
   Argon2id — so a reader knows a six-minute run is expected, not hung.

## Test Plan

The subject under test is the tooling, so verification is by execution, and
each fix is checked by the command it repairs:

- **T1/T2** — `just pr` on a clean `jia` completes; its mutation stage
  reports an empty scope rather than 17 files. `just mutants --file
  src/verify.rs` completes and writes `mutants.out/outcomes.json`.
- **T3/T4** — after `just ci-test`, `target/nextest/ci/junit.xml` has
  `tests="394"` and at least one `<testsuite name="zweimomo">`. Assert the
  count mechanically with the S.3 snippet, not by eye.
- **T5** — `just test update_corpus_signature` passes with no SLOW line
  exceeding the new `terminate-after`; the measured 116s figure is re-taken
  and recorded in the plan.
- **T6** — `rustup show active-toolchain` names the pinned channel, and
  `cargo clippy --version` matches on a second machine or a
  `RUSTUP_TOOLCHAIN`-cleared shell.
- **T7** — `just --dry-run cov` either resolves (if aliases were added) or
  the claim is gone from the document; `grep -n "unambiguous prefixes"
  rust_instructions.md` is empty.
- **T8** — not mechanically testable; verified by the doc rule existing and
  by the plan recording the reproduction already captured in S.1.
- **T9** — `cargo test --lib` after injecting a deliberate panic into one
  `test_context()` consumer produces exactly one failure, not a cascade.
  Revert the injection.
- **T10** — the S.3 extractor, run against a deliberately failing test,
  prints the panic text.
- **T11/T12** — `bash install-dev-tools.sh` on a clean shell yields the
  pinned versions; `git ls-files mutants.out` is empty and
  `git check-ignore -v mutants.out` matches a rule.

No new library tests. This EIMP touches `justfile`, `.config/nextest.toml`,
`.gitignore`, `install-dev-tools.sh`, `rust_instructions.md`, `eimp.md`, and
two `.unwrap()` call sites in test modules.

## Rejected Alternatives

### A. Do nothing — the suite is green and the tooling mostly works

"Mostly" is doing the work in that sentence. The three High findings are not
cosmetic: `just pr` is the designated merge gate and cannot finish;
`just ci-test` is the designated CI gate and tests two thirds of the
workspace; and the JUnit file the guide points agents at reported 55 of 394
tests with no indication of truncation. A gate that cannot be run is worse
than no gate, because the plan checkbox next to it still gets ticked.

### B. Drop `just`/nextest and go back to `cargo test`

Rejected on T9's evidence. `cargo test` shares one process across a test
binary, so a single panic inside `test_context()` poisoned `JOURNAL_ENV_LOCK`
and turned a handful of failures into 96 more. nextest's process-per-test
isolation is not a preference here; it is what makes the failure count
readable. The tooling is right; its configuration is wrong.

### C. Fix the tooling but leave `rust_instructions.md` alone

The document is the interface. A reader who trusts "just accepts unambiguous
prefixes" and "`rust-toolchain.toml` pins it" has been told two false things
about a tree they are about to change, and the second one directly concerns
reproducibility — the subject the surrounding section exists to defend.
Correct tooling described incorrectly still costs the next reader the same
hour it cost this session.

### D. Fold this into EIMP 8

EIMP 8 is a code review of `src/` with 41 tracked findings and its own
execution priority. These findings are about the harness, they were found by
a different method (driving the tools, not reading the code), and they block
EIMP 8's own completion checkboxes — `just pr` is what EIMP 8's plan will
eventually have to run. Separate document, executed first.

## Open Questions

- ~~Which channel should `rust-toolchain.toml` pin — `1.97.1` exactly, or
  `stable` with a `rust-version` floor in `Cargo.toml`?~~ **Half-resolved
  2026-08-01.** The floor landed: `[workspace.package] rust-version = "1.88"`,
  inherited by both members. 1.88 is not the edition requirement (edition 2024
  needs 1.85) but the maximum `rust-version` across the resolved graph —
  `time 0.3.54` (direct) and the `boa_*` crates. Verified: a too-old toolchain
  now fails with `error: rustc 1.97.1 is not supported by the following
  packages: einmo@0.0.5 requires rustc 1.99` rather than a wall of syntax
  errors. Note the manifest form — `rust-version` is a genuine `[package]`
  field, so `rust-version.workspace = true` goes *inside* `[package]`, the
  opposite of `lints` (EIMP 8 §S.0).
  **Still open:** whether to add `rust-toolchain.toml` on top. The floor does
  not give clippy reproducibility across machines; only a pin does. Against
  it: `[workspace.lints.clippy]` already names the lints that matter, so a pin
  buys timing control over the next upstream regrouping rather than immunity
  to it — at the cost of a recurring bump commit. T6's Phase B fix is written
  for the exact pin; the maintainer's call, and "no pin" is now a defensible
  answer.
- Should `[profile.default]` keep `fail-fast = true`? It suits the inner
  loop, but it means `just test` and `just ci-test` disagree about
  completeness as well as scope. Alternative: `fail-fast` only on an
  explicit `just quick` recipe.
- T5 raises the timeout ceiling but leaves ~250 seconds of Argon2id in the
  suite. Is a follow-up EIMP wanted for per-suite key-derivation caching, or
  is the cost accepted as the price of real signatures in tests?

## References

- Prior EIMPs: EIMP 8 §S.0 (toolchain status, the `[workspace.lints.clippy]`
  remediation, and the Argon2id ~515ms measurement); EIMP 0 and `eimp.md`
  ("Plan execution" — work commits directly to `jia`, which is why T2
  exists); EIMP 7 (the 394-test baseline this EIMP measures against).
- Maintainer direction, 2026-08-01: "we can still use normal unit tests /
  einmo test during development, and then queue up coverage/mutant/expensive
  tests in the plan instructions to have separate checkboxes for each of the
  more extensive tests, to be run after feature development is complete" —
  specified in §S.2.
- Code and config: `justfile`; `.config/nextest.toml`; `.cargo/config.toml`;
  `.gitignore`; `install-dev-tools.sh`; `rust_instructions.md` §Developer
  Guide; `Cargo.toml:95-97` (`[workspace.lints.clippy]`);
  `src/review.rs:1244`, `src/review_server.rs:1142` (poisoning);
  `src/signature.rs:35-37` (Argon2id parameters);
  `src/suite.rs` (`update_corpus_signature*`, the 116s tests).
- External: `cargo-nextest` configuration reference (`fail-fast`,
  `slow-timeout`, `[profile.*.junit]`); `cargo-mutants` `--in-diff`.

## Last Updated

**Date**: 2026-08-01 (2)
**Updated By**: Claude Code (Opus 5)
**Changes**: T6 half-resolved — `[workspace.package] rust-version = "1.88"`
declared and inherited by both members; first Open Question narrowed to
whether a `rust-toolchain.toml` is wanted on top of the floor.

**Date**: 2026-08-01
**Updated By**: Claude Code (Opus 5)
**Changes**: Created. Twelve findings (T1–T12) from driving the `ac873c3`
tooling end-to-end, the two-tier test contract, and the
`rust_instructions.md` corrections.
