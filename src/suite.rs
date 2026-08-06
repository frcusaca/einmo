//! `EinmoSuite` (`EIMP-7` §S.5): an in-memory case collection, built by
//! scanning an [`EinmoStorage`] once. Replaces `scan_tests`/`TestRow`
//! (`einmo_suite.rs`) as the shared listing implementation `einmo test`
//! and `einmo review` both already call through today.

use std::collections::BTreeMap;
use std::path::Path;

use crate::case::{EinmoCase, PromoteOutcome};
use crate::config::KeySource;
use crate::corpus_signer::CorpusSigner;
use crate::error::{EinmoError, Result};
use crate::signature::StageKeypair;
use crate::stage::{EinmoId, Stage, mirror_input_path};
use crate::storage::{ArtifactLocation, EinmoStorage};
use crate::transitions::{FlagReport, Promoted, PromotionReport, RetractReport};

/// What [`EinmoSuite::update_corpus_signature`] did to `stage`'s
/// `.section.sig` (`EIMP-7` §S.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpusSignatureUpdate {
    /// No signature existed for this stage; one was written.
    Created {
        /// Number of ids in the manifest that was signed.
        manifest_len: usize,
    },
    /// A signature existed but no longer verified against the current
    /// corpus state, so it was re-signed.
    Updated {
        /// Number of ids in the manifest that was signed.
        manifest_len: usize,
    },
    /// A signature already existed and still verifies; nothing was
    /// written.
    AlreadyCurrent,
}

/// An in-memory snapshot of one suite: every [`EinmoId`] with something at
/// `input/` or any stage, built by one scan.
pub struct EinmoSuite<S: EinmoStorage> {
    storage: S,
    ids: Vec<EinmoId>,
}

impl<S: EinmoStorage> EinmoSuite<S> {
    /// Scan `storage`: union every id found at `Input` and every `Stage`
    /// (a stage's nested flagged sink is never asked — flagging is
    /// retirement, outside the suite's ordinary listing, `EIMP-1` §S.3),
    /// sorted and deduplicated by `EinmoId`'s own `Ord` — identical union
    /// `scan_tests` computes today, now storage-backed instead of walking
    /// the filesystem directly.
    ///
    /// `filter`, when given, keeps only ids containing it as a substring
    /// — matched against the id itself (no `.einmo` suffix), the same
    /// input-relative form `transitions.rs`'s own filter already matches
    /// against.
    ///
    /// # Errors
    /// Propagates any [`EinmoStorage::list_ids`] failure.
    pub fn scan(storage: S, filter: Option<&str>) -> Result<Self> {
        let mut ids = storage.list_ids(ArtifactLocation::Input)?;
        for stage in Stage::ALL {
            ids.extend(storage.list_ids(ArtifactLocation::Stage(stage))?);
        }
        ids.sort();
        ids.dedup();
        if let Some(pat) = filter {
            ids.retain(|id| id.as_str().contains(pat));
        }
        Ok(EinmoSuite { storage, ids })
    }

    /// The storage this suite was scanned from.
    #[must_use]
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Every id in this suite, in `EinmoId`'s `Ord` (sorted, deduplicated
    /// at scan time).
    #[must_use]
    pub fn ids(&self) -> &[EinmoId] {
        &self.ids
    }

