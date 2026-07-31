//! Einmo — directory-based, cryptographically signed snapshot testing with a
//! staged promotion pipeline (`output` → `checked` → `flagged` / `verified`).
//!
//! Einmo is **standalone**: it depends on no other workspace crate and
//! reimplements its signing/format machinery from scratch, so it can be
//! promoted to its own repository later (FOOP-92 §1).
//!
//! # The four stages
//!
//! Each test suite has a work directory with an `input/` tree and four stage
//! directories (`output/`, `checked/`, `flagged/`, `verified/`) that mirror
//! the `input/` tree at any depth. Every generated output is timestamped and
//! signed; promotion between stages appends a stamp and is a deliberate act,
//! never an automated `accept`.
//!
//! # Verify-on-inspect
//!
//! Any operation that reads a `.einmo` file verifies *all* its stamps first;
//! a tampered file is refused, never operated on. The pure verification path
//! lives in [`verify`] with no filesystem/tty/Argon2 dependency (WASM-ready).

mod cli;
mod collation;
mod compare;
mod config;
mod corpus_signer;
mod einmo_suite;
mod error;
mod format;
mod journal;
mod review;
mod review_server;
mod signature;
mod stage;
mod storage;
mod suite_lock;
mod transitions;
mod verify;

pub use cli::cli_main;
pub use collation::Collation;
pub use compare::{ComparisonResult, DiffEntry, MatchSections, compare};
pub use config::{
    KeyCascadeInputs, KeySource, Perspective, PerspectiveOf, StageDirs, TestConfig,
    resolve_stage_key,
};
pub use corpus_signer::{CorpusSigner, SectionDigest, SectionManifest, SectionSig};
pub use einmo_suite::{
    EinmoTestRunner, Evaluator, FailurePolicy, FileResult, Problem, SuiteIntegrity, TestResults,
    ValidationLevel, check_suite_integrity, count_flagged,
};
pub use error::EinmoError;
pub use format::{EinmoFile, Metadata, Section, Status};
pub use journal::{
    Journal, JournalDecision, JournalEvent, JournalLevel, JournalLine, journal_dir, journal_path,
};
pub use review::{
    ActiveClaim, Decision, DiffHunks, DiffLine, EinmoReview, Executed, ExecutionPlan,
    ExecutionReport, PlannedAction, ReviewItem, ReviewMode, ReviewOpts, SectionDiff, SignerSet,
    VerifiedBody,
};
pub use review_server::{
    ApiError, AppState, DiffLineResponse, DiffResponse, SectionDiffResponse, SessionId,
    private_socket_path, router, router_tcp, serve_tcp, serve_uds,
};
pub use signature::{Stamp, StampRole, Stamps};
pub use stage::{EinmoId, Stage};
pub use storage::{ArtifactLocation, EinmoDirectory, EinmoStorage};
pub use suite_lock::{SuiteLock, suite_lock_path};
pub use transitions::{
    FlagReport, NoteReport, PromotionReport, RetractReport, SignatureReport, confirm_signatures,
    flag, promote, promote_flag_to_note, retract,
};
pub use verify::{
    FileVerification, StampVerification, VerificationReport, verify, verify_all, verify_bytes,
};
