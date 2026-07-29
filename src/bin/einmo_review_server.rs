//! `einmo-review-server` — the resident process hosting one `EinmoReview`
//! session over a unix-domain socket (EIMP-2 §7, `docs/eimp/`).
//!
//! Usage: `einmo-review-server [--socket <path>] <suite>`
//!
//! Binds `<path>` (default: `./.einmo-review.sock`, dot-named so einmo's own
//! walkers skip it), opens one session over `<suite>` against itself
//! (`POST /einmo/sessions`), writes `<socket-path>.session` next to the
//! socket containing the session id, and serves until `Ctrl-C`/`SIGTERM`.
//! Both files are removed on exit.

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "einmo-review-server",
    about = "Host one EinmoReview session over a unix-domain socket"
)]
struct Args {
    /// Where to bind the unix-domain socket.
    #[arg(long, default_value = ".einmo-review.sock")]
    socket: PathBuf,
    /// The suite work directory to open a review session over.
    suite: PathBuf,
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args = Args::parse();

    let state = Arc::new(einmo::AppState::default());
    let session = state.create_session(&args.suite);
    let app = einmo::router(state);

    let session_file = session_file_path(&args.socket);
    if let Err(e) = std::fs::write(&session_file, session.to_string()) {
        eprintln!(
            "einmo-review-server: failed to write session file {}: {e}",
            session_file.display()
        );
        return std::process::ExitCode::FAILURE;
    }

    eprintln!(
        "einmo-review-server: suite {} session {session} socket {}",
        args.suite.display(),
        args.socket.display()
    );

    let socket_for_cleanup = args.socket.clone();
    let session_file_for_cleanup = session_file.clone();
    let shutdown = async move {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("einmo-review-server: shutting down");
    };

    let result = einmo::serve_uds(app, &args.socket, shutdown).await;
    let _ = std::fs::remove_file(&session_file_for_cleanup);
    let _ = std::fs::remove_file(&socket_for_cleanup);

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("einmo-review-server: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

/// The session-id sidecar file's path: `<socket-path>.session`.
fn session_file_path(socket_path: &std::path::Path) -> PathBuf {
    let mut s = socket_path.as_os_str().to_os_string();
    s.push(".session");
    PathBuf::from(s)
}
