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

## Regression repair verification — 2026-09-16

The replacement fixture now creates the checked source through the existing
promotion helper before recording either decision. Its assertion checks the
surviving action's case and verified destination, not only the action count.
The separate output-only shortcut-refusal test still passes.

Full run `cf60c2de-a828-4c66-95f7-af6ee88f490a` completed in 383.684 seconds:
445 attempted, 438 passed, seven failed, zero skipped. Formatting and strict
workspace clippy passed. The failures all exercise socket binding in this
restricted environment:

- `private_socket_path_can_be_served_over`
- `serve_tcp_end_to_end_enforces_the_bearer_token`
- `serve_uds_end_to_end_and_cleans_up_on_shutdown`
- `serve_uds_rebinds_a_stale_socket_file`
- `serve_uds_refuses_a_live_socket`
- `acquire_refuses_a_second_lock_while_the_first_socket_is_live`
- `run_serve_refuses_when_suite_lock_is_held`

TCP and the two suite-lock failures explicitly report OS error 1,
`Operation not permitted`; the remaining tests fail their socket-binding
assertions. An unrestricted run is required to close the full gate.

`cargo test --workspace --doc` succeeded for all three crates (zero doctests
defined). EIMP numbering validation and `git diff --check` also passed.

## Last Updated

**Date**: 2026-09-16
**Updated By**: OpenAI Codex (GPT-6)
**Changes**: Corrected the fixture-audit record: the server decision-replacement
test also depended on the removed output-to-verified shortcut. Its fixture now
promotes output to checked before recording decisions and asserts that the
replacement verified decision survives. Isolated reproduction failed with
HTTP 400 versus expected 200; focused replacement and shortcut-refusal tests
then both passed (run `1042ca21-b516-41e4-9643-8e9e2faf2755`). Earlier claims
that the full gate passed were premature and are corrected in the plan.

**Date**: 2026-09-05
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded the approved Gate A removal of `verified → checked`, the
exhaustive matrix, retraction preservation behavior, and surface/fixture audit.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded the Phase 1C forward-correction audit, its one
in-memory test migration, and the absence of signed-fixture migration.
