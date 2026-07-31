# zweimomo (JavaScript-only slice)

Einmo's companion test/demo crate: a pure-Rust JavaScript interpreter
(`boa_engine`) wrapped as an [`einmo::Evaluator`], exercising einmo's
signed-snapshot pipeline against real, previously-reviewed test fixtures.

This is a **demo and debugging tool**, not a library for external use —
`publish = false` in `Cargo.toml`. Ported from `foolish-rust`'s original
three-language `zweimomo` crate (Foolish/Python/JavaScript); see
`docs/eimp/EIMP-2.md` §8 in the repo root for the full port rationale.
Unlike its origin crate, this slice has no `foolish-ubca`/`foolish-core`
dependency (no cross-repo coupling) and no `rustpython-vm`.

## Progressive-difficulty tiers

`suites/javascript/` is organized into tiers named by elapsed time since a
reviewer starts working with einmo, each its own independently-gated
`EinmoSuite` with its own `input/`/`output/`/`checked/`/`verified/` tree and
its own `README.<tier>.md`:

| Tier | Status | Covers |
|---|---|---|
| [`day.1/`](suites/javascript/day.1/README.day.1.md) | populated | The absolute basics — does einmo work end to end on simple content. |
| [`week.2/`](suites/javascript/week.2/README.week.2.md) | design notes only | Mass and randomized re-review (`experimental_reviewer.sh -D`/`-s`). |
| [`month.2/`](suites/javascript/month.2/README.after.a.month.md) | design notes only | Multiple reviewers, conflicting decisions (`EIMP-1` §S.5). |
| [`years.later/`](suites/javascript/years.later/README.years.later.md) | reserved | Not yet designed. |

Run every populated tier's suite:

```bash
cargo test javascript_tiers_generate_and_verify
```

## Signing configuration

`day.1/checked/` is signed under a non-default `checked`-stage passphrase,
configured in `day.1/einmo.toml` (`[signing] checked = "…"`) rather than
documented here — passphrases are configuration, not documentation, even
for a demo crate (see the einmo root `README.md`'s "Configuration
Precedence" and "The default key in configuration" sections for the
`einmo.toml` `[signing]` format and the CLI/env/config/interactive
resolution cascade).

To regenerate `day.1/checked/` after changing `day.1/output/` (e.g. after
editing `BoaEvaluator` or an input):

```bash
cargo run -p einmo --bin einmo -- retract suites/javascript/day.1 checked
cargo run -p einmo --bin einmo -- promote output to checked suites/javascript/day.1
```

The passphrase is read from `day.1/einmo.toml` automatically — no
`--passphrase` flag needed on the command line.

## Reviewing with einmo-review-server

The review server (`einmo-review-server`) provides backend capabilities —
worklist management, decision tracking, verified-body caching, and
execution — exposed as a JSON API over unix-domain sockets. It supports
three review modes: **Full** (every case), **NewOrBroken** (only
mismatches), and **Random** (shuffled order for sampling).

The TUI client script (`einmo_review_client.sh`) is the user-facing tool.
It launches its own private server, passes the mode, drives the review in
vim, and tears everything down on exit.

### Reviewing zweimomo suites

```bash
# Full review of day.1 (every case):
./scripts/einmo_review_client.sh -p suites/javascript/day.1

# Only new or broken cases (skip anything already matching):
./scripts/einmo_review_client.sh -p suites/javascript/day.1 -n

# Filter to cases matching a substring:
./scripts/einmo_review_client.sh -p suites/javascript/day.1 "alarm"
```

### Vim keybindings

| Key | Action |
|---|---|
| `c` | Promote to checked |
| `v` | Promote to verified |
| `f` | Flag with a reason |
| `k` | Kick (retract from highest stage) |
| `u` | Undo decision |
| `\d` | Fetch server-side diff hunks (output vs checked) |
| `\D` | Toggle vim's built-in diff mode across all panes |
| `Enter` | Next case |
| `q` | Quit and show the execution plan |

At the end of the pass, the script shows the plan and asks you to type
`PROMOTE` to execute all pending promotions. If any promotion targets
`verified`, you'll be prompted for a passphrase.

### Session persistence and crash recovery

Every session writes an append-only JSONL journal. If the process crashes
mid-review, resume with `--session <id>` — the journal replays every
decision and the session picks up where it left off. The session id is
printed to stderr on every command.

### Multi-reviewer accumulation

Two reviewers can work the same suite concurrently with different session
ids. Each reviewer's promotions are independently signed — multiple
`stage:verified` stamps on the same file are accumulated attestation, not
a conflict. See `EIMP-1.md` §S.4a for the content-then-key decision
table.

---

## Last Updated

**Date**: 2026-07-31
**Updated By**: Sisyphus (mimo-v2.5-pro)
**Changes**: Added "Reviewing with einmo-review-server" section documenting
the review architecture (server as backend with mode capabilities, TUI as
user-facing tool), TUI client workflow for zweimomo suites, vim
keybindings, session persistence, crash recovery, and multi-reviewer
accumulation.
