//! `EinmoReview` — the minimum viable session slice for `einmo-review-server`
//! (EIMP-2 §2, `docs/eimp/`). A single-implicit-reviewer, single-suite
//! session object: list cases, fetch verified bodies (single-flight
//! cached), record/replace/clear decisions, and execute them (sign +
//! write, or move to `flagged/`).
//!
//! Key custody is deliberately NOT part of this object — see
//! [`SignerSet`], passed into [`EinmoReview::execute`] only at the moment
//! of signing, never stored.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use crate::config::{KeySource, TestConfig};
use crate::einmo_suite::{TestRow, body_sections, scan_tests};
use crate::error::{EinmoError, Result};
use crate::format::EinmoFile;
use crate::stage::EinmoId;
use crate::stage::Stage;
use crate::transitions::{self, FlagReport, PromotionReport, RetractReport};

/// A reviewer's decision about one case. Replace-not-stack: a later
/// [`EinmoReview::decide`] call for the same [`EinmoId`] replaces, never
/// stacks on top of, an earlier one (`EIMP-1` §S.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Promote to `to` (`Checked` or `Verified`).
    Promote {
        /// The destination stage.
        to: Stage,
    },
    /// Retract (demote) from `from` (`Checked` or `Verified`); cascades
    /// `Checked → Verified` per [`transitions::retract`].
    Retract {
        /// The stage to retract from.
        from: Stage,
    },
    /// Move to `flagged/` with an advisory reason. No signing, no gate.
    Flag {
        /// The stage to flag from.
        stage: Stage,
        /// The advisory reason recorded in the flagged file.
        reason: String,
    },
    /// Looked, deliberately chose not to rule.
    Skip,
}

/// The map from case to the (single, implicit) reviewer's current
/// decision. Absence means untouched. `decide` replaces; `undecide` clears.
#[derive(Debug, Default)]
struct DecisionBook {
    entries: HashMap<EinmoId, Decision>,
}

impl DecisionBook {
    fn decide(&mut self, id: EinmoId, decision: Decision) -> Option<Decision> {
        self.entries.insert(id, decision)
    }

    fn undecide(&mut self, id: &EinmoId) -> Option<Decision> {
        self.entries.remove(id)
    }

    fn get(&self, id: &EinmoId) -> Option<&Decision> {
        self.entries.get(id)
    }

    fn iter(&self) -> impl Iterator<Item = (&EinmoId, &Decision)> {
        self.entries.iter()
    }
}

/// A verified envelope's body sections (everything but STAMPS), cached by
/// content fingerprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedBody {
    /// `(section name, section body)` pairs, STAMPS excluded.
    pub sections: Vec<(String, String)>,
}

/// A cheap content fingerprint used as the single-flight cache key: the
/// artifact's path plus its file length and modified time. Not a
/// cryptographic hash — collisions are safe (worst case: one redundant
/// verification), but a changed file almost always changes this tuple, so
/// the cache self-invalidates on edit without a filesystem watch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Fingerprint {
    path: std::path::PathBuf,
    len: u64,
    modified: Option<std::time::SystemTime>,
}

impl Fingerprint {
    fn of(path: &std::path::Path) -> Result<Self> {
        let meta = std::fs::metadata(path).map_err(|e| EinmoError::io(path, e))?;
        Ok(Fingerprint {
            path: path.to_path_buf(),
            len: meta.len(),
            modified: meta.modified().ok(),
        })
    }
}

/// A cached slot's outcome: the verified body, or the verification
/// failure's message (see [`VerifiedCache`]'s doc for why failures are
/// memoized too).
type CachedVerification = std::result::Result<VerifiedBody, String>;

