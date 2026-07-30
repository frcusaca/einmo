//! The review session's audit / crash-recovery journal (`EIMP-1` §S.6):
//! append-only JSONL, one file per session, under a scratch/state
//! directory — never inside the suite itself (the journal is ephemeral
//! session/process state, not part of the reviewed corpus).
//!
//! **Journaling must never fail the review.** Every failure mode here
//! (cannot create the scratch dir, cannot harden its permissions, cannot
//! open or write the file) degrades to "this event was not recorded"
//! rather than propagating an error — the same principle `EIMP-6`
//! (`docs/eimp/EIMP-6.md` §S.2) states for the future test-run journal,
//! applied here for the session journal too. A reviewer's actual work
//! (deciding, executing) must never be blocked by an audit log's own
//! plumbing.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::signature::now_iso8601;
use crate::stage::Stage;

/// How much the journal records (`EIMP-1` §S.6). Ordered — `Terse < Normal
/// < Fine` — so a level check is a plain comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum JournalLevel {
    /// Session open/close, `execute` batches and their outcomes.
    Terse,
    /// The above, plus every decide/undecide/claim.
    #[default]
    Normal,
    /// The above, plus each case as it is read in and verified — one entry
    /// per [`crate::stage::EinmoId`] per verification. This is what makes
    /// the journal able to answer "which case was in flight when this
    /// crashed?" (`EIMP-1` §S.6's crash-crumb-capable claim).
    Fine,
}

/// The wire form of a [`crate::review::Decision`]. Domain types stay
/// serde-free (matching `review_server.rs`'s existing DTO convention for
/// exactly this reason) — this is the journal's own small, explicit
/// encoding, converted at the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum JournalDecision {
    /// See [`crate::review::Decision::Promote`].
    Promote {
        /// The destination stage's directory name.
        to: String,
    },
    /// See [`crate::review::Decision::Retract`].
    Retract {
        /// The source stage's directory name.
        from: String,
    },
    /// See [`crate::review::Decision::Flag`].
    Flag {
        /// The stage's directory name.
        stage: String,
        /// The advisory reason.
        reason: String,
    },
    /// See [`crate::review::Decision::Skip`].
    Skip,
}

impl From<&crate::review::Decision> for JournalDecision {
    fn from(decision: &crate::review::Decision) -> Self {
        match decision {
            crate::review::Decision::Promote { to } => JournalDecision::Promote {
                to: to.dir_name().to_string(),
            },
            crate::review::Decision::Retract { from } => JournalDecision::Retract {
                from: from.dir_name().to_string(),
            },
            crate::review::Decision::Flag { stage, reason } => JournalDecision::Flag {
                stage: stage.dir_name().to_string(),
                reason: reason.clone(),
            },
            crate::review::Decision::Skip => JournalDecision::Skip,
        }
    }
}

impl JournalDecision {
    /// Reconstruct the domain [`crate::review::Decision`], if the stage
    /// names round-trip through [`Stage::parse`].
    #[must_use]
    pub fn into_decision(self) -> Option<crate::review::Decision> {
        Some(match self {
            JournalDecision::Promote { to } => crate::review::Decision::Promote {
                to: Stage::parse(&to).ok()?,
            },
            JournalDecision::Retract { from } => crate::review::Decision::Retract {
                from: Stage::parse(&from).ok()?,
            },
            JournalDecision::Flag { stage, reason } => crate::review::Decision::Flag {
                stage: Stage::parse(&stage).ok()?,
                reason,
            },
            JournalDecision::Skip => crate::review::Decision::Skip,
        })
    }
}

/// One journal event — the payload of one JSONL line, alongside its
/// timestamp (`EIMP-1` §S.6: "session id, reviewer, timestamp,
/// produced_by, every decide/undecide/claim/execute with outcomes").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum JournalEvent {
    /// A session began (fresh `open` or a `resume`).
    SessionOpen {
        /// The session id.
        session: String,
        /// The suite's work directory.
        suite: String,
    },
    /// A session ended cleanly.
    SessionClose {
        /// The session id.
        session: String,
    },
    /// [`crate::review::EinmoReview::decide`] recorded (or replaced) a
    /// decision.
    Decide {
        /// The case, as [`crate::stage::EinmoId::as_str`].
        id: String,
        /// The decision, in its wire form.
        decision: JournalDecision,
    },
    /// [`crate::review::EinmoReview::undecide`] cleared a decision.
    Undecide {
        /// The case, as [`crate::stage::EinmoId::as_str`].
        id: String,
    },
    /// One [`crate::review::EinmoReview::execute`] batch completed.
    ExecuteBatch {
        /// Cases whose action was applied.
        executed: Vec<String>,
        /// Cases whose action was skipped (drifted, or source gone).
        skipped: Vec<String>,
    },
    /// `fine`-level only: a case's verified body began verify-on-inspect.
    VerifyStart {
        /// The case, as [`crate::stage::EinmoId::as_str`].
        id: String,
        /// The stage's directory name.
        stage: String,
    },
    /// `fine`-level only: the matching [`JournalEvent::VerifyStart`]
    /// completed. An unmatched `VerifyStart` (no following `VerifyEnd`) is
    /// the in-flight case a crash interrupted (`EIMP-1` §S.6).
    VerifyEnd {
        /// The case, as [`crate::stage::EinmoId::as_str`].
        id: String,
        /// The stage's directory name.
        stage: String,
        /// Whether verify-on-inspect succeeded.
        ok: bool,
    },
}

