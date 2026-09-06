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

## Gate A — backward-edge removal

The resolved Gate A removes `verified → checked`. The exhaustive transition
matrix now allows only `generated → output`, `output → checked`, and `checked
→ verified`. `retract verified` is the one backward operation: it removes the
verified artifact while preserving the checked bytes and stamps beneath it.

CLI validation reads the authoritative table before resolving configuration or
a signing key. Review planning and the review server already use
`forward_source_for`, so they accept only a physical adjacent forward source;
there is no independent legal-pair list to update. The full source/fixture
audit found no signed fixture or EIMP 01 test using the removed edge.

## Last Updated

**Date**: 2026-09-05
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded the approved Gate A removal of `verified → checked`, the
exhaustive matrix, retraction preservation behavior, and surface/fixture audit.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded the Phase 1C forward-correction audit, its one
in-memory test migration, and the absence of signed-fixture migration.
