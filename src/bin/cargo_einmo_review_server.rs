//! `cargo-einmo-review-server` — the cargo-subcommand alias binary.
//!
//! `cargo install einmo` also installs this alias so that
//! `cargo einmo-review-server …` works identically to
//! `einmo-review-server …`. Cargo invokes a subcommand binary as
//! `cargo-einmo-review-server einmo-review-server <args…>`, so this alias
//! strips the injected subcommand name and delegates.

use std::process::Command;

fn main() -> std::process::ExitCode {
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if args
        .get(1)
        .map(|a| a == "einmo-review-server")
        .unwrap_or(false)
    {
        args.remove(1);
    }
    // Re-exec the real binary rather than duplicating its async main: this
    // alias just needs to forward argv, and the sibling binary already
    // owns the #[tokio::main] entry point.
    let exe = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("einmo-review-server")))
        .unwrap_or_else(|| "einmo-review-server".into());
    match Command::new(exe).args(&args[1..]).status() {
        Ok(status) if status.success() => std::process::ExitCode::SUCCESS,
        _ => std::process::ExitCode::FAILURE,
    }
}
