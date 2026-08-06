//! Stage transitions: shared pieces `EinmoCase`/`EinmoSuite` (`EIMP-7` §S.3,
//! §S.10) and `promote_flag_to_note` build on — [`is_legal_transition`], the
//! report types, and file-selection/normalization helpers
//! ([`matching_mirror_paths_in`], [`normalize_file_path`]).
//!
//! The promote/flag/retract free functions this module used to define
//! directly are retired as of `EIMP-7` Phase F: `EinmoSuite::promote`/
//! `flag`/`retract` (`src/suite.rs`) are the one implementation now, shared
//! by the plain CLI path and `EinmoReview`'s own execute/flag_now/
//! retract_now, instead of a filesystem-direct copy here and a separately
//! -accumulating one in `review.rs`.

use std::path::{Path, PathBuf};

use crate::config::{KeySource, TestConfig};
use crate::einmo_suite::{git_commit_sha, git_diff_sha};
use crate::error::{EinmoError, Result};
use crate::format::{EinmoFile, Metadata, Section, Status};
use crate::signature::{Stamps, derive_keypair, now_iso8601};
use crate::stage::{Stage, ensure_parent_dir, mirror_input_path, walk_input_tree};

/// One promoted file's outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct Promoted {
    /// The mirror-relative path.
    pub rel_path: PathBuf,
    /// The hex pubkey of the appended stamp.
    pub stamp_pubkey: String,
    /// `true` if the appended verified stamp used a well-known computer key
    /// (a non-human attestation — post-hoc detectable, §B.4).
    pub non_human: bool,
    /// The calculated passphrase effectiveness score.
    pub passphrase_score: Option<f64>,
}

/// The result of a promotion over a filter set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PromotionReport {
    /// The files promoted.
    pub promoted: Vec<Promoted>,
}

/// The result of flagging.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlagReport {
    /// The mirror-relative paths moved into `flagged/`.
    pub flagged: Vec<PathBuf>,
}

/// The result of a retraction (demotion).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetractReport {
    /// `(stage, path)` of each artifact removed — including the cascade.
    pub retracted: Vec<(Stage, PathBuf)>,
}

/// A signature-prefix scan result (`confirm-signatures`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SignatureReport {
    /// Files carrying a stamp whose pubkey starts with the prefix.
    pub matched: Vec<PathBuf>,
    /// Files that verified but carry no matching stamp.
    pub unmatched: Vec<PathBuf>,
}

impl SignatureReport {
    /// `true` if every scanned file carries a matching signer.
    #[must_use]
    pub fn all_matched(&self) -> bool {
        self.unmatched.is_empty()
    }
}

/// The legal stage transitions (FOOP-92 §3). Flagging is no longer among
/// these (`EIMP-7` §S.2a): it is not a `Stage`-to-`Stage` transition at
/// all, but a move within a stage into that stage's own nested flagged
/// sink — see [`flag`].
///
/// `pub(crate)`: [`crate::case::EinmoCase::promote`] (`EIMP-7` §S.3)
/// enforces this too — a general-purpose promote primitive must refuse an
/// illegal pair itself, not rely on every caller having already screened
/// it (this crate's own `cli.rs` promote command lets a user name any
/// `--from`/`--to` pair directly).
pub(crate) fn is_legal_transition(from: Stage, to: Stage) -> bool {
    matches!(
        (from, to),
        (Stage::Output, Stage::Checked)
            | (Stage::Output, Stage::Verified)
            | (Stage::Checked, Stage::Verified)
            // console-review demotion (re-promotion appends another stamp)
            | (Stage::Verified, Stage::Checked)
    )
}

// promote/flag (the free functions) are retired as of EIMP-7 Phase F,
// replaced by EinmoSuite::promote/flag (src/suite.rs) -- one implementation
// shared by the plain CLI path and EinmoReview's execute()/flag_now(),
// instead of transitions.rs's own filesystem-direct copy and review.rs's
// separately-accumulating one. is_legal_transition, the report types
// (Promoted/PromotionReport/FlagReport/RetractReport), and the file-
// selection/normalization helpers below (matching_mirror_paths*,
// normalize_file_path) stay -- promote_flag_to_note and confirm_signatures
// still need them, and EinmoSuite's own promote/flag/retract still need
// is_legal_transition and the report shapes.

