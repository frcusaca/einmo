//! `einmo-review-server` — hosts one `EinmoReview` session over a
//! unix-domain socket (`serve`, EIMP-2 §7 / `EIMP-1` §S.7), and also carries
//! `EIMP-1` Phase B's one-shot review verbs (`plan`/`list`/`decide`/
//! `undecide`/`execute`) as sibling subcommands.
//!
//! Per `EIMP-1.md` §S.1 ("Phase B's `einmo review …` verbs belong to
//! `einmo-review-server`") these operate on [`einmo::EinmoReview`], which
//! lives in this same crate as the server — until `EIMP-4` splits the repo
//! into core `einmo` and published `einmo-review-server`, both live here.
//!
//! Every one-shot subcommand is journal-backed session identity in miniature
//! (`EIMP-1` §S.6): each CLI invocation is a separate process with no shared
//! memory, so `--session <id>` lets a caller resume the same session across
//! calls — `decide` without `--session` mints a fresh one and prints it;
//! `execute --session <that-id>` later picks the pending decisions back up.
//! Every subcommand announces the session id it operated under (to stderr,
//! so `--json` stdout stays a clean single payload) for exactly this
//! chaining.
//!
//! `execute`'s confirm gate and `SignerSet` construction mirror
//! `review_server::post_execute` exactly (`EIMP-1` §S.7): any pending
//! promotion requires the literal `--confirm PROMOTE`; `to_checked` is
//! always the computer/empty-passphrase key; `to_verified` is resolved only
//! when a pending promotion actually targets `verified`, via the same
//! `--passphrase`/`--stdin-passphrase`/`--interactive` cascade
//! `src/cli.rs`'s `einmo promote` already uses (`KeyCascadeInputs`,
//! `resolve_stage_key`) — the HTTP endpoint has no interactive-prompt tier
//! to fall back to, but a CLI naturally does.

use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use einmo::{
    Decision, EinmoError, EinmoId, EinmoReview, ExecutionPlan, ExecutionReport, KeyCascadeInputs,
    KeySource, PlannedAction, ReviewMode, ReviewOpts, SignerSet, Stage, TestConfig,
    ValidationLevel, resolve_stage_key,
};

/// This binary's own `Result` alias — `einmo::EinmoError` is the crate's
/// central error type, but its `pub(crate) type Result<T>` alias is not
/// visible across the crate boundary a `src/bin/*.rs` file sits on, so each
/// binary defines its own thin alias over the same public error type
/// (mirrors `src/cli.rs`'s `crate::error::Result`, one crate boundary over).
type Result<T> = std::result::Result<T, EinmoError>;

/// The `einmo-review-server` command-line interface.
#[derive(Parser, Debug)]
#[command(
    name = "einmo-review-server",
    version,
    about = "Host and drive one EinmoReview session"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Host one EinmoReview session over a unix-domain socket (`EIMP-1` §S.7).
    Serve(ServeArgs),
    /// Preview what `execute` would do right now.
    Plan(PlanArgs),
    /// List the worklist and each case's current decision.
    List(ListArgs),
    /// Record (or replace) a decision for one case.
    Decide(DecideArgs),
    /// Clear a case's pending decision back to "untouched".
    Undecide(UndecideArgs),
    /// Apply every pending decision: sign + write, flag, or retract.
    Execute(ExecuteArgs),
}

#[derive(Args, Debug)]
struct ServeArgs {
    /// Where to bind the unix-domain socket. Overridden by `--private`,
    /// which mints its own path instead.
    #[arg(long, default_value = ".einmo-review.sock")]
    socket: PathBuf,
    /// Bind a fresh, unpredictable socket inside a mode-700 scratch
    /// directory instead of `--socket` (`EIMP-1` §S.7a: the TUI-owned
    /// private-server shape a driving script mints per session, rather than
    /// the standalone server's fixed, world-known path). The resulting
    /// socket path is printed as the FIRST line of stdout — the only thing
    /// this mode ever writes to stdout — so a caller can capture it, e.g.
    /// `SOCKET=$(einmo-review-server serve --private <suite>)`. Every other
    /// message (suite/session/socket announcement, shutdown) goes to
    /// stderr, same as the standalone mode.
    #[arg(long)]
    private: bool,
    /// Additionally bind TCP on a loopback address (`EIMP-1` §S.7: "TCP on
    /// 127.0.0.1 with a bearer token only when a browser needs it"). UDS
    /// always binds regardless of this flag — TCP is a pure addition, never
    /// a replacement. Requires `--token`.
    #[arg(long, requires = "token")]
    tcp: Option<SocketAddr>,
    /// The bearer token TCP clients must present
    /// (`Authorization: Bearer <token>`). Required when `--tcp` is given;
    /// meaningless (and ignored) without it.
    #[arg(long)]
    token: Option<String>,
    /// Lift the default refusal to execute a `checked → verified` promotion
    /// (passphrase in the request body) over the TCP listener. `EIMP-2`'s
    /// standing caveat, restated in `EIMP-1` §S.7: that passphrase travels
    /// **plaintext** in the request body, materially riskier over TCP than
    /// over a UDS (a local-only socket) — building an actual mitigation
    /// (TLS, mTLS) is out of this EIMP's scope, so the default is a hard
    /// refusal and this flag is the explicit, separate, informed opt-in
    /// past it. Meaningless without `--tcp`.
    #[arg(long)]
    allow_insecure_tcp_verified_passphrase: bool,
    /// The suite work directory to open a review session over.
    suite: PathBuf,
}

/// Shared by every one-shot subcommand: the suite and (optionally) which
/// session to resume.
#[derive(Args, Debug)]
struct SessionArgs {
    /// The suite work directory.
    work_dir: PathBuf,
    /// Resume this session id (journal-backed, `EIMP-1` §S.6): replays its
    /// journal to reconstruct pending decisions before this command runs.
    /// Omit to start a fresh session — every one-shot subcommand announces
    /// (to stderr) the session id it operated under, for a later
    /// `--session` to pick back up.
    #[arg(long)]
    session: Option<String>,
}

