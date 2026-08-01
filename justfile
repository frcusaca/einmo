# justfile
set shell := ["bash", "-uc"]
set positional-arguments

default: fmt lint test

test *args:
    cargo nextest run --workspace "$@"

ci-test *args:
    cargo nextest run --profile ci "$@"

lint:
    cargo clippy --workspace --all-targets -- -D warnings

fmt:
    cargo fmt --all

pr:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo nextest run --workspace
    cargo mutants --in-diff <(git diff main...HEAD) --test-tool nextest -j `nproc`

mutants *args:
    cargo mutants --test-tool nextest -j `nproc` "$@"

coverage:
    cargo llvm-cov --workspace --html

setup:
    cargo install --locked --version 25.0.0 cargo-mutants
    cargo install --locked --version 0.6.16 cargo-llvm-cov
    cargo install --locked --version 0.9.88 cargo-nextest
