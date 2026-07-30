# EIMP-5.plan — parallelized corpus signing

Read `docs/eimp/EIMP-5.md` before acting on any task below. Tasks run top to
bottom. Work happens directly on `main` (`EIMP-0` §8).

**Deferred by design.** This plan is written now so the deferral is
tracked rather than forgotten, but it does not begin until its
dependencies land — `EIMP-1`'s single-threaded `CorpusSigner` (the
correctness oracle) and the Merkle-tree corpus-signing design TODO (the
structure that makes the machinery small). The STOP gates below enforce
both.

- [ ] STOP — `EIMP-1` is `complete` and its single-threaded `CorpusSigner`
      exists: `manifest`/`digest`/`sign`/`verify` implemented and tested.
      That serial implementation is this EIMP's correctness oracle; without
      it there is nothing to prove equivalence against
- [ ] STOP — the Merkle-tree corpus-signing design TODO
      (`docs/todo/AIAGENT-einmo-repo.todo.md`) has been resolved, either as
      its own EIMP or as an accepted decision not to restructure. If the
      answer is "no restructuring," re-read `EIMP-5.md` Rejected
      Alternative C — this plan's shape changes materially
- [ ] STOP — preconditions: `cargo test --workspace`, `cargo clippy
      --workspace --all-targets -- -D warnings`, `cargo fmt --check` all
      clean
- [ ] Sanity check: consult human on `EIMP-5.md` §Open Questions — the
      machinery choice (`std` pool / feature-gated `rayon` / feature-gated
      `tokio`), now answerable against the actual corpus-signing structure.
      Remind them: "Above message comes from EIMP-5 working to parallelize
      corpus signing; changes are on `main`. PTAL"
- [ ] Begin work: check `begun: [x]` in `EIMP-5.md` frontmatter, commit
      `EIMP-5.md` stating that work has commenced

## Phase A — benchmark first, decide whether to proceed (EIMP-5.md §Test Plan)

Deliberately before any implementation: `EIMP-5.md` Rejected Alternative B
and §Open Questions both make "this is not worth merging" a legitimate
outcome. Measuring first means that outcome costs one benchmark, not a
finished feature.

- [ ] Build a realistic large-corpus fixture (large enough that serial
      signing takes long enough to matter — record the actual size and
      timing here, not a guess)
- [ ] Benchmark the serial `CorpusSigner::digest` on it; record the number
- [ ] Estimate the achievable parallel speedup (cores available, I/O vs
      CPU bound); record the estimate and the reasoning
- [ ] Decision point: if the projected gain does not justify the machinery,
      STOP and cancel this EIMP per `EIMP-0`'s cancellation procedure
      (`[x] Canceled.` + `[-]` on every remaining item), recording the
      measurement as the reason. This is a real possible outcome, not a
      formality

## Phase B — the parallel implementation (EIMP-5.md §S.1)

- [ ] Write the equivalence test FIRST: the same fixtures digested serially
      and in parallel must agree bit-for-bit. This is the load-bearing
      test; write it before there is a parallel path to run it against
- [ ] Write the determinism test FIRST: digesting with 1, 2, and N workers
      yields identical results
- [ ] Write the failure-propagation tests FIRST: a file that shrinks,
      grows, or disappears mid-read aborts the signature with a hard error
      rather than yielding a digest (`EIMP-5.md` §S.1 constraint 5)
- [ ] Implement the parallel digest path using the machinery chosen at the
      sanity-check step above, with a bounded, configurable worker count
      (`EIMP-5.md` §S.1 constraint 4)
- [ ] Confirm core `einmo`'s dependency-tree assertion (`EIMP-4` §Test
      Plan) still passes — if the chosen machinery is feature-gated,
      confirm it passes with default features and that the gated path is
      also tested (`EIMP-5.md` §S.1 constraint 2)
- [ ] Re-run the Phase A benchmark against the finished implementation and
      record the *actual* speedup here, next to the estimate. An
      optimization that did not deliver what it promised is a finding worth
      writing down
- [ ] Phase B tests green; `cargo fmt` / `cargo clippy --workspace
      --all-targets -- -D warnings` clean

## Comprehensive test + completion

- [ ] Comprehensive test, per `EIMP-5.md` §Test Plan: sign a realistic
      corpus via the parallel path, verify it via the serial path, then
      tamper one file and confirm verification fails — proving the parallel
      path emits signatures the serial verifier accepts, and that tamper
      detection survived the optimization
- [ ] All tests pass: `cargo test --workspace`, `cargo clippy --workspace
      --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] Update `EIMP-5.md` frontmatter to `status: complete`
- [ ] Update `docs/eimp/INDEX.md` to reflect EIMP-5's completed status