#[derive(Args, Debug)]
struct PlanArgs {
    #[command(flatten)]
    session: SessionArgs,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct ListArgs {
    #[command(flatten)]
    session: SessionArgs,
    /// Only cases whose id contains this substring.
    #[arg(long)]
    filter: Option<String>,
    /// Only cases with no baseline yet, or a content mismatch between
    /// stages (`ReviewMode::NewOrBroken`) — the same predicate `einmo list
    /// --differing` uses.
    #[arg(long)]
    differing: bool,
    /// Emit machine-readable JSON (one object per line).
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct DecideArgs {
    #[command(flatten)]
    session: SessionArgs,
    /// The case id (mirror-relative, e.g. `foop/23/comprehensive.foo`).
    id: String,
    /// The decision:
    ///   `promote to <stage>`       (or bare `promote <stage>`)
    ///   `retract from <stage>`     (or bare `retract <stage>`)
    ///   `flag <stage> <reason…>`
    ///   `skip`
    /// `<stage>` is `checked` or `verified` — the only two stages a
    /// reviewer decides about. Parsed positionally by [`parse_decision`].
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
    decision: Vec<String>,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct UndecideArgs {
    #[command(flatten)]
    session: SessionArgs,
    /// The case id.
    id: String,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Args, Debug)]
struct ExecuteArgs {
    #[command(flatten)]
    session: SessionArgs,
    /// Must be the literal string `PROMOTE` for any pending promotion to
    /// apply — a typo-resistant confirmation gate, not a security boundary
    /// (matches `POST /einmo/<session>/execute`'s `confirm` field exactly,
    /// `EIMP-1` §S.7).
    #[arg(long, default_value = "")]
    confirm: String,
    /// Explicit passphrase for `* to verified` promotions (tier 1). Only
    /// consulted when a pending promotion actually targets `verified` —
    /// `output`/`* to checked` promotions always use the computer key.
    #[arg(long)]
    passphrase: Option<String>,
    /// Read one passphrase line from stdin (tier 2).
    #[arg(long)]
    stdin_passphrase: bool,
    /// Force the interactive prompt (skips tiers 1–3).
    #[arg(long)]
    interactive: bool,
    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

fn main() -> ExitCode {
    cli_main(std::env::args_os().collect())
}

/// The shared entry point. `serve` is the only subcommand that needs an
/// async runtime — it builds one itself and blocks on it; every other
/// subcommand is `EinmoReview`'s ordinary synchronous API and runs directly
/// on the calling thread.
#[must_use]
fn cli_main(args: Vec<OsString>) -> ExitCode {
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
    match cli.command {
        Command::Serve(args) => run_serve_blocking(args),
        Command::Plan(args) => report(cmd_plan(args)),
        Command::List(args) => report(cmd_list(args)),
        Command::Decide(args) => report(cmd_decide(args)),
        Command::Undecide(args) => report(cmd_undecide(args)),
        Command::Execute(args) => report(cmd_execute(args)),
    }
}

fn report(result: Result<ExitCode>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(e) => {
            eprintln!("einmo-review-server: {e}");
            ExitCode::FAILURE
        }
    }
}

// --- serve ------------------------------------------------------------

/// Build a `#[tokio::main]`-equivalent runtime (multi-thread, every driver
/// enabled) and block on [`run_serve`] — the one subcommand that needs an
/// async runtime at all.
fn run_serve_blocking(args: ServeArgs) -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("einmo-review-server: failed to start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(run_serve(args))
}

/// Bind (`args.socket`, or a fresh private path when `args.private` —
/// `EIMP-1` §S.7a), open one session over `args.suite` against itself
/// (`POST /einmo/sessions`), write `<socket-path>.session`, and serve until
/// `Ctrl-C`/`SIGTERM`. Optionally also binds TCP (`args.tcp`). All files and
/// the suite lock are removed on exit.
///
/// **Suite lockfile (`EIMP-1` §S.5, plan Phase C).** Acquired first, before
/// any socket is touched: a second server — of ANY kind, this standalone
/// mode or a future TUI-owned private one — already reviewing `args.suite`
/// makes this call refuse outright, so a losing race never binds a socket
/// it would then have to unwind.
async fn run_serve(args: ServeArgs) -> ExitCode {
    let socket_path = if args.private {
        match einmo::private_socket_path() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("einmo-review-server: failed to mint a private socket path: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        args.socket.clone()
    };

    let suite_lock = match einmo::SuiteLock::acquire(&args.suite, &socket_path) {
        Ok(lock) => lock,
        Err(e) => {
            eprintln!("einmo-review-server: {e}");
            return ExitCode::FAILURE;
        }
    };

    let state = std::sync::Arc::new(einmo::AppState::default());
    let session = state.create_session(&args.suite);
    let uds_app = einmo::router(state.clone());

    let session_file = session_file_path(&socket_path);
    if let Err(e) = std::fs::write(&session_file, session.to_string()) {
        eprintln!(
            "einmo-review-server: failed to write session file {}: {e}",
            session_file.display()
        );
        drop(suite_lock);
        return ExitCode::FAILURE;
    }

    if args.private {
        // Machine-consumable: the ONLY thing this mode writes to stdout, so
        // a driving script can do `SOCKET=$(einmo-review-server serve
        // --private <suite>)` (EIMP-1 §S.7a).
        println!("{}", socket_path.display());
    }
    eprintln!(
        "einmo-review-server: suite {} session {session} socket {}",
        args.suite.display(),
        socket_path.display()
    );

    // One broadcast shutdown signal feeds both listeners (UDS always; TCP
    // only if requested) so Ctrl-C/SIGTERM tears down whichever are
    // running, not just the first one awaited.
    let (shutdown_tx, _) = tokio::sync::broadcast::channel::<()>(1);

    let tcp_handle = match (args.tcp, args.token.clone()) {
        (Some(addr), Some(token)) => {
            eprintln!(
                "einmo-review-server: TCP {addr}{}",
                if args.allow_insecure_tcp_verified_passphrase {
                    " (bearer token required; INSECURE: verified passphrases allowed in the clear)"
                } else {
                    " (bearer token required; verified-promotion passphrases refused over this listener)"
                }
            );
            let tcp_app =
                einmo::router_tcp(state, token, args.allow_insecure_tcp_verified_passphrase);
            let mut rx = shutdown_tx.subscribe();
            let tcp_shutdown = async move {
                let _ = rx.recv().await;
            };
            Some(tokio::spawn(async move {
                einmo::serve_tcp(tcp_app, addr, tcp_shutdown).await
            }))
        }
        _ => None,
    };

    let ctrl_c_tx = shutdown_tx.clone();
    let ctrl_c_task = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("einmo-review-server: shutting down");
        let _ = ctrl_c_tx.send(());
    });

    let mut uds_shutdown_rx = shutdown_tx.subscribe();
    let uds_shutdown = async move {
        let _ = uds_shutdown_rx.recv().await;
    };
    let uds_result = einmo::serve_uds(uds_app, &socket_path, uds_shutdown).await;

    // The UDS listener returning (cleanly or with an error) means this
    // process is done either way -- make sure the TCP listener (if any)
    // and the ctrl_c watcher wind down too, rather than leaking either.
    let _ = shutdown_tx.send(());
    ctrl_c_task.abort();
    if let Some(handle) = tcp_handle {
        let _ = handle.await;
    }

    let _ = std::fs::remove_file(&session_file);
    let _ = std::fs::remove_file(&socket_path);
    drop(suite_lock);

    match uds_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("einmo-review-server: {e}");
            ExitCode::FAILURE
        }
    }
}