/// The result of promoting flagged content into `notes/`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NoteReport {
    /// The mirror-relative paths written into `notes/`.
    pub noted: Vec<PathBuf>,
}

/// Promote a flagged artifact's concatenated advisory into `notes/` as a
/// signed note (`EIMP-1` §S.3): "the same concatenated annotated content
/// that a flag holds as plaintext can be promoted into `notes/` as the
/// signed body of a note" — a throwaway flag graduating into a durable,
/// attributed record.
///
/// `stage` is the flagged sink to scan (`EIMP-1` §S.2a: `<stage>/flagged/`
/// — flagging is no longer a `Stage` of its own, so the caller names which
/// stage's sink they mean).
///
/// Unlike a flagged sink, a note **is signed** and participates in
/// signature checks (verify-on-inspect via the ordinary
/// `EinmoFile::from_file` path — nothing about verification is
/// note-specific). Unlike promoting a stage, this does **not** consume the
/// flag: the flagged artifact is left in place, so resolving/retracting
/// the flag itself stays a separate, deliberate action.
///
/// **`notes/` is deliberately not a [`Stage`]** — it does not join
/// `is_legal_transition`'s pairs, `compare`'s stage-to-stage matching, the
/// CLI's `--stage` selection, or suite-integrity walks. Extending the
/// `Stage` enum would ripple through every exhaustive match over it across
/// the crate for a stage that isn't part of the promotion pipeline at all
/// (no `retract`, no `compare`, nothing to walk as part of "does this suite
/// have the right shape") — `notes/` sits outside that machinery the same
/// way a flagged sink does. Broader integration (`einmo verify` scanning
/// `notes/`, a `--stage notes` CLI selector) is left for when a concrete
/// need appears — this function's job is the note format and the one
/// promotion operation `EIMP-1` §S.3 actually asks for.
///
/// # Errors
///
/// Returns [`EinmoError::Verification`] if a flagged source fails
/// verify-on-inspect, [`EinmoError::Config`] if a matched flagged file
/// carries no advisory (defensive — `flag` always sets one), or
/// [`EinmoError::Io`] on a filesystem failure.
pub fn promote_flag_to_note(
    config: &TestConfig,
    stage: Stage,
    key: &KeySource,
    filter: Option<&str>,
    files: Option<&[PathBuf]>,
) -> Result<NoteReport> {
    let flagged_dir = config.flagged_dir(stage);
    let notes_dir = config.stage_dir_for_notes();
    let mut report = NoteReport::default();

    // Derive both keys ONCE for the whole batch, matching `promote`'s own
    // discipline (Argon2id is ~1.8s by design). The configured key follows
    // the same plaintext-for-the-call's-duration precedent
    // `write_output`/`write_crash_crumb` already use for it.
    let (configured, _) = derive_keypair(config.configured_passphrase());
    let (notes_signer, _) = derive_keypair(key.passphrase());

    for rel in matching_mirror_paths_in(config, &flagged_dir, filter, files)? {
        let src = flagged_dir.join(&rel);
        let flagged = EinmoFile::from_file(&src)?; // verify-on-inspect
        let Some(advisory) = flagged.advisory() else {
            return Err(EinmoError::Config(format!(
                "{}: nothing flagged to promote into a note",
                rel.display()
            )));
        };

        let metadata = Metadata {
            test: rel.to_string_lossy().into_owned(),
            suite: config.suite_name().to_string(),
            producer: git_commit_sha(),
            producer_diff: git_diff_sha(),
            generated: now_iso8601(),
            status: Status::Normal,
            status_detail: String::new(),
            reference: String::new(),
            sections: vec!["NOTE".to_string(), "STAMPS".to_string()],
        };
        let sections = vec![Section::new("NOTE", advisory.to_string())];
        let mut file = EinmoFile::new(
            config.encoding(),
            config.separator(),
            metadata,
            sections,
            Stamps::new(),
        );
        let stamps = Stamps::generate_for_stage(
            &file.signed_prefix(),
            &configured,
            "stage:notes",
            &notes_signer,
        );
        file.set_stamps(stamps);

        let dst = notes_dir.join(&rel);
        ensure_parent_dir(&dst)?;
        let bytes = file.serialize()?;
        std::fs::write(&dst, &bytes).map_err(|e| EinmoError::io(&dst, e))?;
        report.noted.push(rel);
    }
    Ok(report)
}