    /// Every case, in the same order as [`Self::ids`].
    pub fn cases(&self) -> impl Iterator<Item = EinmoCase<'_, S>> {
        self.ids
            .iter()
            .map(|id| EinmoCase::new(id.clone(), &self.storage))
    }

    /// One case, if `id` is in this suite.
    #[must_use]
    pub fn case(&self, id: &EinmoId) -> Option<EinmoCase<'_, S>> {
        self.ids
            .binary_search(id)
            .ok()
            .map(|_| EinmoCase::new(id.clone(), &self.storage))
    }

    /// The cases [`Self::promote`]/[`Self::flag`]/[`Self::retract`]
    /// select: `ids`, when given, names the EXACT cases and `filter` is
    /// ignored — a name absent from this suite is silently skipped,
    /// matching `transitions.rs`'s existing `files`-overrides-`filter`
    /// precedent. When `ids` is `None`, every case matching `filter` (or
    /// every case, if `filter` is also `None`) is selected.
    fn select(&self, filter: Option<&str>, ids: Option<&[EinmoId]>) -> Vec<EinmoCase<'_, S>> {
        match ids {
            Some(ids) => ids.iter().filter_map(|id| self.case(id)).collect(),
            None => self
                .cases()
                .filter(|c| filter.is_none_or(|f| c.id().as_str().contains(f)))
                .collect(),
        }
    }

    /// Promote every selected case from `from` to `to`, deriving the
    /// destination `StageKeypair` ONCE for the whole batch and lending it
    /// to each [`EinmoCase::promote`] call (`EIMP-7` §S.10 — Argon2id is
    /// ~1.8s by design; per-case derivation made a 161-case promotion
    /// take ~5 minutes of pure CPU). This is the public entry point;
    /// `EinmoCase::promote` itself is `pub(crate)`.
    ///
    /// A case with nothing at `from` is silently skipped, not an error —
    /// matching `transitions::promote`'s existing behavior of only ever
    /// considering cases present at the source stage. Selection follows
    /// [`Self::select`].
    ///
    /// # Errors
    /// Returns [`EinmoError::IllegalTransition`] for a disallowed pair —
    /// checked here, UNCONDITIONALLY, before selection, so this errors
    /// even for an empty selection (an empty suite, or a filter matching
    /// nothing) rather than silently succeeding with an empty report.
    /// `EinmoCase::promote` repeats the same check per case; redundant on
    /// this call path (the suite-level check always fires first) but is
    /// what protects a caller that constructs an `EinmoCase` directly
    /// (`EinmoReview::execute`). Propagates any [`EinmoStorage`] I/O or
    /// verify-on-inspect failure from a case that WAS present at `from`.
    pub fn promote(
        &self,
        from: Stage,
        to: Stage,
        key: &KeySource,
        filter: Option<&str>,
        ids: Option<&[EinmoId]>,
    ) -> Result<PromotionReport> {
        if !crate::transitions::is_legal_transition(from, to) {
            return Err(EinmoError::IllegalTransition {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        let keypair = StageKeypair::derive(key.passphrase());
        let mut report = PromotionReport::default();

        let mut verified_files_concat = Vec::new();
        let mut overall_passphrase_score = None;

        if to == Stage::Verified {
            let is_human = !crate::signature::is_computer_key(&keypair.pubkey_hex());
            if is_human {
                for case in self.cases() {
                    if let Ok(Some(bytes)) = self
                        .storage
                        .read(case.id(), ArtifactLocation::Stage(Stage::Verified))
                    {
                        verified_files_concat.extend_from_slice(&bytes);
                    }
                }

                if let Ok(score) = einmo_tools::calculate_passphrase_score(
                    &verified_files_concat,
                    key.passphrase(),
                ) {
                    if score <= 0.0 {
                        return Err(EinmoError::Verification(format!(
                            "passphrase effectiveness score is {}, must be > 0",
                            score
                        )));
                    }
                    overall_passphrase_score = Some(score);
                }
            } else if key.passphrase().is_empty() {
                return Err(EinmoError::Verification(
                    "empty passphrase is not allowed for verified stage promotion".to_string(),
                ));
            }
        }

        for case in self.select(filter, ids) {
            if case.read(ArtifactLocation::Stage(from))?.is_none() {
                continue;
            }
            let outcome = case.promote(from, to, &keypair)?;

            let non_human = match outcome {
                PromoteOutcome::Promoted { non_human, .. } => non_human,
                PromoteOutcome::CoSigned { non_human, .. } => non_human,
                PromoteOutcome::AlreadySigned { non_human, .. } => non_human,
            };

            report.promoted.push(Promoted {
                rel_path: mirror_input_path(Path::new(case.id().as_str())),
                stamp_pubkey: keypair.pubkey_hex(),
                non_human,
                passphrase_score: overall_passphrase_score,
            });
        }
        Ok(report)
    }

    /// Flag every selected case's `stage` artifact — see
    /// [`EinmoCase::flag`]. A case with nothing at `stage` is silently
    /// skipped. Selection follows [`Self::select`].
    ///
    /// # Errors
    /// Propagates any [`EinmoStorage`] I/O or verify-on-inspect failure
    /// from a case that WAS present at `stage`.
    pub fn flag(
        &self,
        stage: Stage,
        reason: &str,
        filter: Option<&str>,
        ids: Option<&[EinmoId]>,
    ) -> Result<FlagReport> {
        let mut report = FlagReport::default();
        for case in self.select(filter, ids) {
            if case.read(ArtifactLocation::Stage(stage))?.is_none() {
                continue;
            }
            case.flag(stage, reason)?;
            report
                .flagged
                .push(mirror_input_path(Path::new(case.id().as_str())));
        }
        Ok(report)
    }

    /// Retract every selected case from `stage` (cascading `checked` →
    /// `verified`) — see [`EinmoCase::retract`]. Selection follows
    /// [`Self::select`].
    ///
    /// # Errors
    /// Returns [`EinmoError::Config`] if `stage` is `output` — checked
    /// here, UNCONDITIONALLY, before selection, for the same "must error
    /// even on an empty selection" reason as [`Self::promote`]'s
    /// transition check; `EinmoCase::retract` repeats the same check per
    /// case. Propagates any [`EinmoStorage`] I/O failure.
    pub fn retract(
        &self,
        stage: Stage,
        filter: Option<&str>,
        ids: Option<&[EinmoId]>,
    ) -> Result<RetractReport> {
        if stage == Stage::Output {
            return Err(EinmoError::Config(
                "cannot retract from output/: it is regenerated every run".into(),
            ));
        }
        let mut report = RetractReport::default();
        for case in self.select(filter, ids) {
            let retracted = case.retract(stage)?;
            let rel = mirror_input_path(Path::new(case.id().as_str()));
            for target in retracted {
                report.retracted.push((target, rel.clone()));
            }
        }
        Ok(report)
    }

    /// Bring `stage`'s section signature up to date (`EIMP-7` §S.8).
    /// Builds the manifest from ids this suite's own storage already
    /// knows about — no fourth independent directory walk — constructs
    /// the digest by reading each artifact's bytes through
    /// [`EinmoStorage`], and (re)signs `signer`'s `.section.sig` only
    /// where it is absent or stale.
    ///
    /// `signer` is passed in rather than constructed here:
    /// [`CorpusSigner`]'s own construction (`suite_root`, `Collation`) is
    /// a filesystem/`TestConfig` concern this generic-over-storage type
    /// has no business owning — the suite DRIVES an existing signer, it
    /// does not decide how one gets built.
    ///
    /// Currency is checked via [`CorpusSigner::verify_via_storage`]: if
    /// the CURRENT corpus state (through this suite's own storage) still
    /// satisfies whatever `.section.sig` already records, nothing is
    /// written — this is what makes routine re-signing affordable rather
    /// than a full re-read-and-resign every time.
    ///
    /// # Errors
    ///
    /// Returns [`EinmoError::CorpusSignature`] on an unrecognized
    /// collation or a manifest/digest inconsistency, or propagates any
    /// [`EinmoStorage`] I/O failure.
    pub fn update_corpus_signature(
        &self,
        signer: &CorpusSigner,
        stage: Stage,
        key: &KeySource,
    ) -> Result<CorpusSignatureUpdate> {
        let existed = signer.section_sig_exists(stage);
        if existed && signer.verify_via_storage(stage, &self.storage).is_ok() {
            return Ok(CorpusSignatureUpdate::AlreadyCurrent);
        }

        let ids = self.storage.list_ids(ArtifactLocation::Stage(stage))?;
        let manifest_len = ids.len();
        signer.sign_via_storage(stage, key, ids, &self.storage)?;
        Ok(if existed {
            CorpusSignatureUpdate::Updated { manifest_len }
        } else {
            CorpusSignatureUpdate::Created { manifest_len }
        })
    }

    /// Group this suite's cases by their [`EinmoId`]'s path components —
    /// `foop/23/sub_feature/test1` nests under `foop` → `foop/23` →
    /// `foop/23/sub_feature`. Pure and on-demand: no separate tree state
    /// is stored or kept in sync, this is a view computed from `ids` each
    /// call.
    ///
    /// Exists for `EIMP-5` (Merkle-tree corpus signing, still `Draft`):
    /// that EIMP's own drafting raised hashing at every directory level
    /// as an alternative to a flat sorted-leaf binary fold — this is what
    /// such a signer would walk. Not consumed by anything in THIS EIMP.
    #[must_use]
    pub fn directory_tree(&self) -> DirectoryNode<'_, S> {
        let mut root = DirectoryNode {
            component: "",
            cases: Vec::new(),
            children: BTreeMap::new(),
        };
        for id in &self.ids {
            let parts: Vec<&str> = id.as_str().split('/').collect();
            let last = parts.len() - 1;
            let mut node = &mut root;
            for (i, part) in parts.into_iter().enumerate() {
                if i == last {
                    node.cases.push(EinmoCase::new(id.clone(), &self.storage));
                } else {
                    node = node.children.entry(part).or_insert_with(|| DirectoryNode {
                        component: part,
                        cases: Vec::new(),
                        children: BTreeMap::new(),
                    });
                }
            }
        }
        root
    }
}

/// One node of [`EinmoSuite::directory_tree`]'s output: a path component,
/// the cases directly at this level (a case can sit at any depth — a bare
/// `input/test1.foo` is a case at the root), and child nodes for deeper
/// path components. A node is only ever created together with the case or
/// child it exists for — there is no node with both `cases` and
/// `children` empty.
pub struct DirectoryNode<'a, S: EinmoStorage> {
    /// This node's own path component. Empty for the root.
    pub component: &'a str,
    /// Cases whose id ends exactly at this node.
    pub cases: Vec<EinmoCase<'a, S>>,
    /// Deeper path components, keyed by their own component name.
    pub children: BTreeMap<&'a str, DirectoryNode<'a, S>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::collation::Collation;
    use crate::config::TestConfig;
    use crate::einmo_suite::ValidationLevel;
    use crate::storage::{EinmoDirectory, InMemoryStorage};

    fn id(rel: &str) -> EinmoId {
        EinmoId::from_input_rel(std::path::Path::new(rel)).unwrap()
    }

    fn directory_fixture() -> (tempfile::TempDir, EinmoDirectory) {
        let tmp = tempfile::tempdir().unwrap();
        let config = TestConfig::new(tmp.path(), ValidationLevel::Output);
        config.ensure_stage_dirs().unwrap();
        std::fs::create_dir_all(config.input_path()).unwrap();
        (tmp, EinmoDirectory::new(config))
    }

    #[test]
    fn scan_unions_input_and_stages_sorted_and_deduplicated() {
        let storage = InMemoryStorage::new();
        // input-only.
        storage
            .write(&id("b.foo"), ArtifactLocation::Input, b"input b")
            .unwrap();
        // stage-only.
        storage
            .write(
                &id("a.foo"),
                ArtifactLocation::Stage(Stage::Output),
                b"out a",
            )
            .unwrap();
        // both input and a stage -- must appear exactly once.
        storage
            .write(&id("c.foo"), ArtifactLocation::Input, b"input c")
            .unwrap();
        storage
            .write(
                &id("c.foo"),
                ArtifactLocation::Stage(Stage::Checked),
                b"checked c",
            )
            .unwrap();

        let suite = EinmoSuite::scan(storage, None).unwrap();
        assert_eq!(
            suite.ids(),
            &[id("a.foo"), id("b.foo"), id("c.foo")],
            "sorted by EinmoId's Ord, deduplicated"
        );
    }

    /// The nested flagged sink is never part of a suite's ordinary
    /// listing -- flagging is retirement (`EIMP-1` §S.3), outside
    /// `einmo test`/`einmo review`'s worklist.
    #[test]
    fn scan_excludes_flagged_sinks() {
        let storage = InMemoryStorage::new();
        storage
            .write(
                &id("only_flagged.foo"),
                ArtifactLocation::Flagged(Stage::Output),
                b"flagged",
            )
            .unwrap();

        let suite = EinmoSuite::scan(storage, None).unwrap();
        assert!(suite.ids().is_empty());
    }

    #[test]
    fn scan_filter_matches_against_the_id_not_a_dot_einmo_suffix() {
        let storage = InMemoryStorage::new();
        storage
            .write(
                &id("algorithms/sorting/quick.foo"),
                ArtifactLocation::Input,
                b"x",
            )
            .unwrap();
        storage
            .write(&id("other.foo"), ArtifactLocation::Input, b"y")
            .unwrap();

        // Matches the id's own (no ".einmo") form.
        let filtered = EinmoSuite::scan(storage, Some("sorting")).unwrap();
        assert_eq!(filtered.ids(), &[id("algorithms/sorting/quick.foo")]);
    }

    #[test]
    fn cases_iterates_in_id_order_and_case_looks_up_present_and_absent() {
        let storage = InMemoryStorage::new();
        storage
            .write(&id("b.foo"), ArtifactLocation::Input, b"b")
            .unwrap();
        storage
            .write(&id("a.foo"), ArtifactLocation::Input, b"a")
            .unwrap();

        let suite = EinmoSuite::scan(storage, None).unwrap();
        let ordered: Vec<EinmoId> = suite.cases().map(|c| c.id().clone()).collect();
        assert_eq!(ordered, vec![id("a.foo"), id("b.foo")]);

        assert!(suite.case(&id("a.foo")).is_some());
        assert!(suite.case(&id("nonexistent.foo")).is_none());
    }

    #[test]
    fn directory_tree_groups_by_path_components_at_every_depth() {
        let storage = InMemoryStorage::new();
        for rel in [
            "root_case.foo",
            "foop/23/other.foo",
            "foop/23/sub_feature/test1.foo",
        ] {
            storage
                .write(&id(rel), ArtifactLocation::Input, b"x")
                .unwrap();
        }
        let suite = EinmoSuite::scan(storage, None).unwrap();
        let tree = suite.directory_tree();

        // A case with no directory components sits at the root.
        assert_eq!(
            tree.cases
                .iter()
                .map(|c| c.id().clone())
                .collect::<Vec<_>>(),
            vec![id("root_case.foo")]
        );
        assert_eq!(tree.children.keys().copied().collect::<Vec<_>>(), ["foop"]);

        let foop = &tree.children["foop"];
        assert!(
            foop.cases.is_empty(),
            "foop/ itself has no case directly in it"
        );
        assert_eq!(foop.children.keys().copied().collect::<Vec<_>>(), ["23"]);

        let twenty_three = &foop.children["23"];
        assert_eq!(
            twenty_three
                .cases
                .iter()
                .map(|c| c.id().clone())
                .collect::<Vec<_>>(),
            vec![id("foop/23/other.foo")]
        );
        assert_eq!(
            twenty_three.children.keys().copied().collect::<Vec<_>>(),
            ["sub_feature"]
        );

        let sub_feature = &twenty_three.children["sub_feature"];
        assert_eq!(
            sub_feature
                .cases
                .iter()
                .map(|c| c.id().clone())
                .collect::<Vec<_>>(),
            vec![id("foop/23/sub_feature/test1.foo")]
        );
        assert!(sub_feature.children.is_empty());
    }

    /// No node has both `cases` and `children` empty -- a node is only
    /// ever created together with the case or child it exists for.
    #[test]
    fn directory_tree_never_has_an_empty_node() {
        fn assert_no_empty_node<S: EinmoStorage>(node: &DirectoryNode<'_, S>, is_root: bool) {
            if !is_root {
                assert!(
                    !node.cases.is_empty() || !node.children.is_empty(),
                    "node {:?} has neither cases nor children",
                    node.component
                );
            }
            for child in node.children.values() {
                assert_no_empty_node(child, false);
            }
        }

        let storage = InMemoryStorage::new();
        for rel in ["a/b/c/d.foo", "a/b/e.foo", "x.foo"] {
            storage
                .write(&id(rel), ArtifactLocation::Input, b"x")
                .unwrap();
        }
        let suite = EinmoSuite::scan(storage, None).unwrap();
        assert_no_empty_node(&suite.directory_tree(), true);
    }

    /// Parity: the same suite scanned through `EinmoDirectory` (real
    /// filesystem) and through `InMemoryStorage` yields the same id set.
    #[test]
    fn scan_parity_between_einmo_directory_and_in_memory_storage() {
        let (_tmp, dir) = directory_fixture();
        let mem = InMemoryStorage::new();
        for rel in ["a.foo", "sub/b.foo", "sub/deeper/c.foo"] {
            dir.write(&id(rel), ArtifactLocation::Stage(Stage::Output), b"x")
                .unwrap();
            mem.write(&id(rel), ArtifactLocation::Stage(Stage::Output), b"x")
                .unwrap();
        }

        let via_directory = EinmoSuite::scan(dir, None).unwrap();
        let via_memory = EinmoSuite::scan(mem, None).unwrap();
        assert_eq!(via_directory.ids(), via_memory.ids());
    }

    // ---- promote() / flag() / retract() ----
    //
    // Batch/selection-specific behavior for the one promote/flag/retract
    // implementation (`EIMP-7` §S.4, §S.10) -- single-case mechanics
    // (stamp chains, co-signing, tampered-source refusal, illegal-
    // transition refusal, re-flag concatenation) are `EinmoCase`'s own
    // tests in `case.rs`; these prove the SUITE-level selection and batch
    // discipline built on top of it, ported from `transitions.rs`'s
    // former `promote`/`flag`/`retract` free-function tests.

    fn signed_bytes(rel: &str, output: &str) -> Vec<u8> {
        use crate::format::{DEFAULT_SEPARATOR, EinmoFile, Metadata, Section, Status};
        use crate::signature::{Stamps, derive_keypair};
        let bodies = vec![
            Section::new("INPUT", "{5;}"),
            Section::new("OUTPUT", output),
            Section::new("COMMENTS", ""),
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

    fn three_case_suite() -> EinmoSuite<InMemoryStorage> {
        let storage = InMemoryStorage::new();
        for (rel, output) in [("a.foo", "5"), ("b.foo", "6"), ("c.foo", "7")] {
            storage
                .write(
                    &id(rel),
                    ArtifactLocation::Stage(Stage::Output),
                    &signed_bytes(rel, output),
                )
                .unwrap();
        }
        EinmoSuite::scan(storage, None).unwrap()
    }

    #[test]
    fn promote_selects_every_case_matching_filter() {
        let suite = three_case_suite();
        let key = KeySource::from_passphrase("");
        let report = suite
            .promote(Stage::Output, Stage::Checked, &key, Some("a.foo"), None)
            .unwrap();
        assert_eq!(report.promoted.len(), 1);
        assert_eq!(report.promoted[0].rel_path, PathBuf::from("a.foo.einmo"));
    }

    #[test]
    fn promote_ids_overrides_filter() {
        let suite = three_case_suite();
        let key = KeySource::from_passphrase("");
        // A filter that would match nothing is ignored once `ids` is given
        // -- matching `transitions.rs`'s old files-overrides-filter
        // precedent (`EIMP-7` §S.10's `select`).
        let report = suite
            .promote(
                Stage::Output,
                Stage::Checked,
                &key,
                Some("matches-nothing"),
                Some(&[id("a.foo"), id("c.foo")]),
            )
            .unwrap();
        let promoted: Vec<PathBuf> = report.promoted.iter().map(|p| p.rel_path.clone()).collect();
        assert_eq!(promoted.len(), 2);
        assert!(promoted.contains(&PathBuf::from("a.foo.einmo")));
        assert!(promoted.contains(&PathBuf::from("c.foo.einmo")));
    }

    #[test]
    fn promote_silently_skips_a_case_absent_at_the_source_stage() {
        let suite = three_case_suite();
        let key = KeySource::from_passphrase("a non-empty passphrase");
        // Checked is empty for every case -- promoting FROM Checked must
        // select nothing, not error, matching transitions::promote's old
        // "only ever considers cases present at the source" behavior.
        let report = suite
            .promote(Stage::Checked, Stage::Verified, &key, None, None)
            .unwrap();
        assert!(report.promoted.is_empty());
    }

    #[test]
    fn promote_refuses_an_illegal_transition_even_for_an_empty_suite() {
        let storage = InMemoryStorage::new();
        let suite = EinmoSuite::scan(storage, None).unwrap();
        let key = KeySource::from_passphrase("");
        let err = suite
            .promote(Stage::Verified, Stage::Output, &key, None, None)
            .unwrap_err();
        assert!(matches!(err, EinmoError::IllegalTransition { .. }));
    }

    /// The change this EIMP exists to make (`EIMP-7` §S.4): promoting the
    #[test]
    fn promote_to_verified_fails_when_human_passphrase_is_empty() {
        let storage = InMemoryStorage::new();
        storage
            .write(
                &id("a.foo"),
                ArtifactLocation::Stage(Stage::Checked),
                &signed_bytes("a.foo", "something"),
            )
            .unwrap();

        let suite = EinmoSuite::scan(storage, None).unwrap();
        let key = KeySource::from_passphrase("");

        let result = suite.promote(Stage::Checked, Stage::Verified, &key, None, None);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "signature verification failed: empty passphrase is not allowed for verified stage promotion"
        );
    }

    #[test]
    fn promote_the_same_content_with_two_different_keys_co_signs_both() {
        let suite = three_case_suite();
        suite
            .promote(
                Stage::Output,
                Stage::Checked,
                &KeySource::from_passphrase("signer-one"),
                Some("a.foo"),
                None,
            )
            .unwrap();
        suite
            .promote(
                Stage::Output,
                Stage::Checked,
                &KeySource::from_passphrase("signer-two"),
                Some("a.foo"),
                None,
            )
            .unwrap();

        let checked = suite
            .case(&id("a.foo"))
            .unwrap()
            .read(ArtifactLocation::Stage(Stage::Checked))
            .unwrap()
            .unwrap();
        let checked_stamps = checked
            .stamps()
            .entries()
            .iter()
            .filter(|s| s.key() == "stage:checked")
            .count();
        assert_eq!(
            checked_stamps, 2,
            "both signers' stage:checked stamps must survive"
        );
    }

    /// Argon2id derivation is ~1.8s by design (`EIMP-7` §S.10); a 5-case
    /// batch deriving per-case rather than once would take ~9s. This is
    /// the kind of performance invariant a refactor silently loses if
    /// nothing pins it — mirrors `review.rs`'s own
    /// `execute_derives_stage_key_once_per_batch_not_per_case`.
    #[test]
    fn promote_derives_the_stage_key_once_per_batch_not_per_case() {
        let storage = InMemoryStorage::new();
        for i in 0..5 {
            let rel = format!("case{i}.foo");
            storage
                .write(
                    &id(&rel),
                    ArtifactLocation::Stage(Stage::Output),
                    &signed_bytes(&rel, "5"),
                )
                .unwrap();
        }
        let suite = EinmoSuite::scan(storage, None).unwrap();

        let start = std::time::Instant::now();
        let report = suite
            .promote(
                Stage::Output,
                Stage::Checked,
                &KeySource::from_passphrase("a-non-empty-batch-passphrase"),
                None,
                None,
            )
            .unwrap();
        let elapsed = start.elapsed();

        assert_eq!(report.promoted.len(), 5);
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "5-case batch took {elapsed:?} -- looks like the stage key was derived per \
             case (~1.8s each) instead of once for the whole batch"
        );
    }

    #[test]
    fn flag_selects_every_case_matching_filter_and_skips_absent_ones() {
        let suite = three_case_suite();
        let report = suite
            .flag(Stage::Output, "one bad", Some("b.foo"), None)
            .unwrap();
        assert_eq!(report.flagged, vec![PathBuf::from("b.foo.einmo")]);

        let b = suite.case(&id("b.foo")).unwrap();
        assert!(
            b.read(ArtifactLocation::Stage(Stage::Output))
                .unwrap()
                .is_none(),
            "origin vacated"
        );
        assert!(
            b.read(ArtifactLocation::Flagged(Stage::Output))
                .unwrap()
                .is_some()
        );
        // a.foo and c.foo untouched.
        for rel in ["a.foo", "c.foo"] {
            assert!(
                suite
                    .case(&id(rel))
                    .unwrap()
                    .read(ArtifactLocation::Stage(Stage::Output))
                    .unwrap()
                    .is_some()
            );
        }
    }

    #[test]
    fn retract_refuses_output_even_for_an_empty_selection() {
        let storage = InMemoryStorage::new();
        let suite = EinmoSuite::scan(storage, None).unwrap();
        // No cases at all, let alone matching any filter -- must still
        // error, not silently succeed with an empty report. This is what
        // the suite-level Output check (checked unconditionally, before
        // selection) exists for.
        let err = suite.retract(Stage::Output, None, None).unwrap_err();
        assert!(matches!(err, EinmoError::Config(_)));
    }

    #[test]
    fn retract_selects_every_case_matching_filter() {
        let storage = InMemoryStorage::new();
        for rel in ["a.foo", "b.foo"] {
            let bytes = signed_bytes(rel, "5");
            storage
                .write(&id(rel), ArtifactLocation::Stage(Stage::Checked), &bytes)
                .unwrap();
        }
        let suite = EinmoSuite::scan(storage, None).unwrap();

        let report = suite.retract(Stage::Checked, Some("a.foo"), None).unwrap();
        assert_eq!(
            report.retracted,
            vec![(Stage::Checked, PathBuf::from("a.foo.einmo"))]
        );
        assert!(
            suite
                .case(&id("b.foo"))
                .unwrap()
                .read(ArtifactLocation::Stage(Stage::Checked))
                .unwrap()
                .is_some(),
            "b.foo did not match the filter and must survive"
        );
    }

    // ---- update_corpus_signature() ----
    //
    // `EinmoSuite` drives `CorpusSigner` through the `EinmoStorage`-backed
    // methods (`EIMP-7` §S.8) instead of `CorpusSigner` walking a
    // filesystem stage directory independently. These tests fixture a real
    // `EinmoDirectory` (not `InMemoryStorage`) because `CorpusSigner`
    // itself is still filesystem-rooted for `.section.sig` -- the digest
    // it signs over is what now comes from the suite's storage.

    fn corpus_signer_for(dir: &EinmoDirectory) -> CorpusSigner {
        CorpusSigner::new(dir.config().work_dir(), Collation::DEFAULT)
    }

    #[test]
    fn update_corpus_signature_creates_when_none_exists() {
        let (_tmp, dir) = directory_fixture();
        dir.write(&id("a.foo"), ArtifactLocation::Stage(Stage::Output), b"x")
            .unwrap();
        dir.write(&id("b.foo"), ArtifactLocation::Stage(Stage::Output), b"y")
            .unwrap();
        let signer = corpus_signer_for(&dir);
        let suite = EinmoSuite::scan(dir, None).unwrap();
        let key = KeySource::from_passphrase("a-corpus-signing-passphrase");

        assert!(!signer.section_sig_exists(Stage::Output));
        let outcome = suite
            .update_corpus_signature(&signer, Stage::Output, &key)
            .unwrap();
        assert_eq!(outcome, CorpusSignatureUpdate::Created { manifest_len: 2 });
        assert!(signer.section_sig_exists(Stage::Output));
        assert!(
            signer.verify(Stage::Output).is_ok(),
            "a signature written via the storage-backed path must also \
             verify through CorpusSigner's own plain filesystem verify() \
             -- proof the two paths agree on the same bytes"
        );
    }

    #[test]
    fn update_corpus_signature_is_a_no_op_when_already_current() {
        let (_tmp, dir) = directory_fixture();
        dir.write(&id("a.foo"), ArtifactLocation::Stage(Stage::Output), b"x")
            .unwrap();
        let signer = corpus_signer_for(&dir);
        let key = KeySource::from_passphrase("a-corpus-signing-passphrase");
        let suite = EinmoSuite::scan(dir, None).unwrap();
        suite
            .update_corpus_signature(&signer, Stage::Output, &key)
            .unwrap();

        let sig_path = suite
            .storage()
            .config()
            .work_dir()
            .join(Stage::Output.dir_name())
            .join(".section.sig");
        let before = std::fs::read(&sig_path).unwrap();

        let outcome = suite
            .update_corpus_signature(&signer, Stage::Output, &key)
            .unwrap();
        assert_eq!(outcome, CorpusSignatureUpdate::AlreadyCurrent);

        let after = std::fs::read(&sig_path).unwrap();
        assert_eq!(
            before, after,
            "AlreadyCurrent must not rewrite .section.sig -- SLH-DSA \
             signing is randomized, so any re-sign would change these \
             bytes even over the same digest"
        );
    }

    #[test]
    fn update_corpus_signature_re_signs_when_stale() {
        let (_tmp, dir) = directory_fixture();
        dir.write(&id("a.foo"), ArtifactLocation::Stage(Stage::Output), b"x")
            .unwrap();
        let signer = corpus_signer_for(&dir);
        let key = KeySource::from_passphrase("a-corpus-signing-passphrase");
        let suite = EinmoSuite::scan(dir, None).unwrap();
        suite
            .update_corpus_signature(&signer, Stage::Output, &key)
            .unwrap();

        // Mutate the corpus after signing -- the recorded signature no
        // longer verifies, so the next update must re-sign, not skip.
        suite
            .storage()
            .write(&id("b.foo"), ArtifactLocation::Stage(Stage::Output), b"y")
            .unwrap();
        assert!(
            signer.verify(Stage::Output).is_err(),
            "sanity: the mutation above must actually invalidate the \
             existing signature"
        );

        let outcome = suite
            .update_corpus_signature(&signer, Stage::Output, &key)
            .unwrap();
        assert_eq!(outcome, CorpusSignatureUpdate::Updated { manifest_len: 2 });
        assert!(signer.verify(Stage::Output).is_ok());
    }

    #[test]
    fn digest_via_storage_matches_corpus_signers_own_direct_digest() {
        // Baseline parity check (`EIMP-7` §S.8): the digest the suite
        // drives `CorpusSigner` to sign over through `EinmoStorage` must
        // be byte-identical to what `CorpusSigner` computes walking the
        // filesystem directly -- the refactor must not have changed what
        // gets hashed, only how the bytes are read.
        let (_tmp, dir) = directory_fixture();
        for rel in ["a.foo", "b.foo", "sub/c.foo"] {
            dir.write(
                &id(rel),
                ArtifactLocation::Stage(Stage::Output),
                b"same-bytes",
            )
            .unwrap();
        }
        let signer = corpus_signer_for(&dir);
        let direct_digest = signer.digest(Stage::Output).unwrap();

        let suite = EinmoSuite::scan(dir, None).unwrap();
        let ids = suite
            .storage()
            .list_ids(ArtifactLocation::Stage(Stage::Output))
            .unwrap();
        let manifest = signer
            .manifest_from(Stage::Output, Collation::DEFAULT, ids)
            .unwrap();
        let via_storage_digest = signer
            .digest_for_via_storage(&manifest, suite.storage())
            .unwrap();

        assert_eq!(direct_digest.as_bytes(), via_storage_digest.as_bytes());
    }
}
