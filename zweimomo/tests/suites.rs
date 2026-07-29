//! Zweimomo's einmo-driven suites (EIMP-2 §8; ported from `foolish-rust`'s
//! `zweimomo/tests/suites.rs`, JavaScript-only slice).
//!
//! The JavaScript suite is organized into progressive-difficulty tiers —
//! `day.1/`, `week.2/`, `month.2/`, `years.later/` — each its own
//! independently-gated [`einmo::EinmoSuite`] with its own `input/`/
//! `output/`/`checked/` tree and its own `README.*.md` (see each tier
//! directory). Only tiers with content are exercised here; a tier directory
//! that exists but has no `input/` files yet is skipped, not failed — new
//! tiers get real content over time (see the repo's todo list).
//!
//! For each populated tier: evaluate every input, and assert each output
//! was written and re-verified. The `output==checked` correspondence gate
//! is enforced by the `einmo` CLI (`einmo promote output to checked …`)
//! after a human reviews the diffs; this test exercises generation +
//! self-verification (the dog-food of the runner).

use std::path::{Path, PathBuf};

use einmo::{EinmoFile, EinmoSuite, Evaluator, TestConfig, ValidationLevel};
use zweimomo::BoaEvaluator;

/// The tiers, oldest (easiest) first. Directory name doubles as the
/// suite name suffix.
const TIERS: &[&str] = &["day.1", "week.2", "month.2", "years.later"];

/// The absolute path to a tier's work directory under `suites/javascript/`.
fn tier_dir(tier: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("suites")
        .join("javascript")
        .join(tier)
}

#[test]
fn javascript_tiers_generate_and_verify() {
    let mut exercised = 0;
    for &tier in TIERS {
        let dir = tier_dir(tier);
        if !dir.join("input").is_dir() {
            // Tier scaffolded (README only) but not yet populated — skip,
            // don't fail. Content lands incrementally over time.
            continue;
        }
        exercised += 1;
        run_tier(tier, &dir);
    }
    assert!(
        exercised > 0,
        "at least one JavaScript tier must have content (day.1 at minimum)"
    );
}

/// Evaluate + verify one tier's suite.
///
/// Checked level: each tier asserts a reviewed baseline (output <->
/// checked), and makes no claim about verified/ (FOOP-64 §"The escalating
/// validation levels"). einmo has no default level; the suite states it.
fn run_tier(tier: &str, dir: &Path) {
    let config = TestConfig::new(dir, ValidationLevel::Checked)
        .with_suite_name(format!("zweimomo/suites/javascript/{tier}"));

    let suite = EinmoSuite::new(config);
    let results = suite
        .evaluate_all(&BoaEvaluator)
        .unwrap_or_else(|e| panic!("{tier}: evaluate_all should not fail at the fs level: {e}"));

    assert!(
        !results.files.is_empty(),
        "{tier}: suite must discover at least one input"
    );
    for file in &results.files {
        assert!(
            file.written_and_verified,
            "{tier}: {} was not written+verified ({:?})",
            file.rel_path.display(),
            file.detail
        );
    }
}

/// Crash-crumb defense must survive a stack overflow in the evaluator.
///
/// This re-spawns the test binary as a child with `EINMO_ZWEIMOMO_CRASH_CHILD`,
/// which drives `EinmoSuite::evaluate` with a `StackOverflowEvaluator`
/// (infinite recursion) and crashes mid-evaluation. The parent then asserts
/// the crash-crumb (a signed `.einmo` with `TEST IN PROGRESS` status)
/// survived the crash and its stamp chain validates.
#[test]
fn crash_crumb_survives_stack_overflow() {
    use std::path::Path;

    struct StackOverflowEvaluator;
    impl Evaluator for StackOverflowEvaluator {
        fn evaluate(&self, _source: &str) -> std::result::Result<Vec<String>, String> {
            fn recurse(n: usize) -> usize {
                if n == 0 { 0 } else { recurse(n - 1) + 1 }
            }
            recurse(usize::MAX);
            Ok(vec!["unreachable".into()])
        }
    }

    if std::env::var("EINMO_ZWEIMOMO_CRASH_CHILD").is_ok() {
        let dir = std::env::var("EINMO_CRASH_TEST_DIR").unwrap();
        let config = TestConfig::new(&dir, ValidationLevel::Output);
        let suite = EinmoSuite::new(config);
        let input_dir = Path::new(&dir).join("input");
        std::fs::create_dir_all(&input_dir).unwrap();
        std::fs::write(input_dir.join("overflow.js"), "trigger").unwrap();
        let _ = suite.evaluate(Path::new("overflow.js"), &StackOverflowEvaluator);
        return;
    }

    let exe = std::env::current_exe().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(&exe)
        .arg("crash_crumb_survives_stack_overflow")
        .env("EINMO_ZWEIMOMO_CRASH_CHILD", "1")
        .env("EINMO_CRASH_TEST_DIR", tmp.path())
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "child should have crashed, got status: {:?}",
        output.status
    );

    let crumb_path = tmp.path().join("output").join("overflow.js.einmo");
    assert!(
        crumb_path.exists(),
        "crash-crumb should survive stack overflow"
    );

    let file =
        EinmoFile::from_file(&crumb_path).expect("crash-crumb must be a valid signed .einmo");
    assert!(file.metadata().status_detail.contains("TEST IN PROGRESS"));
    assert!(file.chain_valid(), "crash-crumb stamp chain must be valid");
}
