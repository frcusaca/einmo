# rust_instructions.md

Instructions for AI coding agents writing Rust in this repository. These are
**directives, not suggestions.** Every rule traces either to this project's own
conventions or to a real-world agent file, a curated rule set, or the Rust
project's own documentation. Where an external source backs a rule, a numbered
citation `(c#)` appears inline; the **Citations** section at the bottom resolves
them.

> Ported from the `foolish-rust` workspace's `rust_instructions.md` when einmo
> was extracted into its own repository (see `docs/eimp/EIMP-0.md`). Sections
> 1–6 (general Rust guidance) are carried over verbatim; §7 ("Project-specific
> rules") is trimmed to what's relevant to a standalone signing/verification
> library with a CLI — the original's Foolish/FIR, Foretias P2P networking,
> C11 FFI, and cross-language-bindings subsections are dropped.

> **Citations are for maintaining this document, not for everyday coding.** You
> do not need to read or follow the sources while writing code — they exist so a
> future maintainer can verify, update, or challenge a rule. Follow the rule
> text itself.

The document is organized by the *strength and shape* of each instruction:

- **Priorities** — the prioritizing statements: what to optimize for and which
  construct wins when two compete.
- **Task guides** — the same rules re-indexed *by the task you are doing right
  now* (writing a function, a helper, naming, documenting, structuring a module
  or a crate), ordered most-frequent first. Start here when coding.
- **Language patterns** — prescribed instructions stated *with the conditions*
  under which they apply.
- **Do's** — affirmative mandates, including the hard tooling gates.
- **Preferences** — weaker, softer defaults you should lean toward.
- **Don'ts** — prohibitions and anti-patterns.
- **Project-specific rules** — einmo-specific rules (cryptographic/signing code,
  the `.einmo` envelope format, CLI dispatch) that are particular to this
  repository and override any general guidance on conflict.

Some rules appear in more than one section by design (e.g. "use `?`" is a Do,
and "never `.unwrap()` in production" is a Don't).

---

## Project baseline

- **Edition:** `2024` (Rust 1.85+) — unlocks let-chains, async closures, and the
  2024 drop/temporary-scope semantics. All crates in the workspace are
  already on edition 2024. *(c1)*
- **MSRV:** state it in `Cargo.toml`; do not use features newer than it. Where a
  rule below is version-gated and the MSRV predates it, that rule does not apply.
- **Toolchain:** stable only, no nightly features. *(c2)*
- **Error crates already in the workspace:** `thiserror` 2 (matchable enums) and
  `anyhow` 1 (opaque application errors). Use these; do not add competing error
  crates.

---

## 1. Priorities

Two prioritizing axes apply. The **optimization order** governs what to optimize
*for*; the **construct-preference order** governs which *construct* wins when two
compete. They do not conflict — the first is about goals, the second about means.

### 1a. Optimization order (this project's law)

When goals compete, the earlier wins. Do not sacrifice an earlier goal for a
later one.

1. **Correctness** (and soundness — unsound code is never acceptable;
   `clippy::correctness` lints are bugs, not style). *(c3)*
2. **Readability and maintainability.**
3. **Testability.**
4. **Efficiency.**
5. **Style principles** — and delegate as much style as possible to the tools
   (rustfmt, clippy, RFC 430), so you spend no effort on whitespace, import
   order, or casing they already enforce.

Do not sacrifice correctness for cleverness, abstraction, minimalism, or
performance. Do not sacrifice readability unless there is a measured, justified
efficiency need.

### 1b. Construct-preference order

When two constructs compete, the earlier-named or left-hand side wins.

1. **Borrowing over cloning.** A clone is a deliberate runtime cost to be
   justified, never a borrow-checker workaround. Prefer, in order: borrow →
   restructure lifetimes → split the borrow → move → (last) clone.
2. **Immutability over mutability.** `mut` and `&mut` are the exception; reach
   for them only when you actually mutate.
3. **Encapsulation over exposure.** Private fields and behavior-based APIs over
   public fields and raw state.
4. **Make illegal states unrepresentable** over validating at call sites — encode
   invariants in the type system. *(c4, c5)*
5. **Types over generics, generics over `dyn`.** Reach for dynamic dispatch only
   when you need it. *(c6)*
6. **Standard traits over ad-hoc methods.** `From`/`TryFrom`/`Display`/`FromStr`
   over bespoke `to_x`/`from_x`. *(c7, c8)*
7. **Iterators over manual index loops; pattern matching over `unwrap` chains.**
8. **`std` over third-party crates** where `std` now suffices (`LazyLock`/
   `OnceLock` over `lazy_static`/`once_cell`).
9. **Error propagation over panicking.** `?` and typed errors over
   `.unwrap()`/`.expect()`.
10. **Compile time matters.** Prefer fewer, well-scoped crates and avoid
    gratuitous proc-macro dependencies. *(c9, c10)*

---

## 2. Task guides

The rules of this document, re-indexed by the task at hand. Ordered by how often
each task occurs: functions first (constant), crate creation last (rare). Each
guide leads with the advice that matters most and is used most, and ends with
pointers into the reference sections (§3–§7), where the extensive examples live.

### Rule zero — encapsulation (applies to every task below)

Coding agents fail most often at encapsulation, so it is stated first, insisted
upon, and repeated inside the guides. The rule has four clauses:

1. **A function that changes an object's state belongs to that object.** Declare
   it inside the object's `impl` block, taking `&mut self` (or `&self` with the
   type's own interior mutability). The `impl TheType { … }` block is the
   *programmatic marking* of association — never a free function mutating
   another type's fields from outside.
2. **A function that primarily reports information about an object also belongs
   to that object** — predicates, projections, summaries (`is_constanic()`,
   `state()`, `as_i64()`) are methods; callers must never re-derive a fact the
   type can state about itself.
3. **Private defensively, public by design.** Every new field is private. Every
   new method starts private, widens to `pub(super)`/`pub(crate)` only when a
   real caller appears, and becomes `pub` only when it was *designed* to be
   public-facing API. The same applies to modules (`mod`, not `pub mod`).
4. **When the type is foreign** (e.g. a type alias over a standard-library
   type, or a type from another crate), the orphan rule forbids an inherent
   `impl` — attach the behavior with an **extension trait** instead. The
   association is still programmatic, and cross-cutting invariants are
   documented once, at the trait, not at every call site.

Detailed, einmo-grounded examples: §3 "Encapsulation & types" and §3 "The four
OOP pillars in Rust".

### 2a. Writing a new function

1. **Whose function is it? (Rule zero.)** If it changes an object's state or
   reports on an object, it belongs in that object's `impl` block (or extension
   trait for foreign types). A free function is only for logic that genuinely
   belongs to no type.
2. **Signature first**: borrow (`&str`, `&[T]`, `&T`), never `&String`/`&Vec<T>`;
   take owned values only when the function stores, consumes, or returns them.
3. **Return the crate's central `Result<T>`** and propagate with `?`. No
   `.unwrap()`/`.expect()`/`panic!` in library, parser, interpreter, FFI, or
   production paths.
4. **No `mut` you don't use** — and re-audit after every edit; downgrades cascade
   outward to callers.
5. **Iterator chains over index loops**; `let … else` for the refutable happy
   path; `matches!` for boolean discriminant checks.
6. **One responsibility.** If the body mixes validation, transformation, I/O, and
   mutation, split it.
7. **Inject ambient side effects** (tty, stdin, env, clock, RNG) as closures or
   trait params gathered in a small struct — six positional params with `bool`
   mode flags is untestable. *(c25)*
   ```rust
   fn resolve_stage_key(
       inputs: KeyCascadeInputs,
       prompt: impl FnOnce() -> io::Result<String>,  // tests: || panic!("no prompt expected")
   ) -> Result<KeySource>
   ```
8. **Mark it `#[must_use]`** when discarding the return value is a bug.
9. **Implement fully or don't add it.** A function that returns an empty
   placeholder report is worse than no function — callers build on a no-op.
   *(c25)*

Reference: §3 Ownership & borrowing; §3 Loops & matching; §3 Errors; §3 The four
OOP pillars in Rust.

### 2b. Writing a new helper function

1. **Rule zero applies doubly to helpers.** If the helper mutates an object's
   state or reports on an object's data, it is a *method* in that object's
   `impl` block (or extension trait), not a free function poking at fields.
   Most "helpers" are misplaced methods.
2. **Check it doesn't already exist** — in `std`, in the crate, or in a sibling
   module. Four copy-pasted recursive directory walkers in one crate is the
   canonical failure. *(c25)* Reuse or extract the one shared helper.
3. **Home it with the concern it serves**, not in `utils`. A helper used by one
   module stays private in that module; promote to `pub(crate)` only when a
   second module calls it. Never `pub` by reflex.
4. **DRY the serialization/derivation paths**: one function builds the canonical
   bytes and everything else (serialize, sign, verify) calls it — never a second
   hand-rolled copy that must be kept in sync. *(c25)*

Reference: §3 Encapsulation & types; §3 The four OOP pillars in Rust; §5
Preferences (module organization).

### 2c. Naming

1. **RFC 430 casing**; acronyms as words: `Uuid`, `parse_xml`, never `UUID`.
2. **Conversion prefixes carry cost semantics** *(c8)*: `as_` = cheap borrow,
   `to_` = expensive/new allocation, `into_` = consuming. Don't mislabel.
3. **Distinct meaning → newtype with a domain name**: `UserId(u64)`, not a bare
   `u64` (and not an alias).
4. **Finite word-domains → enum** with CamelCase variants (`Status::InputError`),
   never `"input-error"` strings in the domain type. *(c25)*
5. **Modules are named by responsibility** (`transitions`, `verify`, `wire`) —
   a name you can't pick usually means the module does too many things.
6. **Descriptive locals**; the lowercased type name is a fine default.

Reference: §3 Encapsulation & types (newtypes, enums); §4 Do's (RFC 430 casing).

### 2d. Documentation

1. **`///` on every public item**: first sentence one line (≤ ~15 words); add
   `# Errors` / `# Panics` / `# Safety` where they apply; `?` (not `.unwrap()`)
   in examples. *(c11, c22)*
2. **Crate-level `//!` in `lib.rs`** stating purpose, core model, and
   load-bearing invariants — a `lib.rs` that is only re-exports is missing its
   front door. *(c25)*
3. **Module-level `//!` docs** carry a reference to the governing specification
   section (e.g. "FOOP-<N> §4.4") so readers can trace the design to its spec.
4. **Comments explain *why*, not *what*.** If a *what* comment is needed, make
   the code clearer instead.
5. **Honest status over checked boxes**: plan files and docs must state what is
   actually implemented; documentation that overstates completion is a defect.
   *(c25)*

Reference: §4 Do's (documentation mandates); AGENTS.md Markdown File Update
Protocol for `.md` files.

### 2e. Structuring a new module

1. **One responsibility per module.** When a module accretes jobs — a `stage.rs`
   holding the enum, directory ops, promotion, flagging, *and* signature
   scanning — split it (`stage.rs` + `transitions.rs`) before it grows further.
   *(c25)*
2. **Declare it private** (`mod foo;`) and re-export the curated surface from
   `lib.rs`; internals use `pub(crate)`. *(c25, c26)*
3. **Errors go to the crate's central `error.rs`** — a new module does not get
   its own error enum. *(c25)*
4. **Keep the type and its `impl` blocks together**; path-based modules
   (`foo.rs` + `foo/`), never `mod.rs`.
5. **Everything private until designed public (Rule zero).** Fields private;
   functions private; widen deliberately (`pub(super)` → `pub(crate)` → `pub`),
   each widening justified by an actual caller or a designed API surface.
6. **Unit tests inline** in `#[cfg(test)] mod tests`; hermetic (temp dirs, no
   committed-state mutation). Integration tests go in the crate's `tests/` dir.
   *(c25)*

Reference: §3 Encapsulation & types; §3 The four OOP pillars in Rust; §5
Preferences (responsibility-based organization).

### 2f. Structuring a new crate (rare — ask before creating one)

1. **Justify the crate**: a crate boundary is for an independent compilation
   unit, reuse surface, or (like einmo) a deliberately standalone,
   repo-promotable artifact. Otherwise it's a module.
2. **`lib.rs` = crate `//!` doc + private `mod` list + curated `pub use`** —
   the whole public API auditable in one screen. *(c25, c26)*
3. **One `#[non_exhaustive]` error enum** in `error.rs` + `pub(crate) type
   Result<T>` alias, `Io` variants carrying the offending path. *(c25)*
4. **`Cargo.toml` hygiene**: `license`, workspace lints, `[lints.rust]
   unsafe_code = "deny"` (crates that need no `unsafe`, and always for
   crypto-touching crates), every dependency justified, exact `=x.y.z` pins when
   a dep's output text lands in signed baselines. No unused deps. *(c25)*
5. **CLI crates**: dispatch returns `ExitCode` (testable, no `process::exit`);
   `--json` on every subcommand; an alias binary is a one-line wrapper over the
   same parser. *(c25)*
6. **No dead scaffolding**: no stub modules, parsed-but-ignored flags, or config
   fields nothing reads. Implement or omit. *(c25)*

Reference: §3 Errors (central enum); §7 CLI binaries; §7 Dependencies; §7
Cryptographic code (lint gates, parameter pinning).

---

## 3. Language patterns

Prescribed instructions, each stated with the condition that triggers it.

### Ownership & borrowing
- **When writing a function signature**, take `&str` not `&String`, `&[T]` not
  `&Vec<T>`, `&T` not `T` — unless the function stores, consumes, or returns the
  value. *(c11)*
- **When the caller retains ownership**, take borrowed data
  (`fn parse_module(source: &str) -> Result<ModuleAst, ParseError>`); take owned
  data only when the value must outlive the caller or cross threads/tasks.
- **When a borrow removes an allocation at the call site**, accept
  `impl AsRef<str>` / `impl AsRef<Path>` / `impl IntoIterator<Item = T>`.
- **When a value is usually borrowed but occasionally owned**, return/store
  `Cow<'_, T>` instead of cloning unconditionally.
- **When you need shared ownership**, use `Arc` across threads/tasks and `Rc`
  only in single-threaded code. Use interior mutability only when it simplifies a
  real ownership problem, not as a shortcut around design.
- **When you reach for `Arc<Mutex<T>>` reflexively**, stop — it usually signals
  the data model is wrong; use it only for genuine shared ownership, isolated
  behind a small API. *(c11)*
- **When a lock guard would cross an `.await`**, restructure so it doesn't; use
  `tokio::sync` primitives for state held across await points and
  `spawn_blocking` for CPU-bound work. *(c11)*
- **After changing a function body**, re-audit every variable and parameter
  declared `mut` (and every `&mut`): if it is no longer mutated, downgrade it to
  immutable / `&T`, and remove all now-unnecessary cloning. This can cascade —
  removing one `&mut` may make a caller's binding, and that caller's parameter,
  no longer need `mut` either; follow the chain and downgrade each link to
  read-only as far as it propagates.

### Encapsulation & types
- **When constructing a type with invariants**, route through
  `new`/`try_new`/builders that validate, not struct literals from outside the
  module. A type's invariants are enforced by its own constructor/methods, making
  it impossible to construct an invalid instance from outside.
- **When a value has a distinct meaning** (an ID, an email, signature bytes), use
  a newtype (`struct UserId(u64)`) not a type alias, and do not route unrelated
  values through one generic byte/string/integer type.
- **When behavior reasons about a type's data**, put it in an `impl` block on the
  type that owns the data — do not write free functions that reach into the data
  structure. State and methods travel together; fields stay private; callers go
  through methods.
- **When a value must change *type* (not just data)**, use the typestate pattern:
  consume `self` and return the new type for the caller to swap in (e.g. a
  `Search` resolving to an `Int`). When a value changes data but not type,
  self-mutate via `&mut self`.
- **When a type answers a question about itself** (a predicate or projection like
  `state()`, `is_search()`, `as_int()`), expose it as a method that hides the
  `match` inside — do not force callers to match on external tags or variants.
  Reports return owned values or short-lived borrows, never a long-lived handle
  that lets a caller mutate shared state behind the owner's back.
- **When a public enum or struct may gain variants/fields later**, mark it
  `#[non_exhaustive]`.
- **When a function returns a `Result` or a value pointless to discard**, mark it
  `#[must_use]`.
- **When matching on your own enum**, enumerate variants — avoid a catch-all `_`
  so new variants force a compile error.
- **When structuring a library crate's public surface**, keep modules private and
  re-export a curated API from `lib.rs`. Internals then cannot leak by accident,
  and the whole public surface is auditable in one place. *(c25, c26)*
  ```rust
  // lib.rs — not this:        // but this:
  pub mod format;              mod format;
  pub mod signature;           mod signature;
                               pub use format::{EinmoFile, Status};
                               pub use signature::Stamp;
  ```
- **When data is inherently ordered** (file sections, statements, a stamp chain),
  store a `Vec` of proper structs. A `HashMap` plus a side list to remember
  insertion order is a data-structure mismatch. *(c25)*
  ```rust
  struct Section { name: String, body: Vec<u8> }
  sections: Vec<Section>,
  // not: sections: HashMap<String, Vec<u8>>, sections_list: Vec<String>
  ```
- **When a field ranges over a finite set of words** (`"normal"` /
  `"input-error"` / `"output-error"`), make it an enum with `as_str()`/`parse()`,
  never a bare `String` — typos become compile errors and matches become
  exhaustive. When the wire format is textual, keep a separate serde DTO (a
  `StampWire` beside the domain `Stamp`) and convert: the domain type keeps its
  invariants; the wire type absorbs format quirks. *(c25)*
- **When a function's outcome depends on ambient side effects** (a tty prompt,
  stdin, env vars), gather the inputs into a small struct and inject the side
  effect as a closure, so the function is a pure decision over inputs — six
  positional parameters with `bool` flags that trigger real `/dev/tty` reads are
  untestable. *(c25)*
  ```rust
  fn resolve_stage_key(
      inputs: KeyCascadeInputs,
      prompt: impl FnOnce() -> io::Result<String>,  // tests: || panic!("no prompt expected")
  ) -> Result<KeySource>
  ```

