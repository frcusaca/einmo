import json
import base64
from typing import List, Optional
import nacl.signing
import nacl.exceptions
from argon2.low_level import hash_secret_raw, Type
import hashlib
import datetime
import os

ARGON2_MEMORY_KIB = 19_456
ARGON2_TIME_COST = 2
ARGON2_PARALLELISM = 1
SALT = b"einmo:stamp-key:v1"

def derive_seed(passphrase: str, salt: bytes) -> bytes:
    # Rust uses argon2::Argon2id, V0x13, with specific params.
    # argon2-cffi low_level provides exact access.
    # m=19456, t=2, p=1
    return hash_secret_raw(
        secret=passphrase.encode('utf-8'),
        salt=salt,
        time_cost=ARGON2_TIME_COST,
        memory_cost=ARGON2_MEMORY_KIB,
        parallelism=ARGON2_PARALLELISM,
        hash_len=32,
        type=Type.ID
    )

def derive_keypair(passphrase: str) -> tuple[nacl.signing.SigningKey, nacl.signing.VerifyKey]:
    seed = derive_seed(passphrase, SALT)
    signing_key = nacl.signing.SigningKey(seed)
    return signing_key, signing_key.verify_key

def compiled_keypair() -> tuple[nacl.signing.SigningKey, nacl.signing.VerifyKey]:
    return derive_keypair("einmo-stock-compiled-key")

def computer_keypair() -> tuple[nacl.signing.SigningKey, nacl.signing.VerifyKey]:
    return derive_keypair("")

def is_computer_key(pubkey_hex: str) -> bool:
    _, vk = computer_keypair()
    return pubkey_hex == vk.encode().hex()

class StampRole:
    def __init__(self, is_prior_bytes: bool, certifies: str = ""):
        self.is_prior_bytes = is_prior_bytes
        self.certifies = certifies

    def as_field(self) -> str:
        if self.is_prior_bytes:
            return "prior-bytes"
        return f"pubkey:{self.certifies}"

    @classmethod
    def from_field(cls, field: str):
        if field == "prior-bytes":
            return cls(True)
        elif field.startswith("pubkey:"):
            return cls(False, field[len("pubkey:"):])
        raise ValueError(f"unknown signs field {field}")

class Stamp:
    def __init__(self, key: str, pubkey_hex: str, signs: StampRole, signature_b64: str, produced_by: str, timestamp: str):
        self.key = key
        self.pubkey_hex = pubkey_hex
        self.signs = signs
        self.signature_b64 = signature_b64
        self.produced_by = produced_by
        self.timestamp = timestamp

    def is_stage(self) -> bool:
        return self.signs.is_prior_bytes and self.key.startswith("stage:")

    def stage_name(self) -> Optional[str]:
        if self.key.startswith("stage:"):
            return self.key[len("stage:"):]
        return None

    def to_json_line(self) -> str:
        obj = {
            "key": self.key,
            "pubkey": self.pubkey_hex,
            "signs": self.signs.as_field(),
            "signature": self.signature_b64,
            "produced_by": self.produced_by,
            "timestamp": self.timestamp
        }
        return json.dumps(obj, separators=(',', ':'))

    @classmethod
    def from_json_line(cls, line: str):
        obj = json.loads(line)
        key = obj["key"]
        # Validate key
        if key not in ("compiled", "configured") and not (key.startswith("stage:") and key[len("stage:"):]):
            raise ValueError(f"invalid stamp key {key}")

        return cls(
            key=key,
            pubkey_hex=obj["pubkey"],
            signs=StampRole.from_field(obj["signs"]),
            signature_b64=obj["signature"],
            produced_by=obj["produced_by"],
            timestamp=obj["timestamp"]
        )

def produced_by() -> str:
    # Python equivalent
    return "einmo 0.0.5 sha256:unknown"

def now_iso8601() -> str:
    return datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")

def certification_stamp(key: str, signer: nacl.signing.SigningKey, certified_role: str, certified_vk: nacl.signing.VerifyKey, pb: str, ts: str) -> Stamp:
    msg = certified_vk.encode()
    sig = signer.sign(msg).signature
    return Stamp(
        key=key,
        pubkey_hex=signer.verify_key.encode().hex(),
        signs=StampRole(False, certified_role),
        signature_b64=base64.b64encode(sig).decode('ascii'),
        produced_by=pb,
        timestamp=ts
    )

