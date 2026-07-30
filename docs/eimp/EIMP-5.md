---
eimp: 5
title: Parallelized corpus signing — parallel machinery over the Merkle-structured CorpusSigner
author: Claude Code (Opus 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-30
supersedes: []
begun: [ ]
---

# EIMP-5: Parallelized corpus signing — parallel machinery over the Merkle-structured CorpusSigner

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

## Abstract

Bring genuine parallel execution to `CorpusSigner` — the section-level
post-quantum attestation object specified in `EIMP-1` §S.11 and shipped by
`EIMP-1` as **single-threaded** — without regressing core `einmo`'s lean
dependency tree (`EIMP-4` §S.1). This EIMP is deliberately the *second half*
of a two-part split: `EIMP-1` ships correct-but-serial signing, a repo TODO
carries the structural work (a Merkle-tree corpus shape: sign each file,
then fold file digests up into a whole-tree digest) that makes parallelism
a near-drop-in rather than a rewrite, and this EIMP adds the machinery that
exploits that structure. Splitting it this way means einmo gets a correct,
publishable, dependency-light `CorpusSigner` for the `0.0.6` release, and
pays for parallel machinery only when corpus size actually demands it.

## Motivation

`EIMP-1` §S.11 originally specified `CorpusSigner`'s read pass as a
bounded worker pool (`ReadStrategy::ParallelBuffer`, the default) with a
sequential `ReadStrategy::Stream` alternative as a cross-check oracle. The
worker-pool implementation was resolved (2026-07-30) to use `tokio`'s
blocking pool — a reasonable choice at the time, because `tokio` was
already a dependency of the single `einmo` crate via the review server.

`EIMP-4` then split the repository into a lean core `einmo` and a separate
`einmo-review-server`, moving the entire HTTP stack — `tokio` included —
out of core. `CorpusSigner` is corpus-integrity code and belongs in core.
That leaves the original decision self-defeating: implementing it with
`tokio` would drag an async runtime back into the very crate the split
exists to keep lean, for a workload (reading files into disjoint buffer
slices) that has no async character at all.

Rather than relitigate the machinery under time pressure while `EIMP-1` is
mid-flight, the work is split:

- **`EIMP-1` ships single-threaded.** `CorpusSigner::digest` reads the
  section in manifest order and feeds the hasher. Correct, deterministic,
  zero new dependencies, and — critically — it establishes the digest that
  any parallel implementation must reproduce bit-for-bit.
- **`EIMP-6` carries the structural design** (`docs/eimp/EIMP-6.md`):
  restructure corpus signing around a Merkle tree — sign/digest each file
  independently, then fold those digests into a tree digest. This is the
  part that makes parallelism *cheap*: independent per-file work with a
  cheap associative combine, rather than one monolithic byte-join that
  needs careful disjoint-slice choreography to parallelize at all.
- **This EIMP adds the machinery**, over whatever structure that design
  settles on.

**The ordering matters.** Parallelizing the *current* byte-join design
means the two-pass metadata/offset scheme of `EIMP-1` §S.11 — workable, but
it couples read parallelism to a fixed buffer layout and makes
short/long-read races a correctness hazard the code must actively defend
against. Parallelizing a *Merkle* design means mapping an independent
digest over each file and folding — no shared buffer, no offset
choreography, no read-race hazard, and incremental re-signing (only the
changed file's leaf and its ancestors need recomputing) comes almost free.
Doing the structural work first is what makes the machinery small.

## Specification

**This EIMP is a Draft placeholder with deliberately unfrozen internals.**
Specifying the parallel machinery in detail now would be specifying it
against a corpus-signing structure that the TODO is expected to change.
What *is* fixed here are the constraints any implementation must satisfy:

### S.1 Constraints on any implementation

1. **Bit-identical digests.** The parallel path must produce byte-identical
   digests to `EIMP-1`'s single-threaded path for every input, and a test
   must assert this on shared fixtures. The serial implementation is the
   oracle; parallelism is an optimization over a result that is already
   defined.
2. **Core stays lean, or the machinery is optional.** Core `einmo`'s
   dependency-tree assertion (`EIMP-4` §Test Plan) must still pass. That
   admits several shapes — a small `std::thread` pool with no new
   dependency; a feature-gated `rayon`/`tokio` path off by default; or the
   parallel implementation living outside core. Which one is chosen is an
   Open Question below, to be answered against the structure the TODO
   produces, not guessed now.
3. **Determinism is structural, not incidental.** Whatever the machinery,
   the digest must not depend on thread count, scheduling, or completion
   order. Under a Merkle design this is nearly automatic (the fold order is
   fixed by the tree shape); under the byte-join design it requires the
   manifest to pin offsets before any read starts.
4. **Bounded parallelism.** A pathologically large corpus must not spawn
   unbounded workers; the worker count is capped and configurable.
5. **Failure is loud.** A short read, a long read, or a file changing
   underneath the signer is a hard error that aborts the signature — never
   a silently truncated or partially-read digest. `EIMP-1` §S.11's
   concurrency caveat carries forward unchanged.

### S.2 Scope boundary

In scope: the parallel read/digest machinery, its worker-count
configuration, its determinism and equivalence tests, and any benchmark
demonstrating it is actually faster than serial on a realistic corpus (an
optimization that is not measured is not an optimization).

Out of scope: the Merkle restructuring itself (the repo TODO — likely its
own EIMP once designed), the SLH-DSA primitive (`EIMP-1` §S.11, unchanged),
and wiring section signatures into the live promotion flow (`EIMP-1` §S.11
explicitly defers that too).

## Test Plan

- **Equivalence, serial vs parallel**: the same fixtures digested both
  ways must agree bit-for-bit. This is the load-bearing test — everything
  else is performance.
- **Determinism under varying worker counts**: digesting with 1, 2, and N
  workers yields identical results.
- **Failure propagation**: a file that shrinks/grows/disappears mid-read
  aborts the signature with a hard error rather than producing a digest.
- **Benchmark**: parallel vs serial on a corpus large enough for the
  difference to be real, recorded in the plan. If parallel is not
  meaningfully faster, that is a finding worth recording — and possibly
  grounds for cancelling this EIMP rather than merging machinery that buys
  nothing.
- Comprehensive test: sign a realistic corpus in parallel, verify it with
  the serial path, tamper one file, and confirm verification fails —
  proving the parallel path produces signatures the serial verifier
  accepts and that the tamper detection survives the optimization.

## Rejected Alternatives

### A. Implement parallel signing directly in EIMP-1 (the original plan)

Keep `EIMP-1` §S.11's `ParallelBuffer`-by-default design and implement it
now. Rejected: it would either drag `tokio` back into core `einmo` —
undoing `EIMP-4`'s split — or force an immediate machinery decision under
`EIMP-1`'s schedule, before the Merkle restructuring that would make the
machinery much simpler. Shipping serial-and-correct first costs einmo
nothing at current corpus sizes and preserves every option.

### B. Never parallelize; serial signing is enough

Rejected as premature: corpus signing is explicitly the "rarely run, buy
the biggest security margin" operation (`EIMP-1` §S.11), so it is *tolerant*
of being slow — but "rarely" is not "never," and a whole-corpus re-sign on
a large suite is exactly the kind of multi-minute operation that stops
being run at all. Keeping this EIMP open (rather than declaring serial
sufficient) preserves the option; the benchmark in the Test Plan is what
will actually decide whether it is worth merging.

### C. Parallelize the existing byte-join design without restructuring

Implement `EIMP-1` §S.11's two-pass metadata/offset scheme as specified,
just with `std` threads instead of `tokio`. Rejected as the *default* path,
though it remains viable if the Merkle work stalls: it couples read
parallelism to a fixed buffer layout, requires active defense against
short/long-read races, and buys none of the incremental-re-signing benefit
that falls out of a tree structure for free.

## Open Questions

- Which machinery: a small hand-rolled `std::thread` pool (no new
  dependency, keeps core lean unconditionally), a feature-gated `rayon`,
  or a feature-gated `tokio` blocking pool. **Deliberately unanswered** —
  it should be decided against the corpus-signing structure the repo TODO
  produces, since a Merkle fold and a byte-join want quite different
  machinery.
- Whether the Merkle restructuring is large enough to deserve its own EIMP
  (likely) or lands as part of this one. Depends on how far the TODO's
  design pass reaches.
- Whether parallel signing is worth merging at all — the Test Plan's
  benchmark decides, and "no" is an acceptable outcome that would see this
  EIMP cancelled rather than forced through (`EIMP-0`'s cancellation
  procedure).

## References

- `EIMP-1` (`docs/eimp/EIMP-1.md`) §S.11 — the `CorpusSigner` design this
  EIMP optimizes; ships single-threaded, with its parallel read strategies
  deferred here.
- `EIMP-4` (`docs/eimp/EIMP-4.md`) §S.1 — the crate split that removed
  `tokio` from core `einmo` and thereby invalidated the original
  worker-pool choice; §Test Plan's dependency-tree assertion is the
  constraint this EIMP must not break.
- `EIMP-6` (`docs/eimp/EIMP-6.md`) — the Merkle-tree corpus-signing
  restructuring this EIMP builds on; its deterministic filename ordering
  (§S.1) is what makes the tree shape — and therefore the parallel fold —
  reproducible. This EIMP's second STOP gate blocks on it.
