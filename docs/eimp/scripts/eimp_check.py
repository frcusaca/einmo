#!/usr/bin/env python3
"""
eimp_check.py — Utility for managing EIMP numbering.

EIMPs use a little-endian numbering convention. The filename digits ARE
the identifier — written front-to-back. The chronological order is
recovered by reversing the digits and reading as decimal (this is the
sort key, stored in the frontmatter `eimp:` field).

    Filename       Identifier      Sort key (chronological order)
    EIMP-0.md      EIMP-0          (pinned; excluded from the sequence)
    EIMP-1.md      EIMP-1          1
    EIMP-2.md      EIMP-2          2
    ...
    EIMP-9.md      EIMP-9          9
    EIMP-01.md     EIMP-01         10
    EIMP-11.md     EIMP-11         11
    EIMP-21.md     EIMP-21         12
    EIMP-31.md     EIMP-31         13
    EIMP-41.md     EIMP-41         14
    EIMP-51.md     EIMP-51         15
    EIMP-61.md     EIMP-61         16
    EIMP-71.md     EIMP-71         17
    EIMP-81.md     EIMP-81         18
    EIMP-91.md     EIMP-91         19
    EIMP-02.md     EIMP-02         20
    EIMP-12.md     EIMP-12         21
    ...

EIMP-0 is the process meta-document (the einmo analogue of FOOP-1). It is
pinned at 0, outside the normal 1-indexed little-endian sequence, and is
excluded from the consecutive-numbering check the same way EIMP-template.md
and INDEX.md are.

Per eimp.md:
- In free text, use the space form: "EIMP 51" (no dash).
- In filenames, code references, and formal citations, use the dash
  form: "EIMP-51".
- Always use the FILENAME DIGITS as the identifier — never use the sort
  key as the identifier in prose.

This script provides:

  check       — verify all EIMPs (excluding EIMP-0) are consecutive (no gaps).
  get_last    — print the identifier of the most-recently-created EIMP.
  gen_next    — print the identifier for the next EIMP to be created.
  list        — list all EIMPs in chronological order, oldest to newest.

Usage:
    python3 eimp_check.py check
    python3 eimp_check.py get_last
    python3 eimp_check.py gen_next
    python3 eimp_check.py list
"""

import argparse
import re
import sys
from pathlib import Path


EIMP_DIR = Path(__file__).resolve().parent.parent
FILENAME_RE = re.compile(r"^EIMP-(\d+)\.md$")
PINNED_META_FILENAME = "EIMP-0.md"


def filename_to_sort_key(filename: str) -> int | None:
    """
    Convert an EIMP filename to its chronological sort key.

    The filename has the form EIMP-<digits>.md where <digits> are the
    identifier written little-endian (front-to-back). The sort key is
    obtained by reversing the digits and reading as decimal.

    Returns None if the filename does not match the EIMP pattern.

    Examples:
        EIMP-1.md   -> 1
        EIMP-9.md   -> 9
        EIMP-01.md  -> 10
        EIMP-11.md  -> 11
        EIMP-31.md  -> 13
        EIMP-51.md  -> 15
    """
    m = FILENAME_RE.match(filename)
    if not m:
        return None
    digits = m.group(1)
    return int(digits[::-1])


def sort_key_to_filename_digits(sort_key: int) -> str:
    """
    Convert a sort key to the filename's little-endian digits.

    The digits are the decimal representation of sort_key, reversed.

    Examples:
        1  -> "1"
        9  -> "9"
        10 -> "01"
        11 -> "11"
        13 -> "31"
        15 -> "51"
        20 -> "02"
        21 -> "12"
    """
    return str(sort_key)[::-1]


def find_eimps(eimp_dir: Path) -> list[tuple[int, str]]:
    """
    Find all numbered EIMP files (sort key >= 1) in eimp_dir.

    Returns a list of (sort_key, filename) pairs sorted by sort_key
    ascending (chronological order). Excludes plan files (.plan.md),
    the template, INDEX.md, and the pinned EIMP-0 meta-document.
    """
    found: list[tuple[int, str]] = []
    for entry in eimp_dir.iterdir():
        if not entry.is_file():
            continue
        if entry.name.endswith(".plan.md"):
            continue
        if entry.name == "EIMP-template.md":
            continue
        if entry.name == "INDEX.md":
            continue
        if entry.name == PINNED_META_FILENAME:
            continue
        k = filename_to_sort_key(entry.name)
        if k is None:
            continue
        found.append((k, entry.name))
    found.sort(key=lambda pair: pair[0])
    return found


def filename_to_identifier(filename: str) -> str:
    """
    Extract the EIMP identifier from a filename.

    The identifier is "EIMP-<digits>" where <digits> are the
    little-endian digits in the filename.

    Examples:
        EIMP-1.md   -> "EIMP-1"
        EIMP-01.md  -> "EIMP-01"
        EIMP-31.md  -> "EIMP-31"
        EIMP-51.md  -> "EIMP-51"
    """
    return filename[: -len(".md")]


