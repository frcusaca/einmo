//! The manifest collation for section-level attestation (`EIMP-1` §S.11a).
//!
//! The digest `CorpusSigner` signs is a concatenation in manifest order, so
//! the order is part of the signature: two machines that order the same
//! corpus differently compute different digests for identical content. The
//! collation is therefore an explicit, configurable parameter of the
//! signature rather than an accident of directory-walk order.

use crate::error::{EinmoError, Result};
use crate::stage::EinmoId;

/// How a section's files are ordered before digesting. Part of the signed
/// parameters — its stable identifier is recorded in `.section.sig` so a
/// verifier never mistakes a configuration difference for tampering.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Collation {
    /// DEFAULT. Component-wise, byte-wise within a component. No locale, no
    /// normalization, no case folding. Reproducible on every machine.
    PathBytes,
}

impl Collation {
    /// The suite's default collation when `einmo.toml` sets none.
    pub const DEFAULT: Collation = Collation::PathBytes;

    /// The stable identifier recorded in `.section.sig` and `einmo.toml`
    /// (`[signing] collation = "…"`).
    #[must_use]
    pub fn identifier(self) -> &'static str {
        match self {
            Collation::PathBytes => "path-bytes",
        }
    }

    /// Parse a collation identifier as read from `.section.sig` or
    /// `einmo.toml`.
    ///
    /// # Errors
    ///
    /// Returns [`EinmoError::CorpusSignature`] for an unrecognized
    /// identifier — deliberately distinct from a digest/signature mismatch,
    /// so a verifier never confuses "I don't know this configuration" with
    /// "the corpus was tampered with".
    pub fn parse(identifier: &str) -> Result<Self> {
        match identifier {
            "path-bytes" => Ok(Collation::PathBytes),
            other => Err(EinmoError::CorpusSignature(format!(
                "unknown collation identifier {other:?}"
            ))),
        }
    }

    /// Sort `ids` in place per this collation.
    ///
    /// # Errors
    ///
    /// Returns [`EinmoError::CorpusSignature`] if two distinct entries
    /// compare equal under this collation (§S.11a item 5: a tie is always a
    /// hard error, never a silent tiebreak).
    pub fn sort(self, ids: &mut [EinmoId]) -> Result<()> {
        match self {
            Collation::PathBytes => sort_path_bytes(ids),
        }
    }
}

/// A path's components as raw bytes — the unit `PathBytes` compares. Never
/// compare `EinmoId::as_str()` directly: that would let a path separator
/// (`/`, 0x2F) and an in-name character (`-`, 0x2D) decide order by byte
/// value instead of by structure, which is exactly backwards for the `a/b`
/// vs `a-b` case this collation exists to get right.
fn components(id: &EinmoId) -> Vec<&[u8]> {
    id.as_str().split('/').map(str::as_bytes).collect()
}

