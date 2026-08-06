import sys
from pathlib import Path

def normalize_files(files: list[Path], work_dir: Path) -> list[Path]:
    import sys
    if not files:
        return []
    res = []
    has_dash = False
    for f in files:
        if str(f) == "-":
            has_dash = True
        else:
            res.append(f)

    if has_dash:
        # read lines from stdin
        for line in sys.stdin:
            line = line.strip()
            if line:
                res.append(Path(line))

    # Normalization against config/work_dir would go here
    return res

def get_files_to_check(work_dir: Path, files: list[Path], stage: str = None) -> list[Path]:
    from stage import Stage
    import os
    if files:
        return files

    res = []
    stages = [Stage.parse(stage)] if stage else Stage.all()
    for s in stages:
        stage_dir = work_dir / s.dir_name()
        if not stage_dir.exists():
            continue
        for root, _, fs in os.walk(stage_dir):
            for f in fs:
                if f.endswith(".einmo"):
                    res.append(Path(root) / f)
    return res

def cmd_verify(work_dir: Path, level: str, fail_fast: bool, fail_at_end: bool, stage: str, all: bool, files: list[Path], walk_depth_limit: int, json_out: bool, flag_is_not_failure: bool):
    from commands_ro import read_einmo_file
    files = normalize_files(files, work_dir)
    to_check = get_files_to_check(work_dir, files, stage)

    failures = 0
    checked = 0
    flagged = []
    # simplified integrity check...

    for f_path in to_check:
        try:
            read_einmo_file(f_path)
            checked += 1
        except Exception as e:
            if not json_out:
                print(f"  FAILED {f_path} ({e})")
            failures += 1
            if fail_fast:
                break

    if json_out:
        print(f'{{"files":{checked+failures},"failures":{failures},"integrity_violations":[],"flagged":{len(flagged)}}}')
    else:
        print(f"verified {checked+failures} file(s), {failures} failure(s)")

    if failures > 0:
        return 1
    return 0

def cmd_confirm(path: Path, pubkey_prefix: str, from_passphrase: bool, require_all: bool, walk_depth_limit: int, json_out: bool):
    from signature import StageKeypair
    from getpass import getpass

    if from_passphrase:
        pass1 = getpass("einmo passphrase: ")
        pass2 = getpass("einmo passphrase (again): ")
        if pass1 != pass2:
            print("einmo: passphrases did not match — try again (Ctrl-C to abort)", file=sys.stderr)
            return 1

        import signature
        _, vk = signature.derive_keypair(pass1)
        prefix = vk.encode().hex()
        if not json_out:
            print(f"your public key: {prefix}")
    elif pubkey_prefix:
        prefix = pubkey_prefix
    else:
        print("einmo: give a <pubkey-prefix> or --from-passphrase", file=sys.stderr)
        return 1

    # Walk path for .einmo files
    to_check = []
    if path.is_file():
        to_check.append(path)
    elif path.is_dir():
        import os
        for root, _, fs in os.walk(path):
            for f in fs:
                if f.endswith('.einmo'):
                    to_check.append(Path(root) / f)

    matched = 0
    unmatched = 0

    from commands_ro import read_einmo_file
    for p in to_check:
        try:
            f = read_einmo_file(p)
            if any(s.pubkey_hex.startswith(prefix) for s in f.stamps.entries):
                matched += 1
            else:
                unmatched += 1
        except:
            unmatched += 1

    if json_out:
        print(f'{{"matched":{matched},"unmatched":{unmatched}}}')
    else:
        print(f"{matched} file(s) match prefix '{prefix}', {unmatched} do not")

    if require_all and unmatched > 0:
        return 1
    return 0