### The four OOP pillars in Rust — grounded in einmo

Rust is not a class-based language, but the four pillars of object-oriented design map cleanly
onto its constructs, and this codebase uses all four. Agents most often fail on the first
pillar, so it receives the most extensive treatment. Every example below is drawn from einmo's
own source (`src/`).

#### Encapsulation — insist on it

**Association is programmatic marking.** A function that *changes* an object's state, and a
function that *reports* on an object's state, are both declared inside that object's `impl`
block. `impl TheType { … }` is the machine-checked statement "this behavior belongs to this
data." A free function that reaches into another type's fields is a design defect: it forces the
fields `pub`, scatters the type's behavior across the crate, and lets invariants be violated
from anywhere.

```rust
// signature.rs — Stamp's fields are all private; construction from the wire
// format and every accessor live in Stamp's own impl block:
pub struct Stamp {
    key: String,
    pubkey_hex: String,
    signs: StampRole,
    signature_b64: String,
    produced_by: String,
    timestamp: String,
}

impl Stamp {
    pub fn key(&self) -> &str { &self.key }               // reporting
    pub fn is_stage(&self) -> bool { /* … */ }             // reporting
    fn from_json_line(line: &str) -> Result<Self> { /* … */ }  // construction
}
```

Nothing outside `signature.rs` can construct a malformed `Stamp` or read a field the type
doesn't choose to expose — every NYES-style state transition in a signing-heavy crate like this
one should flow through exactly this kind of single audited door.

