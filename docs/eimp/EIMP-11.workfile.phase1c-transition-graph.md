# EIMP-11 Phase 1C — Transition graph correction

This workfile records the two sequential transition corrections required by
EIMP-11 §S.3. The forward shortcut is committed independently before the
accepted Gate A backward-edge removal.

## Forward correction audit

`output → verified` was represented in the library predicate, CLI help, and
review planning fallback. The correction removes it from the one authoritative
transition table. CLI command execution, review planning, and the review
server each reject it before making a decision or attempting a mutation.

The audit found one in-memory unit-test setup in `src/case.rs`; it now creates
the checked artifact through the ordinary adjacent promotion before asserting
the computer-key behavior of the verified promotion. No signed fixture relies
on the shortcut, so no artifact was regenerated or edited.

The current README CLI table was the only current user-facing statement that
advertised the shortcut. Historical EIMP discussion/plan text remains evidence
of the former behavior and is not rewritten as current documentation.

## Last Updated

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded the Phase 1C forward-correction audit, its one
in-memory test migration, and the absence of signed-fixture migration.
