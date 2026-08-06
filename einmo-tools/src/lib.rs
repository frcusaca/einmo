use std::io::Write;

/// Calculate the passphrase effectiveness score for a promotion to the
/// verified stage.
///
/// The score measures how much the passphrase increases the compressed
/// size of the existing verified corpus. A score > 0 means the passphrase
/// adds information not already present in the corpus.
///
/// Returns `Ok(Some(score))` when the check applies (human key, verified
/// stage), `Ok(None)` when the check does not apply (computer key or
/// non-verified stage), or `Err` if the score computation or gate fails.
pub fn check_verified_passphrase(
    is_computer_key: bool,
    verified_files_concat: &[u8],
    passphrase: &str,
) -> Result<Option<f64>, String> {
    // The computer key (empty passphrase) is the well-known non-human
    // attestation. It is allowed through — einmo detects it post-hoc via
    // the `non_human` flag, not by blocking it here.
    if is_computer_key {
        return Ok(None);
    }
    let score = calculate_passphrase_score(verified_files_concat, passphrase)?;
    if score <= 0.0 {
        return Err(format!(
            "passphrase effectiveness score is {score}, must be > 0"
        ));
    }
    Ok(Some(score))
}

pub fn calculate_passphrase_score(
    verified_files_concat: &[u8],
    passphrase: &str,
) -> Result<f64, String> {
    let mut compressed1 = Vec::new();
    let mut encoder1 =
        zstd::stream::write::Encoder::new(&mut compressed1, 0).map_err(|e| e.to_string())?;
    encoder1
        .write_all(verified_files_concat)
        .map_err(|e| e.to_string())?;
    encoder1.finish().map_err(|e| e.to_string())?;

    let size1 = compressed1.len() as f64;

    let mut compressed2 = Vec::new();
    let mut encoder2 =
        zstd::stream::write::Encoder::new(&mut compressed2, 0).map_err(|e| e.to_string())?;
    encoder2
        .write_all(verified_files_concat)
        .map_err(|e| e.to_string())?;
    encoder2
        .write_all(passphrase.as_bytes())
        .map_err(|e| e.to_string())?;
    encoder2.finish().map_err(|e| e.to_string())?;

    let size2 = compressed2.len() as f64;

    let uncompressed_len = passphrase.len() as f64;
    let epsilon = 1e-6;
    let score = (size2 - size1) / (uncompressed_len + epsilon);
    Ok(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_passphrase_score() {
        let verified_files_concat = b"some repeated text some repeated text some repeated text";
        let passphrase = "my secret passphrase";
        let score = calculate_passphrase_score(verified_files_concat, passphrase).unwrap();
        assert!(score > 0.0);
    }
}
