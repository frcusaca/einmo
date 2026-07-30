---
eimp: 5
title: Merkle-tree corpus signing — faster to compute, cheaper to update
author: Claude Code (Opus 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-30
supersedes: []
begun: [ ]
---

# EIMP-5: Merkle-tree corpus signing — faster to compute, cheaper to update

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

## Abstract

Replace `CorpusSigner`'s section digest (`EIMP-1` §S.11) — a monolithic
byte-join: concatenate every file's bytes in manifest order, hash the whole
message — with a **Merkle tree**: digest each file independently into a
leaf, fold the leaves upward, and sign the root. The restructuring and the
parallel machinery that exploits it are one EIMP because they are one
purpose: making corpus signing **faster to compute** (independent per-file
work with an associative combine parallelizes trivially, where a byte-join
does not) and **cheaper to update** (a changed file recomputes its own leaf
and the `O(log n)` nodes above it, where a byte-join re-reads every byte of
every file). The tree's shape is pinned by the filename collation `EIMP-1`
§S.11a already specifies, which this EIMP inherits unchanged.

`EIMP-1` ships the byte-join, single-threaded, first — it is correct, it is
sufficient at current corpus sizes, and it is the baseline this EIMP's
benefit is measured against.

## Motivation

`EIMP-1` §S.11's byte-join is correct and deterministic. Its costs are
structural, and both grow with corpus size:

**1. It is expensive to compute, and awkward to parallelize.** The only way
to parallelize a byte-join is the two-pass metadata→offsets→disjoint-slice
scheme `EIMP-1` §S.11 originally sketched: stat every file to compute its
offset in one large buffer, allocate the buffer, then have workers
`read_exact` into disjoint sub-slices. It works, but the two passes can
disagree — a file that changes size between the stat and the read is a
correctness hazard the code must actively defend against — and it couples
read parallelism to one fixed buffer layout. A Merkle fold has no shared
buffer, no offsets, and no such race: each worker digests one file into one
leaf, and the combine is associative, so workers can finish in any order.

**2. Every re-sign is a full re-read.** Changing a single file invalidates
the entire section digest, so re-signing reads every byte of every file
again. On a large corpus this makes re-signing the kind of multi-minute
operation people stop running — and an integrity mechanism that is not run
provides no integrity. With a tree, a changed file recomputes its leaf and
its ancestors; every untouched subtree's digest is reused.

**A third benefit, not a cost of the byte-join but a capability it cannot
offer:** a tree mismatch is *walkable*. Descend into whichever child's
digest disagrees and arrive at the specific file. A monolithic hash can only
report "something under this section changed." For a mechanism whose job is
telling a human what went wrong, that is a difference in kind, not degree.

**Why this is one EIMP and not two.** An earlier draft split the
restructuring (`EIMP-6`) from the parallel machinery (`EIMP-5`). Merged on
2026-07-30: the restructuring's *purpose* is the speed and the cheap
updates, so separating them would leave a first EIMP that changes the digest
format and delivers no measurable benefit, followed by a second that
delivers all of it. They are one change.

## Specification

### S.1 Ordering — inherited, not redefined

Leaf order is `EIMP-1` §S.11a's configurable `Collation`, unchanged —
defaulting to `PathBytes` (mirror-relative paths, compared component-wise,
byte-wise within each component, no locale, no normalization, no case
folding, ties a hard error), with the chosen collation's identifier recorded
in `.section.sig` so a verifier never mistakes a configuration difference
for tampering.

This matters more here than it did for the byte-join. A tree's *shape* —
which leaves are siblings, hence which internal nodes exist — is a function
of the ordering, and the root is a function of the shape. Because §S.11a
already fixes ordering as a recorded, signed parameter computed from paths
alone, tree shape is reproducible across machines and filesystems for free,
and a suite that configures a non-default collation gets a correspondingly
different — but well-defined and self-describing — tree.

### S.1a Collation conformance: the permutation-invariance test (normative)

`EIMP-1` ships one sensible collation (`PathBytes`) and no tie-detection
harness, because `PathBytes` compares raw bytes with no folding and
therefore *cannot* tie among distinct paths. That changes the moment a
**lossy** collation becomes possible — case-folding, Unicode-normalizing,
or any ordering built on a key that discards information. `EIMP-1` §S.11a
item 5 makes ties a hard error in every collation; this section supplies
the test that can actually catch a violation, and makes passing it
**normative for every `Collation` variant**, present and future.

```rust
// `sort_by` is STABLE (not `sort_unstable_by`) -- the stability is what
// makes this a tie detector rather than a coin flip.
let mut forward = alphabet.to_vec();
forward.sort_by(|a, b| collation.cmp(a, b));

let mut reversed: Vec<_> = alphabet.iter().rev().cloned().collect();
reversed.sort_by(|a, b| collation.cmp(a, b));

assert_eq!(forward, reversed);
```

**Why it works.** A stable sort preserves the input order of elements the
comparator calls `Equal`. So if the collation is a genuine total order over
the alphabet — no ties among distinct elements — every permutation of the
input sorts to the same unique output, and forward and reversed agree. The
instant the comparator returns `Equal` for two *distinct* elements, those
two land in input order, which the reversal flips, and the assertion fails.
One assertion catches the entire class of two-way folding bugs without
enumerating pairs.

Two properties not to weaken:

- **The sort must be stable.** With `sort_unstable_by`, tied elements land
  in an arbitrary order, so the test would fail *sometimes* — a flaky test
  is worse than none, and would likely be "fixed" by deletion.
- **It only catches ties among the elements actually present.** The
  alphabet must *contain* the pairs that would collide under a lossy
  collation: `'a'` and `'A'` for case folding, and the NFC and NFD
  encodings of one grapheme for normalization. An ASCII-lowercase-only
  alphabet would pass under a case-folding collation and prove nothing.

Written as a **reusable harness every `Collation` variant is run through**,
not a one-off — its value is constraining collations that do not exist yet.
It complements rather than replaces §S.11a item 5's runtime error: the test
proves totality over a representative alphabet, the runtime error catches
what a real corpus produces that the alphabet did not anticipate.

**Why this belongs here rather than in `EIMP-1`.** Ordering determines the
byte-join's digest, but it determines a tree's *shape* — and a collation
that ties would make the shape depend on discovery order, silently, in a
way the byte-join would merely reorder. The stakes rise with this EIMP, so
the harness lands with it.

### S.2 Tree construction

- **Leaf digest**: `H(leaf_domain || mirror_path_bytes || file_bytes)` —
  the file's whole signed envelope as it sits on disk (matching `EIMP-1`
  §S.11's "the whole artifact, not just its body"). Binding the path into
  the leaf means renaming a file changes the digest even when its bytes are
  identical, which is correct for a corpus whose paths are meaningful.
- **Internal node**: `H(node_domain || left_digest || right_digest)`.
- **Domain separation is mandatory.** Leaf and internal hashes use distinct
  domain prefixes, so no internal node's digest can be presented as a leaf
  digest or vice versa — the classic second-preimage attack on naive Merkle
  constructions.
- **Odd-node rule** must be specified exactly and pinned by fixture. The two
  common choices — promote the unpaired node unchanged, or duplicate it and
  pair it with itself — produce different roots, and duplication has known
  second-preimage subtleties. An Open Question below, not guessed here.
- **Empty section** has a specified root (a constant derived from the domain
  prefix — not a zero digest, not an error), so signing an empty stage is
  well-defined.
- **The manifest header still binds in**: stage name and parameter-set id
  are folded into the root, so a `checked/` tree cannot be replayed as a
  `verified/` signature.

### S.3 Parallel computation

Leaf digests are independent, so they parallelize with no shared mutable
state and no offset choreography. Constraints:

1. **Determinism is structural.** The digest must not depend on worker
   count, scheduling, or completion order — the collation fixes leaf order
   and the tree shape fixes the fold, so parallelism cannot affect the
   result. Assert it with a test across varying worker counts rather than
   relying on the argument.
2. **Core stays lean.** Core `einmo`'s dependency-tree assertion (`EIMP-4`
   §Test Plan) must still pass — `EIMP-4` split the crate specifically to
   keep an HTTP stack and an async runtime out of it. That admits a small
   hand-rolled `std::thread` pool (no new dependency), or a feature-gated
   `rayon`, but not an unconditional heavyweight runtime. Open Question.