def cmd_check(eimp_dir: Path) -> int:
    """
    Verify EIMPs are consecutive in chronological order (no gaps).

    EIMP-0, if present, is checked for existence but not for sequence
    membership. Returns 0 on success, 1 if gaps or duplicates are found.
    """
    problems: list[str] = []

    if not (eimp_dir / PINNED_META_FILENAME).is_file():
        problems.append(f"Missing pinned meta-document: {PINNED_META_FILENAME}")

    eimps = find_eimps(eimp_dir)
    if not eimps and not problems:
        print(f"No EIMPs found in {eimp_dir}", file=sys.stderr)
        return 1

    seen: dict[int, list[str]] = {}
    for k, fn in eimps:
        seen.setdefault(k, []).append(fn)

    for k, names in sorted(seen.items()):
        if len(names) > 1:
            ids = [filename_to_identifier(n) for n in names]
            problems.append(
                f"Duplicate sort key {k}: {', '.join(ids)} ({', '.join(names)})"
            )

    sort_keys = sorted(seen.keys())
    expected = 1
    for k in sort_keys:
        while expected < k:
            expected_filename = f"EIMP-{sort_key_to_filename_digits(expected)}.md"
            expected_id = filename_to_identifier(expected_filename)
            problems.append(
                f"Missing EIMP at sort key {expected}: expected {expected_id} ({expected_filename})"
            )
            expected += 1
        expected = k + 1

    if problems:
        print("EIMP numbering problems:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        print(file=sys.stderr)
        if eimps:
            last_id = filename_to_identifier(eimps[-1][1])
            print(
                f"Found {len(eimps)} numbered EIMP files; most recent is {last_id} "
                f"(sort key {sort_keys[-1]}).",
                file=sys.stderr,
            )
        return 1

    last_id = filename_to_identifier(eimps[-1][1])
    first_id = filename_to_identifier(eimps[0][1])
    print(
        f"OK: EIMP-0 present; {len(eimps)} numbered EIMPs, consecutive sort keys 1 "
        f"through {sort_keys[-1]}. Range: {first_id} (oldest) through {last_id} (most recent)."
    )
    return 0


def cmd_get_last(eimp_dir: Path) -> int:
    """Print the identifier of the most-recently-created numbered EIMP."""
    eimps = find_eimps(eimp_dir)
    if not eimps:
        print("No numbered EIMPs found", file=sys.stderr)
        return 1
    k, fn = eimps[-1]
    ident = filename_to_identifier(fn)
    print(f"{ident}\t{fn}\t(sort key {k})")
    return 0


def cmd_gen_next(eimp_dir: Path) -> int:
    """Print the identifier and filename for the next EIMP to be created."""
    eimps = find_eimps(eimp_dir)
    next_k = (eimps[-1][0] + 1) if eimps else 1
    digits = sort_key_to_filename_digits(next_k)
    filename = f"EIMP-{digits}.md"
    ident = filename_to_identifier(filename)
    print(f"{ident}\t{filename}\t(sort key {next_k})")
    return 0


def cmd_list(eimp_dir: Path) -> int:
    """List all EIMPs in chronological order (EIMP-0 first, if present)."""
    printed_any = False
    if (eimp_dir / PINNED_META_FILENAME).is_file():
        print(f"EIMP-0\t{PINNED_META_FILENAME}\t(pinned meta-document)")
        printed_any = True

    eimps = find_eimps(eimp_dir)
    for k, fn in eimps:
        ident = filename_to_identifier(fn)
        print(f"{ident}\t{fn}\t(sort key {k})")
        printed_any = True

    if not printed_any:
        print("No EIMPs found", file=sys.stderr)
        return 1
    return 0


COMMANDS = {
    "check": cmd_check,
    "get_last": cmd_get_last,
    "gen_next": cmd_gen_next,
    "list": cmd_list,
}


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check and manage EIMP numbering.",
        epilog="Filenames use little-endian digits; the digits reversed "
        "and read as decimal give the identifier. EIMP-0 is a pinned "
        "meta-document excluded from the 1-indexed sequence.",
    )
    parser.add_argument(
        "command",
        choices=sorted(COMMANDS.keys()),
        help="What to do.",
    )
    parser.add_argument(
        "--dir",
        type=Path,
        default=EIMP_DIR,
        help=f"EIMP directory (default: {EIMP_DIR}).",
    )
    args = parser.parse_args()

    if not args.dir.is_dir():
        print(f"Not a directory: {args.dir}", file=sys.stderr)
        return 2

    return COMMANDS[args.command](args.dir)


if __name__ == "__main__":
    sys.exit(main())
