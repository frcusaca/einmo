from enum import Enum
from pathlib import Path
import json

FORMAT_VERSION = 1
DEFAULT_SEPARATOR = "①\n"
FOOLISH_SEPARATOR = "!!\n"

class Status(Enum):
    NORMAL = "normal"
    INPUT_ERROR = "input-error"
    OUTPUT_ERROR = "output-error"

    def as_str(self):
        return self.value

    @classmethod
    def parse(cls, s):
        for member in cls:
            if member.value == s:
                return member
        raise ValueError(f"unknown status {s}")

class Section:
    def __init__(self, name: str, body: str):
        self.name = name
        self.body = body

class Metadata:
    def __init__(self, test: str, suite: str, producer: str, generated: str, status: Status, sections: list[str], producer_diff: str = "", status_detail: str = "", reference: str = ""):
        self.test = test
        self.suite = suite
        self.producer = producer
        self.producer_diff = producer_diff
        self.generated = generated
        self.status = status
        self.status_detail = status_detail
        self.reference = reference
        self.sections = sections

    def serialize(self) -> str:
        out = f"test: {self.test}\n"
        out += f"suite: {self.suite}\n"
        out += f"producer: {self.producer}\n"
        if self.producer_diff:
            out += f"producer-diff: {self.producer_diff}\n"
        out += f"generated: {self.generated}\n"
        out += f"status: {self.status.as_str()}\n"
        out += f"status-detail: {escape_line(self.status_detail)}\n"
        if self.reference:
            out += f"reference: {self.reference}\n"
        out += f"sections: {', '.join(self.sections)}\n"
        return out

    @classmethod
    def parse(cls, block: str):
        test = None
        suite = None
        producer = None
        producer_diff = ""
        generated = None
        status = None
        status_detail = ""
        reference = ""
        sections = None

        for line in block.split('\n'):
            if ':' not in line:
                continue
            key, value = line.split(':', 1)
            value = value.lstrip(' ')
            if key == "test": test = value
            elif key == "suite": suite = value
            elif key == "producer": producer = value
            elif key == "producer-diff": producer_diff = value
            elif key == "generated": generated = value
            elif key == "status": status = Status.parse(value)
            elif key == "status-detail": status_detail = unescape_line(value)
            elif key == "reference": reference = value
            elif key == "sections":
                sections = [s.strip() for s in value.split(',')]
                sections = [s for s in sections if s]

        if test is None: raise ValueError("missing metadata `test`")
        if suite is None: raise ValueError("missing metadata `suite`")
        if producer is None: raise ValueError("missing metadata `producer`")
        if generated is None: raise ValueError("missing metadata `generated`")
        if status is None: raise ValueError("missing metadata `status`")
        if sections is None: raise ValueError("missing metadata `sections`")

        return cls(test, suite, producer, generated, status, sections, producer_diff, status_detail, reference)

def escape_line(s: str) -> str:
    return s.replace('\\', '\\\\').replace('\n', '\\n').replace('\r', '\\r')

def unescape_line(s: str) -> str:
    out = []
    chars = iter(s)
    try:
        while True:
            c = next(chars)
            if c == '\\':
                try:
                    n = next(chars)
                    if n == 'n': out.append('\n')
                    elif n == 'r': out.append('\r')
                    elif n == '\\': out.append('\\')
                    else:
                        out.append('\\')
                        out.append(n)
                except StopIteration:
                    out.append('\\')
            else:
                out.append(c)
    except StopIteration:
        pass
    return "".join(out)

def escape_separator(sep: str) -> str:
    return sep.replace('\\', '\\\\').replace('\n', '\\n')

def unescape_separator(s: str) -> str:
    return unescape_line(s)

class EinmoFile:
    def __init__(self, encoding: str, separator: str, metadata: Metadata, sections: list[Section], stamps, advisory: str | None = None):
        self.encoding = encoding
        self.separator = separator
        self.metadata = metadata
        self.sections = sections
        self.stamps = stamps
        self.advisory = advisory

    def header_line(self) -> str:
        return f"#einmo {FORMAT_VERSION} encoding={self.encoding} separator={escape_separator(self.separator)}"

    def signed_prefix(self) -> bytes:
        out = self.header_line() + "\n"
        out += self.metadata.serialize()
        out += self.separator
        for section in self.sections:
            out += section.body
            out += self.separator
        return out.encode('utf-8')

    def serialize(self) -> bytes:
        meta = self.metadata.serialize()
        if self.separator in meta:
            raise ValueError("SeparatorCollision metadata")
        for section in self.sections:
            if self.separator in section.body:
                raise ValueError(f"SeparatorCollision {section.name}")

        out = self.signed_prefix()
        out += self.stamps.serialize().encode('utf-8')
        if self.advisory is not None:
            out += b'\n' + self.advisory.encode('utf-8')
        return out

    @classmethod
    def parse(cls, data: bytes):
        text = data.decode('utf-8')
        if '\n' not in text:
            raise ValueError("missing header line")
        header, rest = text.split('\n', 1)

        parts = header.split()
        if not parts or parts[0] != "#einmo":
            raise ValueError("bad header magic")
        if len(parts) < 2 or parts[1] != str(FORMAT_VERSION):
            raise ValueError("unsupported format version")

        encoding = None
        separator = None
        for kv in parts[2:]:
            if kv.startswith("encoding="):
                encoding = kv[len("encoding="):]
            elif kv.startswith("separator="):
                separator = unescape_separator(kv[len("separator="):])

        if not encoding or not separator:
            raise ValueError("header missing encoding or separator")

        main_text, advisory = split_advisory(rest, separator)

        # Split sections
        section_parts = main_text.split(separator)
        if len(section_parts) < 3:
            raise ValueError("too few sections")

        metadata = Metadata.parse(section_parts[0])
        from signature import Stamps
        stamps = Stamps.parse(section_parts[-1])

        declared_bodies = [s for s in metadata.sections if s != "STAMPS"]
        body_parts = section_parts[1:-1]

        if len(body_parts) != len(declared_bodies):
            raise ValueError(f"declared {len(declared_bodies)} body sections but found {len(body_parts)}")

        sections = [Section(name, body) for name, body in zip(declared_bodies, body_parts)]

        return cls(encoding, separator, metadata, sections, stamps, advisory)

def split_advisory(main: str, separator: str):
    tail_start = main.rfind(separator)
    if tail_start != -1:
        tail_start += len(separator)
    else:
        tail_start = 0

    tail = main[tail_start:]
    if "\n# flagged:" in tail:
        idx = tail.find("\n# flagged:")
        body = main[:tail_start + idx]
        adv = main[tail_start + idx:].lstrip('\n')
        return body, adv
    elif tail.startswith("# flagged:"):
        return main[:tail_start], tail
    return main, None
