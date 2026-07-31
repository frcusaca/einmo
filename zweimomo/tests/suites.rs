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

/// EIMP-3 comprehensive test: the full content/key decision table, driven
/// against a scratch copy of `day.1`'s real, already-signed `output/`
/// baseline with the real `BoaEvaluator` — not the synthetic `Echo`
/// evaluator `einmo_suite.rs`'s unit tests use. Exercises, in one pass:
/// a no-op rerun, a second-signer co-sign, a drifted case that fails and
/// leaves `output/` untouched, `regenerate_output` replacing it, and a
/// subsequent clean rerun.
#[test]
fn eimp3_output_drift_comprehensive() {
    let tmp = tempfile::tempdir().unwrap();
    let scratch = tmp.path().join("day.1");
    std::fs::create_dir_all(&scratch).unwrap();
    copy_dir_recursive(&tier_dir("day.1"), &scratch);

    // ---- 1. No-op rerun: unchanged content, the same (computer-key)
    // signer that originally produced this fixture. `output/` must be
    // byte-for-byte untouched.
    let config = TestConfig::new(&scratch, ValidationLevel::Output);
    let suite = EinmoSuite::new(config);
    let out_path = scratch.join("output").join("integer_arithmetic.js.einmo");
    let bytes_before = std::fs::read(&out_path).unwrap();
    let rerun = suite
        .evaluate(Path::new("integer_arithmetic.js"), &BoaEvaluator)
        .unwrap();
    assert!(!rerun.drifted, "unchanged content must not be drift");
    assert!(rerun.written_and_verified);
    assert_eq!(
        std::fs::read(&out_path).unwrap(),
        bytes_before,
        "a true no-op must not touch the file at all"
    );

    // ---- 2. Second-signer co-sign: same content, a different `output`
    // passphrase. Stamps accumulate; content is untouched.
    std::fs::write(
        scratch.join("einmo.toml"),
        "[signing]\noutput = \"zweimomo second signer\"\nchecked = \"We unanimously, unequivocally, categorically and definitively approve these test results !\"\n",
    )
    .unwrap();
    let second_config = TestConfig::new(&scratch, ValidationLevel::Output);
    let second_suite = EinmoSuite::new(second_config);
    let co_signed = second_suite
        .evaluate(Path::new("integer_arithmetic.js"), &BoaEvaluator)
        .unwrap();
    assert!(!co_signed.drifted);
    assert!(co_signed.written_and_verified);
    let file = EinmoFile::from_file(&out_path).unwrap();
    assert_eq!(file.section("OUTPUT").unwrap().body(), "9");
    let stage_output_stamps = file
        .stamps()
        .entries()
        .iter()
        .filter(|s| s.key() == "stage:output")
        .count();
    assert_eq!(
        stage_output_stamps, 2,
        "both the original and the second signer's stage:output stamps must be present"
    );

    // ---- 3. Drift: change what `name_binding.js` evaluates to. The
    // normal (non-forcing) run must fail this case and leave `output/`
    // untouched.
    let nb_out_path = scratch.join("output").join("name_binding.js.einmo");
    let nb_bytes_before = std::fs::read(&nb_out_path).unwrap();
    std::fs::write(
        scratch.join("input").join("name_binding.js"),
        "(() => { let x = 42; let y = x + 9; return y; })()",
    )
    .unwrap();
    let drifted = suite
        .evaluate(Path::new("name_binding.js"), &BoaEvaluator)
        .unwrap();
    assert!(drifted.drifted, "changed evaluator output must be drift");
    assert!(!drifted.written_and_verified);
    assert_eq!(
        std::fs::read(&nb_out_path).unwrap(),
        nb_bytes_before,
        "output/ must be untouched by a drifted (failing) run"
    );

    // ---- 4. `regenerate_output`: deliberately accept the new content.
    let regenerated = suite
        .regenerate_output(Path::new("name_binding.js"), &BoaEvaluator)
        .unwrap();
    assert!(!regenerated.drifted);
    assert!(regenerated.written_and_verified);
    let nb_file = EinmoFile::from_file(&nb_out_path).unwrap();
    assert_eq!(nb_file.section("OUTPUT").unwrap().body(), "51");

    // ---- 5. A subsequent normal run of the regenerated case is clean:
    // no drift, no rewrite.
    let nb_bytes_after_regen = std::fs::read(&nb_out_path).unwrap();
    let clean = suite
        .evaluate(Path::new("name_binding.js"), &BoaEvaluator)
        .unwrap();
    assert!(!clean.drifted);
    assert!(clean.written_and_verified);
    assert_eq!(std::fs::read(&nb_out_path).unwrap(), nb_bytes_after_regen);
}
