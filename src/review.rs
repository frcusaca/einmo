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

use crate::case::{EinmoCase, PromoteOutcome, StagePairAgreement};
use crate::config::{KeySource, TestConfig};
use crate::einmo_suite::body_sections;
use crate::error::{EinmoError, Result};
use crate::format::EinmoFile;
use crate::signature::StageKeypair;
use crate::stage::EinmoId;
use crate::stage::Stage;
use crate::storage::{ArtifactLocation, EinmoDirectory};
use crate::suite::EinmoSuite;

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
    entries: HashMap<EinmoId, DecisionEntry>,
}

/// A decision plus the fingerprint of whatever stage it was based on, taken
/// at `decide()` time — the substrate `EinmoReview::refresh`/`execute`'s
/// drift detection compares against (`EIMP-1` §S.2/§S.5). `None` when the
/// basis stage did not exist or could not be read at decide-time, or the
/// decision (`Skip`) has no content basis at all.
#[derive(Debug)]
struct DecisionEntry {
    decision: Decision,
    basis: Option<Fingerprint>,
}

impl DecisionBook {
    fn decide(
        &mut self,
        id: EinmoId,
        decision: Decision,
        basis: Option<Fingerprint>,
    ) -> Option<Decision> {
        self.entries
            .insert(id, DecisionEntry { decision, basis })
            .map(|e| e.decision)
    }

    fn undecide(&mut self, id: &EinmoId) -> Option<Decision> {
        self.entries.remove(id).map(|e| e.decision)
    }

    fn get(&self, id: &EinmoId) -> Option<&Decision> {
        self.entries.get(id).map(|e| &e.decision)
    }

    fn get_entry(&self, id: &EinmoId) -> Option<&DecisionEntry> {
        self.entries.get(id)
    }

    fn iter(&self) -> impl Iterator<Item = (&EinmoId, &Decision)> {
        self.entries.iter().map(|(id, e)| (id, &e.decision))
    }

    fn iter_entries(&self) -> impl Iterator<Item = (&EinmoId, &DecisionEntry)> {
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

/// One line of a section's diff between two stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// Present, unchanged, on both sides.
    Equal(String),
    /// Present in `left` (the first stage passed to
    /// [`EinmoReview::diff`]), absent or different in `right`.
    Removed(String),
    /// Present in `right`, absent or different in `left`.
    Added(String),
}

/// One section's diff between two stages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDiff {
    /// The section name (e.g. `"OUTPUT"`, `"COMMENTS"`).
    pub name: String,
    /// The line-level diff of that section's body, `left` vs `right`.
    pub lines: Vec<DiffLine>,
}

/// The diff between two stages' verified bodies, section by section
/// (`EIMP-1` §S.7: "hunks between stages, stamp lines excluded" — STAMPS is
/// never a section here because [`VerifiedBody`] already excludes it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffHunks {
    /// One entry per section present on either side, in `left`'s section
    /// order followed by any sections `right` has that `left` does not.
    pub sections: Vec<SectionDiff>,
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

/// How [`EinmoReview::items`] selects and orders the worklist (`EIMP-1` §S.2).
///
/// Not a boolean: the old `differing_only` idea generalizes into a mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReviewMode {
    /// Every item in the worklist, in scan order. The default — matches
    /// `EIMP-2`'s original unfiltered list behavior, so a script that has
    /// always shown everything sees no surprise narrowing.
    #[default]
    Full,
    /// The worklist in randomized order (sampling a large suite). Freshly
    /// shuffled on every [`EinmoReview::items`] call using OS entropy — not
    /// a fixed seed — since `items()` already rescans the suite from
    /// scratch each call (no cached worklist exists to keep a stable order
    /// consistent with); a reviewer who wants "the same random order again"
    /// should record it via decisions, not rely on order stability.
    Random,
    /// Only items with no baseline yet, or a content mismatch between
    /// stages (`ReviewItem::differing`) — the old `differing_only`
    /// boolean's actual predicate, generalized into its own mode.
    NewOrBroken,
}

/// Options controlling what [`EinmoReview::open_with`] considers "the
/// worklist" (`EIMP-1` §S.2).
#[derive(Debug, Clone, Default)]
pub struct ReviewOpts {
    /// Selection/ordering mode.
    pub mode: ReviewMode,
    /// Restrict to cases whose id contains this substring (`EinmoSuite::
    /// scan`'s filter — matched against the bare id, no `.einmo` suffix,
    /// the same form `transitions.rs`'s own filter already uses).
    pub filter: Option<String>,
}

/// Line-level diff of one section's body between two stages, via `similar`
/// (already a dependency — `einmo_suite.rs`'s dependent-`DIFF`-section
/// generation uses it too).
fn section_diff(name: &str, left: &str, right: &str) -> SectionDiff {
    let diff = similar::TextDiff::from_lines(left, right);
    let lines = diff
        .iter_all_changes()
        .map(|change| {
            let value = change.value().to_string();
            match change.tag() {
                similar::ChangeTag::Equal => DiffLine::Equal(value),
                similar::ChangeTag::Delete => DiffLine::Removed(value),
                similar::ChangeTag::Insert => DiffLine::Added(value),
            }
        })
        .collect();
    SectionDiff {
        name: name.to_string(),
        lines,
    }
}

/// Fisher-Yates, OS-entropy seeded. `ReviewMode::Random`'s only consumer —
/// reuses the `rand_core`/`OsRng` primitive `signature.rs` already depends
/// on rather than adding `rand` as a new dependency (`EIMP-4` §S.1 keeps
/// core `einmo` dependency-light).
fn shuffle<T>(items: &mut [T]) {
    use rand_core::{OsRng, RngCore};
    for i in (1..items.len()).rev() {
        let j = (OsRng.next_u32() as usize) % (i + 1);
        items.swap(i, j);
    }
}

