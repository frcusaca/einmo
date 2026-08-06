from typing import Optional
from pathlib import Path
from format import EinmoFile

def read_einmo_file(path: Path) -> EinmoFile:
    with open(path, 'rb') as f:
        data = f.read()
    file = EinmoFile.parse(data)
    if not file.stamps.chain_valid(file.signed_prefix()):
        raise ValueError(f"verify-on-inspect failed for {path}")
    return file

def cmd_show(file_path: Path, json_out: bool):
    try:
        einmo = read_einmo_file(file_path)
    except Exception as e:
        print(f"einmo: {e}")
        return 1

    meta = einmo.metadata
    if json_out:
        import json
        stamps_json = []
        for s in einmo.stamps.entries:
            stamps_json.append(json.dumps({"key": s.key, "pubkey": s.pubkey_hex}))
        stamps_str = ",".join(stamps_json)
        print(f'{{"test":"{meta.test}","status":"{meta.status.as_str()}","stamps":[{stamps_str}]}}')
    else:
        print(f"test:     {meta.test}")
        print(f"suite:    {meta.suite}")
        print(f"producer: {meta.producer}")
        print(f"status:   {meta.status.as_str()}")
        if meta.reference:
            print(f"reference: {meta.reference}")
        print(f"sections: {', '.join(meta.sections)}")
        print("stamps:")
        for s in einmo.stamps.entries:
            short_key = s.pubkey_hex[:8]
            print(f"  {s.key} pubkey={short_key}… {s.timestamp} [{s.produced_by}]")
        if einmo.advisory:
            print(f"advisory: {einmo.advisory}")
    return 0

def cmd_body(file_path: Path, section: Optional[str], bare: bool):
    try:
        einmo = read_einmo_file(file_path)
    except Exception as e:
        print(f"einmo: {e}")
        return 1

    for s in einmo.sections:
        if section and s.name != section:
            continue
        if not bare:
            print(f"=== {s.name} ===")
        print(s.body)

    return 0

def cmd_list(work_dir: Path, filter_str: Optional[str], differing: bool, json_out: bool):
    from suite import EinmoSuite, EinmoDirectory
    from stage import Stage
    from format import split_advisory

    suite = EinmoSuite(EinmoDirectory(work_dir))
    cases = suite.cases()

    rows = []
    for case in cases:
        rel = str(case.rel_path)
        if rel.endswith('.einmo'):
            rel = rel[:-6]
        if filter_str and filter_str not in rel:
            continue

        stages = case.stages()

        # calculate differing
        is_differing = False
        if differing:
            bodies = []
            for st, _ in stages:
                path = work_dir / st.dir_name() / case.rel_path
                if path.exists():
                    try:
                        from commands_ro import read_einmo_file
                        f = read_einmo_file(path)
                        # only check body sections
                        bodies.append([s.body for s in f.sections])
                    except:
                        bodies.append(None)
                else:
                    bodies.append(None)

            if any(b is None for b in bodies):
                is_differing = True
            elif len(bodies) > 1:
                first = bodies[0]
                if any(b != first for b in bodies[1:]):
                    is_differing = True

        if differing and not is_differing:
            continue

        rows.append((rel, is_differing, stages))

    for rel, diff, stages in rows:
        if json_out:
            import json
            stages_json = []
            for s, st in stages:
                val = f'"{st}"' if st else "null"
                stages_json.append(f'"{s.dir_name()}":{val}')
            stages_str = ",".join(stages_json)
            diff_str = "true" if diff else "false"
            print(f'{{"test":"{rel}","differing":{diff_str},{stages_str}}}')
        else:
            diff_str = "differ" if diff else "same"
            marks = []
            for s, st in stages:
                mark = st if st else "—"
                if mark == "normal": mark = "ok"
                marks.append(f"{s.dir_name()}:{mark}")
            print(f"{rel}\t{diff_str}\t{' '.join(marks)}")

    if not json_out:
        import sys
        print(f"{len(rows)} test(s)", file=sys.stderr)

    return 0

def cmd_self_check(expected: Optional[str], quiet: bool):
    import sys
    import hashlib
    try:
        with open(sys.executable, 'rb') as f:
            digest = hashlib.sha256(f.read()).hexdigest()
    except Exception as e:
        print(f"einmo: {e}")
        return 1

    if quiet:
        print(digest)
    else:
        print(f"{sys.executable} sha256:{digest}")

    if expected and expected != digest:
        print(f"einmo: self-check mismatch (expected {expected}, got {digest})")
        return 1

    return 0
