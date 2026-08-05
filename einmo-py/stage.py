from enum import Enum
from pathlib import Path
import os

class Stage(Enum):
    OUTPUT = "output"
    CHECKED = "checked"
    VERIFIED = "verified"

    def dir_name(self) -> str:
        return self.value

    def next(self):
        if self == Stage.OUTPUT: return Stage.CHECKED
        elif self == Stage.CHECKED: return Stage.VERIFIED
        else: return None

    @classmethod
    def parse(cls, s: str):
        for member in cls:
            if member.value == s:
                return member
        raise ValueError(f"unknown stage {s}")

    @classmethod
    def all(cls):
        return [Stage.OUTPUT, Stage.CHECKED, Stage.VERIFIED]

def normalize_file_path(p: Path, input_dir: Path) -> Path:
    # This logic matches Rust's transitions::normalize_file_path
    # Try to make it mirror-relative (relative to input_dir or stage dirs)
    # Very simplified version for now:
    try:
        return p.relative_to(input_dir)
    except ValueError:
        pass
    for s in Stage.all():
        try:
            return p.relative_to(Path(s.dir_name()))
        except ValueError:
            pass
    # fallback
    if not p.name.endswith(".einmo"):
        return p.with_name(p.name + ".einmo")
    return p
