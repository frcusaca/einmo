//! `EinmoCase` (`EIMP-7` §S.3): one case's full cross-stage bundle, plus
//! every operation performable on it as itself. Replaces `TestRow`
//! (`einmo_suite.rs`) and `review.rs`'s private `promote_one_accumulating`
//! — the two independent implementations `EIMP-1`'s P1 finding named.

use std::collections::BTreeMap;

use crate::config::MatchSections;
use crate::error::{EinmoError, Result};
use crate::format::EinmoFile;
use crate::signature::{StageKeypair, is_computer_key, now_iso8601};
use crate::stage::{EinmoId, Stage};
use crate::storage::{ArtifactLocation, EinmoStorage};
use crate::verify::verify_bytes;

/// One case's full cross-stage bundle: its id, and every operation
/// performable on it. Borrows its [`EinmoStorage`] rather than owning it —
/// cheap to construct per-id inside a loop, generic over `S` so tests can
/// plug in an in-memory fake instead of a tempdir.
pub struct EinmoCase<'s, S: EinmoStorage> {
    id: EinmoId,
    storage: &'s S,
}

/// What reading one stage's artifact found — the intermediate result
/// [`EinmoCase::agreement`] builds its [`StagePairAgreement`]s from.
/// Deliberately distinguishes `Tampered` from `Absent`: a stage that
/// exists but fails verify-on-inspect is not the same claim as a stage
/// with nothing there at all.
enum StageRead {
    Absent,
    Present(Box<EinmoFile>),
    Tampered,
}

/// The outcome of one [`EinmoCase::promote`] call — replaces the
/// `(String, bool)` tuple `review.rs`'s `promote_one_accumulating`
/// returned with a named enum, per this crate's state/status/error-is-an-
/// enum convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PromoteOutcome {
    /// A fresh baseline was written (absent, corrupt, or genuinely
    /// different content at the destination).
    Promoted {
        /// `true` if the appended `verified` stamp used a well-known
        /// computer key (a non-human attestation).
        non_human: bool,
        /// Passphrase score if applicable.
        passphrase_score: Option<f64>,
    },
    /// Content at the destination matched; this signer's stamp was
    /// appended onto the existing file, preserving prior signers'.
    CoSigned {
        /// As [`PromoteOutcome::Promoted::non_human`].
        non_human: bool,
        /// Passphrase score if applicable.
        passphrase_score: Option<f64>,
    },
    /// The destination already carried this exact content, already signed
    /// by this exact key. Nothing written.
    AlreadySigned {
        /// As [`PromoteOutcome::Promoted::non_human`] — computed from
        /// `to`/`key` regardless of outcome, so a no-op promotion to
        /// `verified` under the computer key still reports accurately
        /// that the resulting state is non-human-attested, even though
        /// nothing new was written this call.
        non_human: bool,
        /// Passphrase score if applicable.
        passphrase_score: Option<f64>,
    },
}

/// How ONE pair of stages stands for ONE case — the per-case projection of
/// `compare.rs`'s section-aware comparison (`EIMP-7` §S.3/§S.7). Deliberately
/// an enum: the four outcomes are mutually exclusive, and `Tampered` must
/// never be collapsed into `Differ` — the distinction `TestRow`/`scan_tests`
/// loses today by reducing everything to one `differing: bool`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagePairAgreement {
    /// Both present; every section required by the policy is
    /// byte-identical.
    Agree,
    /// Both present; these policy-required sections diverged. Never
    /// empty — an empty divergence list is [`StagePairAgreement::Agree`].
    Differ {
        /// The diverged section names.
        sections: Vec<String>,
    },
    /// Exactly one side is present.
    OneSided {
        /// The stage that has an artifact.
        present: Stage,
        /// The stage that does not.
        absent: Stage,
    },
    /// Neither side is present.
    BothAbsent,
    /// At least one side failed verify-on-inspect. The case is refused,
    /// never compared — matching `compare.rs`'s `tampered` bucket.
    Tampered {
        /// Which of the pair's two stages are tampered (one or both).
        stages: Vec<Stage>,
    },
}

/// Every pairwise agreement a case needs, computed in one pass over the
/// stages the caller asked about. [`EinmoCase::agreement`] returns this;
/// each consumer reads the pairs it actually cares about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageAgreement {
    /// Which of the requested stages have SOMETHING present (verified or
    /// tampered — anything but absent).
    pub present: Vec<Stage>,
    /// Which of the requested stages are absent.
    pub missing: Vec<Stage>,
    /// The policy this agreement was computed under — recorded, not
    /// assumed, so a caller can never compare two `StageAgreement`s
    /// derived under different [`MatchSections`] and think they disagree
    /// about the suite when they disagree about the question.
    pub policy: MatchSections,
    pairs: BTreeMap<(Stage, Stage), StagePairAgreement>,
}

