//! The JavaScript [`einmo::Evaluator`] implementation (EIMP-2 §8; ported
//! from `foolish-rust`'s `zweimomo/src/evaluators.rs`, Boa slice only).

use einmo::Evaluator;

/// Evaluates JavaScript via `boa_engine` 0.21.1 in a fresh `Context` — no
/// `fs`/network/Node APIs.
///
/// **Serialization choice:** the idiomatic JS rendering of a value is
/// `String(value)`; the adapter evaluates the source and returns one OUTPUT
/// chunk (`value.to_string()`).
#[derive(Debug, Default, Clone, Copy)]
pub struct BoaEvaluator;

impl Evaluator for BoaEvaluator {
    fn evaluate(&self, source: &str) -> Result<Vec<String>, String> {
        use boa_engine::{Context, Source};
        // A fresh Context per call (Context is !Send).
        let mut context = Context::default();
        let value = context
            .eval(Source::from_bytes(source))
            .map_err(|err| err.to_string())?;
        let text = value
            .to_string(&mut context)
            .map_err(|err| err.to_string())?
            .to_std_string_escaped();
        Ok(vec![text])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_arithmetic_smoke() {
        let out = BoaEvaluator.evaluate("2 + 3 * 4 - 5").unwrap();
        assert_eq!(out, vec!["9".to_string()]);
    }

    #[test]
    fn js_floor_division_matches_integer() {
        let out = BoaEvaluator.evaluate("Math.floor(7 / 2)").unwrap();
        assert_eq!(out, vec!["3".to_string()]);
    }

    #[test]
    fn js_divide_by_zero_is_infinity_value() {
        // JS `10/0` is `Infinity` — a *value*, not an error.
        let out = BoaEvaluator.evaluate("10 / 0").unwrap();
        assert_eq!(out, vec!["Infinity".to_string()]);
    }

    #[test]
    fn js_throw_is_err() {
        assert!(BoaEvaluator.evaluate("throw new Error('boom')").is_err());
    }
}
