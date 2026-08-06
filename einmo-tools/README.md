# einmo-tools

This crate provides supplementary tools for `einmo`, specifically focusing on passphrase effectiveness measurement.

## Passphrase Effectiveness

The `calculate_passphrase_score` function calculates the effectiveness of a passphrase based on how much it compresses relative to the existing verified files corpus.

It works by:
1. Taking the concatenation of all verified files and compressing it using `zstd` (size1).
2. Taking the concatenation of all verified files, appending the passphrase, and compressing it again (size2).
3. The percentage increase in compressed size relative to the raw passphrase is the score: `(size2 - size1) / (passphrase_length + 1e-6)`.

A higher score indicates a passphrase that adds more entropy/uniqueness relative to the existing test corpus. A score greater than 0 is required for a passphrase to be accepted during promotion to the verified stage.