impl StageAgreement {
    /// How `left` and `right` stand. `None` if that ordered pair was not
    /// among the stages requested.
    #[must_use]
    pub fn pair(&self, left: Stage, right: Stage) -> Option<&StagePairAgreement> {
        self.pairs.get(&(left, right))
    }
}

impl<'s, S: EinmoStorage> EinmoCase<'s, S> {
    /// Bind to `id` over `storage`.
    #[must_use]
    pub fn new(id: EinmoId, storage: &'s S) -> Self {
        EinmoCase { id, storage }
    }

    /// This case's id.
    #[must_use]
    pub fn id(&self) -> &EinmoId {
        &self.id
    }

    /// Read one location's artifact, verify-on-inspect, `None` if absent.
    ///
    /// # Errors
    /// Returns [`EinmoError::Verification`] or [`EinmoError::Parse`] if the
    /// artifact exists but fails verify-on-inspect — never returned
    /// silently as absent. Propagates any [`EinmoStorage`] I/O failure.
    pub fn read(&self, at: ArtifactLocation) -> Result<Option<EinmoFile>> {
        match self.storage.read(&self.id, at)? {
            None => Ok(None),
            Some(bytes) => verify_bytes(&bytes).map(Some),
        }
    }

    fn read_stage_for_agreement(&self, stage: Stage) -> Result<StageRead> {
        match self
            .storage
            .read(&self.id, ArtifactLocation::Stage(stage))?
        {
            None => Ok(StageRead::Absent),
            Some(bytes) => match verify_bytes(&bytes) {
                Ok(file) => Ok(StageRead::Present(Box::new(file))),
                Err(_) => Ok(StageRead::Tampered),
            },
        }
    }

    /// The per-stage presence/status facts `TestRow`/`scan_tests` compute
    /// today — same shape, still needed for `einmo list`'s existing
    /// display. `(stage, status-string-if-present)`, `"TAMPERED"` if
    /// present but verify-on-inspect fails.
    ///
    /// # Errors
    /// Propagates any [`EinmoStorage`] I/O failure.
    pub fn stages(&self) -> Result<Vec<(Stage, Option<String>)>> {
        let mut out = Vec::with_capacity(Stage::ALL.len());
        for stage in Stage::ALL {
            let status = match self.read_stage_for_agreement(stage)? {
                StageRead::Absent => None,
                StageRead::Present(file) => Some(file.metadata().status.to_string()),
                StageRead::Tampered => Some("TAMPERED".to_string()),
            };
            out.push((stage, status));
        }
        Ok(out)
    }

    /// Structured, section-aware, policy-driven stage-agreement facts for
    /// every ordered pair drawn from `stages`, under `policy` (recorded in
    /// the result). Internally this is `compare.rs`'s `compare_sections`
    /// applied per-pair to ONE case rather than `compare`'s whole-tree
    /// walk — same policy, same required-section rules, same
    /// verify-on-inspect refusal, one case at a time. `EIMP-7` §S.7 folds
    /// `compare::compare` itself onto this same core.
    ///
    /// # Errors
    /// Propagates any [`EinmoStorage`] I/O failure.
    pub fn agreement(&self, stages: &[Stage], policy: MatchSections) -> Result<StageAgreement> {
        let mut reads = Vec::with_capacity(stages.len());
        for &stage in stages {
            reads.push((stage, self.read_stage_for_agreement(stage)?));
        }

        let present = reads
            .iter()
            .filter(|(_, r)| !matches!(r, StageRead::Absent))
            .map(|(s, _)| *s)
            .collect();
        let missing = reads
            .iter()
            .filter(|(_, r)| matches!(r, StageRead::Absent))
            .map(|(s, _)| *s)
            .collect();

        let mut pairs = BTreeMap::new();
        for (li, (left, lread)) in reads.iter().enumerate() {
            for (ri, (right, rread)) in reads.iter().enumerate() {
                if li == ri {
                    continue;
                }
                let tampered_stages: Vec<Stage> = [
                    matches!(lread, StageRead::Tampered).then_some(*left),
                    matches!(rread, StageRead::Tampered).then_some(*right),
                ]
                .into_iter()
                .flatten()
                .collect();

                let agreement = if !tampered_stages.is_empty() {
                    StagePairAgreement::Tampered {
                        stages: tampered_stages,
                    }
                } else {
                    match (lread, rread) {
                        (StageRead::Absent, StageRead::Absent) => StagePairAgreement::BothAbsent,
                        (StageRead::Absent, StageRead::Present(_)) => {
                            StagePairAgreement::OneSided {
                                present: *right,
                                absent: *left,
                            }
                        }
                        (StageRead::Present(_), StageRead::Absent) => {
                            StagePairAgreement::OneSided {
                                present: *left,
                                absent: *right,
                            }
                        }
                        (StageRead::Present(lf), StageRead::Present(rf)) => {
                            let diverged = crate::compare::compare_sections(lf, rf, policy);
                            if diverged.is_empty() {
                                StagePairAgreement::Agree
                            } else {
                                StagePairAgreement::Differ { sections: diverged }
                            }
                        }
                        (StageRead::Tampered, _) | (_, StageRead::Tampered) => {
                            unreachable!("handled above: tampered_stages would be non-empty")
                        }
                    }
                };
                pairs.insert((*left, *right), agreement);
            }
        }

        Ok(StageAgreement {
            present,
            missing,
            policy,
            pairs,
        })
    }