/// The session-id sidecar file's path: `<socket-path>.session`.
fn session_file_path(socket_path: &Path) -> PathBuf {
    let mut s = socket_path.as_os_str().to_os_string();
    s.push(".session");
    PathBuf::from(s)
}

// --- one-shot review verbs ---------------------------------------------

/// Open (fresh) or resume (`EIMP-1` §S.6) a session over `work_dir`,
/// depending on whether a `--session` id was given.
fn open_or_resume(work_dir: &Path, session: Option<&str>, opts: ReviewOpts) -> Result<EinmoReview> {
    match session {
        Some(id) => EinmoReview::resume(work_dir, id, opts),
        None => Ok(EinmoReview::open_with(work_dir, opts)),
    }
}

/// Announce the session id a command operated under, to stderr — every
/// one-shot subcommand does this (so `--json` stdout stays one clean
/// payload) to support the `--session`-chaining `EIMP-1` §S.6 describes.
fn announce_session(review: &EinmoReview) {
    eprintln!("einmo-review-server: session {}", review.session_id());
}

/// A short human-readable tag for a [`Decision`] — the CLI's counterpart to
/// `review_server::decision_tag` (private to that module, so this is a
/// deliberate small duplication across the crate-internal module boundary
/// rather than a shared export solely for one-line formatting).
fn decision_tag(d: &Decision) -> String {
    match d {
        Decision::Promote { to } => format!("promote to {to}"),
        Decision::Retract { from } => format!("retract from {from}"),
        Decision::Flag { stage, reason } => format!("flag {stage}: {reason}"),
        Decision::Skip => "skip".to_string(),
    }
}

/// A short human-readable line for a [`PlannedAction`] (`plan`'s text mode).
fn action_line(action: &PlannedAction) -> String {
    match action {
        PlannedAction::Promote { id, to } => format!("promote {id} to {to}"),
        PlannedAction::Retract { id, from } => format!("retract {id} from {from}"),
        PlannedAction::Flag { id, stage, reason } => format!("flag {id} at {stage}: {reason}"),
    }
}

/// `ReviewOpts` from `list`'s flags — pure, so the differing/filter wiring
/// is directly unit-testable without opening a suite.
fn list_opts(args: &ListArgs) -> ReviewOpts {
    ReviewOpts {
        mode: if args.differing {
            ReviewMode::NewOrBroken
        } else {
            ReviewMode::Full
        },
        filter: args.filter.clone(),
    }
}

