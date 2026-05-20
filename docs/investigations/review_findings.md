---
description: "Codebase review findings regarding Rust code smells and defensive programming principles; read when prioritizing Config invariants, CLI output modes, manual trait implementations, boolean flags, wildcard patterns, or temporary mutability cleanup."
---

# Codebase Review: Code Smells & Invariants

This document summarizes review findings for Rust code smells identified in the article **"I have a hobby"** and this repository's [CODESTYLE.md](../CODESTYLE.md). It focuses on smells that are not fully covered by the enabled Clippy lints.

## High Priority

These findings can hide invalid states, ambiguous user-facing behavior, or future drift in code that carries repository-wide policy.

### Finding 1: Public fields on invariant-bearing `Config`

[src/config.rs:246](../../src/config.rs) exposes every field on `Config`, even though `Config::load` canonicalizes paths and validates invariants such as non-empty include patterns and repository-relative configured directories.

**Why it's a smell:** Any module can bypass `Config::load` and construct a partially validated or invalid `Config` directly. [src/lint/mod.rs:225](../../src/lint/mod.rs) already demonstrates this with a test-only literal using `include: Vec::new()`.

**Defensive refactor:** Keep invariant-bearing fields private and provide accessors for read-only consumers. For test setup, prefer a small builder or fixture constructor that preserves the same validation shape as production code.

### Finding 2: `--path-only` and `--explain` are independent booleans

[src/cli.rs:123](../../src/cli.rs) defines `path_only: bool`, [src/cli.rs:129](../../src/cli.rs) defines `explain: bool`, and [src/matching.rs:114](../../src/matching.rs) / [src/matching.rs:122](../../src/matching.rs) silently let `path_only` win when both are set.

**Why it's a smell:** These flags are mutually exclusive output modes, but the type shape allows the ambiguous state. That makes call sites and tests reason about precedence instead of preventing the invalid combination.

**Defensive refactor:** Add a clap conflict or lower the flags into an explicit `OutputMode::{Default, PathOnly, Explain}` before calling the matcher.

### Finding 3: Manual `Debug` implementation on `Config`

[src/config.rs:261](../../src/config.rs) manually lists fields for `Config` debug output.

**Why it's a smell:** If a new `Config` field is added, the implementation still compiles and silently omits the field or any intentional summary of it.

**Defensive refactor:** Destructure `self` fully inside `fmt`, then feed the named bindings into `debug_struct`. New fields will force an explicit include, summary, or named ignore.

### Finding 4: Boolean flags in `CandidateReference::new`

[src/lint/references.rs:78](../../src/lint/references.rs) accepts `uses_relative_syntax: bool` and `uses_workspace_root_syntax: bool`, producing call sites like `CandidateReference::new(value, false, false)`.

**Why it's a smell:** The two booleans describe one syntax mode, but the type shape permits unclear and potentially invalid combinations.

**Defensive refactor:** Replace the pair with a single enum such as `ReferenceSyntax::{Standard, Relative, WorkspaceRoot}`.

## Low Priority

These are useful hardening opportunities, but the current code is small and readable enough that they are lower-risk cleanup.

### Finding 5: Match wildcard `..` on `FrontmatterParseResult::Malformed`

[src/documents.rs:27](../../src/documents.rs) uses:

```rust
FrontmatterParseResult::None | FrontmatterParseResult::Malformed { .. } => (None, None),
```

**Why it's a smell:** `FrontmatterParseResult` is local. If `Malformed` gains another field, this match will continue silently.

**Defensive refactor:** Name the intentionally ignored field:

```rust
FrontmatterParseResult::None | FrontmatterParseResult::Malformed { line: _ } => (None, None),
```

### Finding 6: Temporary mutability in short assembly blocks

[src/lint/mod.rs:184](../../src/lint/mod.rs) keeps `sorted` and `rewritten` mutable while applying edits, and [src/matching.rs:82](../../src/matching.rs) keeps `results` mutable through sorting and truncation.

**Why it's a smell:** The variables only need mutability during assembly. Keeping mutability visible after that operation weakens the compiler's ability to prevent later accidental edits.

**Defensive refactor:** Use small initialization blocks that return immutable `sorted`, `rewritten`, and `results` values after sorting, truncation, or replacement work is complete.

### Finding 7: Threading `style_output: bool` through rendering helpers

[src/matching.rs:143](../../src/matching.rs), [src/matching.rs:159](../../src/matching.rs), [src/matching.rs:191](../../src/matching.rs), and [src/matching.rs:211](../../src/matching.rs) pass `style_output: bool` through multiple rendering helpers.

**Why it's a smell:** The boolean is readable in the function signatures, but call sites still have to remember that it means ANSI styling rather than output mode or escaping behavior.

**Defensive refactor:** Model it as a small rendering context or enum such as `ColorRendering::{Plain, Ansi}` if this formatting surface grows.
