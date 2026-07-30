---
eimp: 6
title: Merkle-tree corpus signing with a deterministic filename ordering
author: Claude Code (Opus 5) <noreply@anthropic.com>
status: Draft
type: Standards
created: 2026-07-30
supersedes: []
begun: [ ]
---

# EIMP-6: Merkle-tree corpus signing with a deterministic filename ordering

EIMP numbering is little-endian; the full rules live in `eimp.md` at the
repository root.

## Abstract

Restructure `CorpusSigner`'s section digest (`EIMP-1` §S.11) from a
monolithic byte-join — concatenate every file's bytes in manifest order,
hash the whole buffer — into a **Merkle tree**: digest each file
independently into a leaf, then fold the leaves upward into a single root
digest that SLH-DSA signs. The tree's shape is pinned by an **explicitly
specified total ordering over filenames**, computed from the paths
themselves rather than inherited from whatever order the filesystem happens
to return, so the same corpus always produces the same tree on every machine
and every filesystem. This buys three things the byte-join cannot: cheap
parallelism (independent per-file work plus an associative combine — the
prerequisite `EIMP-5` waits on), incremental re-signing (a changed file
touches only its leaf and that leaf's ancestors), and localized tamper
reporting (the root mismatch narrows to a subtree, and thus to a file).

## Motivation

`EIMP-1` §S.11 defines a section's signed message as the manifest header
followed by every file's bytes concatenated in manifest order, hashed as one
message. That is correct and deterministic, and `EIMP-1` ships it. It has
three structural costs that grow with corpus size:

**1. Parallelism is expensive to add.** Parallelizing a byte-join means the
two-pass metadata→offsets→disjoint-slice scheme `EIMP-1` §S.11 originally
sketched: stat every file to compute its offset in one big buffer, allocate
the buffer, then have workers `read_exact` into disjoint sub-slices. It
works, but the two passes can disagree — a file that changes size between
the stat and the read is a correctness hazard the code must actively defend
against — and it couples read parallelism to one fixed buffer layout. A
Merkle fold has no shared buffer, no offsets, and no such race: each worker
digests one file into one leaf, and the combine is associative.

**2. Every re-sign is a full re-read.** Changing a single file invalidates
the entire section digest, so re-signing reads every byte of every file
again. On a large corpus this makes re-signing an operation people stop
running. With a tree, a changed file recomputes its own leaf and the
`O(log n)` nodes above it; every untouched subtree's digest is reused.

**3. Tamper reports are corpus-wide.** A monolithic hash mismatch says
"something under this section changed" and nothing more. A tree mismatch is
walkable: descend into whichever child's digest disagrees, and arrive at the
specific file. For an integrity mechanism whose entire job is telling a
human what went wrong, that difference is the point.

**Why the ordering needs specifying, not assuming.** `EIMP-1` §S.11 leans on
"einmo's existing sorted walk (`walk_input_tree` sorts; deterministic)".
That is sufficient when the order only fixes a byte concatenation, but a
tree's *shape* — which leaves are siblings, hence which internal nodes
exist — is a function of the ordering, and the root digest is a function of
the shape. An ordering that is merely "whatever the walk produced, sorted"
is under-specified across the axes that actually vary in practice:
filesystem readdir order, locale-dependent collation, Unicode normalization
of filenames, and case-sensitivity. Two machines could sort the same corpus
into two different orders, build two different trees, and compute two
different roots for identical content. The ordering is therefore promoted
from an implementation detail to a specified, tested property of the format.

## Specification

### S.1 The filename ordering (normative)

A section's files are ordered by a **total order on their mirror-relative
paths**, computed as follows. Every step exists to remove a source of
machine-to-machine variation:

1. **Path form.** Take each file's mirror-relative path (the same path
   `EinmoId` already denotes — `EIMP-2` §0), as a sequence of path
   components. Never an absolute path, never a `readdir` entry order.
2. **Component-wise comparison, not string comparison.** Compare paths by
   comparing their components pairwise, in order; the first differing
   component decides. If one path's components are a prefix of the other's,
   the shorter sorts first. This makes `a/b` and `a-b/c` order by structure
   rather than by the incidental byte value of the separator, so a directory
   boundary can never be confused with a character inside a name.
3. **Byte-wise ordering within a component.** Compare individual components
   as **raw UTF-8 byte sequences** (`[u8]` lexicographic), not as
   locale-collated strings. Locale collation varies by machine, by
   environment variable, and by libc version; byte order does not.
