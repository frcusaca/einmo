import subprocess
from pathlib import Path
from format import EinmoFile, Metadata, Status, Section, DEFAULT_SEPARATOR, split_advisory
from signature import Stamps, computer_keypair, compiled_keypair, produced_by, now_iso8601
import os

def cmd_evaluate(work_dir: Path, command: str, filter_str: str, walk_depth_limit: int, json_out: bool, regenerate: bool = False):
    input_dir = work_dir / "input"
    output_dir = work_dir / "output"
    output_dir.mkdir(parents=True, exist_ok=True)

    if not input_dir.exists():
        if json_out:
            print('{"evaluated":0,"failed":0}')
        else:
            print("evaluated 0 file(s), 0 failure(s)")
        return 0

    inputs = []
    for root, _, files in os.walk(input_dir):
        for f in files:
            p = Path(root) / f
            if filter_str and filter_str not in str(p):
                continue
            inputs.append(p)

    written = 0
    failed = 0

    for in_path in inputs:
        rel = in_path.relative_to(input_dir)
        out_path = output_dir / f"{rel}.einmo"
        out_path.parent.mkdir(parents=True, exist_ok=True)

        # Read input source
        try:
            with open(in_path, 'r', encoding='utf-8') as f:
                source = f.read()
        except Exception as e:
            print(f"  ✗ {rel} — {e}")
            failed += 1
            continue

        # Catastrophe crumb
        crumb_meta = Metadata(
            test=str(rel),
            suite=work_dir.name,
            producer="unknown", # normally git sha
            generated=now_iso8601(),
            status=Status.OUTPUT_ERROR,
            status_detail="TEST IN PROGRESS -- if you see this file, the test harness crashed during evaluation. Escalate to human or other agents for support.",
            sections=["INPUT", "OUTPUT", "COMMENTS", "STAMPS"]
        )
        crumb_sections = [
            Section("INPUT", source),
            Section("OUTPUT", ""),
            Section("COMMENTS", "")
        ]

        comp_sk, _ = compiled_keypair()
        compu_sk, _ = computer_keypair()

        def write_file_to(path: Path, meta: Metadata, sects: list[Section]):
            # create and sign file
            f = EinmoFile("utf-8", DEFAULT_SEPARATOR, meta, sects, Stamps())
            prefix = f.signed_prefix()
            stamps = Stamps.generate(prefix, compu_sk, compu_sk)
            f.stamps = stamps
            with open(path, 'wb') as out_f:
                out_f.write(f.serialize())

        # Write crumb
        try:
            write_file_to(out_path, crumb_meta, crumb_sections)
        except Exception as e:
            print(f"  ✗ {rel} — crumb write failed: {e}")
            failed += 1
            continue

        # Execute command
        try:
            process = subprocess.Popen(
                command,
                shell=True,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            stdout, stderr = process.communicate(input=source)
            status_code = process.returncode

            if status_code != 0:
                meta = crumb_meta
                meta.status_detail = f"evaluator exited {status_code}: {stderr.strip()}"
                write_file_to(out_path, meta, crumb_sections)
                print(f"  ✗ {rel} ({meta.status_detail})")
                failed += 1
                continue

            # Success
            meta = crumb_meta
            meta.status = Status.NORMAL
            meta.status_detail = ""
            sects = [
                Section("INPUT", source),
                Section("OUTPUT", stdout),
                Section("COMMENTS", "")
            ]
            write_file_to(out_path, meta, sects)
            print(f"  ✓ {rel}")
            written += 1

        except Exception as e:
            print(f"  ✗ {rel} — {e}")
            failed += 1

    verb = "regenerated" if regenerate else "evaluated"
    if json_out:
        print(f'{{"{verb}":{written},"failed":{failed}}}')
    else:
        print(f"{verb} {written} file(s), {failed} failure(s)")

    return 1 if failed > 0 else 0