// retract (the free function) is retired as of EIMP-7 Phase F too, for the
// same reason as promote/flag above -- see EinmoSuite::retract.

/// Scan every `.einmo` under `path`, reporting which files carry a stamp whose
/// pubkey starts with `pubkey_prefix`.
///
/// # Errors
///
/// Returns [`EinmoError::Io`] if the directory cannot be walked, or
/// [`EinmoError::Verification`] if a file fails verify-on-inspect.
pub fn confirm_signatures(path: &Path, pubkey_prefix: &str) -> Result<SignatureReport> {
    let mut report = SignatureReport::default();
    let mut files = Vec::new();
    collect_einmo_files(path, &mut files, MAX_EINMO_WALK_DEPTH)?;
    files.sort();
    for file_path in files {
        let file = EinmoFile::from_file(&file_path)?;
        let rel = file_path
            .strip_prefix(path)
            .unwrap_or(&file_path)
            .to_path_buf();
        if file.stamps().stamped_by(pubkey_prefix) {
            report.matched.push(rel);
        } else {
            report.unmatched.push(rel);
        }
    }
    Ok(report)
}

/// Recursively collect all `.einmo` files under `dir`.
fn collect_einmo_files(dir: &Path, out: &mut Vec<PathBuf>, depth_limit: usize) -> Result<()> {
    collect_einmo_files_depth(dir, out, 0, depth_limit)
}

const MAX_EINMO_WALK_DEPTH: usize = 64;

fn collect_einmo_files_depth(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    depth: usize,
    depth_limit: usize,
) -> Result<()> {
    if depth > depth_limit {
        return Err(EinmoError::Io {
            path: dir.to_path_buf(),
            source: std::io::Error::other(format!(
                "directory walk exceeded max depth {depth_limit} (possible symlink cycle)"
            )),
        });
    }
    if !dir.exists() {
        return Ok(());
    }
    if dir.is_file() {
        if dir.extension().map(|e| e == "einmo").unwrap_or(false) {
            out.push(dir.to_path_buf());
        }
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|e| EinmoError::io(dir, e))? {
        let entry = entry.map_err(|e| EinmoError::io(dir, e))?;
        let p = entry.path();
        let file_type = entry.file_type().map_err(|e| EinmoError::io(&p, e))?;
        if file_type.is_symlink() {
            let metadata = match std::fs::metadata(&p) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(EinmoError::io(&p, e)),
            };
            if metadata.is_dir() {
                collect_einmo_files_depth(&p, out, depth + 1, depth_limit)?;
            } else if metadata.is_file() && p.extension().map(|e| e == "einmo").unwrap_or(false) {
                out.push(p);
            }
        } else if file_type.is_dir() {
            collect_einmo_files_depth(&p, out, depth + 1, depth_limit)?;
        } else if p.extension().map(|e| e == "einmo").unwrap_or(false) {
            out.push(p);
        }
    }
    Ok(())
}

/// The mirror-relative `.einmo` paths present under `target_dir`.
///
/// When `files` is `Some`, only those user-provided paths are considered (after
/// normalization via [`normalize_file_path`]); `filter` is ignored. When `None`,
/// the full input tree is walked and optionally narrowed by `filter`. What
/// [`promote_flag_to_note`] uses to scan a stage's nested flagged sink
/// (`EIMP-7` §S.2a) — the `Stage`-based wrapper this once had is retired
/// along with `promote`/`flag`/`retract`, its only other callers.
fn matching_mirror_paths_in(
    config: &TestConfig,
    target_dir: &Path,
    filter: Option<&str>,
    files: Option<&[PathBuf]>,
) -> Result<Vec<PathBuf>> {
    if let Some(files) = files {
        let mut paths: Vec<PathBuf> = files
            .iter()
            .map(|p| normalize_file_path(p, config))
            .filter(|p| target_dir.join(p).exists())
            .collect();
        paths.sort();
        paths.dedup();
        return Ok(paths);
    }
    let inputs = walk_input_tree(&config.input_path(), config.walk_depth_limit())?;
    let mut paths = Vec::new();
    for input_rel in inputs {
        if let Some(pat) = filter
            && !glob_match(&input_rel.to_string_lossy(), pat)
        {
            continue;
        }
        let rel = mirror_input_path(&input_rel);
        if target_dir.join(&rel).exists() {
            paths.push(rel);
        }
    }
    Ok(paths)
}