/// Fingerprint → verified body, single-flight: concurrent requests for the
/// same fingerprint share one verification — `OnceLock::get_or_init`
/// guarantees the initializing closure runs at most once even when many
/// threads race it, so `EinmoFile::from_file` (the actual verify-on-inspect
/// work) executes exactly once per fingerprint, never once per caller.
///
/// A verification failure (tampered file) is memoized too, inside the same
/// slot — a fingerprint that failed once and hasn't changed on disk will
/// fail identically every time, so re-verifying it again is wasted work;
/// [`Fingerprint`] changing (the file was edited) mints a new slot and a
/// fresh verification, which is when a fix actually gets picked up.
#[derive(Debug, Default)]
struct VerifiedCache {
    entries: Mutex<HashMap<Fingerprint, Arc<OnceLock<CachedVerification>>>>,
    /// Test hook: counts actual verifications performed (not cache hits).
    verify_count: std::sync::atomic::AtomicUsize,
}

impl VerifiedCache {
    fn get_or_verify(&self, path: &std::path::Path) -> Result<VerifiedBody> {
        let fp = Fingerprint::of(path)?;
        let slot = {
            let mut entries = self.entries.lock().expect("VerifiedCache lock poisoned");
            entries.entry(fp).or_default().clone()
        };
        let result = slot.get_or_init(|| {
            self.verify_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            EinmoFile::from_file(path)
                .map(|file| VerifiedBody {
                    sections: body_sections(&file, None),
                })
                .map_err(|e| e.to_string())
        });
        result.clone().map_err(EinmoError::Verification)
    }

