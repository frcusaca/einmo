//! Zweimomo — einmo's companion test crate (EIMP-2 §8; ported from
//! `foolish-rust`'s three-language `zweimomo`, FOOP-92 §Use Case D).
//!
//! Embeds **pure-Rust** interpreters — JavaScript (`boa_engine`) and
//! Python (system Python via `pyo3`) — as [`einmo::Evaluator`] impls,
//! exercising einmo's signed-snapshot pipeline against real, previously-
//! reviewed test fixtures.

pub mod evaluators;

pub use evaluators::{BoaEvaluator, Pyo3Evaluator};
