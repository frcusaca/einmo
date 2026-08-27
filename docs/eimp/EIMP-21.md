---
eimp: D12
title: Paired Python and JavaScript Project Euler answer suites
author: OpenAI Codex (GPT-5) <noreply@openai.com>
status: Draft
type: Standards
created: 2026-08-27
supersedes: []
begun: [ ]
---

# EIMP-21: Paired Python and JavaScript Project Euler answer suites

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root — **read it before creating or editing an EIMP.**

## Abstract

EIMP 21 adds a progressively extensible Project Euler corpus to `zweimomo`.
Each admitted problem has independent Python and JavaScript implementations;
each exposes `solve()` and returns the canonical answer as a string. Tests run
both implementations, require identical single-string results, and drive the
real einmo generation pipeline so the returned answer is written into a
`generated/` artifact and compared with reviewed `output/` and `checked/`
baselines. This EIMP establishes the harness and completes Problems 1–10 in
numeric order. Later EIMPs may admit additional finite ranges using the same
contract.

## Motivation

`zweimomo` already proves that einmo evaluates Python and JavaScript expressions,
writes signed artifacts, and checks reviewed baselines. Its current cases are
small language/evaluator examples. Project Euler provides a natural progression
of deterministic, computation-heavy programs that exercise larger source files,
integer algorithms, performance variation, paired language implementations,
and long-term output stability.

The corpus must test einmo rather than become an answer lookup table maintained
by hand. The implementation computes an answer and returns one string; the EIMP
test invokes the implementation and causes einmo to write that returned string.
Committed answer artifacts are accepted only through normal generation,
inspection, and promotion. A test must never embed a second literal copy of the
expected answer in Rust, because two copied literals can agree while both
language implementations are broken or stale.

The scope is finite. Project Euler currently has a growing archive, so “all
problems” cannot be a stable completion criterion. EIMP 21 establishes the
mechanism and paired solutions for Problems 1–10. Later numbered ranges require
new EIMPs so cost, dependencies, licensing, performance, and spoiler policy are
reviewed rather than silently expanding an eternal plan.

## Impact Overview

### Einmo Library User Experience changes

No einmo public API changes are required. `zweimomo` gains internal Euler-aware
Python and JavaScript evaluators that still implement the existing
`einmo::Evaluator` contract and return exactly one `String` output chunk. The
generic expression evaluators remain available and keep their current behavior.

### Einmo Integration changes

`zweimomo` gains parallel `suites/euler/python` and `suites/euler/javascript`
source trees, a checked-in problem manifest, paired unit tests, and an einmo
integration test that writes answers into scratch `generated/` trees and checks
them against committed language baselines. CI runs a fast Problems 1–10 target
on every change and reports the problem id and language on disagreement,
non-string output, evaluator failure, timeout, or baseline drift. Documentation
links to Project Euler rather than copying full problem statements.

### Einmo Development changes

Developers must understand four new rules: every admitted problem is a paired
Python/JavaScript unit; `solve()` returns a canonical string; Rust tests do not
contain answer literals; and signed baselines change only through einmo CLI
generation/inspection/promotion. Tests cover each language separately,
cross-language equality, manifest completeness, deterministic repeatability,
the single-string contract, and the real generated-to-output comparison.

The unfamiliar aspect is that correctness has two independent oracles: the two
implementations must agree, and their generated artifacts must match the
reviewed baseline. Agreement alone is insufficient because both implementations
could share a mistaken algorithm; baseline promotion therefore requires a human
to check the answer against Project Euler or independently derived reasoning.

### Migration

None. This adds a new `zweimomo` corpus and internal evaluators without changing
existing suites, evaluator behavior, signed artifacts, or public einmo APIs.

## Specification

### S.0 — Scope and admission order

EIMP 21 admits Project Euler Problems 1 through 10, strictly in numeric order.
A problem is admitted only when Python, JavaScript, unit tests, generated
artifacts, and reviewed baselines for that problem land together. A later
problem may be researched while an earlier problem is under review, but it may
not be marked admitted first.

The source statement is not copied into the repository. Each implementation
contains attribution and the canonical link
`https://projecteuler.net/problem=<id>`. Project Euler states that its main
problem content is CC BY-NC-SA 4.0 and supplies that direct-link form; avoiding
copied statements also keeps this source-focused corpus small. Solutions and
answer artifacts are spoilers and documentation labels them as such.

### S.1 — Repository layout and manifest

The layout is:

```text
zweimomo/
  suites/euler/
    manifest.toml
    README.md
    python/
      input/problem_001.py
      ...
      output/problem_001.py.einmo
      checked/problem_001.py.einmo
    javascript/
      input/problem_001.js
      ...
      output/problem_001.js.einmo
      checked/problem_001.js.einmo
```

`manifest.toml` has one ordered record per admitted problem:

