from pathlib import Path
from typing import List, Optional
from stage import Stage
import os

class EinmoDirectory:
    def __init__(self, work_dir: Path):
        self.work_dir = work_dir

class EinmoCase:
    def __init__(self, suite, rel_path: Path):
        self.suite = suite
        self.rel_path = rel_path

    def stages(self) -> List[tuple]:
        # returns (Stage, Optional[str]) status
        from commands_ro import read_einmo_file
        res = []
        for stage in Stage.all():
            stage_dir = self.suite.directory.work_dir / stage.dir_name()
            file_path = stage_dir / self.rel_path
            if file_path.exists():
                try:
                    f = read_einmo_file(file_path)
                    res.append((stage, f.metadata.status.as_str()))
                except:
                    res.append((stage, "TAMPERED"))
            else:
                res.append((stage, None))
        return res

class EinmoSuite:
    def __init__(self, directory: EinmoDirectory):
        self.directory = directory

    def cases(self) -> List[EinmoCase]:
        # Simple scan
        all_files = set()
        for stage in Stage.all():
            stage_dir = self.directory.work_dir / stage.dir_name()
            if not stage_dir.exists():
                continue
            for root, _, files in os.walk(stage_dir):
                for file in files:
                    if file.endswith('.einmo'):
                        full_path = Path(root) / file
                        rel_path = full_path.relative_to(stage_dir)
                        all_files.add(rel_path)
        return [EinmoCase(self, p) for p in sorted(list(all_files))]