/// Normalize a user-provided file path to a mirror-relative `.einmo` path.
///
/// Accepts any of:
/// - `test.einmo` — bare mirror-relative name (used as-is)
/// - `subdir/test.einmo` — mirror-relative path (used as-is)
/// - `output/test.einmo` — stage-relative path (strips the stage-dir prefix)
/// - `checked/sub/test.einmo` — stage-relative path (strips the stage-dir prefix)
/// - `/abs/path/to/suite/output/test.einmo` — absolute path (strips everything
///   up to and including the stage dir)
/// - `test.foo` — input name without `.einmo` (appends `.einmo`)
///
/// The stage-dir prefix check uses both the canonical stage dir names
/// ([`Stage::dir_name`]) and the suite's configured stage dir paths
/// ([`TestConfig::stage_dir`]) so customized directory names are honored.
#[must_use]
pub(crate) fn normalize_file_path(path: &Path, config: &TestConfig) -> PathBuf {
    let path_str = path.to_string_lossy().into_owned();

    // Ends with `.einmo` → mirror-relative, stage-relative, or absolute.
    if path_str.ends_with(".einmo") {
        // Stage-relative: `<stage_dir>/<rel>` for any configured stage name.
        for stage in Stage::ALL {
            let stage_name = config
                .stage_dir(stage)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| stage.dir_name().to_string());
            let prefix = format!("{stage_name}/");
            if path_str.starts_with(&prefix) {
                return PathBuf::from(&path_str[prefix.len()..]);
            }
        }
        // Absolute path: strip everything up to and including the stage dir.
        if path.is_absolute() {
            for stage in Stage::ALL {
                let stage_dir = config.stage_dir(stage);
                if let Ok(rel) = path.strip_prefix(&stage_dir) {
                    return rel.to_path_buf();
                }
            }
        }
        return path.to_path_buf();
    }

    // Doesn't end with `.einmo` → treat as an input name, append `.einmo`.
    mirror_input_path(path)
}

