# EIMP-31 Design Supplement: Human Understanding and Responsible Action

## Phase 2 Goal

Complex software can now change faster than a human can responsibly inspect
every generated line. EIMP 31 must not solve that mismatch by treating a green
command, a signature, or a single approval click as proof of human judgment.

The Phase 2 goal is to provide mechanisms that help humans:

1. understand the system under test;
2. understand how the system is being developed; and
3. behave responsibly and effectively when deciding whether implementation,
   validation, or policy should change.

This is not an attempt to eliminate automation or require exhaustive manual
review. It is an attempt to make scarce human attention land on the decisions
where correctness and propriety depend on judgment.

## Correctness and Propriety

**Correctness** asks whether the implementation satisfies the behavior and
invariants the validation claims to check.

**Propriety** asks whether the change and the decision around it are
appropriate: was a behavioral change intended, was an exception justified,
was an expectation revised through the right authority, and is the resulting
claim honest about its scope?

An implementation may produce technically expected bytes while still arriving
there through an improper process—for example, by editing the expectation to
match a regression. Conversely, a deliberate behavior change may initially
fail old expectations while being entirely appropriate once a human reviews
the new specification and updates the verifier through the authorized path.

## What the Human Needs to See

### A model of the protected system

The interface should present suites and cases as a structured map of
capabilities, scenarios, and requirements, not only a flat list of filenames.
It should show why an important case exists, which specification or risk it
addresses, and where it sits in a hierarchy or sequence.

This view is grounded by EIMP 31 contracts 5.1 and 6.1–6.7.

### A model of development since the last relevant success

Before showing test results, einmo should identify:

- the last successful record relevant to the selected inventory/profile;
- the prior and current subject commits;
- whether current history descends from the prior commit;
- commits added, removed, or replayed through cherry-pick/rebase;
- changed components and dependencies;
- changes to the verifier, expectations, inventory, or execution policy; and
- new areas with no mapped protected coverage.

This is not a demand that a human read every diff. It is a change briefing that
makes consequential discontinuities hard to miss. It is grounded by contracts
5.2, 5.7, and 6.7.

### A discrepancy packet rather than a wall of failures

When validation fails, the primary report should contain:

1. a one-sentence assurance outcome: no successful record was created;
2. the exact subject/verifier/inventory tuple;
3. the earliest or most central discrepancies, grouped by structured system
   area and sequence dependency;
4. expected versus generated evidence for those discrepancies;
5. likely downstream/blocked cases kept distinct from independent failures;
6. the relevant development delta and last passing subject; and
7. direct access to every raw result and reproduction command.

Summarization helps route attention but never removes leaf evidence. This is
grounded by contracts 3.1, 3.2, 5.3, and 6.4–6.6.

## Responsible Resolution Paths

The human should see four clearly distinct choices.

### 1. Repair the subject

Use this when the protected behavior is still correct and the implementation
regressed. The current run remains failed and unsigned. The developer or agent
produces a new subject commit, and validation starts again against that new
identity.

### 2. Revise the validation implementation

Use this when the test, adapter, fixture, or reviewed expectation is wrong or
when intended product behavior changed. The change occurs in authoring mode,
uses existing einmo generation/review/promotion procedures, receives separate
review, and becomes a new clean validation-repository commit before official
validation.

### 3. Change protected policy

Use this when a case should become required, be replaced, or receive an
explicitly authorized exception. This creates and activates a new secured
inventory version. It is not a convenient side effect of a failing test run.

### 4. Stop without assurance

Use this when the evidence is insufficient or the decision should be deferred.
Generated diagnostics remain available under retention policy, and no success
record is created.

These choices implement contract 5.4. Permissions and presentation should
make their different consequences unmistakable.

## Escalation Is More Than Printing a Failure

Einmo core should emit a typed attention event for validation failure, history
divergence, uncovered change, or insufficient evidence. It should also return a
conspicuous non-success result locally. This is what the library can guarantee.

A deployment may route that event into an issue tracker, email, chat, a release
dashboard, or another human workflow. “Event emitted,” “notification
delivered,” and “human acknowledged” are three different states. The system
must not claim that a human was alerted unless the configured integration can
support that claim. This implements contract 5.9 while preserving offline and
library-only use.

## Proposal Is Not Approval

An agent may:

- prepare a subject fix;
- propose new or revised validation cases;
- explain a discrepancy;
- identify likely affected components;
- suggest an inventory change; and
- assemble evidence for a reviewer.

The system must preserve that these are proposals. It records separately who
approved validation expectations, who authorized an inventory transition, and
who signed the successful assurance claim. This implements contract 5.5.

The goal is not ritual separation for its own sake. It prevents an agent from
writing an implementation change, weakening the independent oracle, and then
presenting its own combined work as human-reviewed assurance.

## Review Rationale

For consequential decisions, einmo should make it easy to record:

- what the reviewer understood to have changed;
- which requirements or risks were considered;
- which generated, diff, history, or test evidence was inspected;
- whether the decision repairs the subject, revises an expectation, or changes
  policy;
- the exact identities in scope; and
- remaining uncertainty or follow-up work.

This rationale should be concise and structured enough to read later. It is
evidence of what was asserted and by whom, not proof that the reviewer was
careful. This implements contract 5.6.

## Safe Defaults and Human Behavior

The interface should encourage responsible behavior through mechanics:

- put inspection before mutation;
- require an explicit transition between authoring and official validation;
- never offer “accept all generated output” as the default failure response;
- display when a credential changes the level of authority being exercised;
- keep reproduction available without approval credentials;
- surface history divergence, coverage reduction, new exemptions, and stale
  evidence prominently;
- require rationale proportional to the consequence of the action; and
- make “stop and leave unsigned” an ordinary, non-punitive outcome.
- distinguish local attention events, delivered notifications, and human
  acknowledgements.

These are product requirements, not merely documentation advice. They are
grounded by contracts 5.4–5.8.

## What Einmo Cannot Guarantee

Einmo cannot guarantee that:

- a human read or understood a report;
- a reviewer is independent merely because a different name signed;
- a protected test is semantically adequate;
- an automatically produced change explanation is complete;
- organizational incentives reward responsible decisions; or
- an authorized person will never approve an improper change.

Einmo can make evidence precise, gaps conspicuous, authority separable,
shortcuts explicit, and later accountability possible. The assurance claim
must remain no broader than that.

## Story-Driven Validation of This Design

Every significant user story should answer:

1. What does the human understand about the system before acting?
2. What does the human understand about development since the last success?
3. What evidence receives attention, and what raw evidence remains available?
4. Which resolution choices are offered?
5. Which actor may propose, approve, activate, and sign each choice?
6. What exact claim exists after the choice—and what claim does not exist?
7. How does the discrepancy reach the responsible human, and what delivery or
   acknowledgement evidence actually exists?

The removed-commit story is the first adversarial example of this framework.
Future stories should include a legitimate behavior change, a necessary test
retirement, an agent-proposed inventory expansion, a platform-specific
exception, a compromised signer, and a release performed with known uncovered
new code.

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
**Changes**: Created the EIMP 21 Phase 2 human-responsibility design supplement,
defining correctness versus propriety, the protected-system and development-
delta views, discrepancy packets, four explicit resolution paths, proposal/
approval separation, review rationale, safe interaction defaults, honest
limits, actionable escalation with separate event/delivery/acknowledgement
states, and questions every user story must answer.