/// One decoded journal line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalLine {
    /// ISO-8601 UTC.
    pub timestamp: String,
    /// The event itself.
    #[serde(flatten)]
    pub event: JournalEvent,
}

/// The environment variable overriding the journal's scratch directory —
/// same override-with-a-default shape `einmo_review_client.sh`'s
/// `EINMO_REVIEW_CLIENT_DIR` already establishes.
const JOURNAL_DIR_ENV: &str = "EINMO_JOURNAL_DIR";

/// The scratch/state directory journals live under (`EIMP-1` §S.6):
/// `$EINMO_JOURNAL_DIR` if set, else a fixed subdirectory of the system
/// temp dir. Never inside the suite.
#[must_use]
pub fn journal_dir() -> PathBuf {
    match std::env::var_os(JOURNAL_DIR_ENV) {
        Some(dir) => PathBuf::from(dir),
        None => std::env::temp_dir().join("einmo-journal"),
    }
}

/// The path a given session's journal lives at, under [`journal_dir`].
#[must_use]
pub fn journal_path(session_id: &str) -> PathBuf {
    journal_dir().join(format!("{session_id}.jsonl"))
}

/// Create `dir` if needed and harden it to mode 0700 — the same discipline
/// `einmo_review_client.sh`'s `harden_dir` applies to its scratch space,
/// since a journal can carry case ids and decision details a reviewer may
/// not want group/other-readable.
fn harden_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
}

/// An append-only JSONL writer for one session (`EIMP-1` §S.6).
///
/// `writer` is `None` whenever opening failed for any reason — logging
/// degrades silently rather than ever failing the review (see the module
/// doc). `log`/`log_at` are therefore infallible.
pub struct Journal {
    session_id: String,
    level: JournalLevel,
    writer: Mutex<Option<std::fs::File>>,
}

impl Journal {
    /// Open (creating if needed) the journal for `session_id` at `level`.
    /// Never fails — a plumbing failure just means events go unrecorded.
    #[must_use]
    pub fn open(session_id: impl Into<String>, level: JournalLevel) -> Self {
        let session_id = session_id.into();
        let path = journal_path(&session_id);
        let writer = harden_dir(&journal_dir())
            .and_then(|()| {
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
            })
            .ok();
        Journal {
            session_id,
            level,
            writer: Mutex::new(writer),
        }
    }

    /// The session id this journal was opened for.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// The path this journal writes to (for tests, and for a frontend that
    /// wants to point a human at it).
    #[must_use]
    pub fn path(&self) -> PathBuf {
        journal_path(&self.session_id)
    }

    /// Record `event`, unless `self`'s configured level is below `min_level`
    /// — e.g. a `Fine`-only event passed with `min_level: JournalLevel::Fine`
    /// is silently dropped by a `Normal`-level journal.
    pub fn log_at(&self, min_level: JournalLevel, event: JournalEvent) {
        if self.level < min_level {
            return;
        }
        let line = JournalLine {
            timestamp: now_iso8601(),
            event,
        };
        let Ok(json) = serde_json::to_string(&line) else {
            return;
        };
        if let Ok(mut guard) = self.writer.lock()
            && let Some(file) = guard.as_mut()
        {
            let _ = writeln!(file, "{json}");
        }
    }

    /// Replay a journal file back into its decoded lines, in order.
    ///
    /// **Truncated-tail tolerant**: a line that fails to parse (a crash mid
    /// -write leaves a partial final line; any other corruption is treated
    /// the same way) is silently skipped rather than aborting the whole
    /// replay — every line that DOES parse is still returned. Journal
    /// corruption degrades what can be recovered, it does not turn into a
    /// hard failure (same "never fail the review" principle as `log_at`).
    ///
    /// Returns an empty `Vec` if the file does not exist yet (a session
    /// that never wrote anything, or a session id nothing has journaled).
    #[must_use]
    pub fn replay(path: &Path) -> Vec<JournalLine> {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Vec::new();
        };
        contents
            .lines()
            .filter_map(|line| serde_json::from_str::<JournalLine>(line).ok())
            .collect()
    }
}

