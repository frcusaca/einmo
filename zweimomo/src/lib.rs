//! Zweimomo (JavaScript-only slice) — einmo's real-fixture test crate
//! (EIMP-2 §8; ported from `foolish-rust`'s three-language `zweimomo`,
//! FOOP-92 §Use Case D).
//!
//! Embeds a **pure-Rust** JavaScript interpreter (`boa_engine`) as an
//! [`einmo::Evaluator`] impl, exercising einmo's signed-snapshot pipeline
//! against real, previously-reviewed test fixtures
//! (`suites/javascript/`) — the test data `einmo-review-server`'s
//! integration tests are pointed at.

pub mod evaluators;

pub use evaluators::BoaEvaluator;
