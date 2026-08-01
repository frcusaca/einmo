#!/usr/bin/env bash
set -euo pipefail
cargo install cargo-mutants
cargo install cargo-llvm-cov
cargo install --locked cargo-nextest
cargo install just
