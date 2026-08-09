//! Pure-Rust [`einmo::Evaluator`] implementations (EIMP-2 §8; ported from
//! `foolish-rust`'s `zweimomo/src/evaluators.rs`).

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

/// Evaluates Python via the system Python interpreter using pyo3.
///
/// **Serialization choice:** the idiomatic Python rendering of a single
/// expression's value is `str(value)`; the adapter evaluates the source and
/// returns one OUTPUT chunk (`str(result)`).
#[derive(Debug, Default, Clone, Copy)]
pub struct Pyo3Evaluator;

impl Evaluator for Pyo3Evaluator {
    fn evaluate(&self, source: &str) -> Result<Vec<String>, String> {
        use pyo3::prelude::*;
        use pyo3::types::PyString;
        use std::ffi::CString;

        let c_source =
            CString::new(source).map_err(|_| "python source contains null byte".to_string())?;

        Python::attach(|py| {
            let result = py
                .eval(&c_source, None, None)
                .map_err(|err| format!("python error: {err}"))?;
            let text_obj = result
                .str()
                .map_err(|err| format!("python str() error: {err}"))?;
            let py_str = text_obj
                .cast::<PyString>()
                .map_err(|_| "python str() did not return a string".to_string())?;
            let text: &str = py_str
                .to_str()
                .map_err(|err| format!("python string encoding error: {err}"))?;
            Ok(vec![text.to_owned()])
        })
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

    #[test]
    fn python_arithmetic_smoke() {
        let out = Pyo3Evaluator.evaluate("2 + 3 * 4 - 5").unwrap();
        assert_eq!(out, vec!["9".to_string()]);
    }

    #[test]
    fn python_integer_division() {
        let out = Pyo3Evaluator.evaluate("7 // 2").unwrap();
        assert_eq!(out, vec!["3".to_string()]);
    }

    #[test]
    fn python_error_is_err_not_panic() {
        let result = Pyo3Evaluator.evaluate("1 / 0");
        assert!(
            result.is_err(),
            "division by zero must be an Err, got {result:?}"
        );
        assert!(result.unwrap_err().to_lowercase().contains("division"));
    }

    #[test]
    fn python_syntax_error_is_err() {
        assert!(Pyo3Evaluator.evaluate("def (").is_err());
    }
}
