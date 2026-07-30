# EIMP-5.plan — Merkle-tree corpus signing: faster to compute, cheaper to update

Read `docs/eimp/EIMP-5.md` before acting on any task below. Tasks run top to
bottom. Work happens directly on `main` (`EIMP-0` §8).

**Deferred by design.** This plan is written now so the deferral is tracked
rather than forgotten, but it does not begin until `EIMP-1` ships the
byte-join `CorpusSigner` — which is both the correctness reference and the
performance baseline this EIMP's benefit is measured against.

- [ ] STOP — `EIMP-1` is `complete` and its single-threaded byte-join
      `CorpusSigner` exists: `manifest`/`digest`/`sign`/`verify` implemented
      and tested, with `EIMP-1` §S.11a's `Collation` in place. Without it
      there is no baseline to measure against and no collation to inherit
- [ ] STOP — confirm no real `.section.sig` files exist anywhere
      (`EIMP-5.md` §S.5): this EIMP changes the digest construction, so any
      corpus already signed under the byte-join would need migrating.
      `EIMP-1` §S.11 writes them only to fixtures/tempdirs, so this should
      hold — verify rather than assume
- [ ] STOP — preconditions: `cargo test --workspace`, `cargo clippy
      --workspace --all-targets -- -D warnings`, `cargo fmt --check` all
      clean
- [ ] Sanity check: consult human on `EIMP-5.md` §Open Questions — the
      odd-node rule, the parallel machinery (`std` pool vs feature-gated
      `rayon`), leaf granularity, root-only vs per-leaf signing, and the
      incremental cache's change-indicator (§S.4, the correctness question).
      Remind them: "Above message comes from EIMP-5 working to restructure
      corpus signing around a Merkle tree; changes are on `main`. PTAL"
- [ ] Begin work: check `begun: [x]` in `EIMP-5.md` frontmatter, commit
      `EIMP-5.md` stating that work has commenced

## Phase A — benchmark the baseline, decide whether to proceed (EIMP-5.md §Test Plan)

Deliberately before any implementation: `EIMP-5.md` Rejected Alternative B
makes "not worth merging" a legitimate outcome. Measuring first means that
outcome costs one benchmark rather than a finished feature — and this EIMP
changes a signature format, which is not a change to make for an unmeasured
gain.

- [ ] Build a realistic large-corpus fixture — large enough that byte-join
      signing takes long enough to matter. Record the actual file count,
      total size, and timing here, not a guess
- [ ] Benchmark `EIMP-1`'s byte-join `CorpusSigner::digest` on it (full
      sign, and a re-sign after changing one file — the second number is
      what "cheaper to update" is measured against); record both
- [ ] Estimate the achievable gain on each axis: parallel speedup (cores,
      I/O vs CPU bound) and incremental re-sign speedup (`O(log n)` node
      recomputation vs full re-read); record the estimates and reasoning
- [ ] Decision point: if neither axis justifies a format change, STOP and
      cancel this EIMP per `EIMP-0`'s cancellation procedure (`[x] Canceled.`
      + `[-]` on every remaining item), recording the measurement as the
      reason. A real possible outcome, not a formality

## Phase B — the Merkle construction (EIMP-5.md §S.1, §S.2)

Serial first. The tree must be correct before it is fast; a parallel
implementation of a wrong tree is worthless.

- [ ] Write the tree-shape tests FIRST: fixtures with 0, 1, 2, 3, 4, and 5
      leaves, each pinning an expected root, so the odd-node rule resolved
      at the sanity-check step is locked by fixture rather than by comment
- [ ] Write the ordering-independence test FIRST: the same file set fed in
      several shuffled discovery orders produces one root. The property the
      whole EIMP rests on
- [ ] Write the domain-separation test FIRST: an input presenting an
      internal node's digest as a leaf must not verify (`EIMP-5.md` §S.2)
- [ ] Write the digest-sensitivity tests FIRST: content alteration,
      addition, removal, and **rename with bytes unchanged** each change the
      root — the last confirming the path is genuinely bound into the leaf
- [ ] Implement leaf digests (`H(leaf_domain || mirror_path_bytes ||
      file_bytes)`) and internal nodes (`H(node_domain || left || right)`),
      ordered by `EIMP-1` §S.11a's `Collation` — inherited, not
      reimplemented
- [ ] Implement the empty-section root as a specified constant (not a zero
      digest, not an error) and the manifest-header binding (stage name +
      parameter-set id + collation id folded into the root, so a `checked/`
      tree cannot be replayed as a `verified/` signature)
- [ ] Record the construction identifier in `.section.sig` (`EIMP-5.md`
      §S.5) so a verifier reading a byte-join-era file fails with
      "unknown/obsolete construction", never with a generic signature
      mismatch — a wrong-algorithm error and a tampered-corpus error must
      not look alike
- [ ] **Localized tamper reporting**: implement the descent that turns a
      root mismatch into the offending file's path, and test it as a
      feature — a headline benefit, not something assumed to fall out of the
      structure
- [ ] Phase B tests green; `cargo fmt` / `cargo clippy --workspace
      --all-targets -- -D warnings` clean

## Phase C — parallel computation (EIMP-5.md §S.3)

- [ ] Write the determinism test FIRST: 1, 2, and N workers yield identical
      roots (`EIMP-5.md` §S.3 constraint 1)
- [ ] Write the failure-propagation tests FIRST: a file that shrinks, grows,
      or disappears mid-read aborts the signature with a hard error rather
      than yielding a digest (constraint 4)
- [ ] Implement parallel leaf digesting with the machinery chosen at the
      sanity-check step, with a bounded, configurable worker count
      (constraint 3)
- [ ] Confirm core `einmo`'s dependency-tree assertion (`EIMP-4` §Test Plan)
      still passes — if the machinery is feature-gated, confirm it passes
      with default features AND that the gated path is also tested
      (constraint 2)
- [ ] Re-run Phase A's parallel benchmark and record the **actual** speedup
      next to the estimate. An optimization that did not deliver what it
      promised is a finding worth writing down
- [ ] Phase C tests green; `cargo fmt` / `cargo clippy -D warnings` clean

## Phase D — incremental re-signing (EIMP-5.md §S.4)

The "cheaper to update" half. Its change-indicator is this EIMP's
correctness question, so its tests come first and its safe default is
non-negotiable.

- [ ] Write the equivalence test FIRST: re-signing after a change yields the
      same root as a full from-scratch sign
- [ ] Write the **security** test FIRST: a file altered behind the cache's
      back is still caught by `verify`. Per §S.4, `verify` never trusts the
      cache — it always recomputes leaves from bytes. An optimization that
      can be induced to sign or accept stale content is a vulnerability, not
      a speedup
- [ ] Implement the leaf-digest cache with the change-indicator resolved at
      the sanity-check step, recomputing only changed leaves and their
      ancestors on `sign`
- [ ] Re-run Phase A's incremental benchmark; record the actual gain next to
      the estimate
- [ ] Phase D tests green; `cargo fmt` / `cargo clippy -D warnings` clean

## Comprehensive test + completion

- [ ] Comprehensive test, per `EIMP-5.md` §Test Plan: build a realistic
      multi-directory corpus, sign it, verify it, then in one run alter one
      file, rename another, add a third, and remove a fourth — asserting the
      root changes for each, that each is localized to the right path, and
      that the incremental path agrees with a full re-sign
- [ ] All tests pass: `cargo test --workspace`, `cargo clippy --workspace
      --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] Update `EIMP-5.md` frontmatter to `status: complete`
- [ ] Update `docs/eimp/INDEX.md` to reflect EIMP-5's completed status