    /// Promote from `from` to `to`, accumulating onto whatever already
    /// exists at `to` if its content matches (multi-signer safe) — this
    /// is `review.rs`'s `promote_one_accumulating` logic, ported to read
    /// and write through [`EinmoStorage`] instead of a filesystem path
    /// directly.
    ///
    /// `pub(crate)`, not `pub`: takes an already-derived `&StageKeypair`
    /// (itself `pub(crate)` — raw key material never crosses this crate's
    /// public boundary) rather than a `KeySource`, so a batch caller
    /// (`EinmoSuite::promote`, `EIMP-7` §S.10) can derive ONCE — Argon2id
    /// is ~1.8s by design — and lend the same keypair to every case in the
    /// batch. `EinmoSuite::promote` is the public entry point.
    ///
    /// **The destination-match test here is deliberately NOT
    /// [`MatchSections`]-policy-driven**, unlike [`Self::agreement`]: it
    /// compares every non-STAMPS section (`einmo_suite::body_sections`),
    /// carried over unchanged from `promote_one_accumulating`. Promotion
    /// writes bytes and appends an attestation, so "the destination
    /// already holds exactly this content" must mean *exactly*, including
    /// sections a comparison policy is willing to overlook — co-signing a
    /// file whose COMMENTS differ from what the signer actually reviewed
    /// would make the stamp attest to content that was never inspected.
    ///
    /// # Errors
    /// Returns [`EinmoError::IllegalTransition`] for a disallowed
    /// `(from, to)` pair, [`EinmoError::Verification`] if the source is
    /// absent or fails verify-on-inspect. Propagates any [`EinmoStorage`]
    /// I/O failure.
    pub(crate) fn promote(
        &self,
        from: Stage,
        to: Stage,
        key: &StageKeypair,
    ) -> Result<PromoteOutcome> {
        if !crate::transitions::is_legal_transition(from, to) {
            return Err(EinmoError::IllegalTransition {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        let non_human = to == Stage::Verified && is_computer_key(&key.pubkey_hex());

        let src_bytes = self
            .storage
            .read(&self.id, ArtifactLocation::Stage(from))?
            .ok_or_else(|| {
                EinmoError::Verification(format!(
                    "{}: no artifact at the {from} stage to promote",
                    self.id
                ))
            })?;
        let src_file = verify_bytes(&src_bytes)?; // verify-on-inspect the source

        // Absent OR corrupt (tampered/unparsable) destination is treated
        // as absent, matching promote_one_accumulating's own `.ok()`.
        let existing = self
            .storage
            .read(&self.id, ArtifactLocation::Stage(to))?
            .and_then(|bytes| verify_bytes(&bytes).ok());

        if let Some(existing) = existing {
            let sections_same = crate::einmo_suite::body_sections(&existing, None)
                == crate::einmo_suite::body_sections(&src_file, None);
            if sections_same {
                let my_pubkey = key.pubkey_hex();
                if existing
                    .stamps()
                    .has_stage_stamp_from(to.stamp_key(), &my_pubkey)
                {
                    // True no-op: destination already reflects this exact
                    // content, already signed by this exact key.
                    return Ok(PromoteOutcome::AlreadySigned {
                        non_human,
                        passphrase_score: None,
                    });
                }
                // Content matches, new signer: append onto the EXISTING
                // file, preserving every prior stamp.
                let mut appended = existing;
                appended.append_stage_stamp_with(to.stamp_key(), key);
                let bytes = appended.serialize()?;
                self.storage
                    .write(&self.id, ArtifactLocation::Stage(to), &bytes)?;
                return Ok(PromoteOutcome::CoSigned {
                    non_human,
                    passphrase_score: None,
                });
            }
        }

        // Absent, corrupt, or genuinely different content: a fresh
        // baseline — carry the source's own (already verified) stamp
        // chain forward and append exactly one new destination stamp.
        let mut file = src_file;
        file.append_stage_stamp_with(to.stamp_key(), key);
        let bytes = file.serialize()?;
        self.storage
            .write(&self.id, ArtifactLocation::Stage(to), &bytes)?;
        Ok(PromoteOutcome::Promoted {
            non_human,
            passphrase_score: None,
        })
    }

    /// Move this case's `stage` artifact into `stage`'s nested flagged
    /// sink (`EIMP-7` §S.2a), appending an unsigned advisory block.
    /// Re-flagging CONCATENATES a new dated block on top of whatever is
    /// already flagged there, rather than replacing it — ported from
    /// `transitions::flag`'s per-file logic.
    ///
    /// # Errors
    /// Returns [`EinmoError::Verification`] if the source is absent or
    /// fails verify-on-inspect. Propagates any [`EinmoStorage`] I/O
    /// failure.
    pub fn flag(&self, stage: Stage, reason: &str) -> Result<()> {
        let src_bytes = self
            .storage
            .read(&self.id, ArtifactLocation::Stage(stage))?
            .ok_or_else(|| {
                EinmoError::Verification(format!(
                    "{}: no artifact at the {stage} stage to flag",
                    self.id
                ))
            })?;
        let mut file = verify_bytes(&src_bytes)?; // verify-on-inspect before moving
        let new_block = format!("# flagged: {reason} {}", now_iso8601());

        let existing_advisory = self
            .storage
            .read(&self.id, ArtifactLocation::Flagged(stage))?
            .and_then(|bytes| verify_bytes(&bytes).ok())
            .and_then(|existing| existing.advisory().map(str::to_string));
        let advisory = match existing_advisory {
            Some(existing) => format!("{new_block}\n{existing}"),
            None => new_block,
        };
        file.set_advisory(advisory);

        let bytes = file.serialize()?;
        self.storage
            .write(&self.id, ArtifactLocation::Flagged(stage), &bytes)?;
        // Move semantics: remove from origin.
        self.storage
            .remove(&self.id, ArtifactLocation::Stage(stage))?;
        Ok(())
    }

    /// Retract (demote) this case from `stage`, cascading `checked` →
    /// `verified` (a verified stamp attests to a specific checked
    /// baseline, so pulling the baseline leaves it dangling). Returns the
    /// stages actually removed (a target absent to begin with is not
    /// reported), highest-first — ported from `transitions::retract`'s
    /// per-file logic.
    ///
    /// # Errors
    /// Returns [`EinmoError::Config`] if `stage` is `output` (regenerated
    /// every run, nothing to un-promote). Propagates any [`EinmoStorage`]
    /// I/O failure.
    pub fn retract(&self, stage: Stage) -> Result<Vec<Stage>> {
        let cascade: &[Stage] = match stage {
            Stage::Verified => &[Stage::Verified],
            Stage::Checked => &[Stage::Verified, Stage::Checked],
            Stage::Output => {
                return Err(EinmoError::Config(
                    "cannot retract from output/: it is regenerated every run".into(),
                ));
            }
        };
        let mut retracted = Vec::new();
        for &target in cascade {
            if self
                .storage
                .read(&self.id, ArtifactLocation::Stage(target))?
                .is_some()
            {
                self.storage
                    .remove(&self.id, ArtifactLocation::Stage(target))?;
                retracted.push(target);
            }
        }
        Ok(retracted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{DEFAULT_SEPARATOR, Metadata, Section, Status};
    use crate::signature::{Stamps, derive_keypair};
    use crate::storage::InMemoryStorage;

    /// A minimal signed envelope's bytes, matching the
    /// compiled→configured→stage:output chain other test suites in this
    /// crate build by hand (e.g. `transitions.rs`'s `write_output`) — no
    /// filesystem, so it can be handed straight to any `EinmoStorage`.
    fn signed_bytes(rel: &str, output: &str, comments: &str) -> Vec<u8> {
        let bodies = vec![
            Section::new("INPUT", "{5;}"),
            Section::new("OUTPUT", output),
            Section::new("COMMENTS", comments),
        ];
        let meta = Metadata {
            test: rel.into(),
            suite: "s".into(),
            producer: "abc".into(),
            producer_diff: String::new(),
            generated: "2026-07-11T07:00:00Z".into(),
            status: Status::Normal,
            status_detail: String::new(),
            reference: String::new(),
            sections: vec![
                "INPUT".into(),
                "OUTPUT".into(),
                "COMMENTS".into(),
                "STAMPS".into(),
            ],
        };
        let mut file = EinmoFile::new("utf-8", DEFAULT_SEPARATOR, meta, bodies, Stamps::new());
        let (configured, _) = derive_keypair("cfg");
        let (stage, _) = derive_keypair("");
        file.set_stamps(Stamps::generate(&file.signed_prefix(), &configured, &stage));
        file.serialize().unwrap()
    }

    fn id(rel: &str) -> EinmoId {
        EinmoId::from_input_rel(std::path::Path::new(rel)).unwrap()
    }

    // ---- agreement() ----

    #[test]
    fn agreement_all_present_and_agree() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        let bytes = signed_bytes("a.foo", "5", "");
        storage
            .write(&case_id, ArtifactLocation::Stage(Stage::Output), &bytes)
            .unwrap();
        storage
            .write(&case_id, ArtifactLocation::Stage(Stage::Checked), &bytes)
            .unwrap();

        let case = EinmoCase::new(case_id, &storage);
        let agreement = case
            .agreement(&[Stage::Output, Stage::Checked], MatchSections::InputOutput)
            .unwrap();
        assert_eq!(
            agreement.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::Agree)
        );
        assert_eq!(
            agreement.pair(Stage::Checked, Stage::Output),
            Some(&StagePairAgreement::Agree)
        );
        assert_eq!(agreement.present, vec![Stage::Output, Stage::Checked]);
        assert!(agreement.missing.is_empty());
    }

    #[test]
    fn agreement_differ_reports_the_diverged_section_names() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Checked),
                &signed_bytes("a.foo", "6", ""),
            )
            .unwrap();

        let case = EinmoCase::new(case_id, &storage);
        let agreement = case
            .agreement(&[Stage::Output, Stage::Checked], MatchSections::InputOutput)
            .unwrap();
        assert_eq!(
            agreement.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::Differ {
                sections: vec!["OUTPUT".to_string()]
            })
        );
    }

    #[test]
    fn agreement_one_sided_reports_which_side_is_present_both_directions() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();

        let case = EinmoCase::new(case_id, &storage);
        let agreement = case
            .agreement(&[Stage::Output, Stage::Checked], MatchSections::InputOutput)
            .unwrap();
        assert_eq!(
            agreement.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::OneSided {
                present: Stage::Output,
                absent: Stage::Checked,
            })
        );
        assert_eq!(
            agreement.pair(Stage::Checked, Stage::Output),
            Some(&StagePairAgreement::OneSided {
                present: Stage::Output,
                absent: Stage::Checked,
            })
        );
        assert_eq!(agreement.present, vec![Stage::Output]);
        assert_eq!(agreement.missing, vec![Stage::Checked]);
    }

    #[test]
    fn agreement_both_absent() {
        let storage = InMemoryStorage::new();
        let case = EinmoCase::new(id("a.foo"), &storage);
        let agreement = case
            .agreement(&[Stage::Output, Stage::Checked], MatchSections::InputOutput)
            .unwrap();
        assert_eq!(
            agreement.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::BothAbsent)
        );
        assert!(agreement.present.is_empty());
        assert_eq!(agreement.missing, vec![Stage::Output, Stage::Checked]);
    }

    /// The distinction `TestRow`/`scan_tests` loses today: a tampered
    /// artifact must be reported as `Tampered`, never folded into
    /// `Differ` just because its (unparsable) bytes don't match the other
    /// side's.
    #[test]
    fn agreement_tampered_is_never_folded_into_differ() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Checked),
                b"not a valid einmo envelope",
            )
            .unwrap();

        let case = EinmoCase::new(case_id, &storage);
        let agreement = case
            .agreement(&[Stage::Output, Stage::Checked], MatchSections::InputOutput)
            .unwrap();
        assert_eq!(
            agreement.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::Tampered {
                stages: vec![Stage::Checked]
            })
        );
    }

    /// The P1 repro (`EIMP-1.plan.md`): a fresh suite where output and
    /// checked agree and verified is simply unpopulated yet must read as
    /// `Agree`, not `Differ`/missing-flavored — this is the concrete bug
    /// `TestRow::differing` had (`true` whenever ANY stage was absent,
    /// including one nobody asked about).
    #[test]
    fn agreement_p1_repro_unpopulated_verified_does_not_affect_output_checked_agreement() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        let bytes = signed_bytes("a.foo", "5", "");
        storage
            .write(&case_id, ArtifactLocation::Stage(Stage::Output), &bytes)
            .unwrap();
        storage
            .write(&case_id, ArtifactLocation::Stage(Stage::Checked), &bytes)
            .unwrap();
        // Stage::Verified is deliberately never written.

        let case = EinmoCase::new(case_id, &storage);
        let agreement = case
            .agreement(&[Stage::Output, Stage::Checked], MatchSections::InputOutput)
            .unwrap();
        assert_eq!(
            agreement.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::Agree)
        );
    }

    /// The third finding from this EIMP's drafting: `einmo test` and
    /// `einmo review` must be able to agree or disagree about a
    /// COMMENTS-only difference CONSISTENTLY, under one recorded policy —
    /// not one crude bool computed over every non-STAMPS section.
    #[test]
    fn agreement_comments_repro_policy_controls_whether_comments_only_divergence_counts() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", "looks fine"),
            )
            .unwrap();
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Checked),
                &signed_bytes("a.foo", "5", "actually reconsider this"),
            )
            .unwrap();

        let case = EinmoCase::new(case_id, &storage);

        let under_input_output = case
            .agreement(&[Stage::Output, Stage::Checked], MatchSections::InputOutput)
            .unwrap();
        assert_eq!(
            under_input_output.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::Agree),
            "COMMENTS is not policy-required under InputOutput"
        );

        let under_input_output_comments = case
            .agreement(
                &[Stage::Output, Stage::Checked],
                MatchSections::InputOutputComments,
            )
            .unwrap();
        assert_eq!(
            under_input_output_comments.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::Differ {
                sections: vec!["COMMENTS".to_string()]
            })
        );
    }

    #[test]
    fn agreement_records_the_policy_it_was_computed_under() {
        let storage = InMemoryStorage::new();
        let case = EinmoCase::new(id("a.foo"), &storage);
        let agreement = case
            .agreement(
                &[Stage::Output, Stage::Checked],
                MatchSections::InputOutputComments,
            )
            .unwrap();
        assert_eq!(agreement.policy, MatchSections::InputOutputComments);
    }

    // ---- promote() ----

    fn derive(passphrase: &str) -> StageKeypair {
        StageKeypair::derive(passphrase)
    }

    #[test]
    fn promote_writes_a_fresh_baseline_when_destination_absent() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();

        let case = EinmoCase::new(case_id.clone(), &storage);
        let outcome = case
            .promote(Stage::Output, Stage::Checked, &derive(""))
            .unwrap();
        assert_eq!(
            outcome,
            PromoteOutcome::Promoted {
                non_human: false,
                passphrase_score: None
            }
        );

        let checked = case
            .read(ArtifactLocation::Stage(Stage::Checked))
            .unwrap()
            .expect("destination must now exist");
        assert_eq!(
            checked
                .stamps()
                .entries()
                .iter()
                .map(|s| s.key())
                .collect::<Vec<_>>(),
            vec!["compiled", "configured", "stage:output", "stage:checked"]
        );
    }

    #[test]
    fn promote_cosigns_when_destination_content_matches() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        let case = EinmoCase::new(case_id, &storage);
        // First signer.
        case.promote(Stage::Output, Stage::Checked, &derive("signer-one"))
            .unwrap();

        // A second, different signer promoting the SAME (unchanged)
        // content must co-sign, not clobber -- the multi-signer
        // accumulation this whole method exists to preserve.
        let outcome = case
            .promote(Stage::Output, Stage::Checked, &derive("signer-two"))
            .unwrap();
        assert_eq!(
            outcome,
            PromoteOutcome::CoSigned {
                non_human: false,
                passphrase_score: None
            }
        );

        let checked = case
            .read(ArtifactLocation::Stage(Stage::Checked))
            .unwrap()
            .unwrap();
        let checked_stamp_count = checked
            .stamps()
            .entries()
            .iter()
            .filter(|s| s.key() == "stage:checked")
            .count();
        assert_eq!(
            checked_stamp_count, 2,
            "both signers' stage:checked stamps must survive"
        );
    }

    #[test]
    fn promote_already_signed_by_this_key_is_a_true_noop() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        let case = EinmoCase::new(case_id, &storage);
        let key = derive("same-signer");
        case.promote(Stage::Output, Stage::Checked, &key).unwrap();
        let bytes_before = storage
            .read(case.id(), ArtifactLocation::Stage(Stage::Checked))
            .unwrap()
            .unwrap();

        // Re-promote identical content with the SAME key: must be a true
        // no-op, byte-for-byte.
        let outcome = case.promote(Stage::Output, Stage::Checked, &key).unwrap();
        assert_eq!(
            outcome,
            PromoteOutcome::AlreadySigned {
                non_human: false,
                passphrase_score: None
            }
        );
        let bytes_after = storage
            .read(case.id(), ArtifactLocation::Stage(Stage::Checked))
            .unwrap()
            .unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "re-promoting unchanged content under the same signer must not touch the file"
        );
    }

    #[test]
    fn promote_genuinely_different_content_writes_a_fresh_baseline_not_a_cosign() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Checked),
                &signed_bytes("a.foo", "999", ""),
            )
            .unwrap();
        let case = EinmoCase::new(case_id, &storage);
        let outcome = case
            .promote(Stage::Output, Stage::Checked, &derive(""))
            .unwrap();
        assert_eq!(
            outcome,
            PromoteOutcome::Promoted {
                non_human: false,
                passphrase_score: None
            }
        );
        let checked = case
            .read(ArtifactLocation::Stage(Stage::Checked))
            .unwrap()
            .unwrap();
        assert_eq!(checked.section("OUTPUT").unwrap().body(), "5");
    }

    /// The deliberate asymmetry (`EIMP-7` §S.3): unlike `agreement`,
    /// `promote`'s destination-match check is NOT `MatchSections`-policy
    /// driven -- it always compares every non-STAMPS section, including
    /// COMMENTS. A destination differing only in COMMENTS must NOT be
    /// treated as matching, even though `agreement` under
    /// `MatchSections::InputOutput` would call the same two files `Agree`.
    #[test]
    fn promote_deliberate_asymmetry_comments_only_difference_is_not_a_match() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", "reviewer's real comment"),
            )
            .unwrap();
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Checked),
                &signed_bytes("a.foo", "5", "a stale, different comment"),
            )
            .unwrap();
        let case = EinmoCase::new(case_id, &storage);

        // Sanity: agreement() under the default policy WOULD call these two
        // `Agree` (COMMENTS not policy-required) -- promote must not.
        let agreement = case
            .agreement(&[Stage::Output, Stage::Checked], MatchSections::InputOutput)
            .unwrap();
        assert_eq!(
            agreement.pair(Stage::Output, Stage::Checked),
            Some(&StagePairAgreement::Agree)
        );

        let outcome = case
            .promote(Stage::Output, Stage::Checked, &derive(""))
            .unwrap();
        assert_eq!(
            outcome,
            PromoteOutcome::Promoted {
                non_human: false,
                passphrase_score: None
            },
            "a COMMENTS-only difference must still count as genuinely different content for promote"
        );
        let checked = case
            .read(ArtifactLocation::Stage(Stage::Checked))
            .unwrap()
            .unwrap();
        assert_eq!(
            checked.section("COMMENTS").unwrap().body(),
            "reviewer's real comment",
            "the fresh baseline carries the SOURCE's comment, overwriting the stale one"
        );
    }

    #[test]
    fn promote_flags_non_human_for_the_computer_key_on_verified() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        let case = EinmoCase::new(case_id, &storage);
        // Empty passphrase derives the well-known computer key.
        let outcome = case
            .promote(Stage::Output, Stage::Verified, &derive(""))
            .unwrap();
        assert_eq!(
            outcome,
            PromoteOutcome::Promoted {
                non_human: true,
                passphrase_score: None
            }
        );
    }

    #[test]
    fn promote_errors_when_source_is_absent() {
        let storage = InMemoryStorage::new();
        let case = EinmoCase::new(id("a.foo"), &storage);
        let err = case
            .promote(Stage::Output, Stage::Checked, &derive(""))
            .unwrap_err();
        assert!(matches!(err, EinmoError::Verification(_)));
    }

    /// Found during Phase F's audit: this check was missing entirely from
    /// the initial port -- `EinmoCase::promote` must refuse an illegal
    /// `(from, to)` pair itself, the same guarantee `transitions::promote`
    /// always gave, since this is now a general-purpose primitive a CLI
    /// caller can invoke with any pair a user names.
    #[test]
    fn promote_refuses_an_illegal_transition() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Verified),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        let case = EinmoCase::new(case_id, &storage);
        let err = case
            .promote(Stage::Verified, Stage::Output, &derive(""))
            .unwrap_err();
        assert!(matches!(err, EinmoError::IllegalTransition { .. }));
    }

    // ---- flag() / retract() ----

    #[test]
    fn flag_moves_the_artifact_into_the_origin_stages_own_sink() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        let case = EinmoCase::new(case_id, &storage);

        case.flag(Stage::Output, "looks wrong").unwrap();

        assert!(
            case.read(ArtifactLocation::Stage(Stage::Output))
                .unwrap()
                .is_none(),
            "origin must be vacated"
        );
        let flagged = case
            .read(ArtifactLocation::Flagged(Stage::Output))
            .unwrap()
            .expect("flagged sink must hold the artifact");
        assert!(
            flagged
                .advisory()
                .unwrap()
                .starts_with("# flagged: looks wrong")
        );
    }

    #[test]
    fn flag_concatenates_with_an_existing_flagged_note() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        let case = EinmoCase::new(case_id.clone(), &storage);

        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        case.flag(Stage::Output, "first").unwrap();

        // Regenerate at output, flag again.
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        case.flag(Stage::Output, "second").unwrap();

        let flagged = case
            .read(ArtifactLocation::Flagged(Stage::Output))
            .unwrap()
            .unwrap();
        let advisory = flagged.advisory().unwrap();
        assert!(advisory.starts_with("# flagged: second"));
        assert!(advisory.contains("# flagged: first"));
    }

    #[test]
    fn retract_checked_cascades_to_verified_and_reports_only_what_existed() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        let case = EinmoCase::new(case_id.clone(), &storage);
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Checked),
                &signed_bytes("a.foo", "5", ""),
            )
            .unwrap();
        // No verified/ artifact exists.

        let retracted = case.retract(Stage::Checked).unwrap();
        assert_eq!(
            retracted,
            vec![Stage::Checked],
            "verified was never present, so it must not be reported as retracted"
        );
        assert!(
            case.read(ArtifactLocation::Stage(Stage::Checked))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn retract_refuses_output() {
        let storage = InMemoryStorage::new();
        let case = EinmoCase::new(id("a.foo"), &storage);
        assert!(matches!(
            case.retract(Stage::Output).unwrap_err(),
            EinmoError::Config(_)
        ));
    }

    /// Retracting `verified` removes only verified — it is the top of the
    /// chain, and the `checked` baseline beneath it survives untouched.
    #[test]
    fn retract_verified_leaves_checked() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        let case = EinmoCase::new(case_id.clone(), &storage);
        let bytes = signed_bytes("a.foo", "5", "");
        storage
            .write(&case_id, ArtifactLocation::Stage(Stage::Checked), &bytes)
            .unwrap();
        storage
            .write(&case_id, ArtifactLocation::Stage(Stage::Verified), &bytes)
            .unwrap();

        let retracted = case.retract(Stage::Verified).unwrap();
        assert_eq!(retracted, vec![Stage::Verified]);
        assert!(
            case.read(ArtifactLocation::Stage(Stage::Checked))
                .unwrap()
                .is_some(),
            "the checked baseline survives"
        );
    }

    /// `promote` refuses a source that fails verify-on-inspect (tampered),
    /// distinct from an absent source (`promote_errors_when_source_is_
    /// absent`) — both raise `Verification`, but for different reasons,
    /// and this is the "corrupted, not just missing" case
    /// `transitions::promote`'s own test suite used to cover.
    #[test]
    fn promote_refuses_tampered_source() {
        let storage = InMemoryStorage::new();
        let case_id = id("a.foo");
        storage
            .write(
                &case_id,
                ArtifactLocation::Stage(Stage::Output),
                b"not a valid einmo envelope",
            )
            .unwrap();
        let case = EinmoCase::new(case_id, &storage);
        let err = case
            .promote(Stage::Output, Stage::Checked, &derive(""))
            .unwrap_err();
        assert!(matches!(
            err,
            EinmoError::Verification(_) | EinmoError::Parse(_)
        ));
    }
}