4. **No Unicode normalization, and no case folding.** Filenames are compared
   exactly as their bytes appear on disk. einmo does not normalize NFC/NFD
   and does not case-fold. This is deliberate: normalization would make two
   *distinct* files on a case-sensitive filesystem compare equal, which the
   ordering must never do.
5. **Rejection, not silent resolution, of ties.** Two distinct files whose
   paths compare equal under this ordering is impossible on a sane
   filesystem; if it is ever observed, it is a hard error that aborts the
   signature rather than something resolved by a tiebreak. A silent tiebreak
   would be a way for two different corpora to produce one digest.

**The ordering is a property of the paths alone** — not of file contents,
sizes, mtimes, or discovery order — so it can be computed and asserted
without reading a single byte, and it is stable under any change to how the
tree is walked.

**Filesystem enumeration is a source, never an authority.** The walk finds
*which* files exist; this ordering alone decides their sequence. Any code
path that consumes walk order directly is a bug.

### S.2 Tree shape

Given the ordered leaves from §S.1:

- **Leaf digest**: `H(leaf_domain || mirror_path_bytes || file_bytes)` — the
  file's whole signed envelope as it sits on disk (matching `EIMP-1` §S.11's
  existing "the whole artifact, not just its body"). Including the path in
  the leaf means moving a file to a new name changes the digest even if its
  bytes are identical, which is the correct behavior for a corpus whose
  paths are meaningful.
- **Internal node digest**: `H(node_domain || left_digest || right_digest)`.
- **Domain separation is mandatory.** Leaf and internal hashes use distinct
  domain prefixes, so no internal node's digest can ever be forged as a leaf
  digest or vice versa (the classic second-preimage attack on naive Merkle
  constructions).
- **Shape rule for odd node counts** must be specified exactly and tested —
  the two common choices (promote the unpaired node unchanged to the next
  level, or duplicate it and pair it with itself) produce *different* roots,
  and duplication has known second-preimage subtleties. Decide, write it
  down, and pin it with a fixture. Recorded as an Open Question below rather
  than guessed here.
- **Root**: the single remaining digest. An **empty section** has a
  specified root (a fixed constant derived from the domain prefix — not a
  zero digest, not an error), so signing an empty stage is well-defined.
- **The manifest header still participates**: the stage name and parameter
  set id are bound into the root (as a distinguished node or by hashing the
  header with the root), so a `checked/` tree cannot be replayed as a
  `verified/` signature.

### S.3 Relationship to EIMP-1 and EIMP-5

- **`EIMP-1` ships the byte-join, single-threaded, first.** That serial
  implementation is the correctness baseline. This EIMP *replaces* the
  digest construction, so it necessarily changes the digest value — see
  §S.4.
- **`EIMP-5` (parallel machinery) waits on this.** Its plan's second STOP
  gate blocks until this EIMP resolves, precisely because parallelizing the
  tree is a different (and much smaller) job than parallelizing the
  byte-join.
- **Incremental re-signing is enabled here but not necessarily implemented
  here.** The tree makes it possible; whether this EIMP also builds the
  leaf-digest cache that exploits it is an Open Question.

### S.4 This changes the digest — a format break

Any corpus signed under the byte-join construction has a root that this
construction will not reproduce. `.section.sig` files are therefore not
forward-compatible across this change. Two mitigations, both cheap because
of where einmo currently stands:

1. **`EIMP-1` §S.11 explicitly does not write `.section.sig` into any real
   corpus** — it is fixtures-and-tempdirs only, deliberately deferred as "a
   later step." So at the time this EIMP lands there should be *no* real
   signed sections in existence to migrate. Verify that assumption holds
   before relying on it.
2. **The signature file records its construction.** `.section.sig` carries a
   construction identifier (alongside the existing parameter-set id) so a
   verifier reading an old file fails with "unknown/obsolete construction"
   rather than "signature mismatch." A wrong-algorithm error and a
   tampered-corpus error must never look alike.

## Test Plan

- **Ordering, unit-tested against the variation it exists to eliminate**:
  paths differing only by case; paths differing by Unicode normalization
  (NFC vs NFD of the same grapheme); paths where a separator vs an in-name
  character would flip a naive string sort (`a/b` vs `a-b`); nested vs flat
  paths sharing a prefix. Each asserts a specific expected order, so a
  future refactor cannot quietly change the tree shape.
- **Ordering is independent of discovery order**: feed the same file set in
  several shuffled input orders; the computed ordering — and the root — must
  be identical every time. This is the property the whole EIMP rests on.
