use std::io::Write;

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

    if size2 == 0.0 {
        return Ok(0.0);
    }

    let score = (size2 - size1) / size2;
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