**Reporting belongs to the reporter.** A caller must never re-derive a fact the type can state
about itself — the predicate lives with the data it judges:

```rust
// Stamp::is_stage — callers write `stamp.is_stage()`; nobody outside this
// impl re-implements "starts with stage: and signs prior-bytes" by hand.
pub fn is_stage(&self) -> bool {
    matches!(self.signs, StampRole::PriorBytes) && self.key.starts_with("stage:")
}
```

**Private defensively, public by design.** `EinmoFile` is the exemplar: every field private,
construction only through `EinmoFile::new(…)`, every part reached only through its own
accessors. Nothing about its layout is API.

```rust
pub struct EinmoFile {
    encoding: String,
    separator: String,
    metadata: Metadata,
    sections: Vec<Section>,
    stamps: Stamps,
    advisory: Option<String>,
}

impl EinmoFile {
    pub fn metadata(&self) -> &Metadata { &self.metadata }
    pub fn sections(&self) -> &[Section] { &self.sections }
    pub fn stamps(&self) -> &Stamps { &self.stamps }
}
```

The widening ladder is deliberate at every rung: private → `pub(super)` → `pub(crate)` → `pub`,
and each widening is justified by an actual caller or a designed API surface. Defaulting to
`pub` because "someone might need it" is how internals become un-fixable contracts.

