# EIMP-11 Phase 1B — Complete verified-attestation policy

This workfile records the implemented order-independent policy for
`stage:verified` stamps and the surface audit required by EIMP 11 §S.2.

## Policy result

`evaluate_verified_attestation` now returns one internal result containing:

- every verified signer observation in chain order;
- every computer-key offender, including duplicate offending stamps;
- whether a configured reviewer prefix matched any verified signer; and
- the final verdict.

The verdict is true exactly when at least one verified stamp exists, no
verified stamp uses the well-known computer key, and any configured reviewer
matches at least one verified stamp. Duplicate stamps do not alter those
conditions. Unexpected human co-signers remain visible in the envelope's
stamp chain and do not fail an artifact when the expected reviewer is present.

## Diagnostics

`Problem` now distinguishes a missing verified attestation from a missing
configured reviewer. Every problem has a stable machine-readable `kind`.
CLI JSON uses `serde_json` values and includes that kind, level, path, prose,
and remedy; this also prevents diagnostic strings from producing malformed
JSON through interpolation. One computer-key problem is emitted for every
offending stamp, independent of its position.

## Review and server surface audit

Neither `EinmoReview` nor the review server had a second policy for judging
stored verified stamps. Their `non_human` values describe the promotion that
just executed, while body/stamp-chain views expose stored signer identities;
they do not independently decide the verified gate. Consequently no duplicate
verdict implementation needed replacement. All consumers that judge suite
attestation continue through `check_suite_integrity`, which uses the shared
policy result.

## Evidence

The pre-fix run `114c2631-2dc8-46d7-84b2-506bde4850e4` demonstrated four
failures: human-then-computer passed incorrectly, an expected reviewer after
another human failed incorrectly, zero stamps passed incorrectly, and the
missing-reviewer diagnostic described only the first signer.

After implementation, focused run `7a47fb65-576a-40d3-a030-86503c1dbe36`
passed ten tests covering human-only, both human/computer orders, expected and
missing reviewers, zero stamps, duplicate human and computer stamps, and
stable JSON/prose diagnostics. Scoped `cargo-mutants` caught all five viable
policy mutations (`any`/stamp selection, both `all` conjunctions, empty-set
negation, and empty-result bypass); two replacement mutants were unviable.
The tool also included two pre-existing configuration merge mutants outside
the requested regex; those remain configuration-work findings.

The complete gate passed all 440 tests in nextest run
`19aa472b-dd5b-4323-9558-321cb23d0c1f`; formatting, strict workspace clippy,
and workspace doctests passed.

## Last Updated

**Date**: 2026-09-01
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded the complete verified-attestation policy, diagnostics,
review/server surface audit, test-first evidence, mutation results, and full
gate.
