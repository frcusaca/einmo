//! Zweimomo's einmo-driven suites (EIMP-2 §8; ported from `foolish-rust`'s
//! `zweimomo/tests/suites.rs`, JavaScript-only slice).
//!
//! The JavaScript suite is organized into progressive-difficulty tiers —
//! `day.1/`, `week.2/`, `month.2/`, `years.later/` — each its own
//! independently-gated [`einmo::EinmoTestRunner`] with its own `input/`/
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

use einmo::{EinmoFile, EinmoTestRunner, Evaluator, TestConfig, ValidationLevel};
use zweimomo::{BoaEvaluator, Pyo3Evaluator};

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

/// The absolute path to the Python suite's work directory.
fn python_suite_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("suites")
        .join("python")
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

#[test]
fn python_suite_generates_and_verifies() {
    let dir = python_suite_dir();
    if !dir.join("input").is_dir() {
        // Python suite not yet populated — skip, don't fail.
        return;
    }
    run_suite("python", &dir, &Pyo3Evaluator);
}

/// Evaluate + verify one tier's suite.
///
/// Checked level: each tier asserts a reviewed baseline (output <->
/// checked), and makes no claim about verified/ (FOOP-64 §"The escalating
/// validation levels"). einmo has no default level; the suite states it.
fn run_tier(tier: &str, dir: &Path) {
    let config = TestConfig::new(dir, ValidationLevel::Checked)
        .with_suite_name(format!("zweimomo/suites/javascript/{tier}"));

    let suite = EinmoTestRunner::new(config);
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

/// Evaluate + verify a language suite.
fn run_suite(lang: &str, dir: &Path, evaluator: &dyn Evaluator) {
    let config = TestConfig::new(dir, ValidationLevel::Checked)
        .with_suite_name(format!("zweimomo/suites/{lang}"));

    let suite = EinmoTestRunner::new(config);
    let results = suite
        .evaluate_all(evaluator)
        .unwrap_or_else(|e| panic!("{lang}: evaluate_all should not fail at the fs level: {e}"));

    assert!(
        !results.files.is_empty(),
        "{lang}: suite must discover at least one input"
    );
    for file in &results.files {
        assert!(
            file.written_and_verified,
            "{lang}: {} was not written+verified ({:?})",
            file.rel_path.display(),
            file.detail
        );
    }
}

/// Crash-crumb defense must survive a stack overflow in the evaluator.
///
/// This re-spawns the test binary as a child with `EINMO_ZWEIMOMO_CRASH_CHILD`,
/// which drives `EinmoTestRunner::evaluate` with a `StackOverflowEvaluator`
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
        let suite = EinmoTestRunner::new(config);
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

    // EIMP-01 §S.2 moved crumb creation into `generated/`, so a crash leaves
    // no committed stage dirty. The guarantee under test is unchanged and the
    // child still genuinely overflows its stack — only the path moved.
    let crumb_path = tmp.path().join("generated").join("overflow.js.einmo");
    assert!(
        crumb_path.exists(),
        "crash-crumb should survive stack overflow"
    );
    assert!(
        !tmp.path().join("output").join("overflow.js.einmo").exists(),
        "a crash must leave no trace in the committed output/ stage"
    );

    let file =
        EinmoFile::from_file(&crumb_path).expect("crash-crumb must be a valid signed .einmo");
    assert!(file.metadata().status_detail.contains("TEST IN PROGRESS"));
    assert!(file.chain_valid(), "crash-crumb stamp chain must be valid");
}

/// Recursively copy `src` into `dst` (`dst` must already exist). Used to give
/// each comprehensive-test run its own scratch copy of `day.1` — the real
/// suite fixture under `suites/javascript/` is never mutated by tests.
fn copy_dir_recursive(src: &Path, dst: &Path) {
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let ty = entry.file_type().unwrap();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            std::fs::create_dir_all(&dst_path).unwrap();
            copy_dir_recursive(&entry.path(), &dst_path);
        } else if ty.is_file() {
            std::fs::copy(entry.path(), &dst_path).unwrap();
        }
    }
}