3. **Bounded parallelism.** A pathologically large corpus must not spawn
   unbounded workers; the count is capped and configurable.
4. **Failure is loud.** A short read, a long read, or a file changing
   underneath the signer aborts the signature — never a silently truncated
   or partially-read digest.

### S.4 Incremental re-signing

The payoff for "cheaper to update": cache leaf digests keyed by path plus a
cheap change-indicator, so re-signing recomputes only changed leaves and
their ancestors.

**The change-indicator is the entire correctness question.** Keying on
mtime/size is fast and *wrong under adversarial conditions* — a tampered
file can preserve both. Keying on content hash is correct but requires
reading the file, which is the cost the cache exists to avoid. The
resolution must be explicit, and the safe default is that **verification
never trusts the cache**: `verify` always recomputes leaves from bytes,
while `sign` may use the cache to skip work on files the local process
just wrote. An optimization that can be induced to sign stale content is a
vulnerability, not a speedup.

### S.5 This changes the digest — a format break

Any corpus signed under the byte-join has a root this construction will not
reproduce. `.section.sig` files are not forward-compatible across this
change. Two mitigations:

1. **`EIMP-1` §S.11 explicitly writes `.section.sig` only to fixtures and
   tempdirs**, never a real corpus — deliberately deferred as "a later
   step." So there should be no real signed sections to migrate when this
   lands. Verify that assumption rather than assuming it.
