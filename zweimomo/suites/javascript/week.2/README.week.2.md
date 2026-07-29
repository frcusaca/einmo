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

See `docs/eimp/` in the repo root for the design documents this tier's
content should eventually exercise.