/// EIMP-01 comprehensive test: generate → inspect → promote, driven against a
/// scratch copy of `day.1`'s real, already-signed `output/` baseline with the
/// real `BoaEvaluator` — not the synthetic `Echo` evaluator
/// `einmo_suite.rs`'s unit tests use.
///
/// **Inherited from EIMP-3**, whose comprehensive test stood here and covered
/// the content/key decision table: a no-op rerun, a second-signer co-sign, a
/// drifted case that failed and left `output/` untouched, `regenerate_output`
/// replacing it, and a clean rerun. EIMP-01 §S.5 removed drift and the
/// `regenerate-output` verb, so those steps could not survive as written.
///
/// The **requirement** they encoded is kept: *a changed evaluator must not
/// silently redefine the committed baseline*. It is enforced differently now —
/// generation is indifferent to `output/` and simply never writes it, and
/// accepting a new baseline is an explicit, signing promotion. Steps 4–6 below
/// are that requirement, re-expressed. The test was rewritten rather than
/// deleted, deliberately.
#[test]
fn eimp01_generate_promote_comprehensive() {
    let tmp = tempfile::tempdir().unwrap();
    let scratch = tmp.path().join("day.1");
    std::fs::create_dir_all(&scratch).unwrap();
    copy_dir_recursive(&tier_dir("day.1"), &scratch);

    let config = TestConfig::new(&scratch, ValidationLevel::Output);
    let suite = EinmoTestRunner::new(config);
    let generated = scratch
        .join("generated")
        .join("integer_arithmetic.js.einmo");
    let committed = scratch.join("output").join("integer_arithmetic.js.einmo");
    let baseline_bytes = std::fs::read(&committed).unwrap();

    // ---- 1. Generation writes `generated/` and never `output/`.
    let first = suite
        .evaluate(Path::new("integer_arithmetic.js"), &BoaEvaluator)
        .unwrap();
    assert!(first.written_and_verified);
    assert!(generated.is_file(), "generation must write generated/");
    assert_eq!(
        std::fs::read(&committed).unwrap(),
        baseline_bytes,
        "generation must leave the committed output/ baseline byte-untouched"
    );

    // ---- 2. Re-generating unchanged content under the same signer is a
    // true no-op — byte-for-byte, not merely "same sections".
    let bytes_after_first = std::fs::read(&generated).unwrap();
    let second = suite
        .evaluate(Path::new("integer_arithmetic.js"), &BoaEvaluator)
        .unwrap();
    assert!(second.written_and_verified);
    assert_eq!(
        std::fs::read(&generated).unwrap(),
        bytes_after_first,
        "a true no-op must not touch the file at all"
    );

    // ---- 3. A different signer writes their OWN stamp; the runner does not
    // accumulate signers in `generated/` (EIMP-01 §S.5 — co-signing is
    // `promote`'s job, against `output/`).
    std::fs::write(
        scratch.join("einmo.toml"),
        "[signing]\ngenerated = \"zweimomo second signer\"\nchecked = \"We unanimously, unequivocally, categorically and definitively approve these test results !\"\n",
    )
    .unwrap();
    let second_suite = EinmoTestRunner::new(TestConfig::new(&scratch, ValidationLevel::Output));
    assert!(
        second_suite
            .evaluate(Path::new("integer_arithmetic.js"), &BoaEvaluator)
            .unwrap()
            .written_and_verified
    );
    let file = EinmoFile::from_file(&generated).unwrap();
    assert_eq!(file.section("OUTPUT").unwrap().body(), "9");
    assert_eq!(
        file.stamps()
            .entries()
            .iter()
            .filter(|s| s.key() == "stage:generated")
            .count(),
        1,
        "the runner must not accumulate stage:generated stamps"
    );

    // ---- 4. A CHANGED evaluator result is not a generation failure, and
    // still does not touch `output/`. This is EIMP-3's "drift" case,
    // re-expressed: what used to fail here now simply generates, and the
    // divergence is the Output gate's business.
    let nb_generated = scratch.join("generated").join("name_binding.js.einmo");
    let nb_committed = scratch.join("output").join("name_binding.js.einmo");
    let nb_baseline_bytes = std::fs::read(&nb_committed).unwrap();
    std::fs::write(
        scratch.join("input").join("name_binding.js"),
        "(() => { let x = 42; let y = x + 9; return y; })()",
    )
    .unwrap();
    let changed = suite
        .evaluate(Path::new("name_binding.js"), &BoaEvaluator)
        .unwrap();
    assert!(
        changed.written_and_verified,
        "differing from the baseline is not a generation failure (EIMP-01 §S.2)"
    );
    assert_eq!(
        EinmoFile::from_file(&nb_generated)
            .unwrap()
            .section("OUTPUT")
            .unwrap()
            .body(),
        "51",
        "generated/ must hold the NEW result"
    );
    assert_eq!(
        std::fs::read(&nb_committed).unwrap(),
        nb_baseline_bytes,
        "the committed baseline must still be untouched — accepting it is a \
         separate, deliberate act"
    );

    // ---- 5. Accepting the new result is an explicit, SIGNING promotion.
    // This is what replaced `einmo regenerate-output`.
    einmo::EinmoSuite::scan(
        einmo::EinmoDirectory::new(TestConfig::new(&scratch, ValidationLevel::Output)),
        None,
    )
    .unwrap()
    .promote(
        einmo::Stage::Generated,
        einmo::Stage::Output,
        &einmo::KeySource::from_passphrase(""),
        None,
        None,
    )
    .unwrap();

    let promoted = EinmoFile::from_file(&nb_committed).unwrap();
    assert_eq!(
        promoted.section("OUTPUT").unwrap().body(),
        "51",
        "the promoted baseline must hold the accepted content"
    );
    assert!(
        promoted
            .stamps()
            .entries()
            .iter()
            .any(|s| s.key() == "stage:output"),
        "promotion must sign: a stage:output stamp is what makes it a baseline"
    );
    assert!(
        promoted
            .stamps()
            .entries()
            .iter()
            .any(|s| s.key() == "stage:generated"),
        "promotion appends; the generation stamp survives underneath it"
    );

    // ---- 6. A subsequent generation of the accepted case is a clean no-op.
    let nb_bytes_after = std::fs::read(&nb_generated).unwrap();
    assert!(
        suite
            .evaluate(Path::new("name_binding.js"), &BoaEvaluator)
            .unwrap()
            .written_and_verified
    );
    assert_eq!(std::fs::read(&nb_generated).unwrap(), nb_bytes_after);
}