```toml
[[problem]]
id = 1
python = "problem_001.py"
javascript = "problem_001.js"
source = "https://projecteuler.net/problem=1"
```

The manifest contains no answer. IDs are unique, ascending, and contiguous for
this EIMP. Paths must exist and match the zero-padded naming rule. Adding only
one language, one baseline stage, or an unlisted source file fails tests.

#### Impact descriptors

- **Library User Experience:** no public API change; manifest parsing is
  `zweimomo`-internal.
- **Integration:** CI and local filtered tests discover cases from the manifest,
  not directory glob ordering.
- **Development:** a new problem begins by adding the manifest row and paired
  test names, which initially fail before implementations are written.
- **Migration:** None.

### S.2 — Paired solution contract

Every Python file defines a zero-argument `solve()` and every JavaScript file
defines a zero-argument `solve`. Calling it returns a language string containing
only the canonical answer: no label, explanation, whitespace padding, logging,
or multiple output chunks. Decimal integer answers use ordinary base-10 digits;
other answer shapes use Project Euler's submitted textual form.

Conceptually:

```python
def solve() -> str:
    answer = compute_without_answer_literal()
    return str(answer)
```

```javascript
function solve() {
    const answer = computeWithoutAnswerLiteral();
    return String(answer);
}
```

The answer must be computed. Returning the known answer literal, reading it from
the committed `.einmo` baseline, making a network request, or sharing generated
solution logic between languages is prohibited. Problem-specific data published
as part of a Project Euler problem may be vendored only with attribution and
license notice, or generated deterministically when practical.

Each implementation is deterministic and has no network access, environment
dependency, current-time dependency, random seed dependency, or writes outside
einmo's generated scratch tree. It should favor a clear mathematical algorithm;
micro-optimized or code-golf solutions require an explanatory comment.

#### Impact descriptors

- **Library User Experience:** the paired adapters return `Ok(vec![answer])`
  through the existing `Evaluator` trait.
- **Integration:** logging to stdout-equivalent result channels is forbidden
  because it changes signed output; evaluator errors name language and problem.
- **Development:** reviewers inspect algorithm independence, determinism, answer
  formatting, and absence of literal/baseline/network shortcuts. Per-problem
  performance budgets are recorded if the default suite deadline is insufficient.
- **Migration:** None.

### S.3 — Euler evaluators

`zweimomo/src/evaluators.rs` gains internal adapters or helpers that execute a
normal multi-line module and invoke `solve()`:

```rust
struct EulerPyo3Evaluator;
struct EulerBoaEvaluator;
```

The Python adapter executes the module in a fresh namespace, obtains `solve`,
calls it without arguments, and rejects non-`str` results. The JavaScript
adapter evaluates the module in a fresh Boa context, obtains/calls `solve`, and
rejects non-string results. Both return exactly `vec![answer]` and attach the
language/problem identifier to errors without including the expected answer.

These are separate from `Pyo3Evaluator` and `BoaEvaluator`; existing expression
semantics and tests do not change. The adapters expose no filesystem or network
capability beyond what their current runtimes inherently provide; solution
review and tests enforce the no-I/O contract.

#### Impact descriptors

- **Library User Experience:** no public einmo surface changes; the new adapters
  may remain crate-private to `zweimomo` tests.
- **Integration:** Python development headers and Boa remain existing
  prerequisites. CI verifies missing/wrong/non-callable/throwing `solve` and
  non-string return diagnostics in both languages.
- **Development:** contributors must learn the difference between generic
  expression evaluators and module-style Euler adapters, fresh-runtime isolation,
  and why implicit `str()`/`String()` coercion occurs inside solutions rather
  than adapters.
- **Migration:** None.

### S.4 — Unit, paired, and einmo-written-answer tests

For each problem `<NNN>`, tests are named:

```text
euler_problem_<NNN>_python_returns_string
euler_problem_<NNN>_javascript_returns_string
euler_problem_<NNN>_languages_agree
euler_problem_<NNN>_einmo_writes_answer
```

The language tests execute `solve()` and assert exactly one non-empty string,
canonical formatting, determinism across two fresh runtimes, and no evaluator
error. The paired test compares Python and JavaScript strings directly. It does
not contain the answer literal.

The einmo-written-answer test copies the two language suites to scratch space,
runs `EinmoTestRunner::evaluate_all` with the correct adapter, and asserts:

1. one generated artifact is written for the problem in each language;
2. its OUTPUT section contains exactly the returned answer string;
3. generated artifacts verify cryptographically;
4. `generated ↔ output` agrees for each language;
5. committed `output ↔ checked` agrees;
6. Python and JavaScript generated OUTPUT sections are identical.

The test writes only scratch `generated/`; it never modifies repository
`output/` or `checked/`. Initial baselines are produced explicitly with the CLI:
generate, inspect with `compare`/`show`/`body`, promote generated to output after
sanity checking, then promote output to checked only after independently
reviewing correctness. Agent work does not promote checked to verified.

