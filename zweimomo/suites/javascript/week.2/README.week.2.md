# week.2 — mass and randomized re-review

*(Stub — no test content yet. This tier is scaffolded for documentation
first; `.js` inputs land here once designed. See `day.1/` for the currently
populated tier.)*

`day.1` answers "does einmo work on one small suite." This tier answers a
different question: **once a corpus has grown, how do you keep it honest
over time** — not just when a file first changes, but on a schedule, so
baseline rot (a `checked/` file that quietly stopped matching what a human
would actually approve today) gets caught even when nothing *triggered* a
re-look?

Two related but distinct operations, both already supported by
`scripts/experimental_reviewer.sh` today (documented here, demonstrated
against this tier's fixtures once they exist):

## Full re-review (`-D`)

By default, `experimental_reviewer.sh` only visits **differing** tests —
anything where `output`/`checked`/`verified` already agree is skipped, on
the theory that an agreeing, fully-verified test needs no human attention
right now. The `-D` flag overrides that default and visits **every** test,
including ones that fully agree:

```bash
./scripts/experimental_reviewer.sh -D suites/javascript/week.2
```

This is the blunt instrument: a deliberate, full fresh-eyes pass over the
whole suite. Useful before a release, or after a long gap since the last
full look.

## Randomized re-inspection (`-s`)

The `-s` flag shuffles the review order instead of visiting tests in their
natural (sorted, deterministic) order:

```bash
./scripts/experimental_reviewer.sh -s suites/javascript/week.2
```

Combined with `-D`, this gives **randomized full re-inspection**: every
test gets looked at, in an order that doesn't let a reviewer's attention
drift into "I've seen the first twenty, the rest are probably fine too"
autopilot. This is the mechanism `EIMP-1` §S.11's aspirational "randomized
re-inspection" design point (a scheduled random sample demoted and
re-presented for review) is a smaller, unscheduled version of —
`experimental_reviewer.sh`'s `-s` shuffles a review you initiate by hand;
`EIMP-1`'s fuller vision automates *choosing when and how much* to
re-sample. This tier is where that gap gets explored.

## Open design questions this tier should eventually answer

- What fraction of a large corpus is a reasonable random sample size for a
  *periodic* (not full) re-inspection pass?
- Should re-inspection results feed anything back into `einmo` itself (a
  recorded "last re-reviewed" timestamp per case), or does
  `experimental_reviewer.sh`'s ad hoc `-s -D` remain sufficient?
- How does this interact with `einmo-review-server` (`EIMP-2`) once the
  server holds review state — does randomized re-inspection become a
  server-side sampling endpoint, or stay a client-side shuffle of the
  worklist it already fetches?

## Use case: multiple DCs sign off, each with a reason (NOT YET IMPLEMENTED)

**The scenario.** A second data center comes online — different hardware, a
different OS, a newer language/toolchain version. Then, over time, a
third, a fourth. Before trusting any of them, the team wants to know: does
each one produce the *same* signed results as the existing fleet? The
natural way to check is to let each new DC run the test suite and add
**its own** signature to any of the `output`, `checked`, or `verified`
sections — an additional, independent attestation layered on top of the
existing ones, never a replacement for them. Ideally, each additional
signature carries a short reason recorded alongside it (e.g.
"cross-verification from DC-2: aarch64, Ubuntu 26.04, Rust 1.9x") — so a
reader of the stamp chain later can tell *why* there are multiple
signatures at a stage, not just that there are multiple.

**What einmo can do today.** Multiple independent signatures already work
at the **`verified`** stage — that's the existing design (`EIMP-1` §S.5):
any number of humans (or humans and agents) can each promote
`checked → verified` under their own key, and every `stage:verified` stamp
accumulates; `Stamps::stamped_by(prefix)` lets anyone check whether a
particular signer's key is among them. What does **not** exist yet:
`promote` only ever appends **one** `stage:<to>` stamp per legal transition
(`output → checked` or `checked/output → verified`), and a `Stamp` carries
no free-form metadata field at all — just `key`, `pubkey_hex`, `signs`,
`signature_b64`, `produced_by`, `timestamp` (`signature.rs`). So today
there is no way to have a *second, third, or Nth* `output` or `checked`
stamp on an already-`output`/already-`checked` file, with or without a
reason attached — each additional DC's run is a completely separate
signing operation, not an addition to the existing ones. (`verified`
already supports the *multiple-signature* part; it's the *reason metadata*
part that's missing everywhere, including `verified`.)

**What would need to change.** At minimum: (1) `is_legal_transition` (or a
new operation distinct from `promote`) would need to allow re-signing
within a stage — `output → output` and `checked → checked`, not just
`verified` accumulating as it does today — appending rather than replacing
the stamp chain, the same append-only discipline every other stamp already
follows; (2) `Stamp` would need an optional reason/metadata field,
serialized alongside the existing fields, so `einmo show` can display *why*
each additional signature exists, at any of the three sections; (3) the
CLI's `promote` verb (or a new verb) would need a way to pass that reason
in. This is flagged as a **significant feature**, not a small one — it
touches the stamp format (a wire-compatibility question: can old `einmo`
readers parse a stamp with an unrecognized extra field?), the
transition-legality rules for `output`/`checked` specifically, and the CLI
surface. It likely deserves its own EIMP once scoped. See
`docs/todo/AIAGENT-einmo-repo.todo.md` in the repo root for the tracked
follow-up.

See `docs/eimp/` in the repo root for the design documents this tier's
content should eventually exercise.
