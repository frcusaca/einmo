import os
import sys
from pathlib import Path
from stage import Stage
from format import DEFAULT_SEPARATOR
from signature import Stamps, computer_keypair
from commands_verify import normalize_files, get_files_to_check
from commands_ro import read_einmo_file

def cmd_promote(args_list: list[str], filter_str: str, passphrase: str, stdin_passphrase: bool, interactive: bool, walk_depth_limit: int, json_out: bool):
    # Parse args list: from to dest [files...] OR from:to dest [files...]
    from_stage = None
    to_stage = None
    work_dir = None
    files = []

    if len(args_list) >= 3 and args_list[1].lower() == "to":
        from_stage = Stage.parse(args_list[0])
        to_stage = Stage.parse(args_list[2])
        if len(args_list) < 4:
            print("einmo: missing work directory", file=sys.stderr)
            return 1
        work_dir = Path(args_list[3])
        files = [Path(f) for f in args_list[4:]]
    else:
        # Glued
        glued = args_list[0]
        for sep in [":", ".."]:
            if sep in glued:
                f, t = glued.split(sep, 1)
                from_stage = Stage.parse(f)
                to_stage = Stage.parse(t)
                break
        if not from_stage:
            print(f"einmo: transition {glued} must be `<from> to <to>`", file=sys.stderr)
            return 1
        if len(args_list) < 2:
            print("einmo: missing work directory", file=sys.stderr)
            return 1
        work_dir = Path(args_list[1])
        files = [Path(f) for f in args_list[2:]]

    files = normalize_files(files, work_dir)
    to_check = get_files_to_check(work_dir, files, from_stage.dir_name())

    if filter_str:
        to_check = [f for f in to_check if filter_str in str(f)]

    # Key resolution
    import signature
    from getpass import getpass

    sk = None
    if interactive:
        pass1 = getpass("einmo passphrase: ")
        pass2 = getpass("einmo passphrase (again): ")
        if pass1 == pass2:
            sk, _ = signature.derive_keypair(pass1)
        else:
            print("einmo: passphrases did not match", file=sys.stderr)
            return 1
    elif stdin_passphrase:
        pass1 = sys.stdin.readline().strip()
        sk, _ = signature.derive_keypair(pass1)
    elif passphrase is not None:
        sk, _ = signature.derive_keypair(passphrase)
    elif "EINMO_PASSPHRASE" in os.environ:
        sk, _ = signature.derive_keypair(os.environ["EINMO_PASSPHRASE"])
    else:
        # Default computer key for output -> checked if unspecified
        # For checked -> verified, should prompt if no passphrase provided
        if to_stage == Stage.VERIFIED:
            pass1 = getpass("einmo passphrase: ")
            pass2 = getpass("einmo passphrase (again): ")
            if pass1 == pass2:
                sk, _ = signature.derive_keypair(pass1)
            else:
                return 1
        else:
            sk, _ = computer_keypair()

    promoted = 0
    non_human = 0

    for src_path in to_check:
        try:
            rel_path = src_path.relative_to(work_dir / from_stage.dir_name())
        except ValueError:
            rel_path = src_path

        dest_dir = work_dir / to_stage.dir_name() / rel_path.parent
        dest_dir.mkdir(parents=True, exist_ok=True)
        dest_path = work_dir / to_stage.dir_name() / rel_path

        try:
            einmo = read_einmo_file(src_path)

            # append stamp
            prefix = einmo.stamps.prefix_for_next_stamp(einmo.signed_prefix())
            einmo.stamps.append_stage(f"stage:{to_stage.dir_name()}", sk, prefix)

            with open(dest_path, 'wb') as f:
                f.write(einmo.serialize())

            promoted += 1
            if signature.is_computer_key(sk.verify_key.encode().hex()) and to_stage == Stage.VERIFIED:
                print(f"einmo: warning: {rel_path} verified under a well-known computer key (non-human attestation)", file=sys.stderr)
                non_human += 1
        except Exception as e:
            print(f"Failed to promote {src_path}: {e}", file=sys.stderr)

    if json_out:
        print(f'{{"promoted":{promoted},"non_human":{non_human}}}')
    else:
        print(f"promoted {promoted} file(s) {from_stage.dir_name()} to {to_stage.dir_name()}")

    return 0