impl std::fmt::Debug for Journal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Journal")
            .field("session_id", &self.session_id)
            .field("level", &self.level)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes every test that touches the process-global
    /// `EINMO_JOURNAL_DIR`, so parallel `cargo test` threads cannot race on
    /// it (same discipline as `einmo_suite.rs`'s own `ENV_LOCK`). Held for
    /// each test's whole body via the returned guard.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn scratch_env() -> (
        std::sync::MutexGuard<'static, ()>,
        tempfile::TempDir,
        String,
    ) {
        let guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: serialized by `ENV_LOCK` above -- no other test reads or
        // writes EINMO_JOURNAL_DIR while this guard is held.
        unsafe {
            std::env::set_var(JOURNAL_DIR_ENV, tmp.path());
        }
        // The leaf component only -- `tmp.path()` embeds `/`, which would
        // corrupt the journal filename if used whole (session ids become a
        // `{id}.jsonl` filename component).
        let session = format!(
            "test-session-{}",
            tmp.path().file_name().unwrap().to_string_lossy()
        );
        (guard, tmp, session)
    }

    #[test]
    fn journal_level_orders_terse_normal_fine() {
        assert!(JournalLevel::Terse < JournalLevel::Normal);
        assert!(JournalLevel::Normal < JournalLevel::Fine);
        assert_eq!(JournalLevel::default(), JournalLevel::Normal);
    }

    #[test]
    fn log_at_respects_the_configured_level() {
        let (_guard, _tmp, session) = scratch_env();
        let journal = Journal::open(&session, JournalLevel::Terse);
        journal.log_at(
            JournalLevel::Fine,
            JournalEvent::VerifyStart {
                id: "a.foo".into(),
                stage: "output".into(),
            },
        );
        journal.log_at(
            JournalLevel::Terse,
            JournalEvent::SessionClose {
                session: session.clone(),
            },
        );
        let lines = Journal::replay(&journal.path());
        assert_eq!(
            lines.len(),
            1,
            "the Fine-level event must be dropped by a Terse journal"
        );
        assert!(matches!(lines[0].event, JournalEvent::SessionClose { .. }));
    }

    #[test]
    fn replay_round_trips_events_in_order() {
        let (_guard, _tmp, session) = scratch_env();
        let journal = Journal::open(&session, JournalLevel::Fine);
        journal.log_at(
            JournalLevel::Terse,
            JournalEvent::SessionOpen {
                session: session.clone(),
                suite: "/tmp/suite".into(),
            },
        );
        journal.log_at(
            JournalLevel::Normal,
            JournalEvent::Decide {
                id: "a.foo".into(),
                decision: JournalDecision::Promote {
                    to: "checked".into(),
                },
            },
        );
        journal.log_at(
            JournalLevel::Normal,
            JournalEvent::Undecide { id: "a.foo".into() },
        );
        let lines = Journal::replay(&journal.path());
        assert_eq!(lines.len(), 3);
        assert!(matches!(lines[0].event, JournalEvent::SessionOpen { .. }));
        assert!(matches!(lines[1].event, JournalEvent::Decide { .. }));
        assert!(matches!(lines[2].event, JournalEvent::Undecide { .. }));
    }

    #[test]
    fn replay_skips_an_unparseable_line_wherever_it_occurs() {
        let (_guard, _tmp, session) = scratch_env();
        let journal = Journal::open(&session, JournalLevel::Normal);
        journal.log_at(
            JournalLevel::Terse,
            JournalEvent::SessionOpen {
                session: session.clone(),
                suite: "/tmp/suite".into(),
            },
        );
        // Simulate a crash mid-write: append a truncated/garbage line
        // directly, bypassing `log_at`.
        {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(journal.path())
                .unwrap();
            writeln!(f, "{{\"timestamp\":\"2026-01-01").unwrap(); // truncated JSON
        }
        journal.log_at(
            JournalLevel::Terse,
            JournalEvent::SessionClose {
                session: session.clone(),
            },
        );
        let lines = Journal::replay(&journal.path());
        assert_eq!(
            lines.len(),
            2,
            "the truncated line must be skipped, not abort the whole replay"
        );
        assert!(matches!(lines[0].event, JournalEvent::SessionOpen { .. }));
        assert!(matches!(lines[1].event, JournalEvent::SessionClose { .. }));
    }

    #[test]
    fn replay_of_a_never_written_session_is_empty() {
        let (_guard, _tmp, session) = scratch_env();
        let lines = Journal::replay(&journal_path(&format!("{session}-never-opened")));
        assert!(lines.is_empty());
    }

    #[test]
    fn journal_decision_round_trips_through_its_wire_form() {
        use crate::review::Decision;
        for decision in [
            Decision::Promote { to: Stage::Checked },
            Decision::Retract {
                from: Stage::Checked,
            },
            Decision::Flag {
                stage: Stage::Output,
                reason: "looks wrong".into(),
            },
            Decision::Skip,
        ] {
            let wire = JournalDecision::from(&decision);
            let json = serde_json::to_string(&wire).unwrap();
            let back: JournalDecision = serde_json::from_str(&json).unwrap();
            assert_eq!(back.into_decision().unwrap(), decision);
        }
    }

    #[test]
    fn harden_dir_creates_and_secures_mode_0700() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested").join("journal-dir");
        harden_dir(&dir).unwrap();
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
    }
}
