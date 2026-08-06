import argparse
import sys
import os
from pathlib import Path

def parse_transition(s):
    if "to" in s.split():
        # handled differently if space separated, but as a single string fallback:
        pass
    for sep in ["..", ":"]:
        if sep in s:
            from_stage, to_stage = s.split(sep, 1)
            return from_stage.strip(), to_stage.strip()
    raise argparse.ArgumentTypeError(f"transition '{s}' must be '<from> to <to>' or '<from>:<to>'")

def main():
    parser = argparse.ArgumentParser(
        prog="einmo",
        description="Signed directory-based snapshot testing"
    )
    # Replicate clap's version handling simply:
    parser.add_argument("-V", "--version", action="version", version="einmo 0.0.5")

    subparsers = parser.add_subparsers(dest="command", required=True)

    # promote
    promote_parser = subparsers.add_parser("promote", help="Promote files between stages (appends the destination stage's stamp)")
    promote_parser.add_argument("args", nargs="+", help="The stage pair (`<from> to <to>` or glued), then work directory, then files")
    promote_parser.add_argument("--filter", help="Restrict to inputs matching this glob (`*` wildcard)")
    promote_parser.add_argument("--passphrase", help="Explicit passphrase (tier 1). Empty string = the computer key")
    promote_parser.add_argument("--stdin-passphrase", action="store_true", help="Read one passphrase line from stdin (tier 2)")
    promote_parser.add_argument("--interactive", action="store_true", help="Force the interactive prompt (skips tiers 1-4)")
    promote_parser.add_argument("--walk-depth-limit", type=int, help="Override the directory-walk depth limit (tier 1)")
    promote_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    # flag
    flag_parser = subparsers.add_parser("flag", help="Move files from a stage into flagged/ (advisory line, no stamp)")
    flag_parser.add_argument("work_dir", type=Path, help="The suite work directory")
    flag_parser.add_argument("stage", help="The stage to flag from")
    flag_parser.add_argument("files", nargs="*", type=Path, help="Specific .einmo files to act on. Use `-` to read paths from stdin")
    flag_parser.add_argument("--filter", help="Restrict to inputs matching this glob")
    flag_parser.add_argument("--reason", default="", help="The advisory reason recorded in the flagged file")
    flag_parser.add_argument("--walk-depth-limit", type=int, help="Override the directory-walk depth limit (tier 1)")
    flag_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    # retract
    retract_parser = subparsers.add_parser("retract", help="Retract (demote) artifacts from a stage; cascades checked→verified")
    retract_parser.add_argument("work_dir", type=Path, help="The suite work directory")
    retract_parser.add_argument("stage", help="The stage to retract from")
    retract_parser.add_argument("files", nargs="*", type=Path, help="Specific .einmo files to retract")
    retract_parser.add_argument("--filter", help="Restrict to inputs matching this glob")
    retract_parser.add_argument("--walk-depth-limit", type=int, help="Override the directory-walk depth limit")
    retract_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    # compare
    compare_parser = subparsers.add_parser("compare", help="Compare two stages over the mirrored tree")
    compare_parser.add_argument("stage_a", help="Stage A")
    compare_parser.add_argument("stage_b", help="Stage B")
    compare_parser.add_argument("work_dir", type=Path, help="The suite work directory")
    compare_parser.add_argument("files", nargs="*", type=Path, help="Specific .einmo files to act on")
    compare_parser.add_argument("--require-comments-match", action="store_true", help="Require COMMENTS to match too")
    compare_parser.add_argument("--require-match", action="store_true", help="Exit non-zero if any file differs")
    compare_parser.add_argument("--root-cause", action="store_true", help="Report the deepest differing descendants")
    compare_parser.add_argument("--walk-depth-limit", type=int, help="Override the directory-walk depth limit")
    compare_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    # verify
    verify_parser = subparsers.add_parser("verify", help="Verify signatures across a stage (or all stages)")
    verify_parser.add_argument("work_dir", type=Path, help="The suite work directory")
    verify_parser.add_argument("files", nargs="*", type=Path, help="Specific .einmo files to act on")
    verify_parser.add_argument("--level", default="checked", help="The escalating validation level to judge the suite at")
    verify_group = verify_parser.add_mutually_exclusive_group()
    verify_group.add_argument("--fail-fast", action="store_true", help="Stop at the first failure")
    verify_group.add_argument("--fail-at-end", action="store_true", help="Run every check and report all problems together")
    verify_parser.add_argument("--stage", help="Restrict to one stage")
    verify_parser.add_argument("--all", action="store_true", help="Verify all stages (the default)")
    verify_parser.add_argument("--walk-depth-limit", type=int, help="Maximum recursion depth for directory walks")
    verify_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")
    verify_parser.add_argument("--flag-is-not-failure", action="store_true", help="Downgrade a flagged artifact from failing the gate")

    # verify-signatures (legacy)
    verify_sig_parser = subparsers.add_parser("verify-signatures", help="Verify signatures on a path (legacy-compatible subcommand name)")
    verify_sig_parser.add_argument("work_dir", type=Path, help="The suite work directory")
    verify_sig_parser.add_argument("files", nargs="*", type=Path, help="Specific .einmo files to act on")
    verify_sig_parser.add_argument("--level", default="checked", help="The escalating validation level to judge the suite at")
    verify_sig_group = verify_sig_parser.add_mutually_exclusive_group()
    verify_sig_group.add_argument("--fail-fast", action="store_true", help="Stop at the first failure")
    verify_sig_group.add_argument("--fail-at-end", action="store_true", help="Run every check and report all problems together")
    verify_sig_parser.add_argument("--stage", help="Restrict to one stage")
    verify_sig_parser.add_argument("--all", action="store_true", help="Verify all stages (the default)")
    verify_sig_parser.add_argument("--walk-depth-limit", type=int, help="Maximum recursion depth for directory walks")
    verify_sig_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")
    verify_sig_parser.add_argument("--flag-is-not-failure", action="store_true", help="Downgrade a flagged artifact from failing the gate")

    # confirm-signatures
    confirm_parser = subparsers.add_parser("confirm-signatures", help="Report files carrying a signer whose pubkey starts with a prefix")
    confirm_parser.add_argument("path", type=Path, help="A directory (or file) of .einmo files")
    confirm_parser.add_argument("pubkey_prefix", nargs="?", help="The pubkey hex prefix to match")
    confirm_parser.add_argument("--from-passphrase", action="store_true", help="Derive the pubkey prefix from a passphrase instead of typing the hex")
    confirm_parser.add_argument("--require-all", action="store_true", help="Exit non-zero if any file lacks a matching signer")
    confirm_parser.add_argument("--walk-depth-limit", type=int, help="Override the directory-walk depth limit")
    confirm_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    # show
    show_parser = subparsers.add_parser("show", help="Show an envelope's summary and stamp chain")
    show_parser.add_argument("file", type=Path, help="The .einmo file to show")
    show_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    # list
    list_parser = subparsers.add_parser("list", help="List the suite's tests and which stages hold each one")
    list_parser.add_argument("work_dir", type=Path, help="The suite work directory")
    list_parser.add_argument("--filter", help="Only tests whose mirror-relative path contains this substring")
    list_parser.add_argument("--differing", action="store_true", help="Only tests whose stage bodies are not all identical")
    list_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    # body
    body_parser = subparsers.add_parser("body", help="Print an envelope's signed body sections (verify-on-inspect first)")
    body_parser.add_argument("file", type=Path, help="The .einmo file whose body to print")
    body_parser.add_argument("--section", help="Print only this section")
    body_parser.add_argument("--bare", action="store_true", help="Do not print `=== NAME ===` headers between sections")

    # self-check
    selfcheck_parser = subparsers.add_parser("self-check", help="Compute the SHA-256 of this binary (self-attestation)")
    selfcheck_parser.add_argument("--expected", help="Exit non-zero if the computed hash does not match this value")
    selfcheck_parser.add_argument("--quiet", action="store_true", help="Print only the hash")

    # evaluate
    eval_parser = subparsers.add_parser("evaluate", help="Evaluate inputs and write signed output files")
    eval_parser.add_argument("work_dir", type=Path, help="The suite work directory")
    eval_parser.add_argument("--command", required=True, help="The evaluator command")
    eval_parser.add_argument("--filter", help="Only evaluate inputs matching this substring")
    eval_parser.add_argument("--walk-depth-limit", type=int, help="Override the directory-walk depth limit")
    eval_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    # regenerate-output
    regen_parser = subparsers.add_parser("regenerate-output", help="Re-evaluate inputs and deliberately REPLACE any drifted `output/` baseline")
    regen_parser.add_argument("work_dir", type=Path, help="The suite work directory")
    regen_parser.add_argument("--command", required=True, help="The evaluator command")
    regen_parser.add_argument("--filter", help="Only evaluate inputs matching this substring")
    regen_parser.add_argument("--walk-depth-limit", type=int, help="Override the directory-walk depth limit")
    regen_parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON")

    args = parser.parse_args()

    import commands_ro

    import commands_mutate
    import commands_evaluate

    # Command dispatch will go here
    if args.command == "promote":
        sys.exit(commands_mutate.cmd_promote(args.args, args.filter, args.passphrase, args.stdin_passphrase, args.interactive, args.walk_depth_limit, args.json))
    elif args.command == "flag":
        sys.exit(commands_mutate.cmd_flag(args.work_dir, args.stage, args.files, args.filter, args.reason, args.walk_depth_limit, args.json))
    elif args.command == "retract":
        sys.exit(commands_mutate.cmd_retract(args.work_dir, args.stage, args.files, args.filter, args.walk_depth_limit, args.json))
    elif args.command == "compare":
        sys.exit(commands_mutate.cmd_compare(args.stage_a, args.stage_b, args.work_dir, args.files, args.require_comments_match, args.require_match, args.root_cause, args.walk_depth_limit, args.json))
    elif args.command == "verify" or args.command == "verify-signatures":
        import commands_verify
        sys.exit(commands_verify.cmd_verify(args.work_dir, args.level, args.fail_fast, args.fail_at_end, args.stage, args.all, args.files, args.walk_depth_limit, args.json, args.flag_is_not_failure))
    elif args.command == "confirm-signatures":
        import commands_verify
        sys.exit(commands_verify.cmd_confirm(args.path, args.pubkey_prefix, args.from_passphrase, args.require_all, args.walk_depth_limit, args.json))
    elif args.command == "show":
        sys.exit(commands_ro.cmd_show(args.file, args.json))
    elif args.command == "list":
        sys.exit(commands_ro.cmd_list(args.work_dir, args.filter, args.differing, args.json))
    elif args.command == "body":
        sys.exit(commands_ro.cmd_body(args.file, args.section, args.bare))
    elif args.command == "self-check":
        sys.exit(commands_ro.cmd_self_check(args.expected, args.quiet))
    elif args.command == "evaluate":
        sys.exit(commands_evaluate.cmd_evaluate(args.work_dir, args.command, args.filter, args.walk_depth_limit, args.json, regenerate=False))
    elif args.command == "regenerate-output":
        sys.exit(commands_evaluate.cmd_evaluate(args.work_dir, args.command, args.filter, args.walk_depth_limit, args.json, regenerate=True))

if __name__ == "__main__":
    main()