def cmd_flag(work_dir: Path, stage_str: str, files: list[Path], filter_str: str, reason: str, walk_depth_limit: int, json_out: bool):
    stage = Stage.parse(stage_str)
    files = normalize_files(files, work_dir)
    to_check = get_files_to_check(work_dir, files, stage.dir_name())
    if filter_str:
        to_check = [f for f in to_check if filter_str in str(f)]

    from signature import now_iso8601

    flagged_count = 0

    for src_path in to_check:
        try:
            rel_path = src_path.relative_to(work_dir / stage.dir_name())
        except ValueError:
            rel_path = src_path

        dest_dir = work_dir / stage.dir_name() / "flagged" / rel_path.parent
        dest_dir.mkdir(parents=True, exist_ok=True)
        dest_path = work_dir / stage.dir_name() / "flagged" / rel_path

        try:
            einmo = read_einmo_file(src_path)
            adv = f"# flagged: {reason} {now_iso8601()}" if reason else f"# flagged: {now_iso8601()}"
            if einmo.advisory:
                einmo.advisory += "\n" + adv
            else:
                einmo.advisory = adv

            if dest_path.exists():
                # Append timestamp to name
                ts = now_iso8601().replace(':', '')
                dest_path = dest_path.with_name(f"{dest_path.stem}.{ts}.einmo")

            with open(dest_path, 'wb') as f:
                f.write(einmo.serialize())

            os.remove(src_path)
            flagged_count += 1
        except Exception as e:
            print(f"Failed to flag {src_path}: {e}", file=sys.stderr)

    if json_out:
        print(f'{{"flagged":{flagged_count}}}')
    else:
        print(f"flagged {flagged_count} file(s) from {stage.dir_name()}")

    return 0

def cmd_retract(work_dir: Path, stage_str: str, files: list[Path], filter_str: str, walk_depth_limit: int, json_out: bool):
    stage = Stage.parse(stage_str)
    files = normalize_files(files, work_dir)
    to_check = get_files_to_check(work_dir, files, stage.dir_name())
    if filter_str:
        to_check = [f for f in to_check if filter_str in str(f)]

    retracted = []

    for src_path in to_check:
        try:
            os.remove(src_path)
            try:
                rel = src_path.relative_to(work_dir / stage.dir_name())
            except:
                rel = src_path
            retracted.append((stage.dir_name(), rel))
        except:
            pass

    if json_out:
        print(f'{{"retracted":{len(retracted)}}}')
    else:
        print(f"retracted {len(retracted)} artifact(s):")
        for s, r in retracted:
            print(f"  {s}/{r}")

    return 0

def cmd_compare(stage_a: str, stage_b: str, work_dir: Path, files: list[Path], require_comments_match: bool, require_match: bool, root_cause: bool, walk_depth_limit: int, json_out: bool):
    sa = Stage.parse(stage_a)
    sb = Stage.parse(stage_b)
    files = normalize_files(files, work_dir)

    # Needs to walk mirrored tree, for simplicity just gather all relative paths from both
    all_rels = set()
    for s in [sa, sb]:
        sdir = work_dir / s.dir_name()
        if not sdir.exists(): continue
        for root, _, fs in os.walk(sdir):
            for f in fs:
                if f.endswith('.einmo'):
                    all_rels.add(Path(root).relative_to(sdir) / f)

    if files:
        all_rels = set([Path(f) for f in files])

    matching = []
    differing = []
    only_a = []
    only_b = []
    tampered = []

    for rel in all_rels:
        pa = work_dir / sa.dir_name() / rel
        pb = work_dir / sb.dir_name() / rel

        ea = eb = None
        ta = tb = False
        if pa.exists():
            try: ea = read_einmo_file(pa)
            except: ta = True
        if pb.exists():
            try: eb = read_einmo_file(pb)
            except: tb = True

        if ta or tb:
            tampered.append(rel)
            continue

        if ea and not eb:
            only_a.append(rel)
        elif not ea and eb:
            only_b.append(rel)
        elif ea and eb:
            # check matching
            same = True
            sections_to_check = ["INPUT", "OUTPUT"]
            if require_comments_match:
                sections_to_check.append("COMMENTS")

            for sec in sections_to_check:
                seca = next((s.body for s in ea.sections if s.name == sec), None)
                secb = next((s.body for s in eb.sections if s.name == sec), None)
                if seca != secb:
                    same = False
                    break

            if same:
                matching.append(rel)
            else:
                differing.append((rel, ["INPUT", "OUTPUT"]))

    if json_out:
        print(f'{{"matching":{len(matching)},"differing":{len(differing)},"only_in_a":{len(only_a)},"only_in_b":{len(only_b)},"tampered":{len(tampered)}}}')
    else:
        print(f"{sa.dir_name()} vs {sb.dir_name()}: {len(matching)} matching, {len(differing)} differing, {len(only_a)} only-in-{sa.dir_name()}, {len(only_b)} only-in-{sb.dir_name()}, {len(tampered)} tampered")
        for r, sects in differing:
            print(f"  differing {r} [{', '.join(sects)}]")

    if require_match and (differing or only_a or only_b or tampered):
        print(f"einmo: {sa.dir_name()} does not match {sb.dir_name()}.", file=sys.stderr)
        print("  burden: the producer of the divergent output must repair or escalate.", file=sys.stderr)
        return 1

    return 0
