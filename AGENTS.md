# AI Agent Development Guide — einmo

This document provides instructions for AI agents (Claude Code, GitHub
Copilot, Cursor, and other AI coding assistants) working on **einmo**:
directory-based, cryptographically signed snapshot testing with a staged
promotion pipeline.

## Origin

einmo was extracted from the `foolish-rust` workspace (where it was
originally specified as `FOOP-92`) into this standalone repository so it
could be published to crates.io and reused outside the Foolish project. See
`docs/eimp/EIMP-0.md` for the full story and `README.md` for what einmo does
and how to use it.

## How To Write Rust Code

> ## ⛔ STOP — READ `rust_instructions.md` BEFORE TOUCHING ANY RUST ⛔
>
> **EVERY** coding agent — Claude Code, Copilot, Cursor, or any other —
> **MUST** read [`rust_instructions.md`](rust_instructions.md) at the
> repository root **before reading or writing a single line of Rust in this
> repository**, and **MUST** follow it. This is not optional and not
> negotiable.
>
> `rust_instructions.md` is the **single authoritative source** for how Rust
> is written here. It contains the full guidance — optimization priorities,
> ownership and borrowing, encapsulation, enum dispatch, error handling,
> einmo-specific rules (cryptographic/signing code, the `.einmo` envelope
> format, CLI dispatch), testing requirements, and the hard tooling gates
> (`cargo fmt`, `cargo clippy -D warnings`, tests). All of it lives there.

## Development process

Write tests first — unit tests for behavior, invariants, and edge cases, and
tests for the important/unclear corner cases specifically, so they document
what correct behavior looks like as well as check it. See
`rust_instructions.md` §7 "Testing" for what to cover.

## Development Rules

**NEVER** start substantive work while any test is broken. Fix it first
(`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
--check` must all be clean).

## Design and Planning — EIMP (Einmo Improvement Process)

EIMP documents are einmo's equivalent of Python's PEP or Rust's RFC — the
Foolish project's own process (FOOP) adapted for a small, single-maintainer
crate. Each EIMP is two files sharing the same `EIMP-<NUMBER>` stem:
`EIMP-#.md` (the **specification** — the what and why) and
`EIMP-#.plan.md` (the **plan** — a checkboxed, sequentially-executed
roadmap for the how). EIMP numbering is **little-endian** — the filename
digits ARE the identifier (EIMP-1 → EIMP-9 → EIMP-01 → EIMP-11 → EIMP-21…)
— with `EIMP-0` pinned outside that sequence as the process meta-document.
EIMPs progress through statuses: `Draft` → `Brewing` → `Final` →
`Implementing` → complete.

Unlike FOOP, EIMP has **no worktree/per-feature-branch stage** — einmo is
small enough that plans execute directly on `jia` with regular commits.

> **Primary branch is `jia`, not `main`.** The `main` branch has no meaning
> in this repository. All work, commits, and EIMP execution happen on `jia`.

- **Creating or planning an EIMP** → load the `eimp-write-plan` skill.
- **Finding, executing, backburnering, cancelling, or maintaining an
  EIMP** → load the `eimp-use-maintain` skill.
- `eimp.md` at the repository root is the authoritative reference; if a
  skill and `eimp.md` appear to disagree, `eimp.md` wins.

## Skills

| Skill | Scope | Load when… |
|-------|-------|------------|
| `eimp-write-plan` | Creating and planning EIMPs. Little-endian numbering, `eimp_check.py`, the spec template, plan construction rules, checkbox format, sub-tasks. | Creating a new EIMP, writing a specification, or constructing a plan (`EIMP-#.plan.md`). |
| `eimp-use-maintain` | Using and maintaining existing EIMPs. Listing/finding EIMPs, the status lifecycle, plan execution flow, checkbox lifecycle (complete with timestamp, backburnering, cancel/deprecate), comprehensive test verification, human communication protocol. | Finding, executing, resuming, backburnering, cancelling, or maintaining an existing EIMP. |

## Build Commands

```bash
cargo check                                      # Quick check (fastest validation)
cargo build                                      # Build everything
cargo build --release                            # Release build (LTO, stripped)
cargo test                                        # All tests
cargo clippy --all-targets -- -D warnings         # Lint gate
cargo fmt --check                                 # Format gate
```

Binaries after release: `target/release/einmo` and `target/release/cargo-einmo`.

## Clarifications

* Never directly edit files matching `checked/`, `verified/`, or any
  `.einmo` artifact by hand — those are signed envelopes; go through the
  `einmo` CLI (`einmo promote`, `einmo flag`) so the stamp chain stays
  valid.

## Documentation

- **`README.md`** — what einmo is, the `.einmo` file format, the CLI, the
  library API, and the testing-genre background.
- **`eimp.md`** — the EIMP process itself (numbering, file layout, plan
  construction, checkbox lifecycle).
- **`docs/eimp/`** — EIMP specs and plans (`EIMP-0.md` is the process
  meta-document; `EIMP-1.md`/`.plan.md` is `EinmoReview`, ported from the
  original `FOOP-25`).
- **`rust_instructions.md`** — how to write Rust in this repository
  (**required reading** before touching any Rust; see above).

## Markdown File Update Protocol

Whenever any AI agent modifies a `*.md` file in this repository, update the
"## Last Updated" section at the end of that file (or add one, if it
doesn't have one) with: current date (`YYYY-MM-DD`), agent identifier
(model name/version), and a brief summary of what changed. This mirrors the
convention `foolish-rust` (einmo's origin project) uses.

## Crash Stash

The crash stash is a mechansim we use currently to deal with hardware that
frequently reboot due to memory errors or California powergrid instabilities.
When the user calls for a crash-stash. It means to write a new file at the root of
the repo named "CRASH-STASH-UID.md", where UID is generated unique id. Update
top of the current EIMP AND AGENT.md to ask it to read this section and then the 
crash stash file. The text in AGENT.md and FOOP should be unignorable in the front
titled "# A Real Crash Stash, This is NOT a Test" In this section, agent will
write down the full extent of its knowledge regarding the project. what's been done.
What it's thinking about. What was tried what wasn't tried. What's next, etc. The
description can be simple as "finish the rest of EIMP-such-and-such" But in most
cases what's in the memory is important so write down items such as "make sure
to read rust instructions, user pointed out some issues that were clearly
documented in the instructions." Or "the code currently runs infinite loop,
heres what we've done to isolate it to this region of the code." Or "I've been
confused about two conflicting features, thoguht about issues A,B,C, but
probably best to think through D before asking user to clarify." Give clear
instructions to your self. Dump code snippets in code fences if code or pseudo code
is more clear.

## Last Updated

**Date**: 2026-07-31
**Updated By**: Sisyphus (mimo-v2.5-pro)
**Changes**: Updated primary branch from `main` to `jia` throughout (branch
name correction per maintainer). Added journal isolation to test modules to
prevent unbounded log accumulation.
repository.
