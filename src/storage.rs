//! `EinmoStorage` (`EIMP-7` §S.1): where one case's input and stage artifacts
//! actually live, addressed by `(EinmoId, ArtifactLocation)` rather than by
//! path — so a non-filesystem implementation never has to fake directories.
//! [`EinmoDirectory`] (§S.2) is the filesystem implementation, and is the
//! only one this crate ships; it resolves every `(EinmoId,
//! ArtifactLocation)` to exactly the path today's free functions
//! (`EinmoId::to_stage_path`, `stage.rs`'s walk helpers) already resolve —
//! the `input/`+per-stage directory split a human reads is unchanged.
//!
//! `ArtifactLocation` carries `Stage`'s post-`EIMP-7` §S.2a shape: a
//! stage's own artifacts (`Stage(Stage)`) and its nested flagged sink
//! (`Flagged(Stage)`) are two DIFFERENT locations sharing the same origin
//! stage, never conflated — `Stage(s)`'s listing excludes `Flagged(s)`'s
//! contents even though the sink sits physically inside the stage
//! directory being walked.

use std::path::{Path, PathBuf};

use crate::config::TestConfig;
use crate::error::{EinmoError, Result};
use crate::stage::{EinmoId, Stage, ensure_parent_dir, is_in_flagged_sink, walk_input_tree};

/// One place an artifact can live: the `input/` tree, a stage's own
/// directory, or a stage's nested flagged sink (`EIMP-7` §S.2a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactLocation {
    /// The `input/` tree — the source a case is generated from.
    Input,
    /// A stage's own directory: `output/`, `checked/`, `verified/`.
    /// Excludes that stage's nested flagged sink even though it sits
    /// inside the same directory on disk — see [`ArtifactLocation::Flagged`].
    Stage(Stage),
    /// The flagged sink nested inside a stage: `output/flagged/`,
    /// `checked/flagged/`, `verified/flagged/`. Carrying the stage means
    /// a flag's origin is recoverable from its location alone.
    Flagged(Stage),
}

/// Where one case's stage artifacts and its input actually live. The
/// contract is byte-addressed by `(EinmoId, ArtifactLocation)` —
/// deliberately not path-shaped, so a non-filesystem implementation (a
/// database) never has to fake directories.
pub trait EinmoStorage {
    /// Read one artifact's raw bytes, or `None` if it does not exist.
    ///
    /// # Errors
    /// Any I/O failure other than "does not exist".
    fn read(&self, id: &EinmoId, at: ArtifactLocation) -> Result<Option<Vec<u8>>>;

    /// Write (create or overwrite) one artifact's raw bytes.
    ///
    /// # Errors
    /// Any I/O failure.
    fn write(&self, id: &EinmoId, at: ArtifactLocation, bytes: &[u8]) -> Result<()>;

    /// Remove one artifact. A no-op (not an error) if it does not exist.
    ///
    /// # Errors
    /// Any I/O failure other than "does not exist".
    fn remove(&self, id: &EinmoId, at: ArtifactLocation) -> Result<()>;

    /// Every case id with something at `at` — an input file, or a stage
    /// artifact. Building the full cross-stage union is the caller's job
    /// (`EinmoSuite`, a later phase), not this trait's.
    ///
    /// # Errors
    /// Any I/O failure, or an invalid id recovered from a stored path.
    fn list_ids(&self, at: ArtifactLocation) -> Result<Vec<EinmoId>>;
}

/// The filesystem [`EinmoStorage`]: one suite root directory, its `input/`
/// tree, and its stage directories, addressed exactly as
/// `EinmoId::to_stage_path`/`mirror_input_path` (`stage.rs`) already do.
/// Owns no cache and no in-memory state — every call touches disk.
#[derive(Debug, Clone)]
pub struct EinmoDirectory {
    config: TestConfig,
}

impl EinmoDirectory {
    /// Bind to `config`'s suite root.
    #[must_use]
    pub fn new(config: TestConfig) -> Self {
        EinmoDirectory { config }
    }

    /// The suite configuration this directory is bound to.
    #[must_use]
    pub fn config(&self) -> &TestConfig {
        &self.config
    }

