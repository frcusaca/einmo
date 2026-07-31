//! The single `einmo` CLI app (FOOP-92 §B.6, Phase 10).
//!
//! Every subcommand calls the library; every read verifies-on-inspect. Stage
//! pairs use the `to` keyword (`output to checked`) or a glued `:`/`..`
//! separator (`output:checked`, `output..checked`); stage names are
//! validated `[A-Za-z0-9_-]+`.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use crate::config::{KeyCascadeInputs, KeySource, MatchSections, TestConfig, resolve_stage_key};
use crate::einmo_suite::{FailurePolicy, ValidationLevel};
use crate::error::{EinmoError, Result};
use crate::format::EinmoFile;
use crate::stage::Stage;

/// The `einmo` command-line interface.
#[derive(Parser, Debug)]
#[command(
    name = "einmo",
    version,
    about = "Signed directory-based snapshot testing"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Promote files between stages (appends the destination stage's stamp).
    Promote(PromoteArgs),
    /// Move files from a stage into flagged/ (advisory line, no stamp).
    Flag(FlagArgs),
    /// Retract (demote) artifacts from a stage; cascades checked→verified.
    Retract(RetractArgs),
    /// Compare two stages over the mirrored tree.
    Compare(CompareArgs),
    /// Verify signatures across a stage (or all stages).
    Verify(VerifyArgs),
    /// Verify signatures on a path (legacy-compatible subcommand name).
    VerifySignatures(VerifyArgs),
    /// Report files carrying a signer whose pubkey starts with a prefix.
    ConfirmSignatures(ConfirmArgs),
    /// Show an envelope's summary and stamp chain.
    Show(ShowArgs),
    /// List the suite's tests and which stages hold each one.
    List(ListArgs),
    /// Print an envelope's signed body sections (verify-on-inspect first).
    Body(BodyArgs),
    /// Compute the SHA-256 of this binary (self-attestation).
    SelfCheck(SelfCheckArgs),
    /// Evaluate inputs and write signed output files.
    Evaluate(EvaluateArgs),
    /// Re-evaluate inputs and deliberately REPLACE any drifted `output/`
    /// baseline with the freshly evaluated content (EIMP-3). Every other
    /// case (no-op / co-sign / fresh-if-absent) behaves exactly like
    /// `evaluate`.
    RegenerateOutput(EvaluateArgs),
}