/// A fresh, OS-entropy-seeded session id (`EIMP-1` §S.6) — 128 bits, hex
/// encoded, matching this crate's existing prefer-`rand_core`-over-`rand`
/// convention (`EIMP-4` §S.1).
fn random_session_id() -> String {
    use rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// One row of the worklist: a case and where it currently stands.
#[derive(Debug, Clone)]
pub struct ReviewItem {
    /// The case identifier.
    pub id: EinmoId,
    /// `(stage, status if present)` for each of output/checked/verified.
    pub stages: Vec<(Stage, Option<String>)>,
    /// `true` unless output and checked are BOTH present and their
    /// policy-required sections agree (`EinmoCase::agreement(&[Output,
    /// Checked], _)`, `EIMP-7` §S.6). Scoped to exactly what
    /// `ReviewMode::NewOrBroken` promises — "differs between output and
    /// checked" — not whether `verified/` happens to be populated. This
    /// is the fix for `EIMP-1`'s P1 finding: the prior all-stages
    /// semantic (`true` unless EVERY stage among output/checked/verified
    /// was present and agreed) made a fresh suite's unpopulated
    /// `verified/` false-positive as "differing" on every single case.
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
    /// Every currently-active soft claim (`EIMP-1` §S.5) — "a reviewer sees
    /// what another reviewer currently holds (and its remaining TTL) before
    /// deciding, so two reviewers don't collide on the same case." Advisory
    /// only: nothing here changes what `execute` does.
    pub claims: Vec<ActiveClaim>,
}

/// A soft, advisory claim on a case (`EIMP-1` §S.5): "I'm on this one" —
/// never enforced, cannot wedge (an expired claim is silently released, no
/// action needed from the original claimant).
#[derive(Debug, Clone)]
pub struct ActiveClaim {
    /// The claimed case.
    pub id: EinmoId,
    /// How much longer the claim lasts, as of when it was read.
    pub remaining: std::time::Duration,
}

/// The outcome of one executed action.
#[derive(Debug, Clone)]
pub struct Executed {
    /// The case acted on.
    pub id: EinmoId,
    /// A short description of what happened (e.g. `"promoted to checked"`).
    pub detail: String,
    /// `true` if this was a promotion to `verified` signed with a
    /// well-known computer key (a non-human attestation — post-hoc
    /// detectable, `EIMP-1` §B.4, mirroring [`transitions::Promoted`]'s
    /// own field of the same name). Always `false` for retract/flag
    /// actions and for promotions to any stage other than `verified`.
    pub non_human: bool,
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
    opts: ReviewOpts,
    cache: VerifiedCache,
    decisions: RwLock<DecisionBook>,
    exec: Mutex<()>,
    journal: crate::journal::Journal,
    /// Soft claims (`EIMP-1` §S.5): id -> the instant it expires at.
    claims: RwLock<HashMap<EinmoId, std::time::Instant>>,
}

impl EinmoReview {
    /// Open a review session over `suite` with the default options
    /// (`ReviewMode::Full`, no filter — `EIMP-2`'s original unfiltered
    /// behavior), under a fresh, randomly generated session id.
    #[must_use]
    pub fn open(suite: impl Into<std::path::PathBuf>) -> Self {
        Self::open_with(suite, ReviewOpts::default())
    }

    /// Open a review session over `suite` with explicit [`ReviewOpts`]
    /// (`EIMP-1` §S.2), under a fresh, randomly generated session id.
    #[must_use]
    pub fn open_with(suite: impl Into<std::path::PathBuf>, opts: ReviewOpts) -> Self {
        Self::open_internal(suite, opts, random_session_id())
    }

    /// Resume a session previously opened under `session_id`: replays that
    /// session's journal (`EIMP-1` §S.6, "Reopen = replay") and reconstructs
    /// pending decisions by replaying every `decide`/`undecide` event, in
    /// order, through the ordinary [`EinmoReview::decide`]/
    /// [`EinmoReview::undecide`] calls — so a resumed decision's drift-check
    /// basis (`EIMP-1` §S.2/§S.5) is fingerprinted fresh against *current*
    /// disk content, never trusted blindly from before a crash. If nothing
    /// changed since the original `decide()`, the fingerprint matches and
    /// resume is transparent; if something DID change during the gap, that
    /// is a legitimate drift `execute`/`refresh` will still catch.
    ///
    /// A `session_id` with no journal history (never opened, or nothing
    /// written yet) resumes as an ordinary fresh, empty review — this is
    /// not an error.
    ///
    /// # Errors
    ///
    /// Returns an error if a replayed `Decide` event's case id fails to
    /// parse as an [`EinmoId`].
    pub fn resume(
        suite: impl Into<std::path::PathBuf>,
        session_id: impl Into<String>,
        opts: ReviewOpts,
    ) -> Result<Self> {
        let session_id = session_id.into();
        let path = crate::journal::journal_path(&session_id);
        let entries = crate::journal::Journal::replay(&path);
        let review = Self::open_internal(suite, opts, session_id);
        for line in entries {
            match line.event {
                crate::journal::JournalEvent::Decide { id, decision } => {
                    let id = EinmoId::try_from(id.as_str())?;
                    if let Some(decision) = decision.into_decision() {
                        review.decide(id, decision);
                    }
                }
                crate::journal::JournalEvent::Undecide { id } => {
                    let id = EinmoId::try_from(id.as_str())?;
                    review.undecide(&id);
                }
                _ => {}
            }
        }
        Ok(review)
    }

    fn open_internal(
        suite: impl Into<std::path::PathBuf>,
        opts: ReviewOpts,
        session_id: String,
    ) -> Self {
        let config = TestConfig::new(suite, crate::einmo_suite::ValidationLevel::Output);
        let journal =
            crate::journal::Journal::open(session_id, crate::journal::JournalLevel::default());
        journal.log_at(
            crate::journal::JournalLevel::Terse,
            crate::journal::JournalEvent::SessionOpen {
                session: journal.session_id().to_string(),
                suite: config.work_dir().display().to_string(),
            },
        );
        EinmoReview {
            config,
            opts,
            cache: VerifiedCache::default(),
            decisions: RwLock::new(DecisionBook::default()),
            exec: Mutex::new(()),
            journal,
            claims: RwLock::new(HashMap::new()),
        }
    }

    /// This session's id — the journal file (under
    /// [`crate::journal::journal_dir`]) is `<session_id>.jsonl`.
    /// [`EinmoReview::resume`] with this id continues the same journal.
    #[must_use]
    pub fn session_id(&self) -> &str {
        self.journal.session_id()
    }

    /// The path this session's journal writes to.
    #[must_use]
    pub fn journal_path(&self) -> std::path::PathBuf {
        self.journal.path()
    }

    /// Test-only hook: how many times [`VerifiedCache`] actually ran
    /// verification (not cache hits) — exposed `pub(crate)` so a
    /// concurrency test in `review_server.rs` (a sibling module, `EIMP-1`
    /// Phase C's "single-flight verify counts... through the HTTP server
    /// specifically") can confirm N concurrent `GET .../body/<stage>`
    /// requests for the same artifact still verify exactly once.
    #[cfg(test)]
    pub(crate) fn cache_verify_count(&self) -> usize {
        self.cache.verify_count()
    }

    /// The worklist: every case matching `self.opts`, its per-stage status,
    /// and the reviewer's current decision (if any).
    ///
    /// # Errors
    ///
    /// Returns an error if the suite's directories cannot be walked.
    pub fn items(&self) -> Result<Vec<ReviewItem>> {
        let directory = EinmoDirectory::new(self.config.clone());
        let suite = EinmoSuite::scan(directory, self.opts.filter.as_deref())?;
        let decisions = self.decisions.read().expect("decisions lock poisoned");
        let mut items: Vec<ReviewItem> = Vec::new();
        for case in suite.cases() {
            // EIMP-7 §S.6: differing is scoped to output-vs-checked only,
            // under the suite's configured MatchSections policy -- the P1
            // fix. Not `Agree` covers Differ/OneSided/BothAbsent/Tampered
            // alike: any of those is "needs a look", same as the old
            // all-stages bool's intent, just correctly scoped.
            let agreement = case.agreement(
                &[Stage::Output, Stage::Checked],
                self.config.match_sections(),
            )?;
            let differing = !matches!(
                agreement.pair(Stage::Output, Stage::Checked),
                Some(StagePairAgreement::Agree)
            );
            if self.opts.mode == ReviewMode::NewOrBroken && !differing {
                continue;
            }
            let stages = case.stages()?;
            let id = case.id().clone();
            let decision = decisions.get(&id).cloned();
            items.push(ReviewItem {
                id,
                stages,
                differing,
                decision,
            });
        }
        if self.opts.mode == ReviewMode::Random {
            shuffle(&mut items);
        }
        Ok(items)
    }

    /// The verified body of `id` at `stage` (single-flight cached).
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact does not exist or fails
    /// verify-on-inspect.
    pub fn body(&self, id: &EinmoId, stage: Stage) -> Result<VerifiedBody> {
        let path = id.to_stage_path(self.config.work_dir(), stage);
        // `fine`-level only (EIMP-1 S.6): an unmatched VerifyStart (the
        // process crashed between these two log_at calls) identifies the
        // in-flight case after a crash, without touching output/ the way
        // the crash crumb does.
        self.journal.log_at(
            crate::journal::JournalLevel::Fine,
            crate::journal::JournalEvent::VerifyStart {
                id: id.as_str().to_string(),
                stage: stage.dir_name().to_string(),
            },
        );
        let result = self.cache.get_or_verify(&path);
        self.journal.log_at(
            crate::journal::JournalLevel::Fine,
            crate::journal::JournalEvent::VerifyEnd {
                id: id.as_str().to_string(),
                stage: stage.dir_name().to_string(),
                ok: result.is_ok(),
            },
        );
        result
    }

    /// The section-by-section diff of `id` between `left` and `right`
    /// (`EIMP-1` §S.7). Both bodies go through [`EinmoReview::body`], so
    /// each side is verify-on-inspected and single-flight cached exactly as
    /// a direct `body` call would be.
    ///
    /// # Errors
    ///
    /// Returns an error if either stage's artifact does not exist or fails
    /// verify-on-inspect.
    pub fn diff(&self, id: &EinmoId, left: Stage, right: Stage) -> Result<DiffHunks> {
        let left_body = self.body(id, left)?;
        let right_body = self.body(id, right)?;

        let mut sections = Vec::new();
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for (name, left_text) in &left_body.sections {
            let right_text = right_body
                .sections
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, b)| b.as_str())
                .unwrap_or("");
            sections.push(section_diff(name, left_text, right_text));
            seen.insert(name.as_str());
        }
        for (name, right_text) in &right_body.sections {
            if !seen.contains(name.as_str()) {
                sections.push(section_diff(name, "", right_text));
            }
        }
        Ok(DiffHunks { sections })
    }

    /// Record (or replace) the reviewer's decision for `id`. Returns the
    /// previous decision, if any (replace-not-stack, `EIMP-1` §S.3).
    pub fn decide(&self, id: EinmoId, decision: Decision) -> Option<Decision> {
        let basis = decision_basis_path(&self.config, &id, &decision)
            .and_then(|p| Fingerprint::of(&p).ok());
        self.journal.log_at(
            crate::journal::JournalLevel::Normal,
            crate::journal::JournalEvent::Decide {
                id: id.as_str().to_string(),
                decision: crate::journal::JournalDecision::from(&decision),
            },
        );
        self.decisions
            .write()
            .expect("decisions lock poisoned")
            .decide(id, decision, basis)
    }

    /// Rescan for pending decisions whose basis content has changed on disk
    /// since `decide()` was called (`EIMP-1` §S.2/§S.5's fingerprint
    /// re-check). Returns the drifted cases' ids — **decisions are not
    /// cleared**; a frontend decides whether to re-prompt or leave a stale
    /// decision in place. (`items()` itself always reads fresh from disk on
    /// every call, so `refresh` is not "make the worklist current" —
    /// nothing caches it — it is specifically "which pending decisions no
    /// longer match what they were made about.")
    #[must_use]
    pub fn refresh(&self) -> Vec<EinmoId> {
        let decisions = self.decisions.read().expect("decisions lock poisoned");
        decisions
            .iter_entries()
            .filter_map(|(id, entry)| {
                let basis = entry.basis.as_ref()?;
                let current_path = decision_basis_path(&self.config, id, &entry.decision)?;
                let current = Fingerprint::of(&current_path).ok();
                (current.as_ref() != Some(basis)).then(|| id.clone())
            })
            .collect()
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
        // Flags CONCATENATE (EIMP-1 §S.3): a re-flag reads whatever is
        // already at `flagged/<id>` and writes on top of it. Two
        // concurrent flags on the same case must serialize through this
        // read-then-write, or one's dated block can be lost to the other's
        // write — the same `exec` mutex `execute` already holds for its
        // whole duration.
        let _guard = self.exec.lock().expect("exec lock poisoned");
        let directory = EinmoDirectory::new(self.config.clone());
        let case = EinmoCase::new(id.clone(), &directory);
        if case.read(ArtifactLocation::Stage(stage))?.is_none() {
            return Err(EinmoError::io(
                id.to_stage_path(self.config.work_dir(), stage),
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "nothing to flag at that stage",
                ),
            ));
        }
        case.flag(stage, reason)?;
        self.decisions
            .write()
            .expect("decisions lock poisoned")
            .undecide(id);
        Ok(())
    }

    /// Retract (demote) `id` from `stage` immediately — a single atomic
    /// convenience call, unlike promote. Retraction needs no signing and no
    /// gate (it only removes files), so there is nothing a two-call shape
    /// would protect (EIMP-2 §3). Cascades `checked → verified` per
    /// [`EinmoCase::retract`]. Also clears any pending decision for `id`.
    ///
    /// # Errors
    ///
    /// Returns [`EinmoError::Config`] if `stage` is `output` (not a
    /// retractable baseline), or an error if `stage` holds nothing for
    /// `id`.
    pub fn retract_now(&self, id: &EinmoId, stage: Stage) -> Result<()> {
        // Same `exec` mutex `flag_now`/`execute` already take: a concurrent
        // `execute()` batch promoting this SAME id into `checked`/`verified`
        // and a `retract_now()` for it must serialize, or the two can
        // interleave unserialized (e.g. this call's existence check passing
        // right before `execute` writes the destination file, or the
        // reverse), leaving a retract/promote report that doesn't match
        // final disk state.
        let _guard = self.exec.lock().expect("exec lock poisoned");
        let directory = EinmoDirectory::new(self.config.clone());
        let case = EinmoCase::new(id.clone(), &directory);
        // `EinmoCase::retract` checks `stage == Output` FIRST, before
        // touching storage, so this call errors immediately for Output —
        // no separate pre-check needed to match `transitions::retract`'s
        // old ordering.
        let retracted = case.retract(stage)?;
        if retracted.is_empty() {
            return Err(EinmoError::io(
                id.to_stage_path(self.config.work_dir(), stage),
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "nothing to retract at that stage",
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
        self.journal.log_at(
            crate::journal::JournalLevel::Normal,
            crate::journal::JournalEvent::Undecide {
                id: id.as_str().to_string(),
            },
        );
        self.decisions
            .write()
            .expect("decisions lock poisoned")
            .undecide(id)
    }

    /// `id`'s current pending decision, if any — "the answer so far"
    /// (`EIMP-1` §S.2), without going through the full `items()` scan.
    #[must_use]
    pub fn decision(&self, id: &EinmoId) -> Option<Decision> {
        self.decisions
            .read()
            .expect("decisions lock poisoned")
            .get(id)
            .cloned()
    }

    /// The default claim TTL (`EIMP-1` §S.5, resolved): 5 minutes.
    pub const DEFAULT_CLAIM_TTL: std::time::Duration = std::time::Duration::from_secs(5 * 60);

    /// Advertise "I'm on this one" for `id`, for [`EinmoReview::DEFAULT_CLAIM_TTL`]
    /// (`EIMP-1` §S.5). Soft: never enforced, never blocks `decide`/`execute`
    /// for this or any other case — see [`EinmoReview::claim_for`] for the
    /// full contract.
    pub fn claim(&self, id: &EinmoId) {
        self.claim_for(id, Self::DEFAULT_CLAIM_TTL);
    }

    /// As [`EinmoReview::claim`], with an explicit TTL. Claiming an
    /// already-claimed case REFRESHES it (replace-not-stack, same
    /// discipline as decisions) rather than erroring or stacking — a claim
    /// is advisory, not a lock, so there is nothing to contend over.
    /// Surfaced via [`EinmoReview::plan`]'s `claims` field; an expired
    /// claim is silently released (auto-reclaimed) the next time claims
    /// are read — no explicit release call exists, matching `EIMP-1`
    /// §S.5's "no action needed from the original claimant".
    pub fn claim_for(&self, id: &EinmoId, ttl: std::time::Duration) {
        let mut claims = self.claims.write().expect("claims lock poisoned");
        let now = std::time::Instant::now();
        // Opportunistic prune: claiming is the natural, cheap point to drop
        // stale entries, so the map does not grow unboundedly across a long
        // session even though nothing ever explicitly releases a claim.
        claims.retain(|_, expires_at| *expires_at > now);
        claims.insert(id.clone(), now + ttl);
    }

    /// Every currently-active claim, auto-reclaiming (filtering out) any
    /// that have expired. Read-only — does not prune the underlying map
    /// (see [`EinmoReview::claim_for`] for where pruning happens); an
    /// expired entry simply never appears here.
    #[must_use]
    fn active_claims(&self) -> Vec<ActiveClaim> {
        let claims = self.claims.read().expect("claims lock poisoned");
        let now = std::time::Instant::now();
        claims
            .iter()
            .filter(|(_, expires_at)| **expires_at > now)
            .map(|(id, expires_at)| ActiveClaim {
                id: id.clone(),
                remaining: *expires_at - now,
            })
            .collect()
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
        ExecutionPlan {
            actions,
            claims: self.active_claims(),
        }
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

        // Content-fingerprint re-check (EIMP-1 S.2/S.5): an action whose
        // decision basis has changed on disk since decide() must never be
        // silently applied against content the reviewer never actually
        // looked at -- skip and report, exactly like the presence-based
        // drift below. Checked against the LIVE DecisionBook (not `plan`
        // itself, which carries no fingerprint) so a decision changed or
        // cleared between plan() and execute() is caught the same way.
        let actions: Vec<PlannedAction> = {
            let decisions = self.decisions.read().expect("decisions lock poisoned");
            plan.actions
                .iter()
                .filter(|action| {
                    let id = action_id(action);
                    let Some(basis) = decisions.get_entry(id).and_then(|e| e.basis.as_ref()) else {
                        return true; // no recorded basis: nothing to compare, proceed
                    };
                    let current = action_basis_path(&self.config, action)
                        .and_then(|p| Fingerprint::of(&p).ok());
                    if current.as_ref() == Some(basis) {
                        true
                    } else {
                        report.skipped.push(id.clone());
                        false
                    }
                })
                .cloned()
                .collect()
        };

        // Group promotions by (from, to) so each stage pair's key is
        // derived exactly once for the whole batch, not once per case.
        let mut promote_groups: HashMap<(Stage, Stage), Vec<EinmoId>> = HashMap::new();
        for action in &actions {
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
        // Resolve every group's key BEFORE mutating anything for ANY
        // group. A missing verified-stage key must abort the whole batch
        // with zero side effects -- never partway through, after some
        // other group's promotion has already been written to disk. (This
        // used to `?` out of the loop below mid-mutation: a checked-group
        // promotion could land on disk, then a later verified-group's
        // missing key would propagate an `Err` out of `execute` entirely,
        // discarding `report`, skipping the flag/retract pass below, and
        // never clearing the already-applied item's pending decision or
        // journaling the batch -- a real disk mutation with no caller
        // visibility and no audit trail. See
        // `execute_missing_verified_key_aborts_the_whole_batch_untouched`.)
        let mut resolved_groups = Vec::with_capacity(promote_groups.len());
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
            resolved_groups.push((from, to, ids, key));
        }
        // EIMP-7 §S.3: EinmoCase::promote is the one promote
        // implementation (moved from this module's own
        // promote_one_accumulating). One EinmoDirectory suffices for the
        // whole batch -- it holds no mutable state of its own, just a
        // TestConfig clone.
        let directory = EinmoDirectory::new(self.config.clone());
        for (from, to, ids, key) in resolved_groups {
            // Derive the stage key ONCE per (from, to) group (the same
            // discipline `transitions::promote` uses): Argon2id derivation
            // is ~1.8s by design, so per-case derivation would make a
            // multi-case batch promotion unusable
            // (`execute_derives_stage_key_once_per_batch_not_per_case`).
            let keypair = StageKeypair::derive(key.passphrase());
            for id in ids {
                let case = EinmoCase::new(id.clone(), &directory);
                match case.promote(from, to, &keypair) {
                    Ok(outcome) => {
                        let (detail, non_human) = match outcome {
                            PromoteOutcome::Promoted { non_human } => {
                                (format!("promoted {from} to {to}"), non_human)
                            }
                            PromoteOutcome::CoSigned { non_human } => (
                                format!("{from} to {to}: co-signed by a new signer"),
                                non_human,
                            ),
                            PromoteOutcome::AlreadySigned { non_human } => (
                                format!("{from} to {to}: already signed, unchanged"),
                                non_human,
                            ),
                        };
                        report.executed.push(Executed {
                            id,
                            detail,
                            non_human,
                        });
                    }
                    Err(_) => report.skipped.push(id),
                }
            }
        }

        for action in &actions {
            match action {
                PlannedAction::Promote { .. } => {} // handled in the grouped pass above
                PlannedAction::Retract { id, from } => {
                    let case = EinmoCase::new(id.clone(), &directory);
                    match case.retract(*from) {
                        Ok(retracted) if !retracted.is_empty() => {
                            report.executed.push(Executed {
                                id: id.clone(),
                                detail: format!("retracted from {from}"),
                                non_human: false,
                            });
                        }
                        Ok(_) | Err(_) => report.skipped.push(id.clone()),
                    }
                }
                PlannedAction::Flag { id, stage, reason } => {
                    let case = EinmoCase::new(id.clone(), &directory);
                    match case.flag(*stage, reason) {
                        Ok(()) => {
                            report.executed.push(Executed {
                                id: id.clone(),
                                detail: format!("flagged from {stage}"),
                                non_human: false,
                            });
                        }
                        Err(_) => report.skipped.push(id.clone()),
                    }
                }
            }
        }

        // Every id this plan touched — executed or skipped — is no longer
        // "pending": an executed decision has been applied, and a skipped
        // one (source drifted since planning) needs a fresh decision, not a
        // stale one lingering in the next plan() preview.
        {
            let mut decisions = self.decisions.write().expect("decisions lock poisoned");
            for executed in &report.executed {
                decisions.undecide(&executed.id);
            }
            for skipped in &report.skipped {
                decisions.undecide(skipped);
            }
        }

        self.journal.log_at(
            crate::journal::JournalLevel::Terse,
            crate::journal::JournalEvent::ExecuteBatch {
                executed: report
                    .executed
                    .iter()
                    .map(|e| e.id.as_str().to_string())
                    .collect(),
                skipped: report
                    .skipped
                    .iter()
                    .map(|id| id.as_str().to_string())
                    .collect(),
            },
        );

        Ok(report)
    }

    /// Execute `id`'s pending decision immediately — a single-item
    /// convenience over [`EinmoReview::execute`] (`EIMP-1` §S.2), for a
    /// frontend that promotes as it goes rather than batching (§S.4's
    /// "individual vs batch collapses into one design": this is not a
    /// separate code path, just `execute` given a one-action plan, so it
    /// gets the same drift check, the same exec-mutex exclusivity, and the
    /// same undecide-on-completion for free).
    ///
    /// # Errors
    ///
    /// Returns an error if `id` has no pending decision, the decision is
    /// `Skip` (nothing to execute), or the action was skipped rather than
    /// executed (drifted since `decide()`, or its source no longer exists —
    /// see [`EinmoReview::execute`]'s `ExecutionReport::skipped`). In the
    /// last case the decision is still cleared, exactly as a batch
    /// `execute` would: a skip means "this needs a fresh decision," not
    /// "try again unchanged."
    pub fn execute_one(&self, id: &EinmoId, keys: &SignerSet) -> Result<Executed> {
        let decision = {
            let decisions = self.decisions.read().expect("decisions lock poisoned");
            decisions.get(id).cloned()
        };
        let action = match decision {
            Some(Decision::Promote { to }) => PlannedAction::Promote { id: id.clone(), to },
            Some(Decision::Retract { from }) => PlannedAction::Retract {
                id: id.clone(),
                from,
            },
            Some(Decision::Flag { stage, reason }) => PlannedAction::Flag {
                id: id.clone(),
                stage,
                reason,
            },
            Some(Decision::Skip) => {
                return Err(EinmoError::Config(format!(
                    "{id} is Skip: nothing to execute"
                )));
            }
            None => return Err(EinmoError::Config(format!("no pending decision for {id}"))),
        };
        // `claims` is irrelevant here: `execute` only ever reads
        // `plan.actions`, never `plan.claims` (claims are advisory display
        // data for a frontend, not part of execution).
        let plan = ExecutionPlan {
            actions: vec![action],
            claims: Vec::new(),
        };
        let report = self.execute(&plan, keys)?;
        report
            .executed
            .into_iter()
            .find(|e| &e.id == id)
            .ok_or_else(|| {
                EinmoError::Verification(format!(
                    "{id}'s decision drifted or its source no longer exists — \
                     re-decide against current content"
                ))
            })
    }
}

impl Drop for EinmoReview {
    /// Journals `SessionClose` (`EIMP-1` §S.6: "terse: session open/close")
    /// whenever a session ends, however the caller stops using it — no
    /// separate `close()` method to remember to call.
    fn drop(&mut self) {
        self.journal.log_at(
            crate::journal::JournalLevel::Terse,
            crate::journal::JournalEvent::SessionClose {
                session: self.journal.session_id().to_string(),
            },
        );
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

/// The stage whose content a decision is based on — `None` for `Skip` (no
/// content basis) or a `Promote` whose source no longer exists.
fn decision_basis_stage(config: &TestConfig, id: &EinmoId, decision: &Decision) -> Option<Stage> {
    match decision {
        Decision::Promote { to } => source_stage_for_promote(config, id, *to),
        Decision::Retract { from } => Some(*from),
        Decision::Flag { stage, .. } => Some(*stage),
        Decision::Skip => None,
    }
}

/// The path whose fingerprint [`EinmoReview::decide`]/[`EinmoReview::refresh`]
/// treat as `decision`'s basis.
fn decision_basis_path(
    config: &TestConfig,
    id: &EinmoId,
    decision: &Decision,
) -> Option<std::path::PathBuf> {
    let stage = decision_basis_stage(config, id, decision)?;
    Some(id.to_stage_path(config.work_dir(), stage))
}

/// The same basis-path lookup as [`decision_basis_path`], from a
/// [`PlannedAction`] instead of a [`Decision`] — used by `execute`, which
/// only has the plan's actions, not the originating decisions, in hand.
/// The two enums are shape-parallel by construction, so this mirrors
/// [`decision_basis_stage`] exactly rather than converting one into the
/// other.
fn action_basis_path(config: &TestConfig, action: &PlannedAction) -> Option<std::path::PathBuf> {
    let (id, stage) = match action {
        PlannedAction::Promote { id, to } => (id, source_stage_for_promote(config, id, *to)?),
        PlannedAction::Retract { id, from } => (id, *from),
        PlannedAction::Flag { id, stage, .. } => (id, *stage),
    };
    Some(id.to_stage_path(config.work_dir(), stage))
}

/// The case a [`PlannedAction`] concerns.
fn action_id(action: &PlannedAction) -> &EinmoId {
    match action {
        PlannedAction::Promote { id, .. }
        | PlannedAction::Retract { id, .. }
        | PlannedAction::Flag { id, .. } => id,
    }
}

// The per-case promote worker (formerly this module's own private
// `promote_one_accumulating`, applying EIMP-1 §S.4a's content-then-key
// decision table) moved to `EinmoCase::promote` (`EIMP-7` §S.3) — the ONE
// promote implementation, shared with the plain CLI path (`transitions.rs`,
// wired in a later EIMP-7 phase) instead of a private copy living here.

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    static JOURNAL_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct TestContext {
        _journal_guard: Option<std::sync::MutexGuard<'static, ()>>,
        _journal_tmp: Option<tempfile::TempDir>,
        suite: tempfile::TempDir,
    }

    impl TestContext {
        fn path(&self) -> &std::path::Path {
            self.suite.path()
        }

        /// Release `JOURNAL_ENV_LOCK` (and its scratch journal dir) early,
        /// before `self.suite` itself is dropped. `std::sync::Mutex` isn't
        /// reentrant, so a test that needs a *second*, independent
        /// `TestContext` alive at the same time (e.g. comparing the review
        /// path against a fresh CLI-driven suite) must call this once its
        /// own journal-writing calls are done, or the second
        /// `test_context()` call deadlocks waiting on a lock this same
        /// thread already holds.
        fn release_journal_lock(&mut self) {
            self._journal_guard = None;
            self._journal_tmp = None;
        }
    }

    fn test_context() -> TestContext {
        let guard = JOURNAL_ENV_LOCK.lock().unwrap();
        let journal_tmp = tempfile::tempdir().unwrap();
        // SAFETY: serialized by `JOURNAL_ENV_LOCK`.
        unsafe {
            std::env::set_var("EINMO_JOURNAL_DIR", journal_tmp.path());
        }
        TestContext {
            _journal_guard: Some(guard),
            _journal_tmp: Some(journal_tmp),
            suite: tempfile::tempdir().unwrap(),
        }
    }

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

    fn seeded_suite() -> TestContext {
        let ctx = test_context();
        write_input(ctx.path(), "a.foo", "{1+1;}");
        write_input(ctx.path(), "b.foo", "{2+2;}");
        let config = TestConfig::new(ctx.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite.evaluate_all(&Echo).unwrap();
        ctx
    }

    fn promote_output_to_checked(dir: &std::path::Path) {
        let config = TestConfig::new(dir, crate::einmo_suite::ValidationLevel::Output);
        EinmoSuite::scan(EinmoDirectory::new(config.clone()), None)
            .unwrap()
            .promote(
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

    // EIMP-1 S.2: ReviewOpts/ReviewMode.

    #[test]
    fn default_review_opts_is_full_mode_no_filter() {
        let opts = ReviewOpts::default();
        assert_eq!(opts.mode, ReviewMode::Full);
        assert_eq!(opts.filter, None);
    }

    #[test]
    fn signer_set_debug_never_renders_either_raw_passphrase() {
        // SignerSet derives Debug; this proves that's still safe purely
        // because KeySource's own hand-written Debug redacts -- no
        // separate hand-written impl needed here.
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase("checked-secret"),
            to_verified: Some(KeySource::from_passphrase("verified-secret")),
        };
        let rendered = format!("{keys:?}");
        assert!(!rendered.contains("checked-secret"));
        assert!(!rendered.contains("verified-secret"));
    }

    #[test]
    fn open_matches_open_with_default_opts() {
        let tmp = seeded_suite();
        let plain = EinmoReview::open(tmp.path()).items().unwrap();
        let explicit = EinmoReview::open_with(tmp.path(), ReviewOpts::default())
            .items()
            .unwrap();
        let mut plain_ids: Vec<_> = plain.iter().map(|i| i.id.as_str().to_string()).collect();
        let mut explicit_ids: Vec<_> = explicit.iter().map(|i| i.id.as_str().to_string()).collect();
        plain_ids.sort();
        explicit_ids.sort();
        assert_eq!(plain_ids, explicit_ids);
    }

    #[test]
    fn filter_restricts_items_by_substring() {
        let tmp = seeded_suite();
        let review = EinmoReview::open_with(
            tmp.path(),
            ReviewOpts {
                mode: ReviewMode::Full,
                filter: Some("a.foo".to_string()),
            },
        );
        let items = review.items().unwrap();
        let ids: Vec<_> = items.iter().map(|i| i.id.as_str().to_string()).collect();
        assert_eq!(ids, vec!["a.foo".to_string()]);
    }

    #[test]
    fn new_or_broken_mode_excludes_a_fully_matching_case() {
        let tmp = seeded_suite();
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        // Promote ONLY a.foo through checked AND verified with matching
        // content at every stage -- this case is NOT differing.
        //
        // `Some("a.foo")` here is load-bearing, not incidental: this test
        // predates EIMP-7 §S.6's fix and originally called the unfiltered
        // `promote_output_to_checked` (promotes EVERY case), relying on
        // the OLD all-three-stage `differing` bool to separately flag
        // b.foo as differing because its `verified/` was empty -- true,
        // but for the wrong reason. Once `differing` was correctly scoped
        // to output-vs-checked only, b.foo (output==checked after that
        // unfiltered promote) stopped reading as differing too, and this
        // test's own premise ("b.foo stays output-only") was revealed to
        // have been false all along. Filtering to "a.foo" here makes the
        // premise true, not just the assertion.
        EinmoSuite::scan(EinmoDirectory::new(config.clone()), None)
            .unwrap()
            .promote(
                Stage::Output,
                Stage::Checked,
                &KeySource::from_passphrase(""),
                Some("a.foo"),
                None,
            )
            .unwrap();
        EinmoSuite::scan(EinmoDirectory::new(config.clone()), None)
            .unwrap()
            .promote(
                Stage::Checked,
                Stage::Verified,
                &KeySource::from_passphrase(""),
                Some("a.foo"),
                None,
            )
            .unwrap();

        // b.foo stays output-only: no checked/verified baseline at all, so
        // it qualifies as "new" under NewOrBroken.
        let review = EinmoReview::open_with(
            tmp.path(),
            ReviewOpts {
                mode: ReviewMode::NewOrBroken,
                filter: None,
            },
        );
        let items = review.items().unwrap();
        let ids: Vec<_> = items.iter().map(|i| i.id.as_str().to_string()).collect();
        assert!(
            !ids.contains(&"a.foo".to_string()),
            "a fully-matching, fully-promoted case must not appear under NewOrBroken: {ids:?}"
        );
        assert!(ids.contains(&"b.foo".to_string()));
    }

    /// `EIMP-1`'s P1 finding, reproduced and asserted fixed at the
    /// `EinmoReview` level (not just `EinmoCase::agreement`'s own unit
    /// tests): a fresh suite where output and checked agree for every
    /// case, and `verified/` simply hasn't been populated yet -- the
    /// NORMAL starting state -- must report NOTHING under `NewOrBroken`.
    /// The old `TestRow::differing` bool required all THREE stages
    /// present and agreeing, so it read `true` for every single case
    /// here, making `-n`/`NewOrBroken` nearly useless on a typical suite.
    #[test]
    fn new_or_broken_excludes_cases_whose_verified_stage_is_simply_unpopulated_p1_repro() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        // Neither a.foo nor b.foo has a verified/ artifact -- untouched.

        let review = EinmoReview::open_with(
            tmp.path(),
            ReviewOpts {
                mode: ReviewMode::NewOrBroken,
                filter: None,
            },
        );
        let items = review.items().unwrap();
        assert!(
            items.is_empty(),
            "output and checked agree for every case; an unpopulated \
             verified/ must not make NewOrBroken report anything: {:?}",
            items
                .iter()
                .map(|i| i.id.as_str().to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn random_mode_returns_the_same_set_as_full() {
        let tmp = seeded_suite();
        let full = EinmoReview::open_with(
            tmp.path(),
            ReviewOpts {
                mode: ReviewMode::Full,
                filter: None,
            },
        )
        .items()
        .unwrap();
        let random = EinmoReview::open_with(
            tmp.path(),
            ReviewOpts {
                mode: ReviewMode::Random,
                filter: None,
            },
        )
        .items()
        .unwrap();
        let mut full_ids: Vec<_> = full.iter().map(|i| i.id.as_str().to_string()).collect();
        let mut random_ids: Vec<_> = random.iter().map(|i| i.id.as_str().to_string()).collect();
        assert_eq!(full_ids.len(), random_ids.len());
        full_ids.sort();
        random_ids.sort();
        assert_eq!(
            full_ids, random_ids,
            "Random must be a reordering, never a different set"
        );
    }

    // EIMP-1 S.7: EinmoReview::diff.

    #[test]
    fn diff_is_all_equal_when_stage_bodies_match() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let hunks = review.diff(&id, Stage::Output, Stage::Checked).unwrap();
        assert!(!hunks.sections.is_empty());
        for section in &hunks.sections {
            assert!(
                section
                    .lines
                    .iter()
                    .all(|l| matches!(l, DiffLine::Equal(_))),
                "section {:?} should be all-Equal when bodies match: {:?}",
                section.name,
                section.lines
            );
        }
    }

    #[test]
    fn diff_excludes_stamps_section() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let hunks = review.diff(&id, Stage::Output, Stage::Checked).unwrap();
        assert!(!hunks.sections.iter().any(|s| s.name == "STAMPS"));
    }

    #[test]
    fn diff_reports_line_level_changes_between_differing_stages() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path()); // checked := "2" (1+1)
        write_input(tmp.path(), "a.foo", "{3+3;}");
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        let regen = suite
            .regenerate_output(std::path::Path::new("a.foo"), &Echo)
            .unwrap();
        assert!(regen.written_and_verified);

        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let hunks = review.diff(&id, Stage::Output, Stage::Checked).unwrap();
        let output_section = hunks
            .sections
            .iter()
            .find(|s| s.name == "OUTPUT")
            .expect("OUTPUT section present on both sides");
        assert!(
            output_section
                .lines
                .iter()
                .any(|l| matches!(l, DiffLine::Removed(_) | DiffLine::Added(_))),
            "differing OUTPUT bodies must produce a non-Equal line: {:?}",
            output_section.lines
        );
    }

    // EIMP-1 S.2/S.5: EinmoReview::refresh (fingerprint-based drift
    // detection over a pending decision's basis content).

    #[test]
    fn refresh_reports_nothing_when_no_decisions_are_pending() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        assert!(review.refresh().is_empty());
    }

    #[test]
    fn refresh_reports_nothing_when_basis_content_is_unchanged() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id, Decision::Promote { to: Stage::Checked });
        assert!(review.refresh().is_empty());
    }

    #[test]
    fn refresh_reports_a_case_whose_basis_content_changed_since_decide() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });

        // Change what a.foo evaluates to -- the output/ content the
        // decision was based on is no longer what's on disk.
        write_input(tmp.path(), "a.foo", "{9+9;}");
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite
            .regenerate_output(std::path::Path::new("a.foo"), &Echo)
            .unwrap();

        let stale = review.refresh();
        assert_eq!(stale, vec![id]);
    }

    #[test]
    fn refresh_does_not_report_an_undecided_case_even_if_its_output_changes() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        // b.foo has no decision at all.
        write_input(tmp.path(), "b.foo", "{9+9;}");
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite
            .regenerate_output(std::path::Path::new("b.foo"), &Echo)
            .unwrap();
        assert!(review.refresh().is_empty());
    }

    #[test]
    fn execute_skips_a_drifted_decision_and_never_applies_stale_content() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });

        write_input(tmp.path(), "a.foo", "{9+9;}");
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite
            .regenerate_output(std::path::Path::new("a.foo"), &Echo)
            .unwrap();

        let plan = review.plan();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let report = review.execute(&plan, &keys).unwrap();
        assert!(report.skipped.contains(&id));
        assert!(!report.executed.iter().any(|e| e.id == id));
        assert!(
            !tmp.path().join("checked").join("a.foo.einmo").exists(),
            "a drifted decision must never be applied, not even with stale content"
        );
    }

    // EIMP-1 S.2: EinmoReview::execute_one.

    #[test]
    fn execute_one_applies_a_pending_decision() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let executed = review.execute_one(&id, &keys).unwrap();
        assert_eq!(executed.id, id);
        assert!(tmp.path().join("checked").join("a.foo.einmo").exists());
    }

    #[test]
    fn execute_one_clears_the_decision_afterward() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        review.execute_one(&id, &keys).unwrap();
        assert!(review.plan().actions.is_empty());
    }

    #[test]
    fn execute_one_errors_when_no_decision_is_pending() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        assert!(review.execute_one(&id, &keys).is_err());
    }

    #[test]
    fn execute_one_errors_on_a_skip_decision() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Skip);
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        assert!(review.execute_one(&id, &keys).is_err());
    }

    #[test]
    fn execute_one_errors_when_the_decision_has_drifted() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        write_input(tmp.path(), "a.foo", "{9+9;}");
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite
            .regenerate_output(std::path::Path::new("a.foo"), &Echo)
            .unwrap();

        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        assert!(review.execute_one(&id, &keys).is_err());
        assert!(!tmp.path().join("checked").join("a.foo.einmo").exists());
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

    // EIMP-1 S.6: the journal.

    #[test]
    fn open_writes_a_session_open_entry() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let lines = crate::journal::Journal::replay(&review.journal_path());
        assert!(
            lines
                .iter()
                .any(|l| matches!(&l.event, crate::journal::JournalEvent::SessionOpen { session, .. } if session == review.session_id())),
        );
    }

    #[test]
    fn decide_and_undecide_are_journaled() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Skip);
        review.undecide(&id);

        let lines = crate::journal::Journal::replay(&review.journal_path());
        let decided = lines
            .iter()
            .any(|l| matches!(&l.event, crate::journal::JournalEvent::Decide { id: eid, .. } if eid == id.as_str()));
        let undecided = lines
            .iter()
            .any(|l| matches!(&l.event, crate::journal::JournalEvent::Undecide { id: eid } if eid == id.as_str()));
        assert!(decided, "decide() must be journaled: {lines:?}");
        assert!(undecided, "undecide() must be journaled: {lines:?}");
    }

    #[test]
    fn execute_logs_one_execute_batch_entry() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        review
            .execute(
                &review.plan(),
                &SignerSet {
                    to_checked: KeySource::from_passphrase(""),
                    to_verified: None,
                },
            )
            .unwrap();

        let lines = crate::journal::Journal::replay(&review.journal_path());
        let batch = lines.iter().find_map(|l| match &l.event {
            crate::journal::JournalEvent::ExecuteBatch { executed, skipped } => {
                Some((executed.clone(), skipped.clone()))
            }
            _ => None,
        });
        let (executed, _skipped) = batch.expect("an ExecuteBatch entry must be journaled");
        assert!(executed.contains(&id.as_str().to_string()));
    }

    #[test]
    fn resume_reconstructs_a_pending_decision_left_by_a_dropped_session() {
        let tmp = seeded_suite();
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let session_id = {
            let review = EinmoReview::open(tmp.path());
            review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
            review.session_id().to_string()
            // `review` drops here without ever calling execute() -- simulates
            // the process crashing with a decision still pending.
        };

        let resumed = EinmoReview::resume(tmp.path(), &session_id, ReviewOpts::default()).unwrap();
        assert_eq!(
            resumed.decision(&id),
            Some(Decision::Promote { to: Stage::Checked }),
            "resume must reconstruct the decision the dropped session left pending"
        );
    }

    #[test]
    fn resume_replays_undecide_so_a_cleared_decision_stays_cleared() {
        let tmp = seeded_suite();
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let session_id = {
            let review = EinmoReview::open(tmp.path());
            review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
            review.undecide(&id);
            review.session_id().to_string()
        };

        let resumed = EinmoReview::resume(tmp.path(), &session_id, ReviewOpts::default()).unwrap();
        assert_eq!(resumed.decision(&id), None);
    }

    #[test]
    fn resume_of_an_unknown_session_id_is_a_fresh_empty_review() {
        let tmp = seeded_suite();
        let resumed = EinmoReview::resume(
            tmp.path(),
            "never-seen-before-session",
            ReviewOpts::default(),
        )
        .unwrap();
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        assert_eq!(resumed.decision(&id), None);
    }

    #[test]
    fn resume_reconstructs_multiple_decisions_and_undecides() {
        let tmp = seeded_suite();
        let a = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let b = EinmoId::from_input_rel(std::path::Path::new("b.foo")).unwrap();
        let session_id = {
            let review = EinmoReview::open(tmp.path());
            review.decide(a.clone(), Decision::Promote { to: Stage::Checked });
            review.decide(b.clone(), Decision::Skip);
            review.decide(
                a.clone(),
                Decision::Flag {
                    stage: Stage::Output,
                    reason: "changed my mind".into(),
                },
            );
            review.undecide(&b);
            review.session_id().to_string()
        };

        let resumed = EinmoReview::resume(tmp.path(), &session_id, ReviewOpts::default()).unwrap();
        assert_eq!(
            resumed.decision(&a),
            Some(Decision::Flag {
                stage: Stage::Output,
                reason: "changed my mind".into(),
            }),
            "the last decide for a.foo must win (replace-not-stack)"
        );
        assert_eq!(
            resumed.decision(&b),
            None,
            "undecide must clear b.foo even after a decide"
        );
    }

    #[test]
    fn resume_tolerates_a_truncated_journal_tail() {
        let tmp = seeded_suite();
        let a = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let b = EinmoId::from_input_rel(std::path::Path::new("b.foo")).unwrap();
        let session_id = {
            let review = EinmoReview::open(tmp.path());
            review.decide(a.clone(), Decision::Promote { to: Stage::Checked });
            review.decide(b.clone(), Decision::Skip);
            review.session_id().to_string()
        };

        // Simulate a crash mid-write: append a truncated JSON line.
        let journal_path = crate::journal::journal_path(&session_id);
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&journal_path)
                .unwrap();
            writeln!(
                f,
                "{{\"timestamp\":\"2026-01-01T00:00:00Z\",\"event\":\"decide\""
            )
            .unwrap();
        }

        let resumed = EinmoReview::resume(tmp.path(), &session_id, ReviewOpts::default()).unwrap();
        assert_eq!(
            resumed.decision(&a),
            Some(Decision::Promote { to: Stage::Checked }),
            "the valid decide for a.foo must survive a truncated tail"
        );
        assert_eq!(
            resumed.decision(&b),
            Some(Decision::Skip),
            "the valid decide for b.foo must survive a truncated tail"
        );
    }

    // EIMP-1 S.5: soft claims.

    #[test]
    fn claim_appears_in_plan_output() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.claim(&id);

        let plan = review.plan();
        let claim = plan.claims.iter().find(|c| c.id == id);
        assert!(
            claim.is_some(),
            "an active claim must appear in plan().claims"
        );
        assert!(claim.unwrap().remaining <= std::time::Duration::from_secs(5 * 60));
    }

    #[test]
    fn claim_is_advisory_and_never_blocks_decide_or_execute() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.claim(&id);
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let report = review
            .execute(
                &review.plan(),
                &SignerSet {
                    to_checked: KeySource::from_passphrase(""),
                    to_verified: None,
                },
            )
            .unwrap();
        assert!(
            report.executed.iter().any(|e| e.id == id),
            "an active claim must never block a decision from executing"
        );
    }

    #[test]
    fn claim_refreshes_rather_than_stacks() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.claim(&id);
        review.claim(&id);
        let plan = review.plan();
        assert_eq!(
            plan.claims.iter().filter(|c| c.id == id).count(),
            1,
            "re-claiming the same case must refresh, not accumulate entries"
        );
    }

    #[test]
    fn an_expired_claim_is_auto_reclaimed() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.claim_for(&id, std::time::Duration::from_millis(10));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let plan = review.plan();
        assert!(
            !plan.claims.iter().any(|c| c.id == id),
            "an expired claim must be silently released, not linger: {:?}",
            plan.claims
        );
    }

    #[test]
    fn decision_reports_the_answer_so_far() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        assert_eq!(review.decision(&id), None);

        review.decide(id.clone(), Decision::Skip);
        assert_eq!(review.decision(&id), Some(Decision::Skip));

        review.undecide(&id);
        assert_eq!(review.decision(&id), None);
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
        let mut tmp = seeded_suite();
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

        // This test's journal-writing calls against `tmp` are done; release
        // its lock before opening a second, independent `TestContext` below
        // -- `JOURNAL_ENV_LOCK` isn't reentrant, so holding both at once on
        // this one thread would deadlock (`TestContext::release_journal_lock`).
        tmp.release_journal_lock();

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

    fn seeded_suite_with_same_content() -> TestContext {
        seeded_suite()
    }

    // EIMP-1 S.4a: content-then-key decision table for execute's promote.

    #[test]
    fn execute_promote_is_a_true_noop_when_dest_already_matches_and_is_mine() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };

        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        review.execute(&review.plan(), &keys).unwrap();
        let checked_path = tmp.path().join("checked").join("a.foo.einmo");
        let bytes_before = std::fs::read(&checked_path).unwrap();

        // Re-decide and re-execute the SAME promotion, same signer, content
        // unchanged: must be a true no-op, byte-for-byte.
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let report = review.execute(&review.plan(), &keys).unwrap();
        assert!(report.executed.iter().any(|e| e.id == id));

        let bytes_after = std::fs::read(&checked_path).unwrap();
        assert_eq!(
            bytes_before, bytes_after,
            "re-promoting unchanged content under the same signer must not touch the file"
        );
    }

    #[test]
    fn execute_promote_appends_a_second_signers_stamp_when_content_matches() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        review
            .execute(
                &review.plan(),
                &SignerSet {
                    to_checked: KeySource::from_passphrase(""),
                    to_verified: None,
                },
            )
            .unwrap();

        // Second signer, same (unchanged) content.
        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let report = review
            .execute(
                &review.plan(),
                &SignerSet {
                    to_checked: KeySource::from_passphrase("second signer"),
                    to_verified: None,
                },
            )
            .unwrap();
        assert!(report.executed.iter().any(|e| e.id == id));

        let checked_path = tmp.path().join("checked").join("a.foo.einmo");
        let file = EinmoFile::from_file(&checked_path).unwrap();
        assert_eq!(
            body_sections(&file, None),
            body_sections(
                &EinmoFile::from_file(&tmp.path().join("output/a.foo.einmo")).unwrap(),
                None
            ),
            "content must be untouched by the co-sign"
        );
        let checked_stamps = file
            .stamps()
            .entries()
            .iter()
            .filter(|s| s.key() == Stage::Checked.stamp_key())
            .count();
        assert_eq!(
            checked_stamps, 2,
            "both signers' stage:checked stamps must be present"
        );
    }

    #[test]
    fn execute_promote_writes_a_fresh_baseline_when_content_genuinely_differs() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };

        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        review.execute(&review.plan(), &keys).unwrap();

        // Change what a.foo evaluates to, accept it via regenerate_output
        // (EIMP-3), then decide+execute promote again -- output now
        // genuinely differs from the existing checked/ baseline.
        write_input(tmp.path(), "a.foo", "{9+9;}");
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite
            .regenerate_output(std::path::Path::new("a.foo"), &Echo)
            .unwrap();

        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let report = review.execute(&review.plan(), &keys).unwrap();
        assert!(
            report.executed.iter().any(|e| e.id == id),
            "a genuine content change must promote, not skip or fail: {report:?}"
        );

        let checked_path = tmp.path().join("checked").join("a.foo.einmo");
        let file = EinmoFile::from_file(&checked_path).unwrap();
        assert_eq!(file.section("OUTPUT").unwrap().body(), "{9+9;}");
        let checked_stamps = file
            .stamps()
            .entries()
            .iter()
            .filter(|s| s.key() == Stage::Checked.stamp_key())
            .count();
        assert_eq!(
            checked_stamps, 1,
            "a fresh baseline gets a fresh stamp chain, not an accumulated one"
        );
    }

    #[test]
    fn execute_clears_pending_decisions_it_applied() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let plan = review.plan();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let report = review.execute(&plan, &keys).unwrap();
        assert_eq!(report.executed.len(), 1);

        // the applied decision must no longer show up as pending
        let next_plan = review.plan();
        assert!(
            next_plan.actions.is_empty(),
            "an executed decision must not linger in the next plan"
        );
        let items = review.items().unwrap();
        let item = items.iter().find(|i| i.id == id).unwrap();
        assert_eq!(item.decision, None);
    }

    #[test]
    fn execute_clears_pending_decisions_it_skipped() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.decide(id.clone(), Decision::Promote { to: Stage::Checked });
        let plan = review.plan();
        // Source drifts between planning and execution: the output artifact
        // this plan targeted is gone by the time execute actually runs.
        std::fs::remove_file(tmp.path().join("output/a.foo.einmo")).unwrap();
        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let report = review.execute(&plan, &keys).unwrap();
        assert_eq!(report.skipped, vec![id.clone()]);
        assert!(report.executed.is_empty());

        let next_plan = review.plan();
        assert!(
            next_plan.actions.is_empty(),
            "a skipped decision must not linger in the next plan either"
        );
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
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
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
        EinmoSuite::scan(EinmoDirectory::new(config.clone()), None)
            .unwrap()
            .promote(
                Stage::Output,
                Stage::Checked,
                &KeySource::from_passphrase(""),
                None,
                None,
            )
            .unwrap();
        EinmoSuite::scan(EinmoDirectory::new(config.clone()), None)
            .unwrap()
            .promote(
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
    fn execute_retract_from_verified_removes_only_verified() {
        let tmp = seeded_suite();
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        EinmoSuite::scan(EinmoDirectory::new(config.clone()), None)
            .unwrap()
            .promote(
                Stage::Output,
                Stage::Checked,
                &KeySource::from_passphrase(""),
                None,
                None,
            )
            .unwrap();
        EinmoSuite::scan(EinmoDirectory::new(config.clone()), None)
            .unwrap()
            .promote(
                Stage::Checked,
                Stage::Verified,
                &KeySource::from_passphrase(""),
                None,
                None,
            )
            .unwrap();

        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        review.decide(
            id.clone(),
            Decision::Retract {
                from: Stage::Verified,
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
            !tmp.path().join("verified/a.foo.einmo").exists(),
            "verified artifact must be removed"
        );
        assert!(
            tmp.path().join("checked/a.foo.einmo").exists(),
            "checked baseline must survive — verified is the top of the chain"
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
        let flagged =
            std::fs::read_to_string(tmp.path().join("output/flagged/a.foo.einmo")).unwrap();
        assert!(flagged.contains("# flagged: looks wrong"));
    }

    #[test]
    fn flag_now_is_atomic_no_decide_or_execute_needed() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.flag_now(&id, Stage::Output, "looks wrong").unwrap();

        assert!(!tmp.path().join("output/a.foo.einmo").exists());
        let flagged =
            std::fs::read_to_string(tmp.path().join("output/flagged/a.foo.einmo")).unwrap();
        assert!(flagged.contains("# flagged: looks wrong"));
    }

    #[test]
    fn flag_now_concatenates_with_an_existing_flagged_note() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.flag_now(&id, Stage::Output, "first").unwrap();
        // flag_now moves output/a.foo.einmo away; regenerate it so there is
        // something to flag again.
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite
            .evaluate(std::path::Path::new("a.foo"), &Echo)
            .unwrap();
        review.flag_now(&id, Stage::Output, "second").unwrap();

        let flagged =
            std::fs::read_to_string(tmp.path().join("output/flagged/a.foo.einmo")).unwrap();
        assert!(
            flagged.contains("# flagged: first"),
            "the earlier flag must survive: {flagged:?}"
        );
        assert!(
            flagged.contains("# flagged: second"),
            "the new flag must be present: {flagged:?}"
        );
        let first_pos = flagged.find("# flagged: first").unwrap();
        let second_pos = flagged.find("# flagged: second").unwrap();
        assert!(
            second_pos < first_pos,
            "the newest flag's block goes on top: {flagged:?}"
        );
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
    fn retract_now_is_atomic_no_decide_or_execute_needed() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.retract_now(&id, Stage::Checked).unwrap();

        assert!(!tmp.path().join("checked/a.foo.einmo").exists());
    }

    #[test]
    fn retract_now_cascades_checked_to_verified() {
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        EinmoSuite::scan(EinmoDirectory::new(config.clone()), None)
            .unwrap()
            .promote(
                Stage::Checked,
                Stage::Verified,
                &KeySource::from_passphrase(""),
                None,
                None,
            )
            .unwrap();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        review.retract_now(&id, Stage::Checked).unwrap();

        assert!(!tmp.path().join("checked/a.foo.einmo").exists());
        assert!(
            !tmp.path().join("verified/a.foo.einmo").exists(),
            "cascade must remove the verified artifact too"
        );
    }

    #[test]
    fn retract_now_clears_any_pending_decision() {
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

        review.retract_now(&id, Stage::Checked).unwrap();

        let items = review.items().unwrap();
        let item = items.iter().find(|i| i.id == id).unwrap();
        assert_eq!(item.decision, None);
    }

    #[test]
    fn retract_now_errors_when_stage_has_nothing_for_id() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        let err = review.retract_now(&id, Stage::Checked).unwrap_err();
        assert!(matches!(err, EinmoError::Io { .. }));
    }

    #[test]
    fn retract_now_refuses_output_stage() {
        let tmp = seeded_suite();
        let review = EinmoReview::open(tmp.path());
        let id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();

        let err = review.retract_now(&id, Stage::Output).unwrap_err();
        assert!(matches!(err, EinmoError::Config(_)));
    }

    #[test]
    fn retract_now_serializes_against_a_concurrent_execute_on_the_same_id() {
        // retract_now must take the same `exec` mutex flag_now/execute
        // already do -- a concurrent execute() promoting THIS SAME id and
        // a retract_now() for it must never interleave. Whichever runs
        // first, the end state must be internally consistent (never a
        // torn write): if retract wins the race, execute's source file is
        // gone and it reports the id as skipped, not executed; if execute
        // wins, it promotes to verified and retract's cascade then removes
        // both checked and verified. Either way, neither call may panic,
        // deadlock, or leave a half-written `.einmo` file.
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let review = Arc::new(EinmoReview::open(tmp.path()));
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
            to_verified: Some(KeySource::from_passphrase("s3cr3t")),
        };

        let review_for_retract = Arc::clone(&review);
        let id_for_retract = id.clone();
        let retract_handle =
            thread::spawn(move || review_for_retract.retract_now(&id_for_retract, Stage::Checked));
        let review_for_execute = Arc::clone(&review);
        let execute_handle = thread::spawn(move || review_for_execute.execute(&plan, &keys));

        let retract_result = retract_handle.join().unwrap();
        let execute_result = execute_handle.join().unwrap();
        assert!(
            execute_result.is_ok(),
            "execute must not error: {execute_result:?}"
        );

        // Whichever order actually ran, neither `checked/` nor `verified/`
        // may hold a corrupt (unreadable / chain-invalid) file for a.foo --
        // either both are gone (retract's cascade covers both orderings)
        // or, if retract somehow missed the file entirely and returned an
        // IO error, whatever execute wrote must still verify cleanly.
        for stage_dir in ["checked", "verified"] {
            let path = tmp.path().join(stage_dir).join("a.foo.einmo");
            if path.exists() {
                let file = EinmoFile::from_file(&path)
                    .unwrap_or_else(|e| panic!("{stage_dir}/a.foo.einmo must verify: {e}"));
                assert!(
                    file.chain_valid(),
                    "{stage_dir}/a.foo.einmo chain must be valid"
                );
            }
        }
        let _ = retract_result; // either outcome (Ok or NotFound) is valid depending on ordering
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

    #[test]
    fn execute_reports_non_human_for_a_computer_key_verified_promotion_but_not_a_real_one() {
        // The session `execute` path must not silently drop the
        // `non_human` signal `transitions::promote` already computes and
        // tests (`empty_passphrase_verified_is_flagged_non_human`,
        // `transitions.rs`) -- an empty-passphrase verified promotion is
        // indistinguishable from a genuine human attestation otherwise.
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path());
        let review = EinmoReview::open(tmp.path());
        let a = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let b = EinmoId::from_input_rel(std::path::Path::new("b.foo")).unwrap();

        review.decide(
            a.clone(),
            Decision::Promote {
                to: Stage::Verified,
            },
        );
        review.decide(
            b.clone(),
            Decision::Promote {
                to: Stage::Verified,
            },
        );
        let plan = review.plan();

        // a.foo: the well-known computer key (empty passphrase).
        let computer_keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: Some(KeySource::from_passphrase("")),
        };
        let computer_report = review
            .execute(
                &ExecutionPlan {
                    actions: plan
                        .actions
                        .iter()
                        .filter(|act| action_id(act) == &a)
                        .cloned()
                        .collect(),
                    claims: Vec::new(),
                },
                &computer_keys,
            )
            .unwrap();
        assert_eq!(computer_report.executed.len(), 1);
        assert!(
            computer_report.executed[0].non_human,
            "empty-passphrase verified promotion must report non_human: true"
        );

        // b.foo: a real human passphrase.
        let human_keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: Some(KeySource::from_passphrase("a-real-passphrase")),
        };
        let human_report = review
            .execute(
                &ExecutionPlan {
                    actions: plan
                        .actions
                        .into_iter()
                        .filter(|act| action_id(act) == &b)
                        .collect(),
                    claims: Vec::new(),
                },
                &human_keys,
            )
            .unwrap();
        assert_eq!(human_report.executed.len(), 1);
        assert!(
            !human_report.executed[0].non_human,
            "a real passphrase's verified promotion must report non_human: false"
        );
    }

    #[test]
    fn execute_missing_verified_key_aborts_the_whole_batch_untouched() {
        // Two cases in the SAME batch needing DIFFERENT (from, to) groups:
        // `a.foo` -> verified (needs a key `keys` doesn't supply) and
        // `b.foo` -> checked (needs only the always-present checked key).
        // `HashMap` iteration order is unspecified, so the old bug (`?`
        // inside the mutating loop) could let `b.foo`'s promotion land on
        // disk before `a.foo`'s missing key aborted the function -- assert
        // NEITHER group's promotion ever took effect, regardless of which
        // one iteration happens to reach first.
        let tmp = seeded_suite();
        promote_output_to_checked(tmp.path()); // a.foo AND b.foo now in checked/
        let review = EinmoReview::open(tmp.path());
        let a = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let b = EinmoId::from_input_rel(std::path::Path::new("b.foo")).unwrap();

        // Retract b.foo (only) back out of checked/ so promoting it to
        // checked again is a genuine, file-writing `(Output, Checked)`
        // group -- distinct from a.foo's `(Checked, Verified)` group.
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        EinmoSuite::scan(EinmoDirectory::new(config), None)
            .unwrap()
            .retract(Stage::Checked, None, Some(std::slice::from_ref(&b)))
            .unwrap();

        review.decide(
            a.clone(),
            Decision::Promote {
                to: Stage::Verified,
            },
        );
        review.decide(b.clone(), Decision::Promote { to: Stage::Checked });
        let plan = review.plan();
        assert_eq!(plan.actions.len(), 2);

        let keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        };
        let err = review.execute(&plan, &keys).unwrap_err();
        assert!(matches!(err, EinmoError::NoKey(_)));

        assert!(
            !tmp.path().join("checked").join("b.foo.einmo").exists(),
            "b.foo's (Output, Checked) group must not be applied when a.foo's \
             (Checked, Verified) group in the SAME batch is missing its key"
        );
        assert!(
            !tmp.path().join("verified").join("a.foo.einmo").exists(),
            "a.foo must not be (partially) promoted to verified either"
        );

        // Neither decision was consumed -- both are still pending, exactly
        // as if execute() had never been called.
        let items = review.items().unwrap();
        let decision_for = |id: &EinmoId| {
            items
                .iter()
                .find(|i| &i.id == id)
                .and_then(|i| i.decision.clone())
        };
        assert_eq!(
            decision_for(&a),
            Some(Decision::Promote {
                to: Stage::Verified
            })
        );
        assert_eq!(
            decision_for(&b),
            Some(Decision::Promote { to: Stage::Checked })
        );
    }

    #[test]
    fn concurrent_execute_calls_do_not_corrupt_state() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..4 {
            write_input(tmp.path(), &format!("case{i}.foo"), "{1+1;}");
        }
        let config = TestConfig::new(tmp.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite.evaluate_all(&Echo).unwrap();

        let review = Arc::new(EinmoReview::open(tmp.path()));
        for i in 0..4 {
            let id =
                EinmoId::from_input_rel(std::path::Path::new(&format!("case{i}.foo"))).unwrap();
            review.decide(id, Decision::Promote { to: Stage::Checked });
        }

        let keys = Arc::new(SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: None,
        });

        let handles: Vec<_> = (0..2)
            .map(|_| {
                let review = Arc::clone(&review);
                let keys = Arc::clone(&keys);
                thread::spawn(move || {
                    let plan = review.plan();
                    review.execute(&plan, &keys)
                })
            })
            .collect();

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert!(
            results.iter().all(|r| r.is_ok()),
            "both concurrent executes must succeed: {results:?}"
        );

        for i in 0..4 {
            let path = tmp
                .path()
                .join("checked")
                .join(format!("case{i}.foo.einmo"));
            assert!(path.exists(), "case{i} must have been promoted");
            let file = EinmoFile::from_file(&path).unwrap();
            assert!(file.chain_valid(), "case{i} stamp chain must be valid");
        }
    }

    /// EIMP-1 comprehensive integration test: a scripted multi-reviewer
    /// end-to-end session over a 3-case fixture suite.
    ///
    /// Exercises:
    /// 1. Setup — 3 cases, all output promoted to checked.
    /// 2. Reviewer A — promotes `a.foo` to verified.
    /// 3. Reviewer B — promotes `b.foo` to verified (different session, same
    ///    passphrase — both stamps coexist).
    /// 4. Crash-resume — a third session decides `c.foo` then drops without
    ///    executing; `resume` reconstructs the pending decision from the
    ///    journal.
    /// 5. Drift detection — `c.foo`'s output is regenerated; `refresh()`
    ///    reports it; fresh decide + execute succeeds.
    /// 6. Stamp chain verification — every `.einmo` in `checked/` and
    ///    `verified/` passes `EinmoFile::from_file`; all three verified
    ///    files carry a `stage:verified` stamp.
    /// 7. Journal reconstruction — the resumed session's journal contains
    ///    the expected decide and execute-batch events.
    fn collect_einmo_files_recursive(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_einmo_files_recursive(&path, out);
                } else if path.extension().is_some_and(|e| e == "einmo") {
                    out.push(path);
                }
            }
        }
    }

    #[test]
    fn comprehensive_multi_reviewer_end_to_end() {
        // ── Step 1: fixture suite with 3 cases, all promoted to checked ──

        let ctx = test_context();
        write_input(ctx.path(), "a.foo", "{1+1;}");
        write_input(ctx.path(), "b.foo", "{2+2;}");
        write_input(ctx.path(), "c.foo", "{3+3;}");
        let config = TestConfig::new(ctx.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite.evaluate_all(&Echo).unwrap();
        promote_output_to_checked(ctx.path());

        let passphrase = "comprehensive-reviewer";
        let a = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let b = EinmoId::from_input_rel(std::path::Path::new("b.foo")).unwrap();
        let c = EinmoId::from_input_rel(std::path::Path::new("c.foo")).unwrap();

        let verified_keys = SignerSet {
            to_checked: KeySource::from_passphrase(""),
            to_verified: Some(KeySource::from_passphrase(passphrase)),
        };

        // ── Step 2: Reviewer A promotes a.foo to verified ──

        let session_a_id;
        {
            let review_a = EinmoReview::open(ctx.path());
            review_a.decide(
                a.clone(),
                Decision::Promote {
                    to: Stage::Verified,
                },
            );
            let report = review_a.execute(&review_a.plan(), &verified_keys).unwrap();
            assert_eq!(report.executed.len(), 1);
            assert!(report.executed.iter().any(|e| e.id == a));
            session_a_id = review_a.session_id().to_string();
        }

        // ── Step 3: Reviewer B promotes b.foo to verified (different session) ──

        let session_b_id;
        {
            let review_b = EinmoReview::open(ctx.path());
            review_b.decide(
                b.clone(),
                Decision::Promote {
                    to: Stage::Verified,
                },
            );
            let report = review_b.execute(&review_b.plan(), &verified_keys).unwrap();
            assert_eq!(report.executed.len(), 1);
            assert!(report.executed.iter().any(|e| e.id == b));
            session_b_id = review_b.session_id().to_string();
        }

        // Sessions A and B must be distinct — the whole point is two
        // independent reviewers.
        assert_ne!(session_a_id, session_b_id);

        // Both verified files exist with valid stamp chains and carry a
        // stage:verified stamp — the multi-reviewer coexistence proof.
        let a_verified = EinmoFile::from_file(&ctx.path().join("verified/a.foo.einmo")).unwrap();
        assert!(a_verified.chain_valid(), "a.foo chain must be valid");
        assert!(
            a_verified
                .stamps()
                .entries()
                .iter()
                .any(|s| s.key() == Stage::Verified.stamp_key()),
            "a.foo must carry a stage:verified stamp"
        );

        let b_verified = EinmoFile::from_file(&ctx.path().join("verified/b.foo.einmo")).unwrap();
        assert!(b_verified.chain_valid(), "b.foo chain must be valid");
        assert!(
            b_verified
                .stamps()
                .entries()
                .iter()
                .any(|s| s.key() == Stage::Verified.stamp_key()),
            "b.foo must carry a stage:verified stamp"
        );

        // ── Step 4: crash-resume — decide c.foo then drop without execute ──

        let crash_session_id;
        {
            let crash_review = EinmoReview::open(ctx.path());
            crash_review.decide(
                c.clone(),
                Decision::Promote {
                    to: Stage::Verified,
                },
            );
            crash_session_id = crash_review.session_id().to_string();
            // crash_review drops here — simulates a crash with c.foo pending
        }

        let resumed =
            EinmoReview::resume(ctx.path(), &crash_session_id, ReviewOpts::default()).unwrap();
        assert_eq!(
            resumed.decision(&c),
            Some(Decision::Promote {
                to: Stage::Verified
            }),
            "resume must reconstruct the pending c.foo decision from the journal"
        );

        // ── Step 5: drift detection — regenerate c.foo, refresh, re-decide ──

        write_input(ctx.path(), "c.foo", "{99+99;}");
        let config = TestConfig::new(ctx.path(), crate::einmo_suite::ValidationLevel::Output);
        let suite = crate::einmo_suite::EinmoTestRunner::new(config);
        suite
            .regenerate_output(std::path::Path::new("c.foo"), &Echo)
            .unwrap();
        // Re-promote output→checked so the checked baseline (the decision's
        // basis for a checked→verified promotion) reflects the new content.
        EinmoSuite::scan(
            EinmoDirectory::new(TestConfig::new(
                ctx.path(),
                crate::einmo_suite::ValidationLevel::Output,
            )),
            None,
        )
        .unwrap()
        .promote(
            Stage::Output,
            Stage::Checked,
            &KeySource::from_passphrase(""),
            Some("c.foo"),
            None,
        )
        .unwrap();

        let drifted = resumed.refresh();
        assert!(
            drifted.contains(&c),
            "refresh must report c.foo as drifted after content change"
        );

        // Decide fresh against the new output and execute.
        resumed.decide(
            c.clone(),
            Decision::Promote {
                to: Stage::Verified,
            },
        );
        let report = resumed.execute(&resumed.plan(), &verified_keys).unwrap();
        assert_eq!(report.executed.len(), 1);
        assert!(report.executed.iter().any(|e| e.id == c));

        // ── Step 6: stamp chain verification — every .einmo is valid ──

        for stage_dir in &["checked", "verified"] {
            let dir = ctx.path().join(stage_dir);
            if !dir.exists() {
                continue;
            }
            let mut einmo_files: Vec<std::path::PathBuf> = Vec::new();
            collect_einmo_files_recursive(&dir, &mut einmo_files);
            einmo_files.sort();
            for path in &einmo_files {
                let file = EinmoFile::from_file(path).unwrap();
                assert!(
                    file.chain_valid(),
                    "{}/{} stamp chain must be valid",
                    stage_dir,
                    path.strip_prefix(ctx.path()).unwrap_or(path).display()
                );
            }
        }

        // Specific: all three verified files carry stage:verified.
        let c_verified = EinmoFile::from_file(&ctx.path().join("verified/c.foo.einmo")).unwrap();
        assert!(c_verified.chain_valid(), "c.foo chain must be valid");
        assert!(
            c_verified
                .stamps()
                .entries()
                .iter()
                .any(|s| s.key() == Stage::Verified.stamp_key()),
            "c.foo must carry a stage:verified stamp"
        );
        assert_eq!(
            c_verified.section("OUTPUT").unwrap().body(),
            "{99+99;}",
            "c.foo verified content must reflect the regenerated output"
        );

        // ── Step 7: journal reconstruction — verify expected events ──

        let journal_lines =
            crate::journal::Journal::replay(&crate::journal::journal_path(&crash_session_id));

        // Must contain SessionOpen for the crash session.
        assert!(
            journal_lines.iter().any(|l| matches!(
                &l.event,
                crate::journal::JournalEvent::SessionOpen { session, .. }
                    if session == &crash_session_id
            )),
            "journal must contain SessionOpen for the crash session"
        );

        // At least two Decide events for c.foo: the crash session's original
        // decide, the resumed session's replayed decide, and the fresh decide
        // after drift.
        let decide_for_c = journal_lines
            .iter()
            .filter(|l| {
                matches!(
                    &l.event,
                    crate::journal::JournalEvent::Decide { id, .. } if id == c.as_str()
                )
            })
            .count();
        assert!(
            decide_for_c >= 2,
            "journal must contain at least 2 Decide events for c.foo, found {decide_for_c}"
        );

        // Must contain an ExecuteBatch that actually executed c.foo.
        assert!(
            journal_lines.iter().any(|l| matches!(
                &l.event,
                crate::journal::JournalEvent::ExecuteBatch { executed, .. }
                    if executed.contains(&c.as_str().to_string())
            )),
            "journal must contain an ExecuteBatch with c.foo in executed"
        );
    }

    /// A minimal signed envelope's bytes with an explicit COMMENTS body —
    /// `case.rs`'s own `signed_bytes` test helper, duplicated rather than
    /// exported, since it exists purely to fixture the comments-only-
    /// divergence case below by hand, bypassing `evaluate_all` (which has
    /// no way to make output and checked diverge only in COMMENTS).
    fn signed_bytes_with_comments(rel: &str, output: &str, comments: &str) -> Vec<u8> {
        use crate::format::{DEFAULT_SEPARATOR, Metadata, Section, Status};
        use crate::signature::{Stamps, derive_keypair};
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

    /// `EIMP-7`'s own comprehensive test (`EIMP-7.md` §Test Plan, "Comprehensive
    /// test"): a single realistic, multi-depth fixture suite, driven through
    /// BOTH consumers this EIMP was written to stop drifting apart —
    /// `einmo test`-shaped FAE/FF validation
    /// (`EinmoTestRunner`/`check_suite_integrity`) and `einmo review`-shaped
    /// listing (`EinmoReview::items()`) — asserting they agree on every
    /// case's presence and agreement facts.
    ///
    /// Note on "the SAME `EinmoSuite` instance" (as the plan phrases it):
    /// neither `EinmoTestRunner` (config-driven, delegates to
    /// `compare::compare`) nor `EinmoReview` (path-driven) accept a
    /// caller-supplied `EinmoSuite` through their public API — each
    /// independently re-scans the same on-disk directory instead. What this
    /// test actually proves is the property that matters: one on-disk
    /// corpus, three independent readers (`EinmoCase::agreement` directly,
    /// `EinmoTestRunner`'s `Problem`s, `EinmoReview`'s `ReviewItem`s), and
    /// all three agree. A literal shared object is an implementation detail
    /// this EIMP deliberately leaves to each caller's own scan.
    #[test]
    fn comprehensive_suite_wide_consistency_between_test_and_review_consumers() {
        use crate::storage::EinmoStorage as _;

        let ctx = test_context();

        // Two ordinary cases -- one root-level, one multi-depth
        // (`foop/23/sub_feature/test1`-shaped, per §S.5's directory_tree
        // fixture shape) -- run through the real FAE/FF evaluate flow and
        // promoted to checked. This is also the P1 repro's own shape:
        // output==checked, verified/ stays entirely unpopulated.
        write_input(ctx.path(), "a.foo", "{1+1;}");
        write_input(ctx.path(), "foop/23/sub_feature/test1.foo", "{3+3;}");
        // `input/` cover for the two hand-fixtured cases below, so their
        // stage artifacts are never flagged as orphaned/extraneous (O1/O5)
        // -- `check_integrity` only checks that an `input/` file with the
        // matching id exists, never that its bytes correspond to what's
        // signed under output/checked.
        write_input(ctx.path(), "c_comments.foo", "{5;}");
        write_input(ctx.path(), "d_tampered.foo", "{5;}");

        let config = TestConfig::new(ctx.path(), crate::einmo_suite::ValidationLevel::Checked);
        crate::einmo_suite::EinmoTestRunner::new(config.clone())
            .evaluate_all(&Echo)
            .unwrap();
        promote_output_to_checked(ctx.path());

        let directory = EinmoDirectory::new(config.clone());
        let c_comments_id =
            EinmoId::from_input_rel(std::path::Path::new("c_comments.foo")).unwrap();
        directory
            .write(
                &c_comments_id,
                ArtifactLocation::Stage(Stage::Output),
                &signed_bytes_with_comments("c_comments.foo", "5", "an output-side note"),
            )
            .unwrap();
        directory
            .write(
                &c_comments_id,
                ArtifactLocation::Stage(Stage::Checked),
                &signed_bytes_with_comments("c_comments.foo", "5", "a DIFFERENT checked-side note"),
            )
            .unwrap();

        let d_tampered_id =
            EinmoId::from_input_rel(std::path::Path::new("d_tampered.foo")).unwrap();
        let identical_bytes = signed_bytes_with_comments("d_tampered.foo", "5", "");
        directory
            .write(
                &d_tampered_id,
                ArtifactLocation::Stage(Stage::Output),
                &identical_bytes,
            )
            .unwrap();
        directory
            .write(
                &d_tampered_id,
                ArtifactLocation::Stage(Stage::Checked),
                &identical_bytes,
            )
            .unwrap();
        // Corrupt the checked copy on disk (flip a byte inside INPUT
        // `{5;}`) -- verify-on-inspect must refuse it, and both consumers
        // below must report it as TAMPERED, never as an ordinary
        // "differing" content mismatch.
        let tampered_path = config
            .stage_dir(Stage::Checked)
            .join("d_tampered.foo.einmo");
        let mut bytes = std::fs::read(&tampered_path).unwrap();
        let pos = bytes.windows(4).position(|w| w == b"{5;}").unwrap();
        bytes[pos + 1] = b'8';
        std::fs::write(&tampered_path, bytes).unwrap();

        // ---- ground truth, straight from EinmoCase::agreement ----
        let suite = EinmoSuite::scan(EinmoDirectory::new(config.clone()), None).unwrap();
        let agree_output_checked = |id: &EinmoId, policy: crate::config::MatchSections| {
            suite
                .case(id)
                .unwrap()
                .agreement(&[Stage::Output, Stage::Checked], policy)
                .unwrap()
                .pair(Stage::Output, Stage::Checked)
                .cloned()
        };
        let a_id = EinmoId::from_input_rel(std::path::Path::new("a.foo")).unwrap();
        let multi_id =
            EinmoId::from_input_rel(std::path::Path::new("foop/23/sub_feature/test1.foo")).unwrap();
        assert_eq!(
            agree_output_checked(&a_id, crate::config::MatchSections::InputOutput),
            Some(StagePairAgreement::Agree)
        );
        assert_eq!(
            agree_output_checked(&multi_id, crate::config::MatchSections::InputOutput),
            Some(StagePairAgreement::Agree)
        );
        assert_eq!(
            agree_output_checked(&c_comments_id, crate::config::MatchSections::InputOutput),
            Some(StagePairAgreement::Agree),
            "under the default policy a COMMENTS-only divergence must not count as differing"
        );
        assert_eq!(
            agree_output_checked(
                &c_comments_id,
                crate::config::MatchSections::InputOutputComments
            ),
            Some(StagePairAgreement::Differ {
                sections: vec!["COMMENTS".to_string()]
            }),
            "under the strict policy the SAME case now reads as differing -- proving \
             the policy, not a hardcoded assumption, controls the answer"
        );
        assert!(
            matches!(
                agree_output_checked(&d_tampered_id, crate::config::MatchSections::InputOutput),
                Some(StagePairAgreement::Tampered { .. })
            ),
            "a corrupted signature must read as Tampered, never Differ or Agree"
        );

        // ---- einmo test-shaped consumer: EinmoTestRunner/check_suite_integrity ----
        let integrity = crate::einmo_suite::check_suite_integrity(
            &config,
            crate::einmo_suite::FailurePolicy::FailAtEnd,
        )
        .unwrap();
        assert!(
            !integrity.problems.iter().any(|p| matches!(
                p,
                crate::einmo_suite::Problem::SectionDifference { path, .. }
                    if path.to_string_lossy().contains("c_comments")
            )),
            "the comments-only case must not surface as a SectionDifference under \
             the suite's default policy: {:?}",
            integrity.problems
        );
        assert!(
            integrity.problems.iter().any(|p| matches!(
                p,
                crate::einmo_suite::Problem::SignatureDoesNotVerify { path, .. }
                    if path.to_string_lossy().contains("d_tampered")
            )),
            "the tampered case must surface as SignatureDoesNotVerify: {:?}",
            integrity.problems
        );
        assert!(
            !integrity.problems.iter().any(|p| matches!(
                p,
                crate::einmo_suite::Problem::SectionDifference { path, .. }
                    if path.to_string_lossy().contains("d_tampered")
            )),
            "a tampered artifact must never ALSO be reported as an ordinary \
             SectionDifference: {:?}",
            integrity.problems
        );

        // ---- einmo review-shaped consumer: EinmoReview::items() ----
        let review = EinmoReview::open(ctx.path());
        let items = review.items().unwrap();
        let item_for = |rel: &str| {
            items
                .iter()
                .find(|i| i.id.as_str() == rel)
                .unwrap_or_else(|| panic!("{rel} missing from review items: {items:?}"))
        };

        assert!(!item_for("a.foo").differing);
        assert!(!item_for("foop/23/sub_feature/test1.foo").differing);
        assert!(
            !item_for("c_comments.foo").differing,
            "review must agree with the test-shaped consumer: a COMMENTS-only \
             divergence is not \"differing\" under the shared default policy"
        );
        let tampered_item = item_for("d_tampered.foo");
        assert!(
            tampered_item.differing,
            "a tampered checked stage cannot read as clean"
        );
        let checked_status = tampered_item
            .stages
            .iter()
            .find(|(stage, _)| *stage == Stage::Checked)
            .and_then(|(_, status)| status.clone());
        assert_eq!(
            checked_status.as_deref(),
            Some("TAMPERED"),
            "review must report the artifact as TAMPERED, distinguishable from an \
             ordinary content divergence -- not just folded into differing=true"
        );
    }
}