/// A minimal glob: `*` matches any run of characters; everything else literal.
///
/// Kept intentionally small (no `**`, no `?`) — sufficient for `--filter
/// algorithms/sorting/*`; a bare substring is expressible as `*sub*`.
fn glob_match(text: &str, pattern: &str) -> bool {
    // Collapse consecutive `*` into a single `*` to prevent exponential
    // backtracking on pathological patterns like `*****x`.
    let normalized: String = pattern.chars().fold(String::new(), |mut acc, c| {
        if c == '*' && acc.ends_with('*') {
            return acc; // skip: already have a trailing *
        }
        acc.push(c);
        acc
    });
    fn matches(t: &[u8], p: &[u8]) -> bool {
        match (p.first(), t.first()) {
            (None, None) => true,
            (Some(b'*'), _) => {
                // `*` matches zero chars, or one char then `*` again.
                matches(t, &p[1..]) || (!t.is_empty() && matches(&t[1..], p))
            }
            (Some(pc), Some(tc)) if pc == tc => matches(&t[1..], &p[1..]),
            _ => false,
        }
    }
    matches(text.as_bytes(), normalized.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::einmo_suite::ValidationLevel;
    use crate::format::{DEFAULT_SEPARATOR, Metadata, Section, Status};
    use crate::signature::{Stamps, derive_keypair};
    use std::fs;

    fn suite() -> (tempfile::TempDir, TestConfig) {
        let tmp = tempfile::tempdir().unwrap();
        let config = TestConfig::new(tmp.path(), ValidationLevel::Output);
        config.ensure_stage_dirs().unwrap();
        fs::create_dir_all(config.input_path()).unwrap();
        (tmp, config)
    }

    fn write_output(config: &TestConfig, rel: &str, output: &str) {
        fs::write(config.input_path().join(rel), "{5;}").unwrap();
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
        let path = config.stage_dir(Stage::Output).join(format!("{rel}.einmo"));
        ensure_parent_dir(&path).unwrap();
        fs::write(&path, file.serialize().unwrap()).unwrap();
    }

    /// Flag `rel` from the output stage — setup-only helper for the
    /// `promote_flag_to_note_*` tests below, which need a flagged
    /// artifact in place before exercising `promote_flag_to_note` itself.
    /// Uses `EinmoCase::flag` directly rather than the retired
    /// `transitions::flag` free function.
    fn flag_output(config: &TestConfig, rel: &str, reason: &str) {
        let directory = crate::storage::EinmoDirectory::new(config.clone());
        let id = crate::stage::EinmoId::from_input_rel(Path::new(rel)).unwrap();
        crate::case::EinmoCase::new(id, &directory)
            .flag(Stage::Output, reason)
            .unwrap();
    }

    // Single-case promote/flag/retract mechanics (stamp chains, cascade,
    // tampered-source refusal, illegal-transition refusal, re-flag
    // concatenation) are covered by `case.rs`'s own `EinmoCase` test suite
    // now — `transitions::promote`/`flag`/`retract` (the free functions
    // these tests used to exercise) are retired as of `EIMP-7` Phase F.
    // Batch/selection-specific behavior (multiple files, `filter`
    // overridden by explicit `ids`, deriving the key once per batch) is
    // covered by `suite.rs`'s `EinmoSuite` test suite instead.

    // EIMP-1 S.3: the signed notes/ stage.

    #[test]
    fn promote_flag_to_note_writes_a_signed_note_from_the_advisory() {
        let (_tmp, config) = suite();
        write_output(&config, "a.foo", "5");
        flag_output(&config, "a.foo", "needs a second look");

        let key = KeySource::from_passphrase("a note signer");
        let report = promote_flag_to_note(&config, Stage::Output, &key, None, None).unwrap();
        assert_eq!(report.noted, vec![PathBuf::from("a.foo.einmo")]);

        let note_path = config.stage_dir_for_notes().join("a.foo.einmo");
        let note = EinmoFile::from_file(&note_path).unwrap();
        assert!(note.chain_valid(), "a note must be a genuinely signed file");
        assert!(
            note.section("NOTE")
                .unwrap()
                .body()
                .contains("needs a second look"),
            "the note's body must carry the flag's advisory text"
        );
        let (_, notes_vk) = derive_keypair("a note signer");
        let stage_stamp = note
            .stamps()
            .entries()
            .iter()
            .find(|s| s.key() == "stage:notes")
            .expect("a stage:notes stamp must be present");
        assert_eq!(stage_stamp.pubkey_hex(), hex::encode(notes_vk.to_bytes()));
    }

    #[test]
    fn promote_flag_to_note_does_not_consume_the_flag() {
        let (_tmp, config) = suite();
        write_output(&config, "a.foo", "5");
        flag_output(&config, "a.foo", "needs a second look");

        promote_flag_to_note(
            &config,
            Stage::Output,
            &KeySource::from_passphrase(""),
            None,
            None,
        )
        .unwrap();

        assert!(
            config
                .flagged_dir(Stage::Output)
                .join("a.foo.einmo")
                .exists(),
            "promoting to a note must not remove the flag -- resolving the flag is a separate action"
        );
    }

    #[test]
    fn promote_flag_to_note_carries_the_concatenated_history() {
        let (_tmp, config) = suite();
        write_output(&config, "a.foo", "5");
        flag_output(&config, "a.foo", "first");
        write_output(&config, "a.foo", "5");
        flag_output(&config, "a.foo", "second");

        promote_flag_to_note(
            &config,
            Stage::Output,
            &KeySource::from_passphrase(""),
            None,
            None,
        )
        .unwrap();

        let note = EinmoFile::from_file(&config.stage_dir_for_notes().join("a.foo.einmo")).unwrap();
        let body = note.section("NOTE").unwrap().body();
        assert!(body.contains("# flagged: first"));
        assert!(body.contains("# flagged: second"));
    }

    #[test]
    fn confirm_signatures_matches_prefix() {
        let (_tmp, config) = suite();
        write_output(&config, "a.foo", "5");
        // The output stage key is the empty-passphrase computer key.
        let (_, computer) = derive_keypair("");
        let prefix = &hex::encode(computer.to_bytes())[..8];
        let report = confirm_signatures(&config.stage_dir(Stage::Output), prefix).unwrap();
        assert_eq!(report.matched, vec![PathBuf::from("a.foo.einmo")]);
        assert!(report.all_matched());

        let none = confirm_signatures(&config.stage_dir(Stage::Output), "ffffffff").unwrap();
        assert!(!none.all_matched());
        assert_eq!(none.unmatched, vec![PathBuf::from("a.foo.einmo")]);
    }

    #[test]
    fn glob_matches_subtree() {
        assert!(glob_match(
            "algorithms/sorting/quick.foo",
            "algorithms/sorting/*"
        ));
        assert!(!glob_match(
            "algorithms/searching/bin.foo",
            "algorithms/sorting/*"
        ));
        assert!(glob_match("anything", "*"));
        assert!(glob_match("a/b/c", "*b*"));
    }

    #[test]
    fn glob_match_consecutive_stars_dont_backtrack() {
        assert!(glob_match("hello", "*****hello"));
        assert!(!glob_match("hello", "****x"));
    }

    #[test]
    fn normalize_paths() {
        let (_tmp, config) = suite();
        let out_dir = config.stage_dir(Stage::Output);
        let checked_dir = config.stage_dir(Stage::Checked);

        assert_eq!(
            normalize_file_path(Path::new("test.einmo"), &config),
            PathBuf::from("test.einmo")
        );
        assert_eq!(
            normalize_file_path(Path::new("sub/test.einmo"), &config),
            PathBuf::from("sub/test.einmo")
        );
        assert_eq!(
            normalize_file_path(Path::new("output/test.einmo"), &config),
            PathBuf::from("test.einmo")
        );
        assert_eq!(
            normalize_file_path(Path::new("checked/sub/test.einmo"), &config),
            PathBuf::from("sub/test.einmo")
        );
        assert_eq!(
            normalize_file_path(Path::new("test.foo"), &config),
            PathBuf::from("test.foo.einmo")
        );
        assert_eq!(
            normalize_file_path(&out_dir.join("deep/nested.einmo"), &config),
            PathBuf::from("deep/nested.einmo")
        );
        assert_eq!(
            normalize_file_path(&checked_dir.join("x.einmo"), &config),
            PathBuf::from("x.einmo")
        );
    }

    // Multi-file/batch selection behavior (several files, filter overridden
    // by explicit ids, stage-relative/absolute path normalization) is
    // covered by `suite.rs`'s `EinmoSuite` test suite and `normalize_paths`
    // above -- these used to exercise the same thing through the retired
    // `transitions::promote`/`flag` free functions.

    // EIMP-1 S.3: notes/ round-trip — a promoted note passes
    // verify-on-inspect, and its stage:notes stamp verifies against the
    // passphrase-derived key. Also confirms notes/ is signed while
    // flagged/ carries only the original stamps (no stage:notes stamp).

    #[test]
    fn promote_flag_to_note_round_trip_verify_on_inspect_and_key_match() {
        let (_tmp, config) = suite();
        write_output(&config, "a.foo", "5");
        flag_output(&config, "a.foo", "needs attention");

        let note_key = KeySource::from_passphrase("a-note-signer");
        let report = promote_flag_to_note(&config, Stage::Output, &note_key, None, None).unwrap();
        assert_eq!(report.noted, vec![PathBuf::from("a.foo.einmo")]);

        // The note must pass verify-on-inspect (the real from_file path).
        let note_path = config.stage_dir_for_notes().join("a.foo.einmo");
        let note =
            EinmoFile::from_file(&note_path).expect("a promoted note must pass verify-on-inspect");
        assert!(note.chain_valid(), "stamp chain must be fully valid");

        // The stage:notes stamp's pubkey must match the passphrase-derived key.
        let (_, expected_vk) = derive_keypair("a-note-signer");
        let stage_stamp = note
            .stamps()
            .entries()
            .iter()
            .find(|s| s.key() == "stage:notes")
            .expect("stage:notes stamp must be present");
        assert_eq!(
            stage_stamp.pubkey_hex(),
            hex::encode(expected_vk.to_bytes()),
            "the note's stamp must verify against the passphrase-derived key"
        );

        // The flagged file still verifies (it kept its original stamps) but
        // has NO stage:notes stamp — notes/ participates in signature checks
        // while flagged/ does not add a new attestation.
        let flagged_path = config.flagged_dir(Stage::Output).join("a.foo.einmo");
        let flagged =
            EinmoFile::from_file(&flagged_path).expect("flagged file must still verify-on-inspect");
        assert!(
            flagged
                .stamps()
                .entries()
                .iter()
                .all(|s| s.key() != "stage:notes"),
            "flagged/ must not carry a stage:notes stamp"
        );
    }
}