#[derive(Args, Debug)]
struct PromoteArgs {
    /// The stage pair, in any of:
    ///   `<from> to <to>`  (spaced — preferred; needs no quoting)
    ///   `<from>:<to>` · `<from>..<to>`  (glued)
    /// then the work directory, then any specific `.einmo` files.
    ///
    /// Parsed positionally by [`split_promote_args`].
    #[arg(required = true, num_args = 1..)]
    args: Vec<String>,
    /// Restrict to inputs matching this glob (`*` wildcard).
    #[arg(long)]
    filter: Option<String>,
    /// Explicit passphrase (tier 1). Empty string = the computer key.
    #[arg(long)]
    passphrase: Option<String>,
    /// Read one passphrase line from stdin (tier 2).
    #[arg(long)]
    stdin_passphrase: bool,
    /// Force the interactive prompt (skips tiers 1–4).
    #[arg(long)]
    interactive: bool,
    /// Override the directory-walk depth limit (tier 1).
    #[arg(long)]
    walk_depth_limit: Option<usize>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct RetractArgs {
    /// The suite work directory.
    work_dir: PathBuf,
    /// The stage to retract from (`checked` or `verified`).
    stage: String,
    /// Specific `.einmo` files to retract. Use `-` to read paths from stdin.
    #[arg(num_args = 0.., trailing_var_arg = true)]
    files: Vec<PathBuf>,
    /// Restrict to inputs matching this glob.
    #[arg(long)]
    filter: Option<String>,
    /// Override the directory-walk depth limit.
    #[arg(long)]
    walk_depth_limit: Option<usize>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct FlagArgs {
    /// The suite work directory.
    work_dir: PathBuf,
    /// The stage to flag from.
    stage: String,
    /// Specific `.einmo` files to act on. Use `-` to read paths from stdin.
    #[arg(num_args = 0.., trailing_var_arg = true)]
    files: Vec<PathBuf>,
    /// Restrict to inputs matching this glob.
    #[arg(long)]
    filter: Option<String>,
    /// The advisory reason recorded in the flagged file.
    #[arg(long, default_value = "")]
    reason: String,
    /// Override the directory-walk depth limit (tier 1).
    #[arg(long)]
    walk_depth_limit: Option<usize>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct CompareArgs {
    /// Stage A.
    stage_a: String,
    /// Stage B.
    stage_b: String,
    /// The suite work directory.
    work_dir: PathBuf,
    /// Specific `.einmo` files to act on. Use `-` to read paths from stdin.
    #[arg(num_args = 0.., trailing_var_arg = true)]
    files: Vec<PathBuf>,
    /// Require COMMENTS to match too.
    #[arg(long)]
    require_comments_match: bool,
    /// Exit non-zero if any file differs / is one-sided / is tampered.
    #[arg(long)]
    require_match: bool,
    /// Report the deepest differing descendants (root-cause diagnostic).
    #[arg(long)]
    root_cause: bool,
    /// Override the directory-walk depth limit (tier 1).
    #[arg(long)]
    walk_depth_limit: Option<usize>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct VerifyArgs {
    /// The suite work directory.
    work_dir: PathBuf,
    /// The escalating validation level to judge the suite at
    /// (`output` | `checked` | `verified`). The CLI defaults to `checked`; the
    /// library API has no default.
    #[arg(long, default_value = "checked")]
    level: String,
    /// Stop at the first failure instead of gathering every problem.
    /// The default is fail-at-end: run everything, report it all.
    #[arg(long, conflicts_with = "fail_at_end")]
    fail_fast: bool,
    /// Run every check and report all problems together (the default).
    #[arg(long)]
    fail_at_end: bool,
    /// Restrict to one stage.
    #[arg(long)]
    stage: Option<String>,
    /// Verify all stages (the default).
    #[arg(long)]
    all: bool,
    /// Specific `.einmo` files to act on. Use `-` to read paths from stdin.
    #[arg(num_args = 0.., trailing_var_arg = true)]
    files: Vec<PathBuf>,
    /// Maximum recursion depth for directory walks.
    #[arg(long)]
    walk_depth_limit: Option<usize>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
    /// Downgrade a flagged artifact from failing the gate to advisory-only
    /// (`EIMP-1` §S.3). A flagged artifact still ALWAYS produces stderr
    /// output announcing its presence — this flag changes the exit code,
    /// never the visibility.
    #[arg(long)]
    flag_is_not_failure: bool,
}

#[derive(Args, Debug)]
struct ConfirmArgs {
    /// A directory (or file) of `.einmo` files.
    path: PathBuf,
    /// The pubkey hex prefix to match. Supply this OR `--from-passphrase`,
    /// never both, never neither.
    pubkey_prefix: Option<String>,
    /// Derive the pubkey prefix from a passphrase instead of typing the hex:
    /// prompts for the passphrase (twice, confirmed) and matches its public
    /// key. Answers "was this signed by MY passphrase?" without exposing the
    /// key. Mutually exclusive with an explicit `<pubkey-prefix>`.
    #[arg(long)]
    from_passphrase: bool,
    /// Exit non-zero if any file lacks a matching signer.
    #[arg(long)]
    require_all: bool,
    /// Override the directory-walk depth limit (tier 1).
    #[arg(long)]
    walk_depth_limit: Option<usize>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct ShowArgs {
    /// The `.einmo` file to show.
    file: PathBuf,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct ListArgs {
    /// The suite work directory.
    work_dir: PathBuf,
    /// Only tests whose mirror-relative path contains this substring.
    #[arg(long)]
    filter: Option<String>,
    /// Only tests whose stage bodies are not all identical (ignores stamps and
    /// metadata, exactly as `compare` does). Absent artifacts count as differing.
    #[arg(long)]
    differing: bool,
    /// Emit machine-readable JSON (one object per line).
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct BodyArgs {
    /// The `.einmo` file whose body to print.
    file: PathBuf,
    /// Print only this section (e.g. `INPUT`, `OUTPUT`, `COMMENTS`).
    #[arg(long)]
    section: Option<String>,
    /// Do not print `=== NAME ===` headers between sections.
    #[arg(long)]
    bare: bool,
}

#[derive(Args, Debug)]
struct SelfCheckArgs {
    /// Exit non-zero if the computed hash does not match this value.
    #[arg(long)]
    expected: Option<String>,
    /// Print only the hash.
    #[arg(long)]
    quiet: bool,
}

#[derive(Args, Debug)]
struct EvaluateArgs {
    /// The suite work directory.
    work_dir: PathBuf,
    /// The evaluator command. Receives source on stdin, outputs to stdout.
    #[arg(long)]
    command: String,
    /// Only evaluate inputs matching this substring.
    #[arg(long)]
    filter: Option<String>,
    /// Override the directory-walk depth limit.
    #[arg(long)]
    walk_depth_limit: Option<usize>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

/// The shared CLI entry point used by both the `einmo` and `cargo-einmo` bins.
///
/// Returns a process exit code; every error is reported to stderr.
#[must_use]
pub fn cli_main(args: Vec<OsString>) -> ExitCode {
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(e) => {
            // clap prints help/version to stdout with a success code.
            let _ = e.print();
            return if e.use_stderr() {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            };
        }
    };
    match dispatch(cli.command) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("einmo: {e}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(command: Command) -> Result<ExitCode> {
    match command {
        Command::Promote(a) => cmd_promote(a),
        Command::Flag(a) => cmd_flag(a),
        Command::Retract(a) => cmd_retract(a),
        Command::Compare(a) => cmd_compare(a),
        Command::Verify(a) | Command::VerifySignatures(a) => cmd_verify(a),
        Command::ConfirmSignatures(a) => cmd_confirm(a),
        Command::Show(a) => cmd_show(a),
        Command::List(a) => cmd_list(a),
        Command::Body(a) => cmd_body(a),
        Command::SelfCheck(a) => cmd_self_check(a),
        Command::Evaluate(a) => cmd_evaluate(a),
        Command::RegenerateOutput(a) => cmd_regenerate_output(a),
    }
}

/// Parse a glued `<from>:<to>` (or `<from>..<to>`) transition into a stage pair.
fn parse_transition(s: &str) -> Result<(Stage, Stage)> {
    // `->` is no longer accepted here: it collided with shell redirection (an
    // unquoted `checked->verified` had its `>` eaten by the shell, so users
    // who forgot to quote got a baffling error) and `to` is easier to type.
    // The spaced `<from> to <to>` form (handled in `split_promote_args`) is
    // preferred; `:` and `..` remain as quoting-free glued alternatives.
    let split = ["..", ":"].iter().find_map(|sep| s.split_once(sep));
    let (from, to) = split.ok_or_else(|| {
        EinmoError::Config(format!(
            "transition {s:?} must be `<from> to <to>` (or `<from>:<to>`)."
        ))
    })?;
    Ok((Stage::parse(from.trim())?, Stage::parse(to.trim())?))
}

/// Expand any `-` entries in `files` by pulling paths from `stdin_lines` (one
/// per line, blanks skipped). Non-`-` entries are kept verbatim. An empty
/// `files` yields an empty result.
fn resolve_files_from_iter(
    files: Vec<PathBuf>,
    stdin_lines: impl Iterator<Item = String>,
) -> Vec<PathBuf> {
    if files.is_empty() {
        return Vec::new();
    }
    if !files.iter().any(|p| p.to_string_lossy() == "-") {
        return files;
    }
    let stdin_paths: Vec<PathBuf> = stdin_lines
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect();
    let mut result = Vec::new();
    for f in files {
        if f.to_string_lossy() == "-" {
            result.extend(stdin_paths.clone());
        } else {
            result.push(f);
        }
    }
    result
}

/// Resolve the `files` positional list, reading stdin when `-` appears.
///
/// # Errors
///
/// Returns [`EinmoError::Io`] if stdin cannot be read.
fn resolve_files(files: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    if !files.iter().any(|p| p.to_string_lossy() == "-") {
        return Ok(files);
    }
    let mut input = String::new();
    std::io::stdin()
        .lock()
        .read_to_string(&mut input)
        .map_err(|e| EinmoError::io("<stdin>", e))?;
    Ok(resolve_files_from_iter(
        files,
        input.lines().map(String::from),
    ))
}

/// Convert a resolved `Vec<PathBuf>` into the `Option<&[PathBuf]>` the library
/// functions expect (`None` when empty so the walk/filter path is used).
fn files_ref(files: &[PathBuf]) -> Option<&[PathBuf]> {
    if files.is_empty() { None } else { Some(files) }
}

/// Split promote's positional arguments into `(from, to, work_dir, files)`.
///
/// Accepts two shapes:
///   * **spaced** — `<from> to <to> <work_dir> [files…]` (preferred; the `to`
///     keyword needs no quoting and cannot be mistaken for a shell redirect)
///   * **glued**  — `<from><sep><to> <work_dir> [files…]` where `<sep>` is
///     `:` or `..`
fn split_promote_args(raw: &[String]) -> Result<(Stage, Stage, PathBuf, Vec<PathBuf>)> {
    // Spaced form: `<from> to <to> …` — arg[1] is the literal `to`.
    if raw.len() >= 3 && raw[1].eq_ignore_ascii_case("to") {
        let from = Stage::parse(raw[0].trim())?;
        let to = Stage::parse(raw[2].trim())?;
        let work_dir = raw.get(3).cloned().ok_or_else(|| {
            EinmoError::Config("missing work directory after `<from> to <to>`".into())
        })?;
        let files = raw[4.min(raw.len())..].iter().map(PathBuf::from).collect();
        return Ok((from, to, PathBuf::from(work_dir), files));
    }
    // Glued form: `<from><sep><to> <work_dir> [files…]`.
    let (from, to) = parse_transition(&raw[0])?;
    let work_dir = raw
        .get(1)
        .cloned()
        .ok_or_else(|| EinmoError::Config("missing work directory".into()))?;
    let files = raw[2.min(raw.len())..].iter().map(PathBuf::from).collect();
    Ok((from, to, PathBuf::from(work_dir), files))
}

fn cmd_promote(args: PromoteArgs) -> Result<ExitCode> {
    let (from, to, work_dir, positional_files) = split_promote_args(&args.args)?;
    let mut config = TestConfig::new(&work_dir, ValidationLevel::Output);
    if let Some(limit) = args.walk_depth_limit {
        config = config.with_walk_depth_limit(limit);
    }
    let key = resolve_promotion_key(to, &args, &config)?;
    let files = resolve_files(positional_files)?;
    let report = crate::promote(
        &config,
        from,
        to,
        &key,
        args.filter.as_deref(),
        files_ref(&files),
    )?;

    // Warn on any non-human verified attestation.
    for promoted in &report.promoted {
        if promoted.non_human {
            eprintln!(
                "einmo: warning: {} verified under a well-known computer key (non-human attestation)",
                promoted.rel_path.display()
            );
        }
    }
    if args.json {
        println!(
            "{{\"promoted\":{},\"non_human\":{}}}",
            report.promoted.len(),
            report.promoted.iter().filter(|p| p.non_human).count()
        );
    } else {
        println!("promoted {} file(s) {from} to {to}", report.promoted.len());
    }
    Ok(ExitCode::SUCCESS)
}

/// Resolve the destination stage's key (only `* to verified` truly needs one;
/// the `output to checked`/config defaults resolve without a prompt).
fn resolve_promotion_key(to: Stage, args: &PromoteArgs, config: &TestConfig) -> Result<KeySource> {
    let stdin_line = if args.stdin_passphrase {
        Some(read_stdin_line()?)
    } else {
        None
    };
    let env_pass = std::env::var("EINMO_PASSPHRASE").ok();
    let inputs = KeyCascadeInputs {
        cli_passphrase: args.passphrase.as_deref(),
        stdin_passphrase: stdin_line.as_deref(),
        interactive: args.interactive,
        env_passphrase: env_pass.as_deref(),
    };
    resolve_stage_key(to, &inputs, config, prompt_tty)
}

fn cmd_flag(args: FlagArgs) -> Result<ExitCode> {
    let stage = Stage::parse(&args.stage)?;
    let mut config = TestConfig::new(&args.work_dir, ValidationLevel::Output);
    if let Some(limit) = args.walk_depth_limit {
        config = config.with_walk_depth_limit(limit);
    }
    let files = resolve_files(args.files)?;
    let report = crate::flag(
        &config,
        stage,
        args.filter.as_deref(),
        &args.reason,
        files_ref(&files),
    )?;
    if args.json {
        println!("{{\"flagged\":{}}}", report.flagged.len());
    } else {
        println!("flagged {} file(s) from {stage}", report.flagged.len());
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_retract(args: RetractArgs) -> Result<ExitCode> {
    let stage = Stage::parse(&args.stage)?;
    let mut config = TestConfig::new(&args.work_dir, ValidationLevel::Output);
    if let Some(limit) = args.walk_depth_limit {
        config = config.with_walk_depth_limit(limit);
    }
    let files = resolve_files(args.files)?;
    let report = crate::retract(&config, stage, args.filter.as_deref(), files_ref(&files))?;
    if args.json {
        println!("{{\"retracted\":{}}}", report.retracted.len());
    } else {
        println!("retracted {} artifact(s):", report.retracted.len());
        for (st, path) in &report.retracted {
            println!("  {st}/{}", path.display());
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_compare(args: CompareArgs) -> Result<ExitCode> {
    let a = Stage::parse(&args.stage_a)?;
    let b = Stage::parse(&args.stage_b)?;
    let mut config = TestConfig::new(&args.work_dir, ValidationLevel::Output);
    if let Some(limit) = args.walk_depth_limit {
        config = config.with_walk_depth_limit(limit);
    }
    let sections = if args.require_comments_match {
        MatchSections::InputOutputComments
    } else {
        MatchSections::InputOutput
    };
    let files = resolve_files(args.files)?;
    let result = crate::compare(&config, a, b, sections, files_ref(&files))?;

    if args.json {
        println!(
            "{{\"matching\":{},\"differing\":{},\"only_in_a\":{},\"only_in_b\":{},\"tampered\":{}}}",
            result.matching.len(),
            result.differing.len(),
            result.only_in_a.len(),
            result.only_in_b.len(),
            result.tampered.len()
        );
    } else {
        println!(
            "{a} vs {b}: {} matching, {} differing, {} only-in-{a}, {} only-in-{b}, {} tampered",
            result.matching.len(),
            result.differing.len(),
            result.only_in_a.len(),
            result.only_in_b.len(),
            result.tampered.len()
        );
        for entry in &result.differing {
            println!(
                "  differing {} [{}]",
                entry.rel_path.display(),
                entry.sections.join(", ")
            );
        }
        if args.root_cause {
            for root in crate::compare::root_causes(&result) {
                println!("  root-cause {}", root.display());
            }
        }
    }
    if args.require_match && !result.is_clean() {
        eprintln!("einmo: {a} does not match {b}.");
        eprintln!("  burden: the producer of the divergent output must repair or escalate.");
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_verify(args: VerifyArgs) -> Result<ExitCode> {
    let level = ValidationLevel::parse(&args.level)?;
    let mut config = TestConfig::new(&args.work_dir, level);
    if let Some(limit) = args.walk_depth_limit {
        config = config.with_walk_depth_limit(limit);
    }
    let stage = match &args.stage {
        Some(s) => Some(Stage::parse(s)?),
        None => None,
    };
    let files = resolve_files(args.files)?;
    let report = crate::verify(&config, stage, files_ref(&files))?;
    // Signature integrity is only half of "is this suite sound?" — the tree's
    // shape is the other half. Checked only for a whole-suite verify: a
    // file-scoped or single-stage run is not making a claim about the tree.
    let policy = if args.fail_fast {
        FailurePolicy::FailFast
    } else {
        FailurePolicy::FailAtEnd
    };
    let integrity = if files.is_empty() && stage.is_none() {
        crate::check_suite_integrity(&config, policy)?
    } else {
        crate::SuiteIntegrity::default()
    };
    // The goal-state flag check (EIMP-1 S.3), same whole-suite-only scope as
    // integrity: a file-scoped or single-stage verify isn't making a claim
    // about the suite as a whole.
    let flagged = if files.is_empty() && stage.is_none() {
        crate::count_flagged(&config)?
    } else {
        Vec::new()
    };
    if args.json {
        let violations: Vec<String> = integrity
            .problems
            .iter()
            .map(|p| {
                format!(
                    "{{\"level\":\"{}\",\"path\":\"{}\",\"problem\":\"{}\",\"remedy\":\"{}\"}}",
                    p.level(),
                    p.path()
                        .map_or_else(String::new, |x| x.display().to_string()),
                    p,
                    p.remedy()
                )
            })
            .collect();
        println!(
            "{{\"files\":{},\"failures\":{},\"integrity_violations\":[{}],\"flagged\":{}}}",
            report.files.len(),
            report.failures(),
            violations.join(","),
            flagged.len()
        );
    } else {
        println!(
            "verified {} file(s), {} failure(s)",
            report.files.len(),
            report.failures()
        );
        for f in report.files.iter().filter(|f| !f.ok) {
            println!(
                "  FAILED {} ({})",
                f.rel_path.display(),
                f.detail.as_deref().unwrap_or("")
            );
        }
        if !integrity.is_clean() {
            eprintln!("einmo: suite shape is unsound:");
            eprint!("{}", integrity.report());
        }
    }
    // A flag ALWAYS produces stderr output announcing its presence, in
    // EITHER json or text mode -- `--flag-is-not-failure` only changes the
    // exit code below, never whether this is printed. No silent config
    // (EIMP-1 S.3).
    if !flagged.is_empty() {
        eprintln!(
            "einmo: warning: {} flagged artifact(s) present:",
            flagged.len()
        );
        for f in &flagged {
            eprintln!("  {} ({} stage)", f.rel_path.display(), f.stage);
        }
    }
    Ok(
        if report.all_ok()
            && integrity.is_clean()
            && !flags_fail_the_gate(flagged.len(), args.flag_is_not_failure)
        {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        },
    )
}

/// Whether flagged artifacts should fail `einmo verify`'s gate (`EIMP-1`
/// §S.3): by default, any flag fails; `--flag-is-not-failure` downgrades
/// that to advisory-only. Kept as a small pure function, separate from
/// `cmd_verify`'s `ExitCode`/printing, so the actual decision is directly
/// unit-testable.
fn flags_fail_the_gate(flagged_count: usize, flag_is_not_failure: bool) -> bool {
    flagged_count > 0 && !flag_is_not_failure
}

fn cmd_confirm(args: ConfirmArgs) -> Result<ExitCode> {
    // Exactly one source for the prefix: an explicit hex, or the passphrase.
    let prefix = match (&args.pubkey_prefix, args.from_passphrase) {
        (Some(p), false) => p.clone(),
        (None, true) => {
            // Derive the reviewer's key from the passphrase, confirmed twice,
            // and print it so you can see (and reuse) the key you're checking
            // against. This is the only place the passphrase→pubkey mapping is
            // surfaced; the public key is not secret.
            let passphrase = prompt_tty()?;
            let hex = crate::signature::StageKeypair::derive(&passphrase).pubkey_hex();
            if !args.json {
                println!("your public key: {hex}");
            }
            hex
        }
        (Some(_), true) => {
            return Err(EinmoError::Config(
                "give a <pubkey-prefix> OR --from-passphrase, not both".into(),
            ));
        }
        (None, false) => {
            return Err(EinmoError::Config(
                "give a <pubkey-prefix> or --from-passphrase".into(),
            ));
        }
    };
    let report = crate::confirm_signatures(&args.path, &prefix)?;
    if args.json {
        println!(
            "{{\"matched\":{},\"unmatched\":{}}}",
            report.matched.len(),
            report.unmatched.len()
        );
    } else {
        println!(
            "{} file(s) match prefix {:?}, {} do not",
            report.matched.len(),
            prefix,
            report.unmatched.len()
        );
    }
    Ok(if args.require_all && !report.all_matched() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn cmd_show(args: ShowArgs) -> Result<ExitCode> {
    let file = EinmoFile::from_file(&args.file)?;
    let meta = file.metadata();
    if args.json {
        let stamps: Vec<String> = file
            .stamps()
            .entries()
            .iter()
            .map(|s| {
                format!(
                    "{{\"key\":\"{}\",\"pubkey\":\"{}\"}}",
                    s.key(),
                    s.pubkey_hex()
                )
            })
            .collect();
        println!(
            "{{\"test\":\"{}\",\"status\":\"{}\",\"stamps\":[{}]}}",
            meta.test,
            file.metadata().status,
            stamps.join(",")
        );
    } else {
        println!("test:     {}", meta.test);
        println!("suite:    {}", meta.suite);
        println!("producer: {}", meta.producer);
        println!("status:   {}", meta.status);
        if !meta.reference.is_empty() {
            println!("reference: {}", meta.reference);
        }
        println!("sections: {}", meta.sections.join(", "));
        println!("stamps:");
        for stamp in file.stamps().entries() {
            println!(
                "  {} pubkey={}… {} [{}]",
                stamp.key(),
                &stamp.pubkey_hex()[..stamp.pubkey_hex().len().min(8)],
                stamp.timestamp(),
                stamp.produced_by()
            );
        }
        if let Some(adv) = file.advisory() {
            println!("advisory: {adv}");
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_body(args: BodyArgs) -> Result<ExitCode> {
    // from_file verifies every stamp before returning (verify-on-inspect).
    let file = EinmoFile::from_file(&args.file)?;
    let sections = crate::einmo_suite::body_sections(&file, args.section.as_deref());
    if sections.is_empty()
        && let Some(want) = &args.section
    {
        return Err(EinmoError::Parse(format!(
            "no section {want:?} in {}",
            args.file.display()
        )));
    }
    for (name, body) in sections {
        if !args.bare {
            println!("=== {name} ===");
        }
        println!("{body}");
    }
    Ok(ExitCode::SUCCESS)
}

/// `einmo list`'s own notion of "differing": ALL non-STAMPS sections must
/// be identical across ALL THREE stages, not `MatchSections`-policy-scoped
/// — a general suite-shape overview for a human, distinct from
/// `ReviewMode::NewOrBroken`'s narrower output-vs-checked promise
/// (`EIMP-7` §S.6 fixed THAT one; this one was never overloaded the same
/// way and keeps its original all-stages, all-sections behavior exactly).
/// A tampered stage counts as absent, matching `scan_tests`'s prior
/// behavior (verify-on-inspect failure is reported via `stages()`'s own
/// `"TAMPERED"` status, but folds into `differing` the same as missing).
fn list_differing<S: crate::storage::EinmoStorage>(case: &crate::case::EinmoCase<'_, S>) -> bool {
    let bodies: Vec<Option<Vec<(String, String)>>> = Stage::ALL
        .iter()
        .map(
            |&stage| match case.read(crate::storage::ArtifactLocation::Stage(stage)) {
                Ok(Some(file)) => Some(crate::einmo_suite::body_sections(&file, None)),
                Ok(None) | Err(_) => None,
            },
        )
        .collect();
    bodies.iter().any(Option::is_none) || bodies.windows(2).any(|w| w[0] != w[1])
}

fn cmd_list(args: ListArgs) -> Result<ExitCode> {
    let config = TestConfig::new(&args.work_dir, ValidationLevel::Output);
    let directory = crate::storage::EinmoDirectory::new(config);
    // Filtered by hand below, against the mirror-relative path -- ListArgs'
    // own documented contract ("mirror-relative path"), not
    // EinmoSuite::scan's built-in filter (which matches the bare id, no
    // `.einmo` suffix, EIMP-7 §S.5's own choice for the general case).
    let suite = crate::suite::EinmoSuite::scan(directory, None)?;

    struct Row {
        rel: String,
        differing: bool,
        stages: Vec<(Stage, Option<String>)>,
    }
    let mut rows = Vec::new();
    for case in suite.cases() {
        let rel = crate::stage::mirror_input_path(Path::new(case.id().as_str()))
            .to_string_lossy()
            .into_owned();
        if let Some(pat) = &args.filter
            && !rel.contains(pat.as_str())
        {
            continue;
        }
        let differing = list_differing(&case);
        if args.differing && !differing {
            continue;
        }
        let stages = case.stages()?;
        rows.push(Row {
            rel,
            differing,
            stages,
        });
    }

    for row in &rows {
        if args.json {
            let stages: Vec<String> = row
                .stages
                .iter()
                .map(|(s, st)| {
                    format!(
                        "\"{}\":{}",
                        s.dir_name(),
                        st.as_ref()
                            .map_or_else(|| "null".to_string(), |v| format!("\"{v}\""))
                    )
                })
                .collect();
            println!(
                "{{\"test\":\"{}\",\"differing\":{},{}}}",
                row.rel,
                row.differing,
                stages.join(",")
            );
        } else {
            let marks: Vec<String> = row
                .stages
                .iter()
                .map(|(s, st)| {
                    let mark = st.as_ref().map_or("—", |v| match v.as_str() {
                        "normal" => "ok",
                        other => other,
                    });
                    format!("{}:{}", s.dir_name(), mark)
                })
                .collect();
            println!(
                "{}\t{}\t{}",
                row.rel,
                if row.differing { "differ" } else { "same" },
                marks.join(" ")
            );
        }
    }
    if !args.json {
        eprintln!("{} test(s)", rows.len());
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_self_check(args: SelfCheckArgs) -> Result<ExitCode> {
    let exe = std::env::current_exe().map_err(|e| EinmoError::io("<current_exe>", e))?;
    let hash = sha256_file(&exe)?;
    if args.quiet {
        println!("{hash}");
    } else {
        println!("{} sha256:{hash}", exe.display());
    }
    // An expected hash may come from --expected or a sidecar `einmo.sha256`.
    let expected = args.expected.or_else(|| read_sidecar_hash(&exe));
    if let Some(expected) = expected
        && !expected.eq_ignore_ascii_case(&hash)
    {
        eprintln!("einmo: self-check mismatch (expected {expected}, got {hash})");
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

fn read_sidecar_hash(exe: &Path) -> Option<String> {
    let sidecar = exe.with_file_name("einmo.sha256");
    std::fs::read_to_string(sidecar)
        .ok()
        .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
}

fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).map_err(|e| EinmoError::io(path, e))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

/// Read one line from stdin (for `--stdin-passphrase`).
fn read_stdin_line() -> Result<String> {
    use std::io::BufRead;
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| EinmoError::io("<stdin>", e))?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// Prompt for a passphrase on the controlling terminal (cross-platform via
/// rpassword), reading it **twice** and requiring the two to match.
///
/// The passphrase derives the signing key for a `verified` stamp — a human
/// attestation that a typo would silently misdirect: a mistyped passphrase
/// yields a *different, valid* keypair, so the promotion would succeed under a
/// key nobody can reproduce, and the merge gate's reviewer-key check would then
/// reject it with no hint why. Confirming the entry catches the typo at the
/// keyboard instead. Used by the stage-key cascade's interactive tier (§B.5).
fn prompt_tty() -> Result<String> {
    loop {
        let first = rpassword::prompt_password("einmo passphrase: ")
            .map_err(|e| EinmoError::io("<tty>", e))?;
        let second = rpassword::prompt_password("einmo passphrase (again): ")
            .map_err(|e| EinmoError::io("<tty>", e))?;
        if first == second {
            return Ok(first);
        }
        eprintln!("einmo: passphrases did not match — try again (Ctrl-C to abort)");
    }
}

struct CommandEvaluator {
    command: String,
}

impl crate::einmo_suite::Evaluator for CommandEvaluator {
    fn evaluate(&self, source: &str) -> std::result::Result<Vec<String>, String> {
        use std::io::Write;
        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("evaluator command failed: {e}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(source.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        let output = child
            .wait_with_output()
            .map_err(|e| format!("evaluator command failed: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "evaluator exited {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(vec![stdout.to_string()])
    }
}

fn cmd_evaluate(args: EvaluateArgs) -> Result<ExitCode> {
    run_evaluate_like(args, false, "evaluated")
}

/// `einmo regenerate-output`: like `evaluate`, but a drifted case is
/// deliberately replaced instead of failing (`EinmoTestRunner::regenerate_output`,
/// `EIMP-3.md` §Specification).
fn cmd_regenerate_output(args: EvaluateArgs) -> Result<ExitCode> {
    run_evaluate_like(args, true, "regenerated")
}

/// Shared driver for `cmd_evaluate`/`cmd_regenerate_output`: walk+filter the
/// input tree, run each input through `evaluator`, and report per-file
/// success/failure. `force` selects `EinmoTestRunner::regenerate_output` (drift
/// replaces) over `EinmoTestRunner::evaluate` (drift fails); `verb` only affects
/// the summary line's wording/JSON key.
fn run_evaluate_like(args: EvaluateArgs, force: bool, verb: &str) -> Result<ExitCode> {
    let mut config = TestConfig::new(&args.work_dir, ValidationLevel::Output);
    if let Some(limit) = args.walk_depth_limit {
        config = config.with_walk_depth_limit(limit);
    }
    let suite = crate::einmo_suite::EinmoTestRunner::new(config);
    let evaluator = CommandEvaluator {
        command: args.command,
    };

    let inputs = crate::stage::walk_input_tree(
        &args.work_dir.join("input"),
        args.walk_depth_limit.unwrap_or(64),
    )?;
    let filtered: Vec<_> = inputs
        .into_iter()
        .filter(|p| {
            args.filter
                .as_ref()
                .is_none_or(|f| p.to_string_lossy().contains(f.as_str()))
        })
        .collect();

    let mut written = 0usize;
    let mut failed = 0usize;
    for input_rel in &filtered {
        let outcome = if force {
            suite.regenerate_output(input_rel, &evaluator)
        } else {
            suite.evaluate(input_rel, &evaluator)
        };
        match outcome {
            Ok(result) => {
                if result.written_and_verified {
                    written += 1;
                    if !args.json {
                        println!("  ✓ {}", result.rel_path.display());
                    }
                } else {
                    failed += 1;
                    if !args.json {
                        println!(
                            "  ✗ {} ({})",
                            result.rel_path.display(),
                            result.detail.as_deref().unwrap_or("")
                        );
                    }
                }
            }
            Err(e) => {
                failed += 1;
                if !args.json {
                    println!("  ✗ {} — {e}", input_rel.display());
                }
            }
        }
    }
    if args.json {
        println!("{{\"{verb}\":{written},\"failed\":{failed}}}");
    } else {
        println!("{verb} {written} file(s), {failed} failure(s)");
    }
    Ok(if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_transition_ok_and_errors() {
        assert_eq!(
            parse_transition("output:checked").unwrap(),
            (Stage::Output, Stage::Checked)
        );
        assert!(parse_transition("output>checked").is_err());
        assert!(parse_transition("output:bogus").is_err());
    }

    /// `->` is not accepted: it collided with shell redirection (an UNQUOTED
    /// `checked->verified` on the command line had its `>` eaten by the shell
    /// — redirecting stdout to a file named `verified` — so einmo received
    /// only `checked-` and failed with a baffling `transition "checked-"`
    /// error) and `to` is easier to type. The spaced `to` keyword (handled in
    /// `split_promote_args`) is preferred; `:` and `..` remain as
    /// quoting-free glued alternatives.
    #[test]
    fn transition_accepts_redirect_safe_separators() {
        let want = (Stage::Checked, Stage::Verified);
        assert_eq!(parse_transition("checked:verified").unwrap(), want);
        assert_eq!(parse_transition("checked..verified").unwrap(), want);
        // `->` is no longer a valid separator at all.
        assert!(parse_transition("checked->verified").is_err());
    }

    #[test]
    fn split_promote_spaced_and_glued_agree() {
        let want = (Stage::Checked, Stage::Verified);
        let strs = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();

        // Spaced form: `checked to verified <dir> [files]`.
        let (f, t, dir, files) =
            split_promote_args(&strs(&["checked", "to", "verified", "suite", "a.einmo"])).unwrap();
        assert_eq!((f, t), want);
        assert_eq!(dir, PathBuf::from("suite"));
        assert_eq!(files, vec![PathBuf::from("a.einmo")]);

        // `to` is case-insensitive.
        assert_eq!(
            split_promote_args(&strs(&["checked", "TO", "verified", "suite"]))
                .map(|(f, t, _, _)| (f, t))
                .unwrap(),
            want
        );

        // Glued forms all agree with the spaced form.
        for glued in ["checked:verified", "checked..verified"] {
            let (f, t, dir, files) = split_promote_args(&strs(&[glued, "suite"])).unwrap();
            assert_eq!((f, t), want, "{glued}");
            assert_eq!(dir, PathBuf::from("suite"));
            assert!(files.is_empty());
        }

        // `->` is no longer accepted, glued or otherwise.
        assert!(split_promote_args(&strs(&["checked->verified", "suite"])).is_err());
    }

    #[test]
    fn cli_parses_subcommands() {
        // Smoke: the parser accepts each subcommand shape.
        assert!(Cli::try_parse_from(["einmo", "verify", "/tmp/s", "--all"]).is_ok());
        assert!(Cli::try_parse_from(["einmo", "compare", "output", "checked", "/tmp/s"]).is_ok());
        assert!(Cli::try_parse_from(["einmo", "promote", "output:checked", "/tmp/s"]).is_ok());
        assert!(Cli::try_parse_from(["einmo", "self-check", "--quiet"]).is_ok());
        assert!(
            Cli::try_parse_from(["einmo", "regenerate-output", "/tmp/s", "--command", "cat"])
                .is_ok()
        );
        assert!(
            Cli::try_parse_from(["einmo", "verify", "/tmp/s", "--flag-is-not-failure"]).is_ok()
        );
    }

    // EIMP-1 S.3: flags break the goal-state gate by default.

    #[test]
    fn flags_fail_the_gate_by_default() {
        assert!(flags_fail_the_gate(1, false));
        assert!(flags_fail_the_gate(3, false));
    }

    #[test]
    fn flags_do_not_fail_the_gate_when_downgraded() {
        assert!(!flags_fail_the_gate(1, true));
        assert!(!flags_fail_the_gate(3, true));
    }

    #[test]
    fn no_flags_never_fails_the_gate_either_way() {
        assert!(!flags_fail_the_gate(0, false));
        assert!(!flags_fail_the_gate(0, true));
    }

    #[test]
    fn cli_parses_list_and_body() {
        assert!(Cli::try_parse_from(["einmo", "list", "/tmp/s"]).is_ok());
        assert!(Cli::try_parse_from(["einmo", "list", "/tmp/s", "--differing", "--json"]).is_ok());
        assert!(Cli::try_parse_from(["einmo", "list", "/tmp/s", "--filter", "foop/23"]).is_ok());
        assert!(Cli::try_parse_from(["einmo", "body", "/tmp/a.einmo"]).is_ok());
        assert!(
            Cli::try_parse_from([
                "einmo",
                "body",
                "/tmp/a.einmo",
                "--section",
                "OUTPUT",
                "--bare"
            ])
            .is_ok()
        );
    }

    #[test]
    fn cli_promote_collects_positional_args() {
        // Glued + files.
        let cli = Cli::try_parse_from([
            "einmo",
            "promote",
            "output:checked",
            "/tmp/s",
            "a.einmo",
            "b.einmo",
        ])
        .unwrap();
        let Command::Promote(a) = cli.command else {
            panic!("expected Promote");
        };
        let (f, t, dir, files) = split_promote_args(&a.args).unwrap();
        assert_eq!((f, t), (Stage::Output, Stage::Checked));
        assert_eq!(dir, PathBuf::from("/tmp/s"));
        assert_eq!(
            files,
            vec![PathBuf::from("a.einmo"), PathBuf::from("b.einmo")]
        );
    }

    #[test]
    fn cli_promote_spaced_form_parses() {
        // The shell-safe spoken form, the whole point of `to`.
        let cli = Cli::try_parse_from([
            "einmo", "promote", "checked", "to", "verified", "/tmp/s", "x.einmo",
        ])
        .unwrap();
        let Command::Promote(a) = cli.command else {
            panic!("expected Promote");
        };
        let (f, t, dir, files) = split_promote_args(&a.args).unwrap();
        assert_eq!((f, t), (Stage::Checked, Stage::Verified));
        assert_eq!(dir, PathBuf::from("/tmp/s"));
        assert_eq!(files, vec![PathBuf::from("x.einmo")]);
    }

    #[test]
    fn cli_promote_no_files_is_empty() {
        let cli = Cli::try_parse_from([
            "einmo",
            "promote",
            "output:checked",
            "/tmp/s",
            "--filter",
            "*",
        ])
        .unwrap();
        let Command::Promote(a) = cli.command else {
            panic!("expected Promote");
        };
        let (_, _, _, files) = split_promote_args(&a.args).unwrap();
        assert!(files.is_empty());
        assert_eq!(a.filter.as_deref(), Some("*"));
    }

    #[test]
    fn cli_verify_accepts_positional_files() {
        let cli = Cli::try_parse_from(["einmo", "verify", "--all", "/tmp/s", "x.einmo"]).unwrap();
        let Command::Verify(a) = cli.command else {
            panic!("expected Verify");
        };
        assert_eq!(a.files, vec![PathBuf::from("x.einmo")]);
    }

    #[test]
    fn resolve_files_from_iter_empty() {
        assert!(resolve_files_from_iter(Vec::new(), std::iter::empty()).is_empty());
    }

    #[test]
    fn resolve_files_from_iter_no_dash() {
        let files = vec![PathBuf::from("a.einmo"), PathBuf::from("b.einmo")];
        let out = resolve_files_from_iter(files.clone(), ["c".into()].into_iter());
        assert_eq!(out, files);
    }

    #[test]
    fn resolve_files_from_iter_dash_replaced_by_stdin() {
        let out = resolve_files_from_iter(
            vec![PathBuf::from("-")],
            ["a.einmo", "b.einmo"].into_iter().map(String::from),
        );
        assert_eq!(
            out,
            vec![PathBuf::from("a.einmo"), PathBuf::from("b.einmo")]
        );
    }

    #[test]
    fn resolve_files_from_iter_dash_mixed_with_files() {
        let out = resolve_files_from_iter(
            vec![
                PathBuf::from("pre.einmo"),
                PathBuf::from("-"),
                PathBuf::from("post.einmo"),
            ],
            ["a.einmo", "b.einmo"].into_iter().map(String::from),
        );
        assert_eq!(
            out,
            vec![
                PathBuf::from("pre.einmo"),
                PathBuf::from("a.einmo"),
                PathBuf::from("b.einmo"),
                PathBuf::from("post.einmo"),
            ]
        );
    }

    #[test]
    fn resolve_files_from_iter_skips_blank_lines() {
        let out = resolve_files_from_iter(
            vec![PathBuf::from("-")],
            ["a.einmo", "", "  ", "b.einmo"]
                .into_iter()
                .map(String::from),
        );
        assert_eq!(
            out,
            vec![PathBuf::from("a.einmo"), PathBuf::from("b.einmo")]
        );
    }

    #[test]
    fn files_ref_empty_is_none() {
        assert!(files_ref(&[]).is_none());
        let v = vec![PathBuf::from("a.einmo")];
        assert_eq!(files_ref(&v), Some(v.as_slice()));
    }
}
