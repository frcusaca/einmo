# month.2 — multiple reviewers, conflicting decisions

*(Stub — no test content yet. This tier is scaffolded for documentation
first, like `week.2/`; `.js` inputs land here once designed. See `day.1/`
for the currently populated tier.)*

## The scenario

Two months in, the project has grown enough that one person reviewing
every `.einmo` change is a bottleneck. Senior technical staff promotes a
few contributors to case reviewer — now several people can independently
look at the same suite, decide on the same test, and sign. What was a
single-reviewer workflow (`day.1`, `week.2`) becomes a **multi-verifier**
one, and that raises a question none of the earlier tiers had to answer:
**what happens when two reviewers disagree, or act on the same case at the
same time?**

## How einmo is designed to handle this (`EIMP-1` §S.5)

This isn't hypothetical — it's already specified, in `docs/eimp/EIMP-1.md`
§S.5 ("Concurrency semantics for multiple verifiers"), as part of the
`EinmoReview` session object's design (not yet implemented — see `EIMP-1`'s
status). The model, summarized here for this tier's purpose:

- **Decisions are per-reviewer, not global.** Two reviewers looking at the
  same case each have their own decision slot. Reviewer A deciding
  "promote" does not overwrite reviewer B's separate "flag" — they coexist.
  *Within* one reviewer, a new decision replaces their old one
  (replace-not-stack); it does not stack.
- **"Conflicting decisions" mostly isn't a conflict at the data-model
  level.** If A promotes to `checked` and B independently promotes the same
  case to `checked` too, both are just... promoting. If A wants to promote
  and B wants to flag, both decisions are recorded; nothing forces a single
  answer before execution — the ambiguity becomes visible (two named
  reviewers pointing at different outcomes for the same case) rather than
  silently resolved by whoever clicked last.
- **`verified` stamps accumulate — they don't overwrite.** Once a human
  signs, executing a second reviewer's `verified` decision **appends**
  their stamp rather than replacing the first. Two independent `verified`
  signatures on one artifact is *stronger* attestation, not a conflict —
  `Stamps::stamped_by` surfaces who has signed so far. This is the closest
  thing to a built-in "N-of-M agreement" primitive today, though a true
  quorum policy (e.g. "verified requires 2 of 3 release officers") is
  explicitly named as a `EIMP-1` open question, not yet built.
- **Soft claims prevent duplicated effort, not disagreement.** A reviewer
  can advertise "I'm on this one" (a time-limited claim shown in listings)
  so two people don't redundantly review the same case in parallel — but a
  claim is advisory only; it cannot block another reviewer from deciding
  anyway. It solves wasted effort, not conflicting judgment.
- **Disk mutation is serialized and drift-checked, not conflict-resolved.**
  When decisions actually execute (get written and signed), an exclusive
  lock means writes happen one at a time, and each write re-checks the
  artifact's fingerprint first. If the file changed since the decision was
  planned — including because another reviewer's action landed first — the
  write is **skipped and reported**, never silently clobbered. This handles
  *mechanical* races (two executes touching the same file at once); it does
  not by itself decide *whose judgment wins* when two reviewers actually
  disagree about what the right outcome is.

## What this tier should eventually demonstrate

- A worked example: two named reviewers, one case, genuinely conflicting
  decisions (promote vs. flag) — walk through what the corpus looks like
  after both execute, and what a third party (or `einmo verify`) sees.
- How `experimental_reviewer.sh` (or, once built, `einmo_review_client.sh`
  — see `docs/eimp/EIMP-2.md`) surfaces "someone else already has an
  opinion on this case" to a reviewer before they form their own.
- The escalation path when accumulated signatures don't converge (multiple
  `verified` stamps, but from reviewers who'd disagree if asked directly) —
  today this is a human/process question, not something einmo resolves
  automatically; document what the *current* recommended practice is, even
  before quorum policies exist.

See `docs/eimp/EIMP-1.md` §S.5 and §S.6 (the journal — "who decided what,
when, with which key" is the audit trail this scenario ultimately depends
on) for the fuller design this tier's content should exercise once
implemented.
