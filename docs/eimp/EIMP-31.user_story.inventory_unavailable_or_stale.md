# EIMP-31 User Story: The Protected Inventory Is Unavailable or Stale

## Story

A verifier begins an official run while the separately secured inventory
location is unavailable. A locally cached inventory exists, but the runner must
know whether it is the exact policy-required version or merely an old valid
version that is easier to pass.

The design should preserve offline use without turning an outage into a
fail-open rollback.

## Variant A — The Exact Required Inventory Is Available Locally

Release policy explicitly requires authenticated inventory `I5`. The local
machine already has:

- the immutable bytes of `I5`;
- a valid authentication chain for `I5`;
- authenticated activation/supersession state sufficient to establish that
  `I5` is applicable to the requested subject/profile; and
- the compatible clean validation revision.

The secured remote location is offline, but the run does not need a network
round trip merely to retrieve objects already available and verifiable
locally. Einmo may proceed and bind the exact `I5` digest and policy state into
the successful record. **Contracts relied upon: 1.3, 1.6, 2.4, 3.4.**

If current policy requires an online freshness or revocation check, that is an
explicit policy input; the runner cannot pretend it completed such a check.

## Variant B — Only a Cached `current` Label Is Available

The local cache says `current → I4`, but the runner cannot authenticate whether
`I5` was activated after that cache entry was written. The content of `I4` is
itself correctly signed.

Einmo refuses official validation. A valid signature on `I4` proves its origin,
not that `I4` remains current. The run does not substitute an empty inventory,
guess that `I4` is probably adequate, or search for any inventory the subject
can pass. **Contracts relied upon: 1.5, 1.6, 3.8.**

Because execution prerequisites were not established, the failure may occur
before test generation. The diagnostic output is a typed inventory-resolution
failure rather than a fabricated test result. No assurance record is created.
**Contracts relied upon: 3.2, 5.3, 5.9.**

## Variant C — An Older Valid Inventory Is Deliberately Supplied

An agent knows `I5` contains new adversarial cases and supplies authenticated
older inventory `I4`. Policy requires `I5` for this subject/profile.

The runner reports the exact mismatch:

```text
required inventory:  I5 / sha256:<new>
supplied inventory:  I4 / sha256:<old>
authentication:      valid for I4
policy applicability: rejected
validation outcome:  no run / no successful record
```

This is not reported as a signature failure. The artifact is authentic but
inapplicable. **Contracts relied upon: 1.6, 3.6, 3.8, 4.5.**

## Variant D — The Inventory Bytes Are Corrupt or Unauthenticated

The runner obtains something labeled `I5`, but canonical parsing, digest, or
authentication fails. Validation stops before protected cases execute. No
fallback inventory is selected. **Contracts relied upon: 1.3, 1.5.**

## Human View

The human needs to distinguish:

| State | Meaning | May official validation proceed? |
|---|---|---|
| Exact required inventory, authenticated and applicable | Required policy input is locally available | Yes, subject to the rest of validation |
| Valid older inventory | Authentic historical requirements, not current requirements | No |
| Cached mutable label with unverifiable freshness | Current requirement is unknown | No |
| Corrupt or unauthenticated inventory | Requirement bytes cannot be trusted | No |
| Secured service unreachable but online freshness required | Required policy check did not occur | No |

The attention event should say what is missing and how to restore validation,
not flood the user with ordinary test failures that never occurred. **Contracts
relied upon: 5.3, 5.4, 5.9.**

## Recovery

The human may:

1. restore access to the secured location;
2. obtain an authenticated offline bundle containing the required inventory
   and policy state;
3. select a different policy only through an independently authorized policy
   decision; or
4. stop without assurance.

The subject-writing agent cannot make the outage disappear by editing a local
inventory mirror. **Contracts relied upon: 1.2, 1.4–1.6, 5.4, 5.5.**

## Contract Trace

| Story event | Contracts |
|---|---|
| Use exact authenticated local inventory offline | 1.3, 1.6 |
| Refuse unverifiable `current` cache | 1.5, 1.6 |
| Reject authentic but stale inventory | 1.6, 3.6, 3.8, 4.5 |
| Stop on corrupt inventory | 1.3, 1.5 |
| Produce no signed validation result | 3.2 |
| Explain outage versus rollback versus corruption | 5.3, 5.9 |
| Require authorized policy change | 1.2, 1.4, 5.5 |

## Guarantees and Limits

Offline verification requires local trust state. If revocation, expiry, or
inventory activation can change remotely and policy demands knowledge of those
changes, a disconnected runner cannot honestly claim freshness.

EIMP 31 can define signed offline bundles and maximum acceptable age, but it
cannot make mutually disconnected systems share current state.

## Acceptance Scenarios Derived from This Story

- Exact authenticated required inventory can be used without network access
  when policy permits.
- An authenticated older inventory fails applicability without being mislabeled
  corrupt.
- A cached `current` label cannot substitute for authenticated selection state.
- Corrupt inventory cannot trigger fallback to an older version or empty set.
- Inventory-resolution failure invokes no assurance signer.
- Attention output distinguishes unavailability, corruption, and rollback.
- An offline bundle is accepted only for the identity, policy, and validity
  window it names.

## Questions Exposed by This Story

- What signs inventory activation and supersession state?
- Does inventory policy use expiry, monotonic sequence numbers, transparency,
  trusted time, or a combination to resist rollback?
- What is contained in an offline inventory/policy bundle?
- Which deployments require online revocation or freshness, and which accept a
  bounded offline validity window?
- Who may authorize a temporary policy change during a prolonged outage?

## Last Updated

**Date**: 2026-09-07
**Updated By**: OpenAI Codex (GPT-6)
**Changes**: Renumbered this proposal family from EIMP 21 to EIMP 31 at the
maintainer's request; updated active references and retained earlier history.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Renumbered this document from EIMP 11 to EIMP 21 and updated its internal references because EIMP 11 collided with another proposal.

**Date**: 2026-09-04
**Updated By**: OpenAI Codex (GPT-5)
**Changes**: Created the unavailable/stale-inventory story, distinguishing
legitimate offline use from cached-label ambiguity, rollback to an older valid
inventory, corruption, and required online freshness. Added recovery,
attention, and acceptance requirements.