#### Abstraction

Expose the *what*; hide the *how*, behind a small named capability. Einmo's `Evaluator` trait is
the house example: an entire interpreter — lexer, parser, runtime, whatever it needs internally
— sits behind one method, `fn evaluate(&self, source: &str) -> Result<Vec<String>, String>`.
`EinmoSuite::evaluate_all` never knows or cares whether the concrete evaluator shells out to a
subprocess, embeds an interpreter, or is a test double; it only knows the capability.

```rust
pub trait Evaluator: Sync {
    fn evaluate(&self, source: &str) -> std::result::Result<Vec<String>, String>;
}
```

Likewise `ValidationLevel::escalation()` abstracts "which levels does this level imply" behind
one method, so the engine that walks levels never repeats the escalation logic itself.

#### Inheritance

Rust has no implementation inheritance, and this codebase does not miss it: the substitute is
**composition**. `EinmoFile` *has-a* `Metadata`, a `Vec<Section>`, and a `Stamps` — shared
structure reached through composition and this type's own accessors, not a base class:

```rust
impl EinmoFile {
    pub fn new(
        encoding: impl Into<String>,
        separator: impl Into<String>,
        metadata: Metadata,
        sections: Vec<Section>,
        stamps: Stamps,
    ) -> Self {
        EinmoFile { encoding: encoding.into(), separator: separator.into(), metadata, sections, stamps, advisory: None }
    }
}
```

Where behavior (not just data) needs to be shared across variants, prefer a trait's default
method bodies, overridden only where a kind genuinely differs — do not emulate class hierarchies
with `Deref` tricks, giant "base" structs, or macro-generated delegation towers.

#### Polymorphism

`&dyn Evaluator` is einmo's uniform-dispatch point: `EinmoSuite::evaluate`,
`EinmoSuite::evaluate_all`, and the internal `evaluate_capturing` helper all take **any**
`Evaluator` implementation the same way — one operation (`evaluate_all`), many possible
implementations (different `Evaluator` impls), and the suite code never matches on which
concrete evaluator it was given.

```rust
pub fn evaluate_all(&self, evaluator: &dyn Evaluator) -> Result<TestResults> { /* … */ }
```

This is precisely when `dyn` is right: a uniform operation over an open, unbounded set of
implementations supplied by the crate's callers. Its complement is static polymorphism — enum
dispatch (§7): choose the exhaustive enum `match` when the variant set is finite and known and
exhaustiveness matters (e.g. `StampRole`, `ValidationLevel`); choose `dyn` when the
implementation set is open-ended and the caller must stay implementation-agnostic, as with
`Evaluator`.

### Loops & matching
- **When transforming or filtering a collection**, use an iterator chain
  (`map`/`filter`/`filter_map`/`fold`/`zip`/`enumerate`/`windows`/`chunks`) and
  `collect`, not an indexed `for i in 0..n` loop.
- **When a `collect` can fail**, collect into `Result<Vec<_>, _>` with `?` to
  short-circuit.
- **When inserting-or-updating a map**, use the `entry()` API, not a double
  lookup.
- **When the final size is known**, pre-allocate with `with_capacity`.
- **When binding refutably on the happy path with an early return otherwise**,
  use `let … else` (stable 1.65):
  ```rust
  let Some(user) = lookup(id) else { return Err(Error::NotFound); };
  ```
- **When MSRV ≥ 1.88 and edition 2024**, flatten nested conditionals with
  let-chains: `if let Some(x) = a && x.is_valid() && let Ok(y) = f(x) { … }`.
  *(c12)* Otherwise use nested `if let` / `match`.
- **When checking a discriminant for a boolean**, use `matches!` instead of a
  full `match`.
- **When matching on a `&T`**, bind by reference; use `@` bindings, `|`
  alternatives, range patterns, and guards instead of nested `if`s.

### Traits & generics
- **When converting between types**, implement `From`/`TryFrom` (and let `?` use
  the `From` impl for error conversion) rather than ad-hoc methods.
- **When a function returns an iterator**, return `impl Trait` rather than boxing.
- **When defining a trait**, keep it small and named after a capability with
  stable semantics (`trait Clock { fn now(&self) -> Result<Timestamp, ClockError>; }`).
  Avoid broad traits with many unrelated methods.
- **When using generics**, keep bounds close to the function that needs them and
  do not spread complex bounds across the codebase. Use a concrete type until a
  real abstraction or a second caller justifies the generic.
- **When dispatching over a known, finite set of variants**, prefer matching the
  enum and calling a concrete method over a trait object — see *Enum dispatch*
  under Project-specific rules.
- **When writing new trait code with async methods (MSRV ≥ 1.75)**, use native
  `async fn` in traits / RPITIT; fall back to the `async-trait` crate only when
  you need `dyn` dispatch. *(c13)*
- **When a public type is defined**, derive `Debug` (and `Clone`/`PartialEq`/
  `Eq`/`Hash`/`Default`/`Ord` where sensible). *(c8, c14)*