fn sort_path_bytes(ids: &mut [EinmoId]) -> Result<()> {
    // Component-wise, byte-wise comparison: `Vec<&[u8]>`'s derived `Ord`
    // compares element-wise and treats a shorter prefix as `Less`, which is
    // exactly §S.11a item 2's "if one path's components are a prefix of the
    // other's, the shorter sorts first."
    ids.sort_by(|a, b| components(a).cmp(&components(b)));
    for pair in ids.windows(2) {
        if components(&pair[0]) == components(&pair[1]) {
            return Err(EinmoError::CorpusSignature(format!(
                "duplicate path under collation {:?}: {:?}",
                Collation::PathBytes.identifier(),
                pair[0].as_str()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &str) -> EinmoId {
        EinmoId::try_from(s).unwrap()
    }

    fn sorted(mut ids: Vec<EinmoId>) -> Vec<String> {
        Collation::PathBytes.sort(&mut ids).unwrap();
        ids.iter().map(|i| i.as_str().to_string()).collect()
    }

    // ---- identifier round-trip ----

    #[test]
    fn identifier_round_trips() {
        assert_eq!(Collation::PathBytes.identifier(), "path-bytes");
        assert_eq!(
            Collation::parse("path-bytes").unwrap(),
            Collation::PathBytes
        );
    }

    #[test]
    fn default_is_path_bytes() {
        assert_eq!(Collation::DEFAULT, Collation::PathBytes);
    }

    #[test]
    fn unknown_identifier_fails_as_that_not_a_generic_mismatch() {
        let err = Collation::parse("bogus-collation").unwrap_err();
        match err {
            EinmoError::CorpusSignature(msg) => {
                assert!(
                    msg.contains("unknown collation"),
                    "error must name it as an unknown collation, not a generic failure: {msg}"
                );
            }
            other => panic!("expected CorpusSignature, got {other:?}"),
        }
    }

    // ---- total order, discovery-order invariance ----

    #[test]
    fn ordering_is_stable_regardless_of_discovery_order() {
        let a = vec![id("z/1.foo"), id("a/2.foo"), id("m/3.foo")];
        let b = vec![id("a/2.foo"), id("m/3.foo"), id("z/1.foo")];
        let c = vec![id("m/3.foo"), id("z/1.foo"), id("a/2.foo")];
        let (sa, sb, sc) = (sorted(a), sorted(b), sorted(c));
        assert_eq!(sa, sb);
        assert_eq!(sb, sc);
        assert_eq!(sa, vec!["a/2.foo", "m/3.foo", "z/1.foo"]);
    }

    // ---- structural, not byte-value, separator handling ----

    #[test]
    fn separator_vs_in_name_character_sorts_structurally() {
        // "a/b" splits into ["a", "b"]; "a-b" splits into ["a-b"] (one
        // component). Component 0 compares "a" vs "a-b": "a" is a strict
        // prefix, so "a/b" sorts FIRST — the opposite of a raw string
        // compare, which would put "a-b" first since '-' (0x2D) < '/' (0x2F).
        let ids = vec![id("a-b"), id("a/b")];
        assert_eq!(sorted(ids), vec!["a/b", "a-b"]);
    }

    // ---- nested vs flat sharing a prefix ----

    #[test]
    fn shorter_prefix_sorts_first_when_nested() {
        // "a" (one component) vs "a/b" (two components, first component
        // equal): the shorter (fewer components) sorts first.
        let ids = vec![id("a/b"), id("a")];
        assert_eq!(sorted(ids), vec!["a", "a/b"]);
    }

    #[test]
    fn flat_and_nested_sharing_a_prefix_order_by_structure() {
        let ids = vec![id("ab"), id("a/b"), id("a")];
        // Component 0: "a" < "a/b"'s "a" == "ab"? "a" vs "ab": "a" is a
        // strict prefix of "ab", so "a" sorts before "ab". "a/b" has first
        // component "a" too, tying with the bare "a" id at component 0, but
        // "a" is shorter (fewer components) so it sorts before "a/b". Final
        // order: "a", "a/b", "ab".
        assert_eq!(sorted(ids), vec!["a", "a/b", "ab"]);
    }

    // ---- case sensitivity: no folding ----

    #[test]
    fn case_differing_paths_sort_deterministically_without_folding() {
        let ids = vec![id("B.foo"), id("a.foo"), id("A.foo"), id("b.foo")];
        // Raw byte order: uppercase ASCII (0x41-0x5A) sorts before lowercase
        // (0x61-0x7A) — no case folding, so all four remain distinct.
        assert_eq!(sorted(ids), vec!["A.foo", "B.foo", "a.foo", "b.foo"]);
    }

    #[test]
    fn case_sort_is_deterministic_across_discovery_orders() {
        let a = vec![id("Zebra.foo"), id("apple.foo"), id("Banana.foo")];
        let b = vec![id("apple.foo"), id("Banana.foo"), id("Zebra.foo")];
        assert_eq!(sorted(a), sorted(b));
    }

    // ---- no Unicode normalization ----

    #[test]
    fn distinct_normalization_forms_are_not_folded_together() {
        // "é" as one precomposed codepoint (U+00E9, NFC) vs "e" + combining
        // acute accent (U+0065 U+0301, NFD). Distinct byte sequences must
        // remain distinct components — no normalization would silently
        // treat them as the same path.
        let nfc = "caf\u{00e9}.foo"; // café (precomposed)
        let nfd = "cafe\u{0301}.foo"; // cafe + combining acute
        assert_ne!(nfc.as_bytes(), nfd.as_bytes());
        let ids = vec![id(nfd), id(nfc)];
        let out = sorted(ids);
        // Both survive as distinct entries (no error, no collapse).
        assert_eq!(out.len(), 2);
        assert!(out.contains(&nfc.to_string()));
        assert!(out.contains(&nfd.to_string()));
    }

    // ---- ties are a hard error ----

    #[test]
    fn duplicate_path_is_a_hard_error_never_a_silent_tiebreak() {
        // Two independently constructed EinmoIds for the same path.
        let mut ids = vec![id("dup/case.foo"), id("dup/case.foo")];
        let err = Collation::PathBytes.sort(&mut ids).unwrap_err();
        match err {
            EinmoError::CorpusSignature(msg) => {
                assert!(
                    msg.contains("duplicate"),
                    "expected a duplicate-path error: {msg}"
                );
            }
            other => panic!("expected CorpusSignature, got {other:?}"),
        }
    }

    #[test]
    fn no_tie_among_many_distinct_paths() {
        let ids = vec![
            id("a.foo"),
            id("a/b.foo"),
            id("a/b/c.foo"),
            id("ab.foo"),
            id("b.foo"),
            id("nested/deep/path.foo"),
        ];
        assert!(Collation::PathBytes.sort(&mut ids.clone()).is_ok());
    }
}
