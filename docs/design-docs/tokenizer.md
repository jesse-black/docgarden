---
description: "Design draft for `docgarden match` tokenization and Lucene analyzer comparison; read when changing query parsing, apostrophe handling, CamelCase splitting, compound-word handling, or tokenizer dependencies."
---

# Match Tokenization

## Purpose

This document owns the tokenization contract for `docgarden match`.

The tokenizer's job is to turn short repository metadata fields and user queries into the shared lexical tokens used by:

- corpus statistics
- BM25F scoring
- explain-mode coverage
- matched-term highlighting

Scoring details live in [`scoring.md`](scoring.md). This document describes the surface tokens emitted before later analyzer steps.

## Current State

The shipped tokenizer lives in `src/score.rs` and is reused by `src/matching.rs`.

It has two splitter wrappers:

- `normalize_text` for query strings, names, and descriptions
- `normalize_path` for path prefixes

The per-token analyzer is `analyze_token`.

The current tokenizer behavior is:

1. split text fields on whitespace or ASCII punctuation
2. split path prefixes on `/`, `_`, `-`, `.`, whitespace, or ASCII punctuation, after stripping a trailing `.md`

Index-time and query-time tokenization must stay symmetric. If a candidate field can produce a token, a query should produce the same token from the same surface form. Highlighting should also use the same tokenization path, so displayed matches do not drift from scoring.

## Current Differences From Lucene

`docgarden` is intentionally much smaller than Lucene or Tantivy, but Lucene remains a useful comparison point because the scorer is BM25F-shaped.

Lucene does not have one universal tokenizer. The closest baseline tokenizer is `StandardTokenizer`; code-identifier behavior such as CamelCase splitting comes from the separate `WordDelimiterGraphFilter`, usually after a tokenizer that preserves intra-word delimiter context.

Differences from Lucene `StandardAnalyzer`:

- **Unicode segmentation:** Lucene's standard tokenizer follows Unicode text-segmentation behavior. `docgarden` currently uses simple ASCII punctuation and whitespace splitting.
- **Token classes:** Lucene emits token types such as alphanumeric, numeric, Southeast Asian, ideographic, Hiragana, Katakana, Hangul, and emoji. `docgarden` emits only strings.
- **Internal apostrophes:** Lucene keeps internal apostrophes inside word tokens, so examples such as `O'Reilly`, `you're`, and `Jim's` remain one surface token. `docgarden` treats apostrophes as ASCII punctuation separators.
- **Numbers and dotted numeric forms:** Lucene keeps forms such as `21.35`, `216.239.63.104`, `R2D2`, and `C3PO` together. `docgarden` splits on ASCII punctuation and does not give numeric forms special handling.
- **Underscore and connector punctuation:** Lucene's Unicode word-break grammar treats connector characters such as `_` as part of alphanumeric tokens in some contexts. `docgarden` treats `_` as a separator for paths and as ASCII punctuation for text.
- **Emoji and CJK behavior:** Lucene has explicit tokenizer rules for emoji sequences and CJK scripts. `docgarden` has no script-aware token classes.
- **Maximum token length:** Lucene's `StandardAnalyzer` has a configurable max token length and splits overlong tokens at that length. `docgarden` currently has no tokenizer-level token length policy.

Differences from optional Lucene analysis components:

- **CamelCase:** `WordDelimiterGraphFilter` can split `PowerShot` into `Power` and `Shot` when `splitOnCaseChange` is enabled. `docgarden` currently keeps `ExecPlan` as one token, `execplan`.
- **Letter-number boundaries:** `WordDelimiterGraphFilter` can split `SD500` into `SD` and `500` when `splitOnNumerics` is enabled. `docgarden` currently keeps `sd500`.
- **Possessives:** `WordDelimiterGraphFilter` can remove trailing English possessives from subwords. `docgarden` currently treats apostrophes as punctuation separators.
- **Original-token preservation and catenation:** `WordDelimiterGraphFilter` can emit subwords, preserved originals, and concatenated variants. `docgarden` currently emits only one flat token stream.
- **URLs and email addresses:** Lucene's separate `UAX29URLEmailTokenizer` preserves URL and email shapes. `docgarden` currently splits them mechanically.
- **Compound words:** Lucene and Elasticsearch have dictionary or hyphenation decompounders for some languages and domains. `docgarden` currently does no decompounding, so lowercase compounds such as `execplan` remain one token.
- **Aliases and synonyms:** Lucene-style systems can add synonym filters or token override maps. `docgarden` currently has no alias layer, so domain terms such as `execplan` do not emit `exec` and `plan`.

## Recommended Direction

The next tokenizer improvement should target repository and code-identifier metadata, not broad fuzzy search.

Adopt:

- **CamelCase splitting.** `ExecPlan` should emit `Exec` and `Plan`; `PowerShot` should emit `Power` and `Shot`.
- **Internal apostrophe preservation with trailing possessive removal.** `O'Reilly` and `you're` should stay one token, while `Jim's` should emit `Jim` and `O'Reilly's` should emit `O'Reilly`.
- **Existing ASCII punctuation splitting except apostrophes.** Keep current punctuation boundary behavior for forms such as `planner-execplan`, `repository-local`, `rust-stemmers`, and `docs/PLANS.md`; apostrophes are the only punctuation class called out for near-term special handling.
- **A single tokenizer helper used by text, paths, scoring, and highlighting.** Any new split rule must affect query analysis and candidate analysis identically.