### Globals & formatting
- **When you need a lazily-initialized static (MSRV ≥ 1.80)**, use
  `std::sync::LazyLock` (or `OnceLock`, 1.70); use `once_cell` only below that.
  Prefer avoiding global mutable state entirely. *(c15, c16)*
- **When interpolating a variable into a format string**, inline it:
  `format!("{x}")`. (Field access like `{self.x}` still needs the positional
  form.)

### Errors
- **When callers must branch on the failure mode**, define a matchable error enum
  with `thiserror`, with domain-specific variants. *(c17)*

  ```rust
  pub enum AttestationError {
      InvalidTimestamp,
      InvalidSignature,
      UnknownPeer,
      ReplayDetected,
      StorageFailure(StorageError),
  }
  ```
- **When a library crate spans several modules**, centralize on **one**
  `#[non_exhaustive]` error enum in `error.rs` with a `pub(crate) type Result<T>`
  alias. Five per-module enums (`ConfigError`, `FormatError`, `StageError`, …)
  force consumers to juggle types and lose context through `#[from]` chains.
  Annotate I/O variants with the offending path. *(c25)*
  ```rust
  #[non_exhaustive]
  #[derive(Debug, thiserror::Error)]
  pub enum EinmoError {
      #[error("i/o error at {path}")]
      Io { path: PathBuf, #[source] source: std::io::Error },
      #[error("malformed envelope: {0}")]
      Parse(String),
      // …
  }
  pub(crate) type Result<T> = std::result::Result<T, EinmoError>;
  ```
- **When callers only report or propagate**, use an opaque error — `anyhow`/
  `eyre` with `.context(...)` in applications; don't mix application error types.
  *(c18)*
- **When wrapping an underlying error**, preserve the chain with `#[source]` /
  `#[from]` and use `#[error(transparent)]`.
- **When recovering from a failure**, return `Result<T, E>` — error messages may
  be human-readable, but program logic must never depend on parsing error
  strings.

### `unsafe`
- **When `unsafe` is unavoidable**, keep the block minimal, wrap it in a small
  safe abstraction, document invariants in a `// SAFETY:` comment with plain-text
  reasoning, and ensure it passes Miri. Unsafe code should be rare, isolated, and
  easy to audit. *(c19)*
- **When a crate needs no `unsafe` at all** (most crates here — and every
  crypto-touching crate), declare `[lints.rust] unsafe_code = "deny"` in its
  `Cargo.toml` so none can creep in later. `"warn"` is not a gate. *(c25)*

---

## 4. Do's

Affirmative mandates. A change is not complete until these hold.

### Tooling gates (hard — run before "done")
```bash
cargo fmt --all                                            # rustfmt owns formatting
cargo clippy --all-targets --all-features -- -D warnings   # warnings are errors
cargo test          # or: cargo nextest run
```
*(c4, c20)*

- **Do** configure lints centrally in `[workspace.lints]` / `[lints]` (Rust
  1.74+): deny `clippy::correctness`, warn and cherry-pick from
  `clippy::pedantic`.
- **Do** override a lint with `#[expect(...)]` plus a one-line reason, so stale
  overrides surface when fixed. *(c4, c21)*
- **Do** use `?` to propagate errors. (Reserve `expect` for true invariants, with
  a message saying *why* it cannot fail.)
- **Do** keep fields private and expose behavior; scope visibility with
  `pub(crate)` / `pub(super)`, and keep public module surfaces small with
  intentional re-exports.
- **Do** associate behavior with data (Rule zero, §2): state-changing and
  state-reporting functions go in the owning type's `impl` block — or an
  extension trait when the type is foreign — never as free functions reaching
  into another type's fields.
- **Do** re-audit mutability after changing a function: if a `mut` variable or
  `&mut` parameter is no longer mutated, downgrade it to read-only, and follow
  the resulting cascade outward to callers.
- **Do** prefer iterator adaptors, `let … else`, `match`, and `matches!` over
  `.unwrap()` chains.
- **Do** derive standard traits and implement `From`/`Display`/`FromStr`.
- **Do** write tests *first* — write the most important behaviors, invariants,
  and unclear corner cases as tests before coding, so the tests document the
  feature. Pass the tests before committing. (See *Testing* in Project-specific
  rules for what to cover.)
- **Do** start a bug fix with a regression test that reproduces the failure, then
  repair, then commit the code that passes the new test.
- **Do** write `///` docs on public items: first sentence one line (≤ ~15 words),
  with `# Examples` / `# Errors` / `# Panics` / `# Safety` where they apply, and
  use `?` (not `.unwrap()`) in doc examples. *(c11, c22)*
- **Do** write a crate-level `//!` doc in `lib.rs` stating the crate's purpose,
  core model, and load-bearing invariants — a new reader learns the model before
  the API. A `lib.rs` that is only re-exports is missing its front door. *(c25)*
- **Do** implement or omit — a capability either works or does not exist. Never
  ship a no-op stub: a CLI flag that parses and is ignored, a config field
  nothing reads, a `verify()` that returns an empty report. Stubs in `--help`
  and in the API mislead users, reviewers, and other agents. *(c25)*
- **Do** comment to explain *why*, not *what*. If a comment is needed to explain
  what the code does, first make the code clearer.
- **Do** follow RFC 430 casing: acronyms as words (`Uuid`, `parse_xml`).

### Self-check before submitting
1. Did I add any `.clone()`? Can each be a borrow or move?
2. Is every `mut` actually mutated? Is every `&mut` written through? After
   editing a function, did I re-check and downgrade any now-unused `mut`/`&mut`,
   following the cascade to callers?
3. Are new struct fields private with intentional visibility?
4. Did I use an index loop where an iterator reads clearer?
5. Any `.unwrap()`/`.expect()` that should be `?` or `let else`?
6. Did I match exhaustively and destructure rather than poke at fields?
7. Did I derive standard traits and use `From`/`Display`/`FromStr`?
8. Are lazy statics on `LazyLock`/`OnceLock`?
9. Are format args inlined (`{x}`)?
10. Did I write/run tests first, and a regression test for any bug fix?
11. Did I leave any no-op stub — a parsed-but-ignored flag, a dead config field,
    a function returning an empty placeholder?
12. Are my tests hermetic (temp dirs, injected side effects, no writes to
    committed baselines), and does none of them assert a stub's empty result?
13. Did I touch generated/vendored/do-not-edit code? (Revert if so.)
14. Does it pass `cargo fmt --check`, `cargo clippy -D warnings`, and tests?

---

## 5. Preferences

Weaker directives — defaults to lean toward, not hard gates.

- **Prefer** explicit data flow, small functions with clear names, local
  reasoning over global cleverness, and boring, obvious code over clever code.
