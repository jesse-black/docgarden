---
description: "Decision to build our own `docgarden match` compound-word splitter dictionary rather than adopting the `decompound` crate or a full English dictionary."
---

# Build our own compound-word splitter dictionary

## Context and Problem Statement

Stemming does not address the orthogonal problem of lowercase compounds where two distinct routing terms have been concatenated, such as `execplan` (combining `exec` and `plan`). Without expansion, BM25F sees only one rare opaque term, so queries for `exec` or `plan` do not match it.

The set of such compounds in this repository's vocabulary is small and curated. A new entry is added only when both halves carry routing signal in this repository's metadata; common-English compounds whose halves are too generic to improve routing are rejected.

How should the analyzer expand these compounds: its own explicit dictionary, a published Rust decompounder crate, or a full English dictionary?

## Considered Options

- **Build our own explicit dictionary.** Maintain a curated mapping from known compound tokens to replacement component tokens, applied per token alongside the rest of the analyzer chain.
- **`decompound` crate.** Rust dictionary-driven compound-word splitter that uses a caller-supplied word dictionary to detect compound forms at runtime, useful for languages whose compound vocabulary is too large or open-ended to enumerate.
- **Use a full English dictionary.** Scan a general-English word list across all lowercase tokens and split wherever halves are real words.

## Decision Outcome

Chosen option: **build our own explicit dictionary**, because:

- The set of repository compounds is bounded and curated. An explicit mapping keeps the analyzer behavior narrow, reviewable, and independent of a general-purpose decomposition dependency.
- The dictionary is a curatorial artifact. Each entry asserts that both halves carry routing signal in this repository's vocabulary, which is a judgment about repository content rather than about the English language.
- `decompound` is valuable when a dictionary of root words needs to be extended at runtime to cover immense or effectively unbounded compound vocabularies, such as German compound words. `docgarden match` does not need runtime discovery of arbitrary compounds; it needs explicit repository-routing mappings for known tokens. Using `decompound` would add a dependency and a broader decomposition model without removing the need for curated entries.
- Splitting against a full English dictionary creates noisy splits whenever a substring happens to be a real word. The curated dictionary is bounded by editorial review rather than by a lexical algorithm, and the explicit mapping directly enforces that boundary.
