//! The single error type for the einmo library.

use std::path::PathBuf;

/// Errors produced by einmo operations.
///
/// Callers branch on the variant; every fallible einmo function returns this.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EinmoError {
    /// An I/O failure, annotated with the path involved.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// The path the operation was acting on.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The `.einmo` envelope was malformed and could not be parsed.
    #[error("malformed .einmo envelope: {0}")]
    Parse(String),

    /// A section's content contained the configured separator, so the
    /// envelope cannot be serialized unambiguously (§4.1 collision rule).
    #[error("section {section:?} contains the configured separator; configure a different one")]
    SeparatorCollision {
        /// The name of the offending section.
        section: String,
    },

    /// Signature verification failed — a stamp did not validate. The file is
    /// refused (verify-on-inspect); it is never operated on.
    #[error("signature verification failed: {0}")]
    Verification(String),

    /// A stamp's JSON could not be parsed or was structurally invalid.
    #[error("invalid stamp: {0}")]
    Stamp(String),

    /// A stage or stage-directory name failed validation (`[A-Za-z0-9_-]+`).
    #[error("invalid stage name {0:?}: must match [A-Za-z0-9_-]+")]
    InvalidStageName(String),

    /// A case identifier ([`crate::stage::EinmoId`]) failed validation or
    /// could not be derived from the given path.
    #[error("invalid einmo id {0:?}")]
    InvalidId(String),

    /// A requested transition is not one of the legal stage transitions.
    #[error("illegal transition: {from} to {to}")]
    IllegalTransition {
        /// The origin stage name.
        from: String,
        /// The destination stage name.
        to: String,
    },

    /// A configuration value (e.g. `einmo.toml`) was invalid.
    #[error("configuration error: {0}")]
    Config(String),

    /// No key material could be resolved for a signing operation.
    #[error("no signing key available: {0}")]
    NoKey(String),

    /// Section-level post-quantum attestation (`CorpusSigner`, EIMP-1 §S.11)
    /// failed: an unrecognized collation, a manifest/digest inconsistency, or
    /// an SLH-DSA signature that did not verify. Deliberately distinct from
    /// [`EinmoError::Verification`] (the per-file Ed25519 stamp-chain
    /// checker) — a wrong-configuration error (e.g. an unknown collation
    /// identifier) must never look like a tampered-corpus mismatch.
    #[error("corpus signature error: {0}")]
    CorpusSignature(String),
}

impl EinmoError {
    /// Build an [`EinmoError::Io`] carrying the offending path.
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

/// Convenience alias used throughout the crate.
pub(crate) type Result<T> = std::result::Result<T, EinmoError>;