    #[cfg(test)]
    fn verify_count(&self) -> usize {
        self.verify_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// One row of the worklist: a case and where it currently stands.
#[derive(Debug, Clone)]
pub struct ReviewItem {
    /// The case identifier.
    pub id: EinmoId,
    /// `(stage, status if present)` for each of output/checked/verified.
    pub stages: Vec<(Stage, Option<String>)>,
    /// `true` unless every stage is present and their bodies agree.
    pub differing: bool,
    /// The reviewer's current decision for this case, if any.
    pub decision: Option<Decision>,
}

/// Key custody, held separately from [`EinmoReview`] (`EIMP-1` §S.4). A
/// `checked`-stage promotion always uses `to_checked`; a `verified`-stage
/// promotion always uses `to_verified` (absent means the promotion cannot
/// proceed — [`EinmoReview::execute`] reports it, never silently falls
/// back to the computer key). Neither key is stored anywhere beyond this
/// struct's lifetime.
#[derive(Debug, Clone)]
pub struct SignerSet {
    /// The key used for `output`/`* → checked` promotions.
    pub to_checked: KeySource,
    /// The key used for `* → verified` promotions, if any pending decision
    /// needs one.
    pub to_verified: Option<KeySource>,
}

/// One pending action the plan will apply.
#[derive(Debug, Clone)]
pub enum PlannedAction {
    /// Promote `id` to `to`.
    Promote {
        /// The case.
        id: EinmoId,
        /// The destination stage.
        to: Stage,
    },
    /// Retract `id` from `from`.
    Retract {
        /// The case.
        id: EinmoId,
        /// The stage retracted from.
        from: Stage,
    },
    /// Flag `id` at `stage` with `reason`.
    Flag {
        /// The case.
        id: EinmoId,
        /// The stage flagged from.
        stage: Stage,
        /// The advisory reason.
        reason: String,
    },
}

/// A preview of what [`EinmoReview::execute`] would do — every pending
/// decision translated into a concrete action, in the order they'll run.
/// `Skip` decisions produce no action.
#[derive(Debug, Clone, Default)]
pub struct ExecutionPlan {
    /// The actions this plan will apply, in order.
    pub actions: Vec<PlannedAction>,
}

/// The outcome of one executed action.
#[derive(Debug, Clone)]
pub struct Executed {
    /// The case acted on.
    pub id: EinmoId,
    /// A short description of what happened (e.g. `"promoted to checked"`).
    pub detail: String,
}

/// The result of [`EinmoReview::execute`].
#[derive(Debug, Clone, Default)]
pub struct ExecutionReport {
    /// Actions that completed.
    pub executed: Vec<Executed>,
    /// Actions skipped because the source drifted since planning (never
    /// clobbered — reported instead).
    pub skipped: Vec<EinmoId>,
}

/// A thread-safe, single-suite, single-implicit-reviewer review session
/// (EIMP-2 §2). Holds no key material — see [`SignerSet`].
pub struct EinmoReview {
    config: TestConfig,
    cache: VerifiedCache,
    decisions: RwLock<DecisionBook>,
    exec: Mutex<()>,
}

impl EinmoReview {
    /// Open a review session over `suite`.
    #[must_use]
    pub fn open(suite: impl Into<std::path::PathBuf>) -> Self {
        let config = TestConfig::new(suite, crate::einmo_suite::ValidationLevel::Output);
        EinmoReview {
            config,
            cache: VerifiedCache::default(),
            decisions: RwLock::new(DecisionBook::default()),
            exec: Mutex::new(()),
        }
    }

    /// The worklist: every case, its per-stage status, and the reviewer's
    /// current decision (if any).
    ///
    /// # Errors
    ///
    /// Returns an error if the suite's directories cannot be walked.
    pub fn items(&self) -> Result<Vec<ReviewItem>> {
        let rows: Vec<TestRow> = scan_tests(&self.config, None)?;
        let decisions = self.decisions.read().expect("decisions lock poisoned");
        rows.into_iter()
            .map(|row| {
                let id = EinmoId::from_input_rel(&strip_einmo_suffix(&row.rel))?;
                let decision = decisions.get(&id).cloned();
                Ok(ReviewItem {
                    id,
                    stages: row.stages,
                    differing: row.differing,
                    decision,
                })
            })
            .collect()
    }

    /// The verified body of `id` at `stage` (single-flight cached).
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact does not exist or fails
    /// verify-on-inspect.
    pub fn body(&self, id: &EinmoId, stage: Stage) -> Result<VerifiedBody> {
        let path = id.to_stage_path(self.config.work_dir(), stage);
        self.cache.get_or_verify(&path)
    }

    /// Record (or replace) the reviewer's decision for `id`. Returns the
    /// previous decision, if any (replace-not-stack, `EIMP-1` §S.3).
    pub fn decide(&self, id: EinmoId, decision: Decision) -> Option<Decision> {
        self.decisions
            .write()
            .expect("decisions lock poisoned")
            .decide(id, decision)
    }

    /// Flag `id` at `stage` immediately — a single atomic convenience call,
    /// unlike promote (which stays two-call: record a decision, then a
    /// separately gated execute). Flagging needs no signing and no gate, so
    /// there is nothing a two-call shape would protect (EIMP-2 §3). Also
    /// clears any pending decision for `id`, since flagging removes the
    /// artifact from the stage that decision would have acted on.
    ///
    /// # Errors
    ///
    /// Returns an error if `stage` holds nothing for `id`, or the
    /// underlying artifact fails verify-on-inspect.
    pub fn flag_now(&self, id: &EinmoId, stage: Stage, reason: &str) -> Result<()> {
        let rel = crate::stage::mirror_input_path(std::path::Path::new(id.as_str()));
        let files = [rel];
        let report = transitions::flag(&self.config, stage, None, reason, Some(&files))?;
        if report.flagged.is_empty() {
            return Err(EinmoError::io(
                id.to_stage_path(self.config.work_dir(), stage),
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "nothing to flag at that stage",
                ),
            ));
        }
        self.decisions
            .write()
            .expect("decisions lock poisoned")
            .undecide(id);
        Ok(())
    }

    /// Clear `id`'s decision back to "untouched". Returns the cleared
    /// decision, if any; a no-op (returns `None`) if it was already
    /// undecided.
    pub fn undecide(&self, id: &EinmoId) -> Option<Decision> {
        self.decisions
            .write()
            .expect("decisions lock poisoned")
            .undecide(id)
    }

    /// A preview of what [`EinmoReview::execute`] would do right now.
    #[must_use]
    pub fn plan(&self) -> ExecutionPlan {
        let decisions = self.decisions.read().expect("decisions lock poisoned");
        let mut actions = Vec::new();
        for (id, decision) in decisions.iter() {
            match decision {
                Decision::Promote { to } => actions.push(PlannedAction::Promote {
                    id: id.clone(),
                    to: *to,
                }),
                Decision::Retract { from } => actions.push(PlannedAction::Retract {
                    id: id.clone(),
                    from: *from,
                }),
                Decision::Flag { stage, reason } => actions.push(PlannedAction::Flag {
                    id: id.clone(),
                    stage: *stage,
                    reason: reason.clone(),
                }),
                Decision::Skip => {}
            }
        }
        ExecutionPlan { actions }
    }

    /// Apply `plan`'s actions: sign and write promotions (via
    /// [`transitions::promote`]), remove retractions (via
    /// [`transitions::retract`]), move flags (via [`transitions::flag`]).
    /// Exclusive — only one execution runs at a time.
    ///
    /// **Key hygiene.** `keys` is read only long enough to group and issue
    /// the underlying `transitions::promote` calls below, one call per
    /// distinct `(from, to)` stage pair (not one call per case) — this
    /// preserves `transitions::promote`'s own "derive once, sign many" KEK
    /// discipline (`StageKeypair::derive` + `with_signing_key`,
    /// `signature.rs`): the expensive Argon2id derivation happens once per
    /// stage pair in this batch, the plaintext seed exists in the clear
    /// only for the microseconds of each individual signature inside that
    /// one call, and is zeroized between signatures and again once the
    /// call returns. `execute` itself never derives a key or touches
    /// plaintext key material directly — it only ever forwards `keys`'
    /// `KeySource`s into `transitions::promote`, which owns the KEK.
    ///
    /// # Errors
    ///
    /// Returns an error if a promotion needs a `verified`-stage key that
    /// `keys` does not supply, or if the underlying filesystem operation
    /// fails for a reason other than the source having drifted (a drifted
    /// source is reported in [`ExecutionReport::skipped`], not an error).
    pub fn execute(&self, plan: &ExecutionPlan, keys: &SignerSet) -> Result<ExecutionReport> {
        let _guard = self.exec.lock().expect("exec lock poisoned");
        let mut report = ExecutionReport::default();

        // Group promotions by (from, to) so each stage pair's key is
        // derived exactly once for the whole batch, not once per case.
        let mut promote_groups: HashMap<(Stage, Stage), Vec<EinmoId>> = HashMap::new();
        for action in &plan.actions {
            if let PlannedAction::Promote { id, to } = action {
                let Some(from) = source_stage_for_promote(&self.config, id, *to) else {
                    report.skipped.push(id.clone());
                    continue;
                };
                promote_groups
                    .entry((from, *to))
                    .or_default()
                    .push(id.clone());
            }
        }
        for ((from, to), ids) in promote_groups {
            let key = match to {
                Stage::Verified => keys.to_verified.as_ref().ok_or_else(|| {
                    EinmoError::NoKey(format!(
                        "promoting to verified needs a verified-stage key ({} case(s) pending)",
                        ids.len()
                    ))
                })?,
                _ => &keys.to_checked,
            };
            let files: Vec<_> = ids
                .iter()
                .map(|id| crate::stage::mirror_input_path(std::path::Path::new(id.as_str())))
                .collect();
            // One transitions::promote call per stage pair: it derives the
            // stage key once internally and signs every file in `files`
            // under that single derivation (transitions.rs's own KEK
            // discipline), so this loop's key material is never held
            // outside that one call.
            let outcome: Result<PromotionReport> =
                transitions::promote(&self.config, from, to, key, None, Some(&files));
            match outcome {
                Ok(promoted) => {
                    let promoted_rels: std::collections::HashSet<_> = promoted
                        .promoted
                        .iter()
                        .map(|p| p.rel_path.clone())
                        .collect();
                    for id in ids {
                        let rel =
                            crate::stage::mirror_input_path(std::path::Path::new(id.as_str()));
                        if promoted_rels.contains(&rel) {
                            report.executed.push(Executed {
                                id,
                                detail: format!("promoted {from} to {to}"),
                            });
                        } else {
                            report.skipped.push(id);
                        }
                    }
                }
                Err(_) => report.skipped.extend(ids),
            }
        }

        for action in &plan.actions {
            match action {
                PlannedAction::Promote { .. } => {} // handled in the grouped pass above
                PlannedAction::Retract { id, from } => {
                    let rel = crate::stage::mirror_input_path(std::path::Path::new(id.as_str()));
                    let files = [rel];
                    let outcome: Result<RetractReport> =
                        transitions::retract(&self.config, *from, None, Some(&files));
                    match outcome {
                        Ok(r) if !r.retracted.is_empty() => {
                            report.executed.push(Executed {
                                id: id.clone(),
                                detail: format!("retracted from {from}"),
                            });
                        }
                        Ok(_) => report.skipped.push(id.clone()),
                        Err(_) => report.skipped.push(id.clone()),
                    }
                }
                PlannedAction::Flag { id, stage, reason } => {
                    let rel = crate::stage::mirror_input_path(std::path::Path::new(id.as_str()));
                    let files = [rel];
                    let outcome: Result<FlagReport> =
                        transitions::flag(&self.config, *stage, None, reason, Some(&files));
                    match outcome {
                        Ok(r) if !r.flagged.is_empty() => {
                            report.executed.push(Executed {
                                id: id.clone(),
                                detail: format!("flagged from {stage}"),
                            });
                        }
                        Ok(_) => report.skipped.push(id.clone()),
                        Err(_) => report.skipped.push(id.clone()),
                    }
                }
            }
        }
        Ok(report)
    }
}

/// The origin stage a promotion to `to` should read from: whichever of
/// `checked`/`output` currently holds the artifact, preferring the higher
/// stage (so a `checked → verified` promotion is chosen over `output →
/// verified` when both exist).
fn source_stage_for_promote(config: &TestConfig, id: &EinmoId, to: Stage) -> Option<Stage> {
    let candidates: &[Stage] = match to {
        Stage::Checked => &[Stage::Output],
        Stage::Verified => &[Stage::Checked, Stage::Output],
        _ => &[],
    };
    candidates
        .iter()
        .find(|&&s| id.to_stage_path(config.work_dir(), s).exists())
        .copied()
}

/// Strip the `.einmo` mirror suffix and, when present, the stage-relative
/// wrapping — `scan_tests` already yields the mirror-relative path
/// (`<input-rel>.einmo`), so this is the inverse of `mirror_input_path`.
fn strip_einmo_suffix(mirror_rel: &std::path::Path) -> std::path::PathBuf {
    let s = mirror_rel.to_string_lossy();
    let stripped = s.strip_suffix(".einmo").unwrap_or(&s);
    std::path::PathBuf::from(stripped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    fn write_input(dir: &std::path::Path, rel: &str, content: &str) {
        let path = dir.join("input").join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    struct Echo;
    impl crate::einmo_suite::Evaluator for Echo {
        fn evaluate(&self, source: &str) -> std::result::Result<Vec<String>, String> {
            Ok(vec![source.trim().to_string()])
        }
    }

    fn seeded_suite() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        write_input(tmp.path(), "a.foo", "{1+1;}");
        write_input(tmp.path(), "b.foo", "{2+2;}");
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoSuite::new(config);
        suite.evaluate_all(&Echo).unwrap();
        tmp
    }

    fn promote_output_to_checked(dir: &std::path::Path) {
        let config = TestConfig::new(dir, crate::einmo_suite::ValidationLevel::Output);
        transitions::promote(
            &config,
            Stage::Output,
            Stage::Checked,
            &KeySource::from_passphrase(""),
            None,
            None,
        )
        .unwrap();
    }

    #[test]
    fn items_reflects_suite_scan() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let items = review.items().unwrap();
        let ids: Vec<_> = items.iter().map(|i| i.id.as_str().to_string()).collect();
        assert!(ids.contains(&"a.foo".to_string()));
        assert!(ids.contains(&"b.foo".to_string()));
        for item in &items {
            assert!(item.decision.is_none());
        }
    }

    #[test]
    fn body_is_single_flight_verified() {
        let tmp = seeded_suite();
        let review = Arc::new(EinmoReview::open(tmp.path()));
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let review = Arc::clone(&review);
                let id = id.clone();
                thread::spawn(move || review.body(&id, Stage::Output).unwrap())
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert_eq!(
            review.cache.verify_count(),
            1,
            "8 concurrent requests for the same artifact must verify exactly once"
        );
    }

    #[test]
    fn body_reverifies_a_different_artifact() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let a = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let b = EinmoId::from_input_rel(std::path::Path::new("b.foo")).unwrap();
        review.body(&a, Stage::Output).unwrap();
        review.body(&b, Stage::Output).unwrap();
        assert_eq!(review.cache.verify_count(), 2);
    }