2. **The signature file records its construction** (alongside the
   parameter-set id), so a verifier reading an old file fails with
   "unknown/obsolete construction" rather than "signature mismatch." A
   wrong-algorithm error and a tampered-corpus error must never look alike.

## Test Plan

- **Benchmark first, and again after** (see the plan's Phase A): serial
  byte-join vs. the tree, on a corpus large enough for the difference to be
  real. Both the *estimate* and the *achieved* number are recorded. An
  optimization that is not measured is not an optimization, and "the gain
  does not justify the change" is a legitimate outcome.
- **Tree shape**: fixtures with 0, 1, 2, 3, 4, and 5 leaves, each pinning an
  expected root, locking the odd-node rule by test rather than by comment.
- **Determinism across worker counts**: 1, 2, and N workers yield identical
  roots.
- **Ordering independence from discovery order**: the same file set fed in
  several shuffled orders produces one root. The property the whole EIMP
  rests on.
- **Domain separation**: an input attempting to present an internal node's
  digest as a leaf must not verify.
- **Digest changes on**: content alteration, addition, removal, and
  **rename with bytes unchanged** — the last confirming the path is truly
  bound into the leaf.
- **Localized tamper reporting**: tamper one file in a many-file section and
  assert the verifier names *that file*. A headline benefit, tested as a
  feature rather than assumed to fall out of the structure.
- **Incremental re-sign equivalence**: re-signing after a change yields the
  same root as a full from-scratch sign — and, critically, a test that a
  file altered *behind the cache's back* is still caught by `verify`
  (§S.4).
- Comprehensive test: build a realistic multi-directory corpus, sign it,
  verify it, then in one run alter one file, rename another, add a third,
  and remove a fourth — asserting the root changes for each, that each is
  localized to the right path, and that the incremental path agrees with a
  full re-sign.

## Rejected Alternatives

### A. Keep the byte-join and parallelize it with the two-pass offset scheme

Implement `EIMP-1` §S.11's original sketch: stat for offsets, allocate one
buffer, parallel `read_exact` into disjoint slices. Rejected as the
long-term structure, though it remains the fallback if this EIMP is
declined: it buys compute parallelism only, at the cost of a stat/read race
the code must actively defend against, and it forecloses both incremental
re-signing and localized tamper reporting entirely.

### B. Do nothing — corpus signing is rare, so its cost does not matter

`EIMP-1` §S.11 explicitly frames section attestation as the "runs rarely,
buy the biggest security margin" operation, so it is *tolerant* of being
slow. Rejected: "rarely" is not "never," and a whole-corpus re-sign that
takes minutes is one people stop running — an integrity mechanism nobody
runs provides no integrity. Localized tamper reporting is additionally a
correctness-of-diagnosis issue that no amount of tolerating slowness
produces. That said, this alternative stays live: the plan's Phase A
benchmarks *before* implementing, and a poor measured gain is grounds for
cancelling rather than merging machinery that buys nothing.

### C. Split the restructuring and the parallelism into two EIMPs

The original 2026-07-30 drafting had `EIMP-6` (Merkle restructuring) and
`EIMP-5` (parallel machinery) as separate documents with a STOP-gate
dependency. Rejected and merged the same day: the restructuring exists *for*
the speed and the cheap updates, so splitting them would produce a first
EIMP that breaks the digest format while delivering no measurable benefit,
followed by a second delivering all of it. One purpose, one EIMP.

### D. Normalize filenames (NFC) before ordering

Rejected in `EIMP-1` §S.11a, and inherited here: normalization maps distinct
byte sequences to one key, so on a filesystem where both forms coexist as
separate files, two different files would compare equal — the tie §S.11a
makes a hard error. Cross-platform filename portability is real, but belongs
at the corpus-authoring layer, not inside a signature's ordering rule.

## Open Questions

- **Odd-node rule**: promote the unpaired node, or duplicate it? Different
  roots; duplication has known second-preimage subtleties. Decide at
  begun-time and pin with a fixture.
- **Parallel machinery**: hand-rolled `std::thread` pool (no new dependency,
  keeps core lean unconditionally) or a feature-gated `rayon`. An
  unconditional async runtime is ruled out by `EIMP-4`'s dependency-tree
  assertion.
- **Leaf granularity**: whole file per leaf, or ranged chunks for very large
  files? Chunking parallelizes within one huge file and localizes tamper
  reports below file level, at the cost of a more complex tree. Whole-file
  is the simpler default; revisit if a corpus with very large individual
  artifacts appears.
- **Are per-file leaves independently signed, or only the root?** Signing
  each leaf would let one file's attestation be verified in isolation, but
  multiplies SLH-DSA signing cost by file count — and the conservative
  parameter set is deliberately slow. Root-only is the expected answer;
  record the reasoning.
- **Incremental cache change-indicator** (§S.4) — the correctness question
  of this EIMP.
- **Confirm no real `.section.sig` files exist** when this lands, so §S.5's
  format break costs nothing.

## References

- `EIMP-1` (`docs/eimp/EIMP-1.md`) §S.11 — the byte-join construction this
  EIMP replaces and benchmarks against; §S.11a — the filename collation this
  EIMP inherits unchanged and which pins the tree's shape.
- `EIMP-4` (`docs/eimp/EIMP-4.md`) §Test Plan — the core dependency-tree
  assertion that constrains §S.3's machinery choice.
- `EIMP-2` (`docs/eimp/EIMP-2.md`) §0 — `EinmoId` and the mirror-relative
  path form the collation operates on.
- Code: `src/stage.rs` (`walk_input_tree`, `mirror_input_path`, `EinmoId`).