    /// The real filesystem path `(id, at)` resolves to. Never fails: an
    /// `EinmoId` is already validated at construction, so building a path
    /// from it cannot fail — only reading/writing that path can.
    fn artifact_path(&self, id: &EinmoId, at: ArtifactLocation) -> PathBuf {
        match at {
            ArtifactLocation::Input => self.config.input_path().join(Path::new(id.as_str())),
            ArtifactLocation::Stage(stage) => id.to_stage_path(self.config.work_dir(), stage),
            ArtifactLocation::Flagged(stage) => self
                .config
                .flagged_dir(stage)
                .join(crate::stage::mirror_input_path(Path::new(id.as_str()))),
        }
    }
}

impl EinmoStorage for EinmoDirectory {
    fn read(&self, id: &EinmoId, at: ArtifactLocation) -> Result<Option<Vec<u8>>> {
        let path = self.artifact_path(id, at);
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(EinmoError::io(&path, e)),
        }
    }

    fn write(&self, id: &EinmoId, at: ArtifactLocation, bytes: &[u8]) -> Result<()> {
        let path = self.artifact_path(id, at);
        ensure_parent_dir(&path)?;
        std::fs::write(&path, bytes).map_err(|e| EinmoError::io(&path, e))
    }

    fn remove(&self, id: &EinmoId, at: ArtifactLocation) -> Result<()> {
        let path = self.artifact_path(id, at);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(EinmoError::io(&path, e)),
        }
    }

    fn list_ids(&self, at: ArtifactLocation) -> Result<Vec<EinmoId>> {
        match at {
            ArtifactLocation::Input => {
                let dir = self.config.input_path();
                walk_input_tree(&dir, self.config.walk_depth_limit())?
                    .into_iter()
                    .map(|rel| EinmoId::from_input_rel(&rel))
                    .collect()
            }
            ArtifactLocation::Stage(stage) => {
                let dir = self.config.stage_dir(stage);
                let flagged_name = self.config.flagged_dir_name();
                walk_input_tree(&dir, self.config.walk_depth_limit())?
                    .into_iter()
                    // The nested flagged sink lives inside this same
                    // directory (EIMP-7 §S.2a) but is a DIFFERENT
                    // location (`Flagged(stage)`) — exclude it here or
                    // every flagged artifact would double as a phantom
                    // ordinary one.
                    .filter(|rel| !is_in_flagged_sink(rel, flagged_name))
                    .map(|rel| EinmoId::from_stage_artifact_path(&dir, &dir.join(&rel)))
                    .collect()
            }
            ArtifactLocation::Flagged(stage) => {
                let dir = self.config.flagged_dir(stage);
                walk_input_tree(&dir, self.config.walk_depth_limit())?
                    .into_iter()
                    .map(|rel| EinmoId::from_stage_artifact_path(&dir, &dir.join(&rel)))
                    .collect()
            }
        }
    }
}

/// An in-memory [`EinmoStorage`] fake, for tests that need storage
/// behavior without touching a tempdir. `#[cfg(test)]` but `pub(crate)`
/// so later phases' own unit tests (`EinmoCase`, `EinmoSuite`) can use it
/// too. `ArtifactLocation` already derives `Hash`/`Eq`, so it is the map
/// key directly — no separate key type needed.
#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub(crate) struct InMemoryStorage {
    artifacts: std::cell::RefCell<std::collections::HashMap<(EinmoId, ArtifactLocation), Vec<u8>>>,
}

#[cfg(test)]
impl InMemoryStorage {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
impl EinmoStorage for InMemoryStorage {
    fn read(&self, id: &EinmoId, at: ArtifactLocation) -> Result<Option<Vec<u8>>> {
        Ok(self.artifacts.borrow().get(&(id.clone(), at)).cloned())
    }

    fn write(&self, id: &EinmoId, at: ArtifactLocation, bytes: &[u8]) -> Result<()> {
        self.artifacts
            .borrow_mut()
            .insert((id.clone(), at), bytes.to_vec());
        Ok(())
    }

    fn remove(&self, id: &EinmoId, at: ArtifactLocation) -> Result<()> {
        self.artifacts.borrow_mut().remove(&(id.clone(), at));
        Ok(())
    }