Consider later:

- **Unicode word-boundary splitting** if non-ASCII repository metadata becomes common enough that ASCII punctuation rules produce surprising output.
- **Conservative letter-number boundary splitting** if compact identifier patterns such as `ADR0004`, `RFC9110`, `PR42`, or `issue123` become common routing misses. Avoid splitting existing shapes such as `BM25F`, `v1`, `f32`, `u32`, and `R2D2` without a separate identifier policy.
- **A small explicit alias layer** for repository vocabulary such as `execplan => exec, plan` or carefully chosen role/task mappings. This should be a separate tokenizer expansion step.

Avoid for now:

- **General lowercase compound splitting.** There is no reliable boundary in `execplan`; substring or dictionary guessing can create noisy matches.
- **Preserving originals or catenating split parts by default.** Extra tokens inflate term frequency and field length in BM25F, so this should wait until there is a clear scoring design for multi-position or alternate tokens.
- **Using tokenizer rules to make `planner` emit `plan`.** That is a semantic alias problem, not a token-boundary problem. Broad derivational guessing would make unrelated words collide.

## Library Options

The current in-house tokenizer is small and easy to reason about. It remains the best default unless the project needs Unicode segmentation, CJK support, or a full search-engine tokenizer pipeline.

Options worth knowing:

- [`unicode-segmentation`](https://docs.rs/unicode-segmentation/latest/unicode_segmentation/) implements Unicode Standard Annex #29 grapheme, word, and sentence boundaries. It is the most plausible dependency if `docgarden` wants Unicode word splitting without adopting a search engine. It would not replace path splitting, CamelCase splitting, or apostrophe handling.
- [`tantivy`](https://docs.rs/tantivy/latest/tantivy/tokenizer/index.html) and [`tantivy-tokenizer-api`](https://docs.rs/tantivy-tokenizer-api/latest/tantivy_tokenizer_api/) provide Rust search-engine tokenizer traits and built-in tokenizers. They are a better fit if `docgarden` later adopts Tantivy-like indexing. For the current metadata router, they are probably more framework than needed.
- [`tokenizers`](https://docs.rs/tokenizers/latest/tokenizers/) is Hugging Face's Rust tokenizer pipeline for ML subword tokenization. It is optimized for BPE and model vocabularies, not transparent lexical routing, so it is not a good fit for `docgarden match`.
- [`lindera`](https://docs.rs/crate/lindera/latest) and [`jieba-rs`](https://docs.rs/crate/jieba-rs/latest) are useful for Japanese and Chinese segmentation. They are language-specific and heavier than the current English-oriented metadata use case.
- [`language-tokenizer`](https://docs.rs/language-tokenizer/latest/language_tokenizer/) wraps several language-processing paths, including CJK segmentation. It may be worth a spike for multilingual support, but it would outsource more policy than `docgarden` currently needs.

Recommended near-term choice: continue building the tokenizer in-house, add small code-identifier split rules, and consider `unicode-segmentation` only if Unicode word-boundary behavior becomes a real product requirement.

## References

- Lucene `StandardAnalyzer` source: https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/analysis/standard/StandardAnalyzer.java
- Lucene `StandardTokenizer` source: https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/analysis/standard/StandardTokenizer.java
- Lucene `StandardTokenizerImpl.jflex` source: https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/analysis/standard/StandardTokenizerImpl.jflex
- Lucene `WordDelimiterGraphFilter` source: https://github.com/apache/lucene/blob/main/lucene/analysis/common/src/java/org/apache/lucene/analysis/miscellaneous/WordDelimiterGraphFilter.java
- Lucene `WordDelimiterIterator` source: https://github.com/apache/lucene/blob/main/lucene/analysis/common/src/java/org/apache/lucene/analysis/miscellaneous/WordDelimiterIterator.java
- Lucene `UAX29URLEmailTokenizer` source: https://github.com/apache/lucene/blob/main/lucene/analysis/common/src/java/org/apache/lucene/analysis/email/UAX29URLEmailTokenizer.java
- Lucene `DictionaryCompoundWordTokenFilter` source: https://github.com/apache/lucene/blob/main/lucene/analysis/common/src/java/org/apache/lucene/analysis/compound/DictionaryCompoundWordTokenFilter.java
- Lucene `HyphenationCompoundWordTokenFilter` source: https://github.com/apache/lucene/blob/main/lucene/analysis/common/src/java/org/apache/lucene/analysis/compound/HyphenationCompoundWordTokenFilter.java
- Lucene `SynonymGraphFilter` source: https://github.com/apache/lucene/blob/main/lucene/analysis/common/src/java/org/apache/lucene/analysis/synonym/SynonymGraphFilter.java
- Elasticsearch `word_delimiter_graph`: https://www.elastic.co/docs/reference/text-analysis/analysis-word-delimiter-graph-tokenfilter
- Elasticsearch `synonym_graph`: https://www.elastic.co/guide/en/elasticsearch/reference/current/analysis-synonym-graph-tokenfilter.html