#### Impact descriptors

- **Library User Experience:** no API change; this is executable coverage of the
  existing evaluator, runner, envelope, compare, and stage contracts.
- **Integration:** each admitted problem adds four filterable tests plus the
  aggregate manifest/suite tests; CI failures name problem and language.
- **Development:** developers must not “fix” a mismatch by editing signed
  artifacts or copying the expected answer into Rust. They inspect generated
  output, correct algorithms, and promote through CLI gates.
- **Migration:** None.

### S.5 — Progressive problem workflow

Each problem follows one repeatable vertical slice:

1. read/link the official problem and record any licensed data dependency;
2. write the four named tests and manifest entry first;
3. implement Python `solve() -> str`;
4. independently implement JavaScript `solve() -> string`;
5. run language and equality tests;
6. run einmo generation in scratch and inspect the returned answer;
7. independently validate the mathematical answer;
8. create/promote output and checked baselines through einmo CLI;
9. run the focused subset, aggregate Euler suite, then full repository gate;
10. commit the complete paired problem slice.

Problems may use shared language-specific mathematical helpers only after at
least two admitted problems demonstrate the abstraction. Helpers cannot encode
problem answers, cross the Python/JavaScript independence boundary, or obscure
the primary algorithm.

### S.6 — Publication and responsible-use boundary

The repository links and attributes Project Euler. It does not copy bonus
content, member statistics, forum posts, or third-party solutions. The main
problem text, if ever vendored, must follow Project Euler's CC BY-NC-SA 4.0
terms; this EIMP instead links to it.

Solutions and checked answer artifacts reveal answers. `zweimomo/suites/euler`
therefore starts with a prominent spoiler warning. The corpus is an einmo
engineering fixture, not an account-submission service: it does not log in,
submit answers, scrape private content, or claim Project Euler credit. EIMP 21
is limited to the long-established first ten problems. Expansion requires a
finite-range EIMP and a fresh check of Project Euler policy.

## Test Plan

- Evaluator contract tests for missing, non-callable, throwing, and non-string
  `solve` in Python and JavaScript.
- Manifest tests for ordering, contiguity, filenames, paired sources, links, and
  complete output/checked baselines.
- Four named tests per Problem 001–010 as specified in §S.4.
- Aggregate `euler_python_suite_generates_and_matches`,
  `euler_javascript_suite_generates_and_matches`, and
  `euler_languages_match_for_every_manifest_problem` tests.
- Negative tests proving a one-language-only problem, answer literal in the
  manifest, extra output chunks, non-string result, nondeterminism, or baseline
  drift fails with the problem/language identified.
- EIMP 21 comprehensive test running all ten paired solutions through fresh
  evaluators and scratch einmo generation, followed by the full `just` gate.

## Rejected Alternatives

### A. Store answers directly in Rust tests

Rejected. It duplicates the answer and lets a hard-coded implementation agree
with a hard-coded test without exercising einmo's returned-output path.

### B. Implement one language first and port it mechanically

Rejected. Paired admission prevents permanent asymmetry, and independent
algorithms provide a stronger oracle than transliteration.

### C. Keep using expression-only evaluators

Rejected. It makes realistic Python solutions contort into expressions and
encourages unreadable JavaScript final-expression tricks. Explicit `solve()`
modules scale while preserving one returned string.

### D. Copy every Project Euler statement into the repository

Rejected. Direct attributed links are sufficient, avoid licensing duplication,
and keep the corpus focused on evaluator behavior and solutions.

### E. Make this EIMP cover the entire growing archive

Rejected. The archive is not finite and later problems can require large data,
long runtimes, specialized mathematics, or revised publication policy. Finite
ranges produce reviewable plans and honest completion.

### F. Let tests write repository baselines automatically

Rejected. Tests write scratch generated artifacts. Automatically replacing
committed output/checked would bypass einmo's deliberate sanity and correctness
review claims.

## Open Questions

None. Later ranges decide their own size, performance budgets, and data needs.

## References

- [Project Euler problem archive](https://projecteuler.net/archives) — the
  archive currently lists a growing problem set.
- [Project Euler copyright information](https://projecteuler.net/copyright) —
  main problem content license and preferred direct-link form.
- [Project Euler generative-AI policy](https://projecteuler.net/genai) —
  responsible-use context; this corpus does not submit answers or post material.
- Prior EIMPs: EIMP 2 (`zweimomo` and real evaluators), EIMP 01 (generated
  stage and adjacent gates), EIMP 11 (persistence and test hardening).
- Code locations: `zweimomo/src/evaluators.rs`, `zweimomo/tests/suites.rs`,
  `zweimomo/suites/python/`, `zweimomo/suites/javascript/`.

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Created EIMP 21 specifying paired Python and JavaScript
`solve() -> string` implementations for Project Euler Problems 1–10, with
manifest-driven tests that write returned answers through einmo generation.
