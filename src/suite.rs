//! `EinmoSuite` (`EIMP-7` §S.5): an in-memory case collection, built by
//! scanning an [`EinmoStorage`] once. Replaces `scan_tests`/`TestRow`
//! (`einmo_suite.rs`) as the shared listing implementation `einmo test`
//! and `einmo review` both already call through today.

use std::collections::BTreeMap;

use crate::case::EinmoCase;
use crate::error::Result;
use crate::stage::{EinmoId, Stage};
use crate::storage::{ArtifactLocation, EinmoStorage};

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
}