- **Prefer** descriptive local names; the lowercased type name is a fine default.
  *(c9)*
- **Prefer** grouping imports std → external → `crate::` → `super`, with a blank
  line between groups.
- **Prefer** a plain `for` loop for purely side-effecting work over forcing
  `for_each` just to look functional.
- **Prefer** concrete types until a second caller or real abstraction justifies a
  generic.
- **Prefer** error combinators (`map`, `and_then`, `ok_or_else`,
  `unwrap_or_else`, `map_err`) where they read more clearly than a `match`.
- **Prefer** fewer crates; be wary of small helper crates and proc-macro deps
  that add compile time (`itertools`/`either` are reasonable exceptions).
  *(c9, c10)*
- **Prefer** avoiding global mutable state even where a `static` would compile.
  *(c16)*
- **Prefer** keeping a type and its `impl` blocks together in one module.
- **Prefer** organizing code by responsibility (`parser`, `lexer`,
  `diagnostics`, `wire`, `storage`, `clock`, `ffi`) over dumping helpers into a
  large `utils` module. When one module accretes several jobs — e.g. a `stage.rs`
  holding the `Stage` enum, directory ops, promotion, flagging, *and* signature
  scanning — split it (`stage.rs` + `transitions.rs` + `error.rs`) rather than
  letting it grow past one responsibility; the split also kills the
  copy-paste-a-fourth-directory-walker temptation. *(c25)*
- **Prefer** macros sparingly: a macro is acceptable only when it removes
  unavoidable repetition while preserving clarity. Reach for functions, traits,
  or ordinary modules first.
- **Prefer** measuring before obscuring code for performance: optimize algorithms
  before micro-optimizing syntax, and document any performance-driven decision
  with a `//`-comment saying why.

---

## 6. Don'ts

Prohibitions and anti-patterns. The right-hand side is the replacement.

- **Don't `.clone()` to satisfy the borrow checker.** → borrow `&T`, restructure,
  or move.
- **Don't add `mut` you don't use** (binding or `&mut` parameter). → immutable
  binding / `&T`.
- **Don't `.unwrap()` / `.expect()` / `panic!` in library, protocol, parser,
  interpreter, FFI, or production paths.** → `?`, `let … else`, real error types.
  *(c2)*
- **Don't expose `pub` struct fields** to skip an accessor (unless the type is
  intentionally plain data). → private fields + `pub(crate)` + newtypes.
- **Don't use stringly-typed errors or `Box<dyn Error>` everywhere.** →
  `thiserror` enums (matchable) / `anyhow` (opaque), chosen by caller intent.
- **Don't hand-write `Display`/`From`/`Default`** that derive or `thiserror`
  gives you. → derive macros.
- **Don't write `&Vec<T>` / `&String` parameters.** → `&[T]` / `&str` (or
  `impl AsRef<…>`).
- **Don't loop by index to build a `Vec`.** → iterator chain + `collect`.
- **Don't double-lookup a map to insert/update.** → `entry()` API.
- **Don't use `lazy_static!` / bare `once_cell`** on a modern MSRV. →
  `LazyLock` / `OnceLock`.
- **Don't keep `extern crate`** (pre-2018). → plain `use`.
- **Don't use the `try!(...)` macro.** → `?`.
- **Don't add `mod.rs` files.** → path-based modules (`foo.rs` + `foo/`).
- **Don't write `format!("{}", x)`.** → `format!("{x}")`.
- **Don't reach for `async-trait` in new code** unless you need `dyn`. → native
  async fn in traits (1.75).
- **Don't use `#[allow(lint)]`.** → `#[expect(lint)]` with a reason.
- **Don't use `UUID` / `parse_XML` casing.** → `Uuid` / `parse_xml` (RFC 430).
- **Don't hold a `Mutex`/`RwLock` guard across `.await`.** → `tokio::sync`
  primitives / restructure.
- **Don't catch-all `_` on your own enums.** → enumerate variants.
- **Don't replace clear enum dispatch with trait objects** just because
  "polymorphism is cleaner." → match the enum; use `dyn` only for genuine runtime
  extensibility.
- **Don't write large functions** that mix validation, transformation, I/O, and
  mutation. → split by responsibility.
- **Don't bury protocol/decision logic inside async tasks** where it can't be
  tested. → separate protocol state from I/O.
- **Don't `pub mod` internal modules from `lib.rs`.** → private `mod` + curated
  `pub use` re-exports. *(c25, c26)*
- **Don't scatter one error enum per module in a library.** → one central
  `#[non_exhaustive]` enum in `error.rs` + `pub(crate) type Result<T>` alias.
  *(c25)*
- **Don't store ordered data in a `HashMap` with a side list tracking order.** →
  `Vec` of structs; order lives in the container. *(c25)*
- **Don't type a finite word-domain as `String`.** → enum + `as_str()`/`parse()`
  (+ a serde wire DTO when the format is textual). *(c25)*
- **Don't ship no-op stubs** — parsed-but-ignored flags, dead config fields,
  functions returning empty placeholder reports. → implement or omit. *(c25)*
- **Don't call `process::exit` inside CLI dispatch.** → return
  `std::process::ExitCode` from `main()` and the dispatch function, so tests can
  call the dispatcher and assert codes. *(c25)*
- **Don't edit generated, vendored, or do-not-edit code.** *(c7)*
- **Don't write unsound code, ever.** *(c3)*

---

## 7. Project-specific rules (einmo)

These are particular to this repository. **On any conflict with the general
guidance above, these win.** Ported from the original `foolish-rust`
workspace's `rust_instructions.md`; the Foolish/FIR-specific, Foretias
networking/FFI-specific, and client-bindings-specific subsections have been
dropped — einmo has no FIR, no P2P networking, no C11 core, and no
cross-language bindings. What remains below is directly load-bearing for
einmo's own domain: it is a cryptographic signing/verification library with
a CLI.

### Project-aware priority: code is always security-critical
Treat all Rust here as security-critical. Make invalid protocol states difficult
or impossible to represent: prefer explicit state machines, newtypes, checked
constructors, and narrow APIs. Be strict with parsing, validation, serialization,
signatures, timestamps, and boundary checks.

### Enum dispatch
Matching on an enum and calling a concrete method is acceptable and often
preferred, including a fully qualified method path when clearer or more efficient:

```rust
match stage {
    Stage::Output => OutputStage::promote(ctx),
    Stage::Checked => CheckedStage::promote(ctx),
    Stage::Verified => VerifiedStage::promote(ctx),
}
```

