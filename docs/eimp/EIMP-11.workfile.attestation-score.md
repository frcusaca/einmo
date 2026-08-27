# EIMP 11 Workfile — Compression Score and Attestation Claims

> **Design workfile, not normative specification.** This is the Gate D
> presentation behind EIMP 11 §S.15.

## Dependency position

This policy was narrowed after maintainer review. It does not depend on the
persistence architecture. Complete-stamp verification remains authoritative;
compression-score interface work is deferred and non-urgent.

## What this decision standardizes

This decision establishes einmo's forward-looking attestation rule: verified
identity claims come only from cryptographic evidence that a verifier can name
and check. A heuristic about passphrase text may be useful advice, but it is not
part of artifact validity, reviewer identity, or proof that a human was present.
The purpose is to keep library results, CLI language, tests, and future security
features from gradually turning an attractive number into an unsupported trust
claim.

## Proposed steady state — sign, classify, authorize, report

Here is the proposed attestation behavior from start to finish:

1. A reviewer selects a signing key and promotes checked content to verified.
2. Einmo derives or loads the key, signs the exact artifact, and records its
   public key in the stamp chain. Passphrase spelling or compressibility does
   not decide whether the artifact is valid.
3. Verification checks every stamp, rejects known computer-key attestation where
   a human is required, and—when configured—requires a stamp matching the
   expected reviewer public-key prefix.
4. Results report signer evidence and precise failures. They may retain the
   compression score under an explicitly heuristic name, but imply no entropy,
   uniqueness, identity, or humanity from it.
5. Any future second password-quality score runs before key use as a separately
   named advisory subsystem with its own threat model. It does not replace
   cryptographic signer evidence or silently become verification policy.
6. CI varies passphrase contents while holding keys and authorization policy
   constant, and proves computer-key and expected-reviewer failures remain
   authoritative.

Library users will notice fewer misleading fields and unrelated failures.
Integrations will notice removal of score output and threshold errors.
Developers gain a durable security-review rule: every trust claim identifies the
cryptographic evidence and verifier configuration that establishes it.

## Decision D — compression-derived passphrase score (resolved/deferred)

### Maintainer decision

Retain the compression score and its current API/dependencies. Do not describe
it as entropy, identity, or proof of human participation. Improving the
interface with a second password-quality measure is a possible EIMP-E design,
but is not urgent EIMP 11 work.

### Evidence

- `einmo-tools/src/lib.rs:10-26` rejects a score at or below zero.
- `src/suite.rs:156-185` and `src/review.rs:970-1001` turn that result into a
  blocking verification failure.
- `einmo-tools/src/lib.rs:29-60` calculates compressed-size delta against the
  existing verified corpus divided by passphrase bytes. This is corpus-relative
  compressibility, not guessing cost, entropy, identity, or human participation.
- `README.md:589` currently overclaims a minimum level of uniqueness.
- `src/transitions.rs:20-35` exposes the score in `Promoted`, making an
  unreliable diagnostic look like a durable security result.
- Root `Cargo.toml:84` and `einmo-tools/Cargo.toml:8` show that `einmo-tools`
  and `zstd` exist for this mechanism; their removal would also support §S.17's
  dependency-surface goal.

Cryptographic identity remains possession of the private key verified against
the expected reviewer key. The known computer key remains a public label used
to reject computer attestation, not a trust anchor.

## Alternatives

1. Warning-only: least disruptive, but preserves false assurance, unstable
   corpus-relative numbers, corpus reads, and `zstd`.
2. Blocking: current behavior; rejects legitimate keys for an unrelated metric
   while allowing weak but incompressible passphrases.
3. Conventional password-strength estimator: potentially useful advice before
   key derivation, but still cannot establish identity or human presence and
   belongs in a separate local-advisory design if desired.

## Consequences of the decision

- complete signature, expected-reviewer, and computer-key checks remain the
  source of verified attestation identity claims;
- expected-reviewer-key and known-computer-key checks remain authoritative;
- retain `Promoted::passphrase_score`, `einmo-tools`, and `zstd` for now;
- rewrite README and tool documentation to name the score as a compression
  heuristic and remove uniqueness/identity/humanity claims;
- defer changes to blocking behavior or a second quality score to EIMP-E, where
  units, threat model, false results, compatibility, and advisory-versus-policy
  status must be decided together.

## Impact descriptors

### Einmo Library User Experience changes

`PromoteOutcome`/`Promoted` retain the passphrase score. Callers must treat it as
a compression heuristic, not identity or entropy evidence. Verified identity
continues to depend on signature verification, expected reviewer identity, and
computer-key rules.

### Einmo Integration changes

CLI and JSON consumers retain the score. Documentation loses “minimum
uniqueness,” identity, and humanity language. `einmo-tools` and `zstd` remain.

### Einmo Development changes

Security reviews must reject metrics that are presented as identity or entropy
without a threat model and validated meaning. Any future password-strength
advice is a separate pre-key-derivation UX facility, typed as advisory and kept
out of signed promotion results and verification policy.

Developers must understand the separation between key-derivation UX, signature
validity, signer classification, expected-reviewer authorization, and
human-attestation policy. Tests target those layers independently. A new metric
cannot become blocking or appear in a durable result merely because it
correlates with a desirable property; it needs a threat model, defined units,
false-positive/false-negative analysis, and an explicit advice-versus-authority
decision.

### Migration

None for retained API or artifacts. Documentation must stop making unsupported
identity, humanity, entropy, or uniqueness claims. Any later score/interface
change belongs to EIMP-E and supplies its own migration.

## Last Updated

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Reassigned any non-urgent password-quality interface work to the
renamed envelope/attestation candidate EIMP-E.

**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Recorded the maintainer decision to retain the compression score
and dependencies, restrict its claims to a heuristic, and defer any second
password-quality interface to non-urgent EIMP-E work.


**Date**: 2026-08-27
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Added a forward-facing attestation standard and complete lifecycle
that makes named cryptographic evidence authoritative and keeps passphrase
heuristics outside artifact validity and identity claims.