- **Tree shape**: fixtures with 0, 1, 2, 3, 4, and 5 leaves, each pinning an
  expected root, so the odd-node rule is locked by test rather than by
  comment.
- **Domain separation**: a constructed input attempting to present an
  internal node's digest as a leaf must not verify.
- **Digest changes on**: content alteration, file addition, file removal,
  and file *rename* (bytes unchanged) — the last confirming the path is
  genuinely bound into the leaf.
- **Localized tamper reporting**: tamper one file in a many-file section and
  assert the verifier identifies *that file*, not merely that the section
  failed. This is a headline benefit and must be tested as a feature, not
  assumed to fall out of the structure.
- **Incremental re-sign equivalence** (if implemented): re-signing after
  changing one file yields the same root as a full from-scratch sign.
- Comprehensive test: build a realistic multi-directory corpus, sign it,
  verify it, then — in one run — alter one file, rename another, add a
  third, and remove a fourth, asserting the root changes for each and that
  each is localized to the right path.

## Rejected Alternatives

### A. Keep the byte-join; parallelize it with the two-pass offset scheme

Implement `EIMP-1` §S.11's original design as written. Rejected as the
long-term structure, though it remains the fallback if this EIMP is
declined: it buys parallelism only, at the cost of a stat/read race the code
must defend against, and it forecloses incremental re-signing and localized
tamper reporting entirely. `EIMP-5`'s Rejected Alternative C keeps this path
open should the tree work stall.

### B. Rely on `walk_input_tree`'s existing sort rather than specifying an ordering

Reuse the sorted walk and call the shape determined. Rejected: that sort is
an implementation detail of directory traversal, not a specified property of
the signature format. Nothing today prevents it from changing — a
performance refactor, a locale-sensitive comparator, or a different walk
crate would silently alter the tree shape and therefore every root digest,
turning correct corpora into apparently-tampered ones with no code change
visibly related to signing. Specifying and testing the ordering is what
makes the format independent of the walker.

### C. Normalize filenames (NFC) before ordering

Apply Unicode normalization so visually-identical names sort together across
platforms that store them differently. Rejected: normalization maps distinct
byte sequences to one key, so on a filesystem where both forms can coexist
as separate files, two different files would compare equal — exactly the tie
§S.1 item 5 makes a hard error. Cross-platform filename portability is a
real problem, but it belongs at the corpus-authoring layer, not inside a
signature's ordering rule.

### D. Do nothing — corpus signing is rare, so its cost does not matter

Rejected on the same grounds as `EIMP-5`'s Rejected Alternative B: "rare" is
not "never," and the three costs in §Motivation are structural rather than
merely slow. Localized tamper reporting in particular is a correctness-of-
diagnosis issue, not a performance one, and no amount of tolerating slowness
produces it.

## Open Questions

- **Odd-node rule**: promote the unpaired node unchanged, or duplicate it?
  They produce different roots, and duplication has known second-preimage
  subtleties. Decide at begun-time and pin with a fixture.
- **Leaf granularity**: whole file per leaf, or ranged chunks for very large
  files? Chunking would let one huge file parallelize internally and would
  localize tamper reports below file level, at the cost of a more complex
  tree. Whole-file is the simpler default; revisit if a corpus with
  very large individual artifacts appears.
- **Are per-file leaves independently signed, or only the root?** Signing
  each leaf would let a single file's attestation be verified in isolation
  without the rest of the corpus, but multiplies SLH-DSA signing cost by the
  file count — and SLH-DSA at the conservative parameter set is deliberately
  slow. Root-only is the expected answer; record the reasoning.
- **Does this EIMP also implement the leaf-digest cache** that makes
  incremental re-signing real, or only make it possible? (§S.3)
- **Confirm no real `.section.sig` files exist** when this lands, so §S.4's
  format break costs nothing.

## References

- `EIMP-1` (`docs/eimp/EIMP-1.md`) §S.11 — the byte-join construction this
  EIMP replaces, and the single-threaded `CorpusSigner` that ships first.
- `EIMP-5` (`docs/eimp/EIMP-5.md`) — parallel machinery; its plan's second
  STOP gate blocks on this EIMP being resolved or explicitly declined.
- `EIMP-2` (`docs/eimp/EIMP-2.md`) §0 — `EinmoId` and the mirror-relative
  path form §S.1's ordering operates on.
- Code: `src/stage.rs` (`walk_input_tree`, `mirror_input_path`, `EinmoId`).