This is fine even when the method belongs to a trait implemented by the struct
holding the data. Prefer **enums** when the variant set is known and finite,
exhaustiveness matters, state transitions must be explicit, serialization depends
on variant identity, or static dispatch helps optimization. Prefer **traits**
when multiple independent types share behavior, the implementor set may grow
externally, or you need behavior abstraction more than variant inspection.

### Serialization & parsing
Parsing must be strict: reject malformed, ambiguous, non-canonical, or trailing
data unless the format explicitly allows it, and don't accept multiple encodings
for one logical value in security-sensitive formats (the `.einmo` envelope
format is exactly this). Keep parsing and validation separate where useful
(`let raw = RawMessage::decode(bytes)?; let msg = raw.validate()?;`).

When parsed content is covered by a signature, verification must check the
**actual raw bytes** on disk (or an explicitly documented canonical form) — a
parser that trims or normalizes inside a signed region lets whitespace tampering
verify successfully. Either keep `parse` byte-exact under signatures, or
canonicalize first and sign the canonical bytes (RFC 8785-style); never an
undocumented mix of the two. This is the exact discipline `format.rs`'s
separator-collision refusal and `signature.rs`'s stamp-chain verification
already implement — extend it the same way. *(c25, c28)*

### Cryptographic and security-sensitive code
- Never invent cryptographic protocols or alter protocol details casually.
- Use constant-time comparison for secrets, signatures, MACs, and auth tags where
  required.