fn cmd_plan(args: PlanArgs) -> Result<ExitCode> {
    let review = open_or_resume(
        &args.session.work_dir,
        args.session.session.as_deref(),
        ReviewOpts::default(),
    )?;
    announce_session(&review);
    let plan = review.plan();
    if args.json {
        let actions: Vec<serde_json::Value> = plan.actions.iter().map(action_json).collect();
        let claims: Vec<serde_json::Value> = plan
            .claims
            .iter()
            .map(|c| serde_json::json!({"id": c.id.as_str(), "remaining_secs": c.remaining.as_secs()}))
            .collect();
        println!(
            "{}",
            serde_json::json!({"session": review.session_id(), "actions": actions, "claims": claims})
        );
    } else {
        if plan.actions.is_empty() {
            println!("no pending actions");
        }
        for action in &plan.actions {
            println!("{}", action_line(action));
        }
        for claim in &plan.claims {
            println!(
                "claimed {} ({}s remaining)",
                claim.id,
                claim.remaining.as_secs()
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn action_json(action: &PlannedAction) -> serde_json::Value {
    match action {
        PlannedAction::Promote { id, to } => {
            serde_json::json!({"kind": "promote", "id": id.as_str(), "to": to.dir_name()})
        }
        PlannedAction::Retract { id, from } => {
            serde_json::json!({"kind": "retract", "id": id.as_str(), "from": from.dir_name()})
        }
        PlannedAction::Flag { id, stage, reason } => {
            serde_json::json!({"kind": "flag", "id": id.as_str(), "stage": stage.dir_name(), "reason": reason})
        }
    }
}

fn cmd_list(args: ListArgs) -> Result<ExitCode> {
    let opts = list_opts(&args);
    let review = open_or_resume(
        &args.session.work_dir,
        args.session.session.as_deref(),
        opts,
    )?;
    announce_session(&review);
    let items = review.items()?;
    for item in &items {
        if args.json {
            let stages: serde_json::Map<String, serde_json::Value> = item
                .stages
                .iter()
                .map(|(s, st)| {
                    (
                        s.dir_name().to_string(),
                        st.clone()
                            .map_or(serde_json::Value::Null, serde_json::Value::String),
                    )
                })
                .collect();
            println!(
                "{}",
                serde_json::json!({
                    "id": item.id.as_str(),
                    "differing": item.differing,
                    "decision": item.decision.as_ref().map(decision_tag),
                    "stages": stages,
                })
            );
        } else {
            let marks: Vec<String> = item
                .stages
                .iter()
                .map(|(s, st)| {
                    let mark = st.as_deref().map_or("—", |v| match v {
                        "normal" => "ok",
                        other => other,
                    });
                    format!("{}:{}", s.dir_name(), mark)
                })
                .collect();
            let decision = item
                .decision
                .as_ref()
                .map_or_else(|| "-".to_string(), decision_tag);
            println!(
                "{}\t{}\t{}\t{}",
                item.id,
                if item.differing { "differ" } else { "same" },
                marks.join(" "),
                decision
            );
        }
    }
    if !args.json {
        eprintln!("{} test(s)", items.len());
    }
    Ok(ExitCode::SUCCESS)
}

/// Parse `decide`'s trailing decision tokens into a [`Decision`]. Kept
/// separate from `cmd_decide` so the grammar is directly unit-testable.
///
/// Accepts the spoken forms `promote to <stage>` / `retract from <stage>`
/// (also their bare `promote <stage>` / `retract <stage>` shorthand),
/// `flag <stage> <reason…>`, and `skip` — matching `src/cli.rs`'s
/// `split_promote_args`/`parse_transition` convention of a spoken `to`
/// keyword that needs no shell quoting, adapted to `Decision`'s single-stage
/// shape (a decision names only a destination/source stage, never a pair).
fn parse_decision(tokens: &[String]) -> Result<Decision> {
    let Some((kind, rest)) = tokens.split_first() else {
        return Err(EinmoError::Config(
            "decision requires at least a kind (promote/retract/flag/skip)".to_string(),
        ));
    };
    match kind.to_ascii_lowercase().as_str() {
        "promote" => {
            let stage = match rest {
                [word, stage] if word.eq_ignore_ascii_case("to") => stage,
                [stage] => stage,
                _ => {
                    return Err(EinmoError::Config(
                        "promote needs `to <stage>` (or bare `<stage>`)".to_string(),
                    ));
                }
            };
            Ok(Decision::Promote {
                to: parse_decidable_stage(stage)?,
            })
        }
        "retract" => {
            let stage = match rest {
                [word, stage] if word.eq_ignore_ascii_case("from") => stage,
                [stage] => stage,
                _ => {
                    return Err(EinmoError::Config(
                        "retract needs `from <stage>` (or bare `<stage>`)".to_string(),
                    ));
                }
            };
            Ok(Decision::Retract {
                from: parse_decidable_stage(stage)?,
            })
        }
        "flag" => {
            let Some((stage, reason_words)) = rest.split_first() else {
                return Err(EinmoError::Config(
                    "flag needs `<stage> <reason…>`".to_string(),
                ));
            };
            if reason_words.is_empty() {
                return Err(EinmoError::Config(
                    "flag needs a non-empty reason".to_string(),
                ));
            }
            Ok(Decision::Flag {
                stage: parse_decidable_stage(stage)?,
                reason: reason_words.join(" "),
            })
        }
        "skip" => {
            if !rest.is_empty() {
                return Err(EinmoError::Config(
                    "skip takes no further arguments".to_string(),
                ));
            }
            Ok(Decision::Skip)
        }
        other => Err(EinmoError::Config(format!(
            "unknown decision kind {other:?} (want promote/retract/flag/skip)"
        ))),
    }
}

/// The two stages a reviewer ever decides about — matches
/// `review_server::DecidableStage` (`output`/`flagged` are not legal
/// decision targets: output is the source everything starts from, and
/// flagging goes through its own `Decision::Flag { stage, .. }` shape whose
/// `stage` names where the flag is read FROM, still only `checked`/`verified`
/// in practice since those are the only stages worth flagging out of).
fn parse_decidable_stage(s: &str) -> Result<Stage> {
    match Stage::parse(s)? {
        stage @ (Stage::Checked | Stage::Verified) => Ok(stage),
        Stage::Output | Stage::Flagged => Err(EinmoError::Config(format!(
            "decision stage must be checked or verified, not {s:?}"
        ))),
    }
}

fn cmd_decide(args: DecideArgs) -> Result<ExitCode> {
    let id = EinmoId::try_from(args.id.as_str())?;
    let decision = parse_decision(&args.decision)?;
    let review = open_or_resume(
        &args.session.work_dir,
        args.session.session.as_deref(),
        ReviewOpts::default(),
    )?;
    announce_session(&review);
    let previous = review.decide(id.clone(), decision.clone());
    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "session": review.session_id(),
                "id": id.as_str(),
                "decision": decision_tag(&decision),
                "replaced": previous.as_ref().map(decision_tag),
            })
        );
    } else {
        match &previous {
            Some(prev) => println!(
                "decided {id}: {} (replacing: {})",
                decision_tag(&decision),
                decision_tag(prev)
            ),
            None => println!("decided {id}: {}", decision_tag(&decision)),
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn cmd_undecide(args: UndecideArgs) -> Result<ExitCode> {
    let id = EinmoId::try_from(args.id.as_str())?;
    let review = open_or_resume(
        &args.session.work_dir,
        args.session.session.as_deref(),
        ReviewOpts::default(),
    )?;
    announce_session(&review);
    let cleared = review.undecide(&id);
    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "session": review.session_id(),
                "id": id.as_str(),
                "cleared": cleared.as_ref().map(decision_tag),
            })
        );
    } else {
        match &cleared {
            Some(prev) => println!("undecided {id} (was: {})", decision_tag(prev)),
            None => println!("{id} was already undecided"),
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The confirm gate `POST /einmo/<session>/execute` enforces
/// (`review_server::post_execute`, `EIMP-1` §S.7): any pending promotion
/// requires the literal string `PROMOTE`. Kept separate from `cmd_execute`'s
/// I/O, matching `src/cli.rs`'s `flags_fail_the_gate` precedent for a pure,
/// directly unit-testable gate decision.
fn confirm_gate(plan: &ExecutionPlan, confirm: &str) -> Result<()> {
    let has_promotion = plan
        .actions
        .iter()
        .any(|a| matches!(a, PlannedAction::Promote { .. }));
    if has_promotion && confirm != "PROMOTE" {
        return Err(EinmoError::Config(
            "execute needs --confirm PROMOTE when any pending decision promotes".to_string(),
        ));
    }
    Ok(())
}

/// Whether `plan` needs a `* to verified` human signer at all (`EIMP-1`
/// §S.4: promotions to `verified` require a human key; `output`/`* to
/// checked` always use the computer key). Kept separate from `cmd_execute`
/// so it's directly unit-testable without ever reaching the interactive
/// prompt tier.
fn plan_needs_verified_key(plan: &ExecutionPlan) -> bool {
    plan.actions.iter().any(|a| {
        matches!(
            a,
            PlannedAction::Promote {
                to: Stage::Verified,
                ..
            }
        )
    })
}

/// Resolve the `* to verified` signing key via the same cascade `einmo
/// promote` uses (`src/cli.rs`'s `resolve_promotion_key`): `--passphrase` >
/// `--stdin-passphrase` > `EINMO_PASSPHRASE` > `einmo.toml` >
/// interactive prompt. Only called when [`plan_needs_verified_key`] is
/// true — `to_checked` never goes through this cascade at all; it is always
/// the computer key (matches `review_server::post_execute`'s `SignerSet`).
fn resolve_verified_key(args: &ExecuteArgs, config: &TestConfig) -> Result<KeySource> {
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
    resolve_stage_key(Stage::Verified, &inputs, config, prompt_tty)
}

fn cmd_execute(args: ExecuteArgs) -> Result<ExitCode> {
    let review = open_or_resume(
        &args.session.work_dir,
        args.session.session.as_deref(),
        ReviewOpts::default(),
    )?;
    announce_session(&review);
    let plan = review.plan();
    confirm_gate(&plan, &args.confirm)?;

    // The passphrase, if resolved at all, lives in this local `keys` value
    // only long enough to hand it to `execute` below, exactly as
    // `review_server::post_execute` scopes it to one request (EIMP-1 §S.4).
    let to_verified = if plan_needs_verified_key(&plan) {
        let config = TestConfig::new(&args.session.work_dir, ValidationLevel::Output);
        Some(resolve_verified_key(&args, &config)?)
    } else {
        None
    };
    let keys = SignerSet {
        to_checked: KeySource::from_passphrase(""),
        to_verified,
    };
    let report = review.execute(&plan, &keys)?;
    print_execute_report(review.session_id(), &report, args.json);
    Ok(ExitCode::SUCCESS)
}

fn print_execute_report(session_id: &str, report: &ExecutionReport, json: bool) {
    if json {
        let executed: Vec<serde_json::Value> = report
            .executed
            .iter()
            .map(|e| serde_json::json!({"id": e.id.as_str(), "detail": e.detail}))
            .collect();
        let skipped: Vec<&str> = report.skipped.iter().map(EinmoId::as_str).collect();
        println!(
            "{}",
            serde_json::json!({"session": session_id, "executed": executed, "skipped": skipped})
        );
    } else {
        for e in &report.executed {
            println!("executed {}: {}", e.id, e.detail);
        }
        for id in &report.skipped {
            println!("skipped {id} (drifted since decide)");
        }
        println!(
            "{} executed, {} skipped",
            report.executed.len(),
            report.skipped.len()
        );
    }
}

/// Read one line from stdin (for `--stdin-passphrase`).
fn read_stdin_line() -> Result<String> {
    use std::io::BufRead;
    let mut line = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut line)
        .map_err(|e| io_err("<stdin>", e))?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// Prompt for a passphrase on the controlling terminal, reading it twice and
/// requiring the two to match — same rationale and shape as `src/cli.rs`'s
/// `prompt_tty` (a typo in a `verified`-stamp passphrase silently derives a
/// different, unreproducible key rather than failing loudly, so catching the
/// typo at entry matters). Duplicated rather than shared: `cli.rs`'s version
/// is private to core's own CLI module, unreachable across this crate's
/// `src/bin/*.rs` boundary.
fn prompt_tty() -> Result<String> {
    loop {
        let first =
            rpassword::prompt_password("einmo passphrase: ").map_err(|e| io_err("<tty>", e))?;
        let second = rpassword::prompt_password("einmo passphrase (again): ")
            .map_err(|e| io_err("<tty>", e))?;
        if first == second {
            return Ok(first);
        }
        eprintln!("einmo-review-server: passphrases did not match — try again (Ctrl-C to abort)");
    }
}

/// Build an [`EinmoError::Io`] carrying the offending path — `EinmoError`'s
/// own `pub(crate) fn io` constructor helper is not visible across the
/// crate boundary a `src/bin/*.rs` file sits on, but the `Io` variant's
/// fields are public (enum variant fields are exactly as visible as the
/// enum itself), so this is the direct equivalent from outside the crate.
fn io_err(path: impl Into<PathBuf>, source: std::io::Error) -> EinmoError {
    EinmoError::Io {
        path: path.into(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- clap parsing smoke tests (mirrors src/cli.rs's cli_parses_subcommands) ---

    #[test]
    fn cli_parses_serve_preserving_its_original_flags() {
        let cli = Cli::try_parse_from([
            "einmo-review-server",
            "serve",
            "--socket",
            "/tmp/s.sock",
            "/tmp/suite",
        ])
        .unwrap();
        let Command::Serve(a) = cli.command else {
            panic!("expected Serve");
        };
        assert_eq!(a.socket, PathBuf::from("/tmp/s.sock"));
        assert_eq!(a.suite, PathBuf::from("/tmp/suite"));
    }

    #[test]
    fn cli_serve_socket_defaults() {
        let cli = Cli::try_parse_from(["einmo-review-server", "serve", "/tmp/suite"]).unwrap();
        let Command::Serve(a) = cli.command else {
            panic!("expected Serve");
        };
        assert_eq!(a.socket, PathBuf::from(".einmo-review.sock"));
        assert!(!a.private);
        assert_eq!(a.tcp, None);
        assert_eq!(a.token, None);
        assert!(!a.allow_insecure_tcp_verified_passphrase);
    }

    #[test]
    fn cli_parses_serve_private_flag() {
        let cli = Cli::try_parse_from(["einmo-review-server", "serve", "--private", "/tmp/suite"])
            .unwrap();
        let Command::Serve(a) = cli.command else {
            panic!("expected Serve");
        };
        assert!(a.private);
    }

    #[test]
    fn cli_parses_serve_tcp_and_token() {
        let cli = Cli::try_parse_from([
            "einmo-review-server",
            "serve",
            "--tcp",
            "127.0.0.1:9000",
            "--token",
            "s3cr3t",
            "/tmp/suite",
        ])
        .unwrap();
        let Command::Serve(a) = cli.command else {
            panic!("expected Serve");
        };
        assert_eq!(a.tcp, Some("127.0.0.1:9000".parse().unwrap()));
        assert_eq!(a.token.as_deref(), Some("s3cr3t"));
        assert!(!a.allow_insecure_tcp_verified_passphrase);
    }

    /// `--tcp` without `--token` is a clap-level parse error (`requires`) —
    /// the server must never accept an unauthenticated TCP listener even by
    /// omission.
    #[test]
    fn cli_serve_tcp_without_token_is_rejected() {
        let result = Cli::try_parse_from([
            "einmo-review-server",
            "serve",
            "--tcp",
            "127.0.0.1:9000",
            "/tmp/suite",
        ]);
        assert!(result.is_err(), "--tcp must require --token");
    }

    #[test]
    fn cli_parses_serve_allow_insecure_tcp_verified_passphrase() {
        let cli = Cli::try_parse_from([
            "einmo-review-server",
            "serve",
            "--tcp",
            "127.0.0.1:9000",
            "--token",
            "s3cr3t",
            "--allow-insecure-tcp-verified-passphrase",
            "/tmp/suite",
        ])
        .unwrap();
        let Command::Serve(a) = cli.command else {
            panic!("expected Serve");
        };
        assert!(a.allow_insecure_tcp_verified_passphrase);
    }

    #[test]
    fn cli_parses_plan_list_decide_undecide_execute() {
        assert!(Cli::try_parse_from(["einmo-review-server", "plan", "/tmp/s"]).is_ok());
        assert!(
            Cli::try_parse_from(["einmo-review-server", "plan", "/tmp/s", "--session", "abc"])
                .is_ok()
        );
        assert!(
            Cli::try_parse_from(["einmo-review-server", "list", "/tmp/s", "--differing"]).is_ok()
        );
        assert!(
            Cli::try_parse_from(["einmo-review-server", "list", "/tmp/s", "--filter", "a.foo"])
                .is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "einmo-review-server",
                "decide",
                "/tmp/s",
                "a.foo",
                "promote",
                "to",
                "checked",
            ])
            .is_ok()
        );
        assert!(
            Cli::try_parse_from(["einmo-review-server", "undecide", "/tmp/s", "a.foo"]).is_ok()
        );
        assert!(
            Cli::try_parse_from([
                "einmo-review-server",
                "execute",
                "/tmp/s",
                "--confirm",
                "PROMOTE",
            ])
            .is_ok()
        );
    }

    #[test]
    fn cli_decide_collects_trailing_decision_tokens() {
        let cli = Cli::try_parse_from([
            "einmo-review-server",
            "decide",
            "/tmp/s",
            "a.foo",
            "flag",
            "checked",
            "looks",
            "wrong",
        ])
        .unwrap();
        let Command::Decide(a) = cli.command else {
            panic!("expected Decide");
        };
        assert_eq!(a.id, "a.foo");
        assert_eq!(a.decision, vec!["flag", "checked", "looks", "wrong"]);
    }

    // --- parse_decision / parse_decidable_stage (pure) ---

    fn toks(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_decision_promote_spoken_and_bare() {
        assert_eq!(
            parse_decision(&toks(&["promote", "to", "checked"])).unwrap(),
            Decision::Promote { to: Stage::Checked }
        );
        assert_eq!(
            parse_decision(&toks(&["promote", "verified"])).unwrap(),
            Decision::Promote {
                to: Stage::Verified
            }
        );
        // Case-insensitive kind.
        assert_eq!(
            parse_decision(&toks(&["PROMOTE", "to", "checked"])).unwrap(),
            Decision::Promote { to: Stage::Checked }
        );
    }

    #[test]
    fn parse_decision_retract_spoken_and_bare() {
        assert_eq!(
            parse_decision(&toks(&["retract", "from", "verified"])).unwrap(),
            Decision::Retract {
                from: Stage::Verified
            }
        );
        assert_eq!(
            parse_decision(&toks(&["retract", "checked"])).unwrap(),
            Decision::Retract {
                from: Stage::Checked
            }
        );
    }

    #[test]
    fn parse_decision_flag_joins_reason_words() {
        assert_eq!(
            parse_decision(&toks(&["flag", "checked", "looks", "wrong"])).unwrap(),
            Decision::Flag {
                stage: Stage::Checked,
                reason: "looks wrong".to_string(),
            }
        );
    }

    #[test]
    fn parse_decision_flag_needs_a_reason() {
        assert!(parse_decision(&toks(&["flag", "checked"])).is_err());
    }

    #[test]
    fn parse_decision_skip_takes_nothing_else() {
        assert_eq!(parse_decision(&toks(&["skip"])).unwrap(), Decision::Skip);
        assert!(parse_decision(&toks(&["skip", "extra"])).is_err());
    }

    #[test]
    fn parse_decision_rejects_unknown_kind() {
        assert!(parse_decision(&toks(&["frobnicate", "checked"])).is_err());
    }

    #[test]
    fn parse_decision_rejects_empty() {
        assert!(parse_decision(&[]).is_err());
    }

    #[test]
    fn parse_decidable_stage_accepts_only_checked_and_verified() {
        assert_eq!(parse_decidable_stage("checked").unwrap(), Stage::Checked);
        assert_eq!(parse_decidable_stage("verified").unwrap(), Stage::Verified);
        assert!(parse_decidable_stage("output").is_err());
        assert!(parse_decidable_stage("flagged").is_err());
        assert!(parse_decidable_stage("bogus").is_err());
    }

    // --- list_opts (pure) ---

    #[test]
    fn list_opts_maps_differing_to_new_or_broken_mode() {
        let args = ListArgs {
            session: SessionArgs {
                work_dir: PathBuf::from("/tmp/s"),
                session: None,
            },
            filter: Some("a.foo".to_string()),
            differing: true,
            json: false,
        };
        let opts = list_opts(&args);
        assert_eq!(opts.mode, ReviewMode::NewOrBroken);
        assert_eq!(opts.filter.as_deref(), Some("a.foo"));
    }

    #[test]
    fn list_opts_defaults_to_full_mode() {
        let args = ListArgs {
            session: SessionArgs {
                work_dir: PathBuf::from("/tmp/s"),
                session: None,
            },
            filter: None,
            differing: false,
            json: false,
        };
        assert_eq!(list_opts(&args).mode, ReviewMode::Full);
    }

    // --- confirm_gate / plan_needs_verified_key (pure) ---

    fn promote_plan(to: Stage) -> ExecutionPlan {
        ExecutionPlan {
            actions: vec![PlannedAction::Promote {
                id: EinmoId::try_from("a.foo").unwrap(),
                to,
            }],
            claims: Vec::new(),
        }
    }

    #[test]
    fn confirm_gate_refuses_a_pending_promotion_without_promote() {
        let plan = promote_plan(Stage::Checked);
        assert!(confirm_gate(&plan, "").is_err());
        assert!(confirm_gate(&plan, "promote").is_err());
    }

    #[test]
    fn confirm_gate_accepts_the_literal_promote_token() {
        let plan = promote_plan(Stage::Checked);
        assert!(confirm_gate(&plan, "PROMOTE").is_ok());
    }

    #[test]
    fn confirm_gate_is_a_noop_with_no_pending_promotion() {
        let plan = ExecutionPlan {
            actions: vec![PlannedAction::Retract {
                id: EinmoId::try_from("a.foo").unwrap(),
                from: Stage::Checked,
            }],
            claims: Vec::new(),
        };
        assert!(confirm_gate(&plan, "").is_ok());
    }

    #[test]
    fn plan_needs_verified_key_only_for_verified_promotions() {
        assert!(plan_needs_verified_key(&promote_plan(Stage::Verified)));
        assert!(!plan_needs_verified_key(&promote_plan(Stage::Checked)));
        let empty = ExecutionPlan::default();
        assert!(!plan_needs_verified_key(&empty));
    }

    // --- functional tests over a seeded suite ---

    static JOURNAL_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct TestContext {
        _journal_guard: std::sync::MutexGuard<'static, ()>,
        _journal_tmp: tempfile::TempDir,
        suite: tempfile::TempDir,
    }

    impl TestContext {
        fn path(&self) -> &std::path::Path {
            self.suite.path()
        }
    }

    fn test_context() -> TestContext {
        let guard = JOURNAL_ENV_LOCK.lock().unwrap();
        let journal_tmp = tempfile::tempdir().unwrap();
        // SAFETY: serialized by `JOURNAL_ENV_LOCK`.
        unsafe {
            std::env::set_var("EINMO_JOURNAL_DIR", journal_tmp.path());
        }
        TestContext {
            _journal_guard: guard,
            _journal_tmp: journal_tmp,
            suite: tempfile::tempdir().unwrap(),
        }
    }

    struct Echo;
    impl einmo::Evaluator for Echo {
        fn evaluate(&self, source: &str) -> std::result::Result<Vec<String>, String> {
            Ok(vec![source.trim().to_string()])
        }
    }

    fn write_input(dir: &Path, rel: &str, content: &str) {
        let path = dir.join("input").join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn seeded_suite() -> TestContext {
        let ctx = test_context();
        write_input(ctx.path(), "a.foo", "{1+1;}");
        let config = TestConfig::new(ctx.path(), ValidationLevel::Output);
        let suite = einmo::EinmoSuite::new(config);
        suite.evaluate_all(&Echo).unwrap();
        ctx
    }

    fn session_args(work_dir: &Path, session: &str) -> SessionArgs {
        SessionArgs {
            work_dir: work_dir.to_path_buf(),
            session: Some(session.to_string()),
        }
    }

    fn serve_args(suite: &Path, socket: PathBuf) -> ServeArgs {
        ServeArgs {
            socket,
            private: false,
            tcp: None,
            token: None,
            allow_insecure_tcp_verified_passphrase: false,
            suite: suite.to_path_buf(),
        }
    }

    /// The suite lockfile (`EIMP-1` §S.5, plan Phase C's "Suite lockfile")
    /// must refuse `run_serve` outright when another server already holds
    /// the same suite — proven here by pre-acquiring the lock ourselves
    /// (rather than actually running a second server and racing `Ctrl-C`,
    /// which `run_serve`'s own shutdown path has no test-injectable hook
    /// for). `std::process::ExitCode` has no public equality check on
    /// stable Rust (the same limitation `cli.rs`'s `flags_fail_the_gate`
    /// note already documents), so this asserts the refusal's actual
    /// consequence — no socket ever bound — rather than the exit code
    /// value itself.
    #[tokio::test]
    async fn run_serve_refuses_when_suite_lock_is_held() {
        let tmp = seeded_suite();
        let socket_dir = tempfile::tempdir().unwrap();
        let held_socket = socket_dir.path().join("held.sock");
        let listener = std::os::unix::net::UnixListener::bind(&held_socket).unwrap();
        let _keep_alive = listener; // must stay bound for the probe to see it as live
        let _lock = einmo::SuiteLock::acquire(tmp.path(), &held_socket).unwrap();

        let losing_socket = socket_dir.path().join("losing.sock");
        let _ = run_serve(serve_args(tmp.path(), losing_socket.clone())).await;

        assert!(
            !losing_socket.exists(),
            "a suite-lock refusal must never even attempt to bind a socket"
        );
        assert!(
            !session_file_path(&losing_socket).exists(),
            "a suite-lock refusal must never write a session file either"
        );
    }

    #[test]
    fn decide_then_undecide_across_separate_calls_survives_journal_replay() {
        // Each cmd_* call opens and drops its own EinmoReview -- exactly
        // like two separate process invocations sharing a --session id
        // (EIMP-1 S.6).
        let tmp = seeded_suite();
        let session = "undecide-chain";

        cmd_decide(DecideArgs {
            session: session_args(tmp.path(), session),
            id: "a.foo".to_string(),
            decision: vec!["skip".to_string()],
            json: false,
        })
        .unwrap();

        let review = EinmoReview::resume(tmp.path(), session, ReviewOpts::default()).unwrap();
        let id = EinmoId::try_from("a.foo").unwrap();
        assert_eq!(review.decision(&id), Some(Decision::Skip));
        drop(review);

        cmd_undecide(UndecideArgs {
            session: session_args(tmp.path(), session),
            id: "a.foo".to_string(),
            json: false,
        })
        .unwrap();

        let review = EinmoReview::resume(tmp.path(), session, ReviewOpts::default()).unwrap();
        assert_eq!(review.decision(&id), None);
    }

    #[test]
    fn list_reflects_a_pending_decision_across_a_resumed_session() {
        let tmp = seeded_suite();
        let session = "list-chain";
        cmd_decide(DecideArgs {
            session: session_args(tmp.path(), session),
            id: "a.foo".to_string(),
            decision: vec![
                "promote".to_string(),
                "to".to_string(),
                "checked".to_string(),
            ],
            json: false,
        })
        .unwrap();

        assert!(
            cmd_list(ListArgs {
                session: session_args(tmp.path(), session),
                filter: None,
                differing: false,
                json: false,
            })
            .is_ok()
        );

        let review = EinmoReview::resume(tmp.path(), session, ReviewOpts::default()).unwrap();
        let id = EinmoId::try_from("a.foo").unwrap();
        assert_eq!(
            review.decision(&id),
            Some(Decision::Promote { to: Stage::Checked })
        );
    }

    #[test]
    fn plan_over_a_resumed_session_is_ok_with_no_side_effects() {
        let tmp = seeded_suite();
        assert!(
            cmd_plan(PlanArgs {
                session: SessionArgs {
                    work_dir: tmp.path().to_path_buf(),
                    session: None,
                },
                json: false,
            })
            .is_ok()
        );
    }

    #[test]
    fn execute_without_confirm_refuses_a_pending_promotion_and_touches_nothing() {
        let tmp = seeded_suite();
        let session = "confirm-refused";
        cmd_decide(DecideArgs {
            session: session_args(tmp.path(), session),
            id: "a.foo".to_string(),
            decision: vec![
                "promote".to_string(),
                "to".to_string(),
                "checked".to_string(),
            ],
            json: false,
        })
        .unwrap();

        let err = cmd_execute(ExecuteArgs {
            session: session_args(tmp.path(), session),
            confirm: String::new(),
            passphrase: None,
            stdin_passphrase: false,
            interactive: false,
            json: false,
        })
        .unwrap_err();
        assert!(matches!(err, EinmoError::Config(_)));
        assert!(!tmp.path().join("checked").join("a.foo.einmo").exists());
    }

    #[test]
    fn execute_with_confirm_applies_a_checked_promotion_needing_no_passphrase() {
        let tmp = seeded_suite();
        let session = "confirm-applied";
        cmd_decide(DecideArgs {
            session: session_args(tmp.path(), session),
            id: "a.foo".to_string(),
            decision: vec![
                "promote".to_string(),
                "to".to_string(),
                "checked".to_string(),
            ],
            json: false,
        })
        .unwrap();

        cmd_execute(ExecuteArgs {
            session: session_args(tmp.path(), session),
            confirm: "PROMOTE".to_string(),
            passphrase: None,
            stdin_passphrase: false,
            interactive: false,
            json: false,
        })
        .unwrap();

        assert!(tmp.path().join("checked").join("a.foo.einmo").exists());
    }

    #[test]
    fn execute_to_verified_uses_the_explicit_passphrase_never_prompting() {
        let tmp = seeded_suite();
        let session = "verified-with-passphrase";
        // checked must exist before verified can promote from it.
        cmd_decide(DecideArgs {
            session: session_args(tmp.path(), session),
            id: "a.foo".to_string(),
            decision: vec![
                "promote".to_string(),
                "to".to_string(),
                "checked".to_string(),
            ],
            json: false,
        })
        .unwrap();
        cmd_execute(ExecuteArgs {
            session: session_args(tmp.path(), session),
            confirm: "PROMOTE".to_string(),
            passphrase: None,
            stdin_passphrase: false,
            interactive: false,
            json: false,
        })
        .unwrap();

        cmd_decide(DecideArgs {
            session: session_args(tmp.path(), session),
            id: "a.foo".to_string(),
            decision: vec![
                "promote".to_string(),
                "to".to_string(),
                "verified".to_string(),
            ],
            json: false,
        })
        .unwrap();
        cmd_execute(ExecuteArgs {
            session: session_args(tmp.path(), session),
            confirm: "PROMOTE".to_string(),
            passphrase: Some("s3cr3t".to_string()),
            stdin_passphrase: false,
            interactive: false,
            json: false,
        })
        .unwrap();

        assert!(tmp.path().join("verified").join("a.foo.einmo").exists());
    }

    /// `EIMP-1` Phase B's second checkbox: `review execute`'s promotion must
    /// match `einmo promote`'s, at the CLI surface (not just the library
    /// level `EIMP-2` Phase C already proved in `review.rs`'s
    /// `execute_promote_matches_cli_promote_byte_for_byte`).
    #[test]
    fn cli_execute_promote_matches_einmo_promote_cli_at_the_cli_surface() {
        // Baseline: the existing `einmo promote` CLI path, via the exported
        // `einmo::cli_main` entry point -- the actual binary users run.
        let tmp_baseline = seeded_suite();
        let _ = einmo::cli_main(vec![
            OsString::from("einmo"),
            OsString::from("promote"),
            OsString::from("output"),
            OsString::from("to"),
            OsString::from("checked"),
            tmp_baseline.path().as_os_str().to_os_string(),
        ]);
        let baseline_path = tmp_baseline.path().join("checked").join("a.foo.einmo");
        assert!(
            baseline_path.exists(),
            "einmo promote must have written checked/a.foo.einmo"
        );

        // The review path: `decide` then `execute`, chained across two
        // separate cmd_* calls sharing one --session id -- exactly how two
        // separate `einmo-review-server` process invocations chain
        // (EIMP-1 S.6).
        let tmp_review = seeded_suite();
        let session = "cli-equivalence";
        cmd_decide(DecideArgs {
            session: session_args(tmp_review.path(), session),
            id: "a.foo".to_string(),
            decision: vec![
                "promote".to_string(),
                "to".to_string(),
                "checked".to_string(),
            ],
            json: false,
        })
        .unwrap();
        cmd_execute(ExecuteArgs {
            session: session_args(tmp_review.path(), session),
            confirm: "PROMOTE".to_string(),
            passphrase: None,
            stdin_passphrase: false,
            interactive: false,
            json: false,
        })
        .unwrap();
        let review_path = tmp_review.path().join("checked").join("a.foo.einmo");
        assert!(review_path.exists());

        // Timestamps legitimately differ between two independent runs (each
        // stamp signs its own generation time), so compare body sections,
        // not raw bytes -- the same approach review.rs's own
        // execute_promote_matches_cli_promote_byte_for_byte test takes.
        let baseline_file = einmo::EinmoFile::from_file(&baseline_path).unwrap();
        let review_file = einmo::EinmoFile::from_file(&review_path).unwrap();
        assert_eq!(
            body_sections_excluding_stamps(&baseline_file),
            body_sections_excluding_stamps(&review_file),
            "review execute's promotion body must match einmo promote's"
        );
    }

    fn body_sections_excluding_stamps(file: &einmo::EinmoFile) -> Vec<(String, String)> {
        file.sections()
            .iter()
            .filter(|s| !s.name().eq_ignore_ascii_case("STAMPS"))
            .map(|s| (s.name().to_string(), s.body().to_string()))
            .collect()
    }
}
