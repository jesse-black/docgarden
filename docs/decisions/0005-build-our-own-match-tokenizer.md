---
description: "Decision to build our own `docgarden match` tokenizer rather than adopting a published Rust tokenizer crate; read when changing the tokenizer surface, evaluating Unicode word-boundary support, considering Tantivy or Hugging Face tokenizer pipelines, or considering non-English language tokenizers."
---

# Build our own match tokenizer

## Context and Problem Statement

`docgarden match` ranks repository guidance with BM25F (per [ADR 0002](0002-use-bm25f-as-the-scoring-model.md)) over a small curated corpus of English-dominant repository metadata: short names, frontmatter descriptions, and document path prefixes. The tokenizer feeds query analysis, corpus statistics, scoring, and matched-term highlighting through one shared chain so an indexed token and a query token produce the same result from the same surface form.

The split rules this corpus needs do not match any single off-the-shelf tokenizer. The chain has to:

- split on whitespace and most ASCII punctuation
- keep internal apostrophes inside word tokens
- split path-prefix segments on path separators
- split CamelCase and CamelCase-acronym shapes
- expose one per-token entry point so scoring and highlighting cannot drift

Should `docgarden` build its own tokenizer, or adopt a published Rust tokenizer crate?

## Considered Options

- **Build our own rule-based tokenizer.** Implement the splitter and per-token analyzer directly in this repository.
- **`unicode-segmentation`** — Unicode Standard Annex #29 grapheme, word, and sentence boundaries.
- **Tantivy tokenizer pipeline** (`tantivy`, `tantivy-tokenizer-api`) — Rust search-engine tokenizer traits plus built-in tokenizers and filters.
- **Hugging Face `tokenizers`** — BPE / WordPiece / subword tokenizer pipeline aimed at ML model vocabularies.
- **CJK-specialized tokenizers** (`lindera`, `jieba-rs`) — Japanese and Chinese segmentation.
- **`language-tokenizer`** — wrapper crate over several language-specific paths, including CJK segmentation.

## Decision Outcome

Chosen option: **build our own rule-based tokenizer**, because:

- The split rules the corpus needs are not a single feature in any off-the-shelf tokenizer. CamelCase, internal apostrophe preservation, path-segment splitting, and a curated compound-word expansion are all shaped around repository routing rather than around a general language model. Wrapping a third-party tokenizer to add these rules on top would carry both the dependency and the rule set.
- The corpus is English-dominant and short. Unicode word-boundary segmentation is not a current product requirement; if it becomes one, `unicode-segmentation` can be added as a focused dependency for that one rule without adopting a framework.
- Subword tokenizers such as Hugging Face `tokenizers` are optimized for ML model vocabularies, not transparent lexical routing. Their token streams do not align with BM25F term identity.
- The Tantivy tokenizer pipeline assumes a Tantivy-shaped indexing path that `docgarden` does not have. Adopting it would force a broader architectural commitment that the curated-corpus router does not need.
- CJK-specialized tokenizers solve a segmentation problem that `docgarden` does not currently have, and they are heavier than the English-oriented metadata use case.
- The in-house tokenizer is small and easy to test. The cost of carrying it is bounded by the small set of rules expressed, and each new rule is a localized change to the shared chain.