- Validate before trust, and prefer types that distinguish unverified from
  verified data:
  ```rust
  let signed = EinmoFile::from_file(path)?;   // verifies all stamps
  let body = signed.verified_body();          // only reachable after verification
  ```
  Only trusted constructors create verified types. Do not continue after a
  verification failure unless the protocol explicitly requires it
  (verify-on-inspect is einmo's core invariant — see `README.md`).
- Do not log secrets, private keys, raw passphrases, or unreduced protocol
  internals. Do not expose test-only shortcuts in production APIs.
- **Pin key-derivation and work-factor parameters as named constants** with a
  comment citing the rationale — never rely on a dependency's defaults, which a
  minor version bump can silently change (every previously derived key would
  stop matching). Domain-separate salts. Changing pinned parameters invalidates
  all derived keys, so it implies a corpus re-sign. *(c25, c27)*
  ```rust
  // OWASP Password Storage Cheat Sheet minimum baseline for Argon2id.
  const ARGON2_MEMORY_KIB: u32 = 19_456;
  const ARGON2_TIME_COST: u32 = 2;
  const ARGON2_PARALLELISM: u32 = 1;
  const SALT: &[u8] = b"einmo:stamp-key:v1"; // domain-separated
  ```
- **Provenance and attestation fields carry real values.** A
  `sha256:placeholder` in a signed stamp defeats the field's entire purpose;
  compute the real hash (`env::current_exe()` + SHA-256) or do not emit the
  field. *(c25)*

### Time handling
Do not call system time deep inside signing/verification logic — inject a
`Clock` trait so tests are deterministic. Distinguish local observation time,
claimed timestamp, and verified (stamped) timestamp. Never compare timestamps
without knowing which kind they are.

### Concurrency & async
Make concurrency explicit and testable. Prefer message-passing or narrow
synchronization over wide shared locks; never hold a lock across `.await`. Every
spawned task has a clear owner, a shutdown path, error handling, and tests where
practical — don't ignore a `JoinHandle` unless the task is intentionally detached
and documented. (Directly relevant to the `EinmoReview` session object —
`EIMP-1` §S.2 — whose `VerifiedCache` and `exec` mutex are exactly this
discipline.)

### CLI binaries
`main()` and the CLI dispatch function return `std::process::ExitCode`; never
call `process::exit` inside dispatch (it makes the dispatcher untestable).
Every subcommand supports `--json` machine output — agents script against the
CLI, and a verb without `--json` forces them to parse prose. The `cargo-einmo`
alias binary beside `einmo` is a one-line wrapper over the same parser, never a
second implementation. *(c25)*

### Logging & observability
Log state transitions, promotion/flag events, and signature-verification
failures. Never log private keys, secret material, raw passphrases, or
unsanitized untrusted payloads. Errors carry enough context to debug, but no
sensitive data.

### Dependencies
Do not add dependencies casually. Before adding a crate, weigh security posture,
maintenance status, API stability, transitive weight, and whether it touches
cryptography, parsing, or serialization. Prefer mature, audited, widely used
crates for security-sensitive needs. Do not change cryptographic dependencies,
serialization formats, protocol behavior, or public APIs without understanding
compatibility and security impact.

### Panics & assertions
Use `debug_assert!` for internal invariants during development; use normal error
handling for invalid external input — files, passphrases, serialized `.einmo`
data, clock or storage failures. A malformed `.einmo` file or a bad timestamp is
not a reason to panic.

### Testing
Write tests for behavior, invariants, and edge cases, and prefer deterministic
tests (inject clocks, RNGs, storage backends). Cover: valid signature
verification, invalid/tampered signatures, timestamp boundaries, malformed
`.einmo` files, serialization round trips, and illegal stage transitions. Use
property or fuzz tests for parsers, decoders, serialization, and the `.einmo`
envelope format.

Tests are **hermetic**: run in temp directories (`tempfile`), inject side
effects, and never modify committed repository state. Test *count* is not the
metric; behavioral coverage is — one `parallel_and_serial_agree` or
`illegal_transition_refused` outweighs a dozen granular roundtrip variants, and
a test asserting a stub's empty result pins the absence of a feature. *(c25)*

### Final rule
When uncertain, choose the design that is easiest to prove correct, easiest to
test, and easiest for the next human to understand. Correctness first, then
readability and maintainability, then efficiency, then principles and aesthetics.

---

## Disclosure

Disclose AI involvement in commit messages and PR descriptions, and ensure all
AI-generated code is human-verified before submission. *(c23, c24)*

---

## Citations

> **Maintenance reference only.** These sources back the rules above for the
> benefit of whoever updates or audits this document. You do not consult them to
> write code — follow the rule text.

- **c1** — Rust Blog, "Announcing Rust 1.85.0 and Rust 2024," 2025-02-20.
- **c2** — nushell/AGENTS.md (stable-only toolchain; "Never use `.unwrap()`").
- **c3** — Microsoft Rust guidelines, M-UNSOUND.
- **c4** — astral-sh/ruff project conventions (illegal-states-unrepresentable;
  central lint config; `#[expect]`; CI gates).
- **c5** — Microsoft Rust guidelines, M-STRONG-TYPES.
- **c6** — Microsoft Rust guidelines, M-DI-HIERARCHY.
- **c7** — Azure/azure-sdk-for-rust conventions (standard-trait conversions;
  do-not-edit generated code).
- **c8** — Rust API Guidelines, C-CONV and C-COMMON-TRAITS.
- **c9** — rust-analyzer style guide (naming; fewer crates; compile time).
- **c10** — Microsoft Rust guidelines, M-SMALLER-CRATES.
- **c11** — leonardomso/rust-skills (signatures; `Arc<Mutex>` smell; locks across
  await; doc examples).
- **c12** — Rust Blog, "Announcing Rust 1.88.0," 2025-06-26 (let-chains).
- **c13** — Rust Blog, async fn in traits, 2023-12-21.
- **c14** — Microsoft Rust guidelines, M-PUBLIC-DEBUG.
- **c15** — Rust Blog, `LazyLock` stabilization, 2024-07-25.
- **c16** — Microsoft Rust guidelines, M-AVOID-STATICS.
- **c17** — Microsoft Rust guidelines, M-ERRORS-CANONICAL-STRUCTS.
- **c18** — Microsoft Rust guidelines, M-APP-ERROR.
- **c19** — Microsoft Rust guidelines, M-UNSAFE.
- **c20** — astral-sh/ruff and uv CI (fmt + clippy `-D warnings` + test gates).
- **c21** — Microsoft Rust guidelines, M-LINT-OVERRIDE-EXPECT.
- **c22** — Microsoft Rust documentation guidelines.
- **c23** — rust-analyzer CLAUDE.md (AI disclosure / human verification).
- **c24** — pola-rs/polars AI_POLICY.md.
- **c25** — FOOP-54 §9 "Best Practices Review" (`docs/foop/FOOP-54.md`) — lessons
  from the two-agent einmo implementation comparison (FOOP-54, mimo-opencode vs
  FOOP-92, Claude Opus 4.8; both under two hours; neutral-agent analysis).
- **c26** — Kobzol, "Two ways of interpreting visibility in Rust" (2025); insta's
  `lib.rs` tiered-visibility layout (private modules, curated re-exports).
- **c27** — OWASP Password Storage Cheat Sheet (Argon2id baselines); RustCrypto
  Book "Password Hashing"; age/rage scrypt parameter handling.
- **c28** — RFC 8785 (JSON Canonicalization Scheme); in-toto metadata model and
  DSSE migration (canonical bytes under signatures).

---

## Last Updated

**Date**: 2026-07-29
**Updated By**: Claude Code (Sonnet 5)
**Changes**: Ported this document from the `foolish-rust` workspace into
einmo's own repository (`docs/eimp/EIMP-0.md`). Sections 1–6 (Priorities
through Don'ts) carried over verbatim — they are general Rust guidance, not
Foolish-specific. §7 "Project-specific rules" was rewritten: dropped the
Foolish/FIR semantics, Foretias P2P networking/FFI/C11-boundary, and
client-bindings subsections (no analogue in einmo), and kept — rewording
examples to einmo's own types (`Stage`, `EinmoFile`) where the original used
Foolish/Foretias examples — the subsections directly relevant to a
cryptographic signing/verification library with a CLI: the security-critical
priority statement, enum dispatch, serialization & parsing (byte-exact
parsing under signatures), cryptographic/security-sensitive code, time
handling, concurrency & async, CLI binaries, logging & observability,
dependencies, panics & assertions, and testing. Everything below this entry
is the original document's history, left as written for the historical
record (per this project's own Markdown-update convention).

**Date**: 2026-07-14
**Updated By**: Claude Code 2.1.119 (Claude Code); Fable 5
**Changes**: Imported ALL fifteen recommendations of the FOOP-54 §9 "Best Practices Review"
(the mimo-opencode vs Claude Opus 4.8 einmo implementation comparison), each with a short
example, cited as *(c25)* (+ new c26–c28): private-modules + curated `pub use` (§9.1), one
central `#[non_exhaustive]` error enum + `Result` alias (§9.2), `Vec`-of-structs for ordered
data (§9.3), enums + wire DTOs over stringly-typed domains (§9.4), closure-injected side
effects in a params struct (§9.5), implement-or-omit / no no-op stubs (§9.6), real provenance
hashes (§9.7), pinned KDF parameters with domain-separated salts (§9.8), byte-exact parsing
under signatures (§9.9), `ExitCode` over `process::exit` (§9.10), module decomposition by
responsibility (§9.11), `unsafe_code = "deny"` (§9.12), hermetic behavior-focused tests
(§9.13), crate-level `//!` docs (§9.14), `--json` on every subcommand (§9.15). Added new
**§2 Task guides** — the rules re-indexed by task, most-frequent first: writing a function →
helper functions → naming → documentation → structuring a module → structuring a crate
(rare, last). Renumbered subsequent sections 3–7; added a "CLI binaries" project rule; added
self-check items 11–12 (stubs, hermetic tests).

Same day, second pass (Atlas direction): added **Rule zero — encapsulation** at the head of §2
(the four clauses: state-changing functions belong to the object's `impl`; reporting functions
likewise; private defensively / public by design; extension traits for foreign types), repeated
insistently in guides 2a/2b/2e; each §2 guide now ends with pointers into the reference
sections. Added the extensive **§3 "The four OOP pillars in Rust — grounded in UBCa"** reference
section: Encapsulation (programmatic marking via `impl`, `ProtoBrane::set_nyes` right/wrong
pair, `NyesExt::is_constanic`, `FirRefExt` extension-trait association, the private→`pub`
widening ladder), Abstraction (`ContextfulSearch` one-engine model, `CandidateNavigator`,
einmo's `Evaluator`, `Fir::core()`), Inheritance (composition + trait default methods;
`as_i64` override; no `Deref` hierarchies), Polymorphism (`dyn Fir` — `fir_op_step` dynamically
invoked: different impls do the same thing to different objects; enum dispatch as the static
complement). Added the matching Do (associate behavior with data).

**Date**: 2026-06-09
**Updated By**: Claude Code (Claude Code); Opus 4.8
**Changes**: Created `rust_instructions.md` by merging all Rust guidance from
`AGENTS.md` "How To Write Rust Code" into the cited general-Rust draft. Kept both
priority axes (project optimization order + construct-preference order).
Renumbered all inline citations to `(c#)` with a maintenance-only Citations
section. Added a Project-specific rules section (Foolish semantics, enum dispatch,
phase separation, crypto, time, concurrency, FFI, bindings, logging, deps, panics,
testing) that overrides general guidance on conflict. Confirmed all workspace
crates are on edition 2024.