def stage_stamp(key: str, signer: nacl.signing.SigningKey, prior_bytes: bytes, pb: str, ts: str) -> Stamp:
    sig = signer.sign(prior_bytes).signature
    return Stamp(
        key=key,
        pubkey_hex=signer.verify_key.encode().hex(),
        signs=StampRole(True),
        signature_b64=base64.b64encode(sig).decode('ascii'),
        produced_by=pb,
        timestamp=ts
    )

class StampCheck:
    def __init__(self, key: str, ok: bool):
        self.key = key
        self.ok = ok

class Stamps:
    def __init__(self, entries: List[Stamp] = None):
        self.entries = entries or []

    def serialize(self) -> str:
        return "\n".join(e.to_json_line() for e in self.entries)

    @classmethod
    def parse(cls, section: str):
        entries = []
        for line in section.split('\n'):
            line = line.strip()
            if not line: continue
            entries.append(Stamp.from_json_line(line))
        return cls(entries)

    def highest_stage_stamp(self) -> Optional[Stamp]:
        for s in reversed(self.entries):
            if s.is_stage():
                return s
        return None

    @classmethod
    def generate(cls, prior_bytes: bytes, configured: nacl.signing.SigningKey, stage_output: nacl.signing.SigningKey):
        return cls.generate_for_stage(prior_bytes, configured, "stage:output", stage_output)

    @classmethod
    def generate_for_stage(cls, prior_bytes: bytes, configured: nacl.signing.SigningKey, stage_key: str, stage_signer: nacl.signing.SigningKey):
        compiled_sk, _ = compiled_keypair()
        pb = produced_by()
        ts = now_iso8601()

        compiled = certification_stamp("compiled", compiled_sk, "configured", configured.verify_key, pb, ts)
        configured_stamp = certification_stamp("configured", configured, stage_key, stage_signer.verify_key, pb, ts)

        before_stage = bytearray(prior_bytes)
        before_stage.extend(compiled.to_json_line().encode('utf-8'))
        before_stage.append(ord('\n'))
        before_stage.extend(configured_stamp.to_json_line().encode('utf-8'))
        before_stage.append(ord('\n'))

        stage = stage_stamp(stage_key, stage_signer, bytes(before_stage), pb, ts)
        return cls([compiled, configured_stamp, stage])

    def append_stage(self, stage_key: str, signing: nacl.signing.SigningKey, file_before_new_stamp: bytes):
        pb = produced_by()
        ts = now_iso8601()
        stamp = stage_stamp(stage_key, signing, file_before_new_stamp, pb, ts)
        self.entries.append(stamp)

    def prefix_for_next_stamp(self, body_with_separator: bytes) -> bytes:
        prefix = bytearray(body_with_separator)
        for stamp in self.entries:
            prefix.extend(stamp.to_json_line().encode('utf-8'))
            prefix.append(ord('\n'))
        return bytes(prefix)

    def verify_chain(self, body_with_separator: bytes) -> List[StampCheck]:
        checks = []
        prior = bytearray(body_with_separator)

        for stamp in self.entries:
            ok = False
            try:
                vk = nacl.signing.VerifyKey(bytes.fromhex(stamp.pubkey_hex))
                sig = base64.b64decode(stamp.signature_b64)

                if stamp.signs.is_prior_bytes:
                    vk.verify(bytes(prior), sig)
                    ok = True
                else:
                    cert_role = stamp.signs.certifies
                    certified = next((s for s in self.entries if s.key == cert_role), None)
                    if certified:
                        cert_vk = bytes.fromhex(certified.pubkey_hex)
                        vk.verify(cert_vk, sig)
                        ok = True
            except (nacl.exceptions.BadSignatureError, ValueError):
                pass

            checks.append(StampCheck(stamp.key, ok))

            prior.extend(stamp.to_json_line().encode('utf-8'))
            prior.append(ord('\n'))

        return checks

    def chain_valid(self, body_with_separator: bytes) -> bool:
        return all(c.ok for c in self.verify_chain(body_with_separator))