    fn list_ids(&self, at: ArtifactLocation) -> Result<Vec<EinmoId>> {
        let mut ids: Vec<EinmoId> = self
            .artifacts
            .borrow()
            .keys()
            .filter(|(_, k)| *k == at)
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TestConfig;
    use crate::einmo_suite::ValidationLevel;

    fn directory_fixture() -> (tempfile::TempDir, EinmoDirectory) {
        let tmp = tempfile::tempdir().unwrap();
        let config = TestConfig::new(tmp.path(), ValidationLevel::Output);
        config.ensure_stage_dirs().unwrap();
        std::fs::create_dir_all(config.input_path()).unwrap();
        (tmp, EinmoDirectory::new(config))
    }

    /// Behavior every `EinmoStorage` implementation must satisfy —
    /// exercised against both `EinmoDirectory` and `InMemoryStorage` below,
    /// so the two backends cannot silently drift on the contract's basics.
    fn assert_round_trip(storage: &impl EinmoStorage, at: ArtifactLocation) {
        let id = EinmoId::from_input_rel(Path::new("foop/23/sub_feature/test1.foo")).unwrap();

        // Absent is None, not an error.
        assert_eq!(storage.read(&id, at).unwrap(), None);

        // Write, then read back byte-identical.
        storage.write(&id, at, b"hello world").unwrap();
        assert_eq!(
            storage.read(&id, at).unwrap(),
            Some(b"hello world".to_vec())
        );

        // Overwrite.
        storage.write(&id, at, b"replaced").unwrap();
        assert_eq!(storage.read(&id, at).unwrap(), Some(b"replaced".to_vec()));

        // Remove, then absent again.
        storage.remove(&id, at).unwrap();
        assert_eq!(storage.read(&id, at).unwrap(), None);

        // Removing an already-absent artifact is a no-op, not an error.
        storage.remove(&id, at).unwrap();
    }

    #[test]
    fn einmo_directory_round_trips_input_and_every_stage() {
        let (_tmp, dir) = directory_fixture();
        assert_round_trip(&dir, ArtifactLocation::Input);
        for stage in Stage::ALL {
            assert_round_trip(&dir, ArtifactLocation::Stage(stage));
            assert_round_trip(&dir, ArtifactLocation::Flagged(stage));
        }
    }

    #[test]
    fn in_memory_storage_round_trips_input_and_every_stage() {
        let storage = InMemoryStorage::new();
        assert_round_trip(&storage, ArtifactLocation::Input);
        for stage in Stage::ALL {
            assert_round_trip(&storage, ArtifactLocation::Stage(stage));
            assert_round_trip(&storage, ArtifactLocation::Flagged(stage));
        }
    }

    /// `EIMP-7` §S.2a: a case flagged from `Checked` resolves under
    /// `checked/flagged/`, not `output/flagged/` — the property the whole
    /// per-stage-sink move exists for (origin recoverable from location
    /// alone).
    #[test]
    fn einmo_directory_flagged_resolves_under_its_own_origin_stage_only() {
        let (_tmp, dir) = directory_fixture();
        let id = EinmoId::from_input_rel(Path::new("a.foo")).unwrap();
        dir.write(
            &id,
            ArtifactLocation::Flagged(Stage::Checked),
            b"from checked",
        )
        .unwrap();

        assert_eq!(
            dir.read(&id, ArtifactLocation::Flagged(Stage::Checked))
                .unwrap(),
            Some(b"from checked".to_vec()),
        );
        // Not visible under any other stage's flagged sink.
        assert_eq!(
            dir.read(&id, ArtifactLocation::Flagged(Stage::Output))
                .unwrap(),
            None,
        );
        assert_eq!(
            dir.read(&id, ArtifactLocation::Flagged(Stage::Verified))
                .unwrap(),
            None,
        );
        // Confirm it actually landed on disk at checked/flagged/, not
        // merely that the trait's own read/write agree with each other.
        assert!(
            dir.config()
                .work_dir()
                .join("checked")
                .join("flagged")
                .join("a.foo.einmo")
                .exists()
        );
    }

    /// The nested-recursion hazard `EIMP-7` §S.2a introduces: a stage's
    /// flagged sink lives physically INSIDE the stage directory being
    /// walked for `Stage(stage)`, so listing that location must exclude
    /// it — otherwise every flagged artifact doubles as a phantom
    /// ordinary one.
    #[test]
    fn einmo_directory_list_ids_stage_excludes_its_own_nested_flagged_sink() {
        let (_tmp, dir) = directory_fixture();
        let ordinary = EinmoId::from_input_rel(Path::new("a.foo")).unwrap();
        let flagged = EinmoId::from_input_rel(Path::new("b.foo")).unwrap();

        dir.write(&ordinary, ArtifactLocation::Stage(Stage::Output), b"ok")
            .unwrap();
        dir.write(&flagged, ArtifactLocation::Flagged(Stage::Output), b"bad")
            .unwrap();

        let stage_ids = dir
            .list_ids(ArtifactLocation::Stage(Stage::Output))
            .unwrap();
        assert_eq!(stage_ids, vec![ordinary]);

        let flagged_ids = dir
            .list_ids(ArtifactLocation::Flagged(Stage::Output))
            .unwrap();
        assert_eq!(flagged_ids, vec![flagged]);
    }

    #[test]
    fn einmo_directory_write_creates_missing_parent_directories() {
        let (_tmp, dir) = directory_fixture();
        let id = EinmoId::from_input_rel(Path::new("a/b/c/deep.foo")).unwrap();
        dir.write(&id, ArtifactLocation::Stage(Stage::Output), b"x")
            .unwrap();
        assert_eq!(
            dir.read(&id, ArtifactLocation::Stage(Stage::Output))
                .unwrap(),
            Some(b"x".to_vec())
        );
    }

    #[test]
    fn einmo_directory_list_ids_unions_input_and_each_stage_independently() {
        let (_tmp, dir) = directory_fixture();
        let a = EinmoId::from_input_rel(Path::new("a.foo")).unwrap();
        let b = EinmoId::from_input_rel(Path::new("sub/b.foo")).unwrap();

        std::fs::write(dir.config().input_path().join("a.foo"), "input a").unwrap();
        dir.write(&a, ArtifactLocation::Stage(Stage::Output), b"out a")
            .unwrap();
        dir.write(&b, ArtifactLocation::Stage(Stage::Checked), b"checked b")
            .unwrap();

        let input_ids = dir.list_ids(ArtifactLocation::Input).unwrap();
        assert_eq!(input_ids, vec![a.clone()]);

        let output_ids = dir
            .list_ids(ArtifactLocation::Stage(Stage::Output))
            .unwrap();
        assert_eq!(output_ids, vec![a.clone()]);

        let checked_ids = dir
            .list_ids(ArtifactLocation::Stage(Stage::Checked))
            .unwrap();
        assert_eq!(checked_ids, vec![b.clone()]);

        // `b` has nothing at Output, and `a` has nothing at Checked —
        // list_ids does NOT union across locations; that is EinmoSuite's
        // job (a later phase), not this trait's.
        assert!(!output_ids.contains(&b));
        assert!(!checked_ids.contains(&a));
    }

    #[test]
    fn einmo_directory_list_ids_empty_when_stage_dir_has_nothing() {
        let (_tmp, dir) = directory_fixture();
        assert_eq!(
            dir.list_ids(ArtifactLocation::Stage(Stage::Verified))
                .unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn in_memory_storage_list_ids_unions_independently_per_location() {
        let storage = InMemoryStorage::new();
        let a = EinmoId::from_input_rel(Path::new("a.foo")).unwrap();
        storage
            .write(&a, ArtifactLocation::Stage(Stage::Output), b"x")
            .unwrap();
        assert_eq!(
            storage
                .list_ids(ArtifactLocation::Stage(Stage::Output))
                .unwrap(),
            vec![a.clone()]
        );
        assert_eq!(
            storage
                .list_ids(ArtifactLocation::Stage(Stage::Checked))
                .unwrap(),
            Vec::new()
        );
    }

    /// Parity: the same bytes at the same `(id, at)`, read back identically
    /// through both backends. Not a claim that the two share internal
    /// representation — just that the trait's contract means the same
    /// thing to both.
    #[test]
    fn einmo_directory_and_in_memory_storage_agree_on_the_contract() {
        let (_tmp, dir) = directory_fixture();
        let mem = InMemoryStorage::new();
        let id = EinmoId::from_input_rel(Path::new("parity/case.foo")).unwrap();

        for storage in [&dir as &dyn EinmoStorage, &mem as &dyn EinmoStorage] {
            assert_eq!(
                storage
                    .read(&id, ArtifactLocation::Stage(Stage::Output))
                    .unwrap(),
                None
            );
            storage
                .write(&id, ArtifactLocation::Stage(Stage::Output), b"payload")
                .unwrap();
            assert_eq!(
                storage
                    .read(&id, ArtifactLocation::Stage(Stage::Output))
                    .unwrap(),
                Some(b"payload".to_vec())
            );
        }
    }
}