    #[test]
    fn decide_replaces_not_stacks() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        let prev = review.decide(id.clone(), Decision::Skip);
        assert_eq!(prev, None);

        let prev = review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        assert_eq!(prev, Some(Decision::Skip));

        let items = review.items().unwrap();
        let item = items.iter().find(|i| i.id == id).unwrap();
        assert_eq!(
            item.decision,
            Some(Decision::Promote { to: Stage::Checked })
        );
    }

    #[test]
    fn undecide_clears_back_to_untouched() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        assert_eq!(
            review.undecide(&id),
            None,
            "undecide on untouched is a no-op"
        );

        review.decide(id.clone(), Decision::Skip);
        let cleared = review.undecide(&id);
        assert_eq!(cleared, Some(Decision::Skip));

        let items = review.items().unwrap();
        let item = items.iter().find(|i| i.id == id).unwrap();
        assert!(item.decision.is_none());
    }

    #[test]
    fn execute_promote_matches_cli_promote_byte_for_byte() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let plan = review.plan();
        assert_eq!(plan.actions.len(), 1);
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let report = review.execute(&plan, &keys).unwrap();
        assert_eq!(report.executed.len(), 1);
        assert!(report.skipped.is_empty());

        let via_review = std::fs::read(tmp.path().join("checked").join("a.foo.einmo")).unwrap();

        // Independently reproduce via the CLI-level promote() on a fresh copy.
        let tmp2 = seeded_suite_with_same_content();
        promote_output_to_checked(tmp2.path());
        let via_cli = std::fs::read(tmp2.path().join("checked").join("a.foo.einmo")).unwrap();

        // Timestamps differ between the two independent runs (each stamp signs
        // its own generation time), so compare structure, not raw bytes: both
        // must verify and carry the same section bodies.
        let review_file = EinmoFile::from_file(&tmp.path().join("checked/a.foo.einmo")).unwrap();
        let cli_file = EinmoFile::from_file(&tmp2.path().join("checked/a.foo.einmo")).unwrap();
        assert_eq!(
            body_sections(&review_file, None),
            body_sections(&cli_file, None),
            "EinmoReview::execute's promotion body must match CLI promote's"
        );
        assert!(!via_review.is_empty());
        assert!(!via_cli.is_empty());
    }

    fn seeded_suite_with_same_content() -> tempfile::TempDir {
        seeded_suite()
    }

    /// `execute` must derive the stage key ONCE per `(from, to)` pair in a
    /// batch, not once per case — `StageKeypair::derive` (the Argon2id
    /// step `transitions::promote` calls internally) is ~1.8s regardless of
    /// passphrase (it has no empty-passphrase fast path, unlike the
    /// process-cached `COMPUTER_KEYPAIR` used elsewhere — `signature.rs`).
    /// Five cases promoted in one batch must complete in well under
    /// 5 × 1.8s; a per-case-derive regression would blow this bound.
    #[test]
    fn execute_derives_stage_key_once_per_batch_not_per_case() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..5 {
            write_input(tmp.path(), &format!("case{i}.foo"), "{1+1;}");
        }
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoSuite::new(config);
        suite.evaluate_all(&Echo).unwrap();

        let review = EinmoReview::open(tmp.path());
        for i in 0..5 {
            let id =
                EinmoId::from_input_rel(std::path::Path::new(&format!("case{i}.foo"))).unwrap();
            review.decide(id, Decision::Promote { to: Stage::Checked });
        }
        let plan = review.plan();
        assert_eq!(plan.actions.len(), 5);

        let keys = SignerSet {
            to_checked: KeySource::from_passphrase("a-non-empty-batch-passphrase"),
            to_verified: None,
        };
        let start = std::time::Instant::now();
        let report = review.execute(&plan, &keys).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(report.executed.len(), 5);
        assert!(report.skipped.is_empty());
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "5-case batch took {elapsed:?} — looks like the stage key was derived per case \
             (~1.8s each) instead of once for the whole (from, to) group"
        );
    }

    #[test]
    fn execute_retract_matches_cli_retract_and_cascades() {
        let tmp = seeded_suite();
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        transitions::promote(
            &config,
            Stage::Output,
            Stage::Checked,
            &KeySource::from_passphrase(""),
            None,
            None,
        )
        .unwrap();
        transitions::promote(
            &config,
            Stage::Checked,
            Stage::Verified,
            &KeySource::from_passphrase(""),
            None,
            None,
        )
        .unwrap();
        assert!(tmp.path().join("verified/a.foo.einmo").exists());

        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(
            id.clone(),
            Decision::Retract {
                from: Stage::Checked,
            },
        );
        let plan = review.plan();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let report = review.execute(&plan, &keys).unwrap();
        assert_eq!(report.executed.len(), 1);
        assert!(
            !tmp.path().join("checked/a.foo.einmo").exists(),
            "checked artifact removed"
        );
        assert!(
            !tmp.path().join("verified/a.foo.einmo").exists(),
            "cascade must remove the verified artifact too"
        );
    }

    #[test]
    fn execute_flag_moves_and_writes_advisory_no_signing() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(
            id.clone(),
            Decision::Flag {
                stage: Stage::Output,
                reason: "looks wrong".to_string(),
            },
        );
        let plan = review.plan();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let report = review.execute(&plan, &keys).unwrap();
        assert_eq!(report.executed.len(), 1);
        assert!(!tmp.path().join("output/a.foo.einmo").exists());
        let flagged = std::fs::read_to_string(tmp.path().join("flagged/a.foo.einmo")).unwrap();
        assert!(flagged.contains("# flagged: looks wrong"));
    }

    #[test]
    fn flag_now_is_atomic_no_decide_or_execute_needed() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.flag_now(&id, Stage::Output, "looks wrong").unwrap();

        assert!(!tmp.path().join("output/a.foo.einmo").exists());
        let flagged = std::fs::read_to_string(tmp.path().join("flagged/a.foo.einmo")).unwrap();
        assert!(flagged.contains("# flagged: looks wrong"));
    }

    #[test]
    fn flag_now_clears_any_pending_decision() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });

        review.flag_now(&id, Stage::Output, "actually no").unwrap();

        let items = review.items().unwrap();
        let item = items.iter().find(|i| i.id == id).unwrap();
        assert_eq!(item.decision, None);
    }

    #[test]
    fn flag_now_errors_when_stage_has_nothing_for_id() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        let err = review.flag_now(&id, Stage::Verified, "n/a").unwrap_err();
        assert!(matches!(err, EinmoError::Io { .. }));
    }

    #[test]
    fn execute_promote_to_verified_without_key_errors() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(
            id.clone(),
            Decision::Promote {
                to: Stage::Verified,
            },
        );
        let plan = review.plan();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let err = review.execute(&plan, &keys).unwrap_err();
        assert!(matches!(err, EinmoError::NoKey(_)));
    }
}
