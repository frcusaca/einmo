# einmo repo — TODO

> **Note:** Hand-written following the same convention used in the
> `foolish-rust` workspace's `docs/todo/AIAGENT-*.todo.md` files (no `/todo`
> skill is configured in this repo).

## Log

- 2026-07-29 17:15:01 — Session: EIMP-2 implementation (Phases A–B). Built
  `EinmoId`, ported a JavaScript-only (Boa) `zweimomo` demo crate with
  progressive-difficulty tiers (`day.1/`, `week.2/`, `month.2/`,
  `years.later/`), added a root `[workspace]`. Corrected an earlier mistake
  where a passphrase was disclosed in prose documentation (`README.md`,
  `EIMP-2.md`) — the `checked`-stage passphrase for `day.1/` now lives only
  in `day.1/einmo.toml` (`[signing] checked = "…"`), never in prose.

## Open items

- [ ] Enhance einmo's documentation to establish a secure and integral
  operational mindset around passphrases: passphrases are secret and are
  kept secret — kept secret by storing them in protected files (e.g.
  `einmo.toml` with appropriate filesystem permissions, a secrets manager),
  or by producing them from a human mind (typed interactively, never
  written down) — never by pasting them into README prose, commit messages,
  or any other document meant to be read/shared casually. Document how
  einmo's multi-stage aggregation of authenticated approvals (the
  `compiled`/`configured`/`stage:output`/`stage:checked`/`stage:verified`
  stamp chain, each a distinct key role) depends on that discipline holding
  at every stage — a leaked `checked`-stage passphrase undermines the
  "reviewed baseline" claim the same way a leaked `verified`-stage
  passphrase undermines human attestation, just at a different point in the
  chain. This session's own mistake (briefly documenting a real,
  in-use `checked` passphrase in `zweimomo/README.md` and `EIMP-2.md`
  before being corrected) is a concrete cautionary example worth citing
  when writing this guidance.

- [ ] Document, then implement, a **double-signature** use case in
  `zweimomo`'s `week.2/` tier: a second DC (data center / build machine)
  comes online with new hardware, a new OS, and a new language/toolchain
  version. To check whether it produces the same results, einmo should let
  that second DC run the test suite and add its own signature to the
  `checked` section — a second, independent attestation on top of the
  first, not a replacement. The second signature's stamp should be able to
  carry additional metadata explaining *why* a second signature was
  provided (e.g. "cross-verification from DC-2: aarch64, Ubuntu 26.04, Rust
  1.9x"), distinguishing "a second party independently confirms this" from
  "the same party re-signed." This may not be fully implementable with
  einmo's current stamp format/API (multiple stamps at the same stage,
  each with a reason field, is not confirmed to exist yet — needs a design
  pass, likely its own EIMP) but is flagged as a significant feature worth
  building. Write it up as a documented use case in `week.2/README.week.2.md`
  first (per the tier's existing "what's the use case, what's the present —
  possibly unimplemented — solution" pattern), then implement once
  designed.
