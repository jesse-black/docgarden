---
description: "Design draft for the `docgarden match` analyzer chain — lowercasing, tokenization, stopword filtering, and Snowball English stemming; read when changing query parsing, apostrophe handling, CamelCase splitting, compound-word handling, stopword behavior, or stemming."
---

# Match Analyzer

## Purpose

This document owns the analyzer chain for `docgarden match`: lowercasing, tokenization, stopword filtering, and stemming.

The analyzer's job is to turn short repository metadata fields and user queries into the shared lexical tokens used by:

- corpus statistics
- BM25F scoring
- explain-mode coverage
- matched-term highlighting

BM25F mechanics live in [`scoring.md`](scoring.md). This document owns the full analyzer chain that produces the tokens BM25F operates on.

## Current State

The shipped analyzer lives in `src/score.rs` and is reused by `src/matching.rs`.

It has two splitter wrappers:

- `normalize_text` for query strings, names, and descriptions
- `normalize_path` for path prefixes

Both call the shared per-token entry point `analyze_token`, which is also called by `flush_render_token` in `src/matching.rs` so highlighting cannot drift from scoring.

### Analyzer Order

The shared analyzer chain is:

1. lowercase
2. split text fields on whitespace or ASCII punctuation, or split path prefixes on `/`, `_`, `-`, `.`, whitespace, or ASCII punctuation after stripping a trailing `.md`
3. drop empty tokens and English stopwords (the shipped stopword list contains unstemmed surface forms, so stopword filtering happens before stemming, matching the analyzer order accepted in [ADR 0003](../decisions/0003-use-stemming-for-match-tokens.md))
4. apply Snowball English (Porter2) stemming through the [`rust-stemmers`](https://docs.rs/rust-stemmers/latest/rust_stemmers/) crate, per the implementation choice in [ADR 0004](../decisions/0004-use-snowball-english-via-rust-stemmers.md)

`analyze_token` performs steps 1, 3, and 4 on a single token; `normalize_text` and `normalize_path` are splitter wrappers that apply step 2 and feed each chunk through `analyze_token` via `filter_map`.

This means:

- corpus statistics, BM25F scoring, explain-mode coverage, and matched-term highlighting all observe the same analyzed token stream
- index-time and query-time analysis use the same entry point, so a candidate field and a query produce the same token from the same surface form
- highlighting analyzes each displayed surface token and wraps it in the highlight escape when its stem matches an analyzed query term, so a plural surface form (`plans`) can highlight for a singular query (`plan`)
- stopword-only queries are rejected before scoring because every token analyzes to `None`

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
- **Internal apostrophe preservation with trailing possessive removal.** `O'Reilly` and `you're` should stay one token, while `Jim's` should emit `Jim` and `O'Reilly's` should emit `O'Reilly`. Plural possessives are stripped the same way, so `dogs'` should emit `dogs` (stemmed alongside the singular).
- **Existing ASCII punctuation splitting except apostrophes.** Keep current punctuation boundary behavior for forms such as `planner-execplan`, `repository-local`, `rust-stemmers`, and `docs/PLANS.md`; apostrophes are the only punctuation class called out for near-term special handling.
- **A single analyzer entry point used by text, paths, scoring, and highlighting.** Any new split rule must affect query analysis and candidate analysis identically.
- **A small explicit compound-word splitter dictionary** for repository compounds where both halves carry routing signal but the lowercase compound hides it. Splits replace the original token rather than coexisting with it (matching the "Avoid preserving originals" stance below), and the same dictionary is applied at query time so symmetry is preserved. Initial entries:
  - `execplan` → `exec`, `plan`

  New entries should be added only when both halves carry routing signal in this repo's vocabulary (e.g., not `frontmatter`, where `front` and `matter` are too generic to improve routing).

Consider later:

- **Unicode word-boundary splitting** if non-ASCII repository metadata becomes common enough that ASCII punctuation rules produce surprising output.
- **Conservative letter-number boundary splitting** if compact identifier patterns such as `ADR0004`, `RFC9110`, `PR42`, or `issue123` become common routing misses. Avoid splitting existing shapes such as `BM25F`, `v1`, `f32`, `u32`, and `R2D2` without a separate identifier policy.
- **A semantic alias layer** for role or task mappings beyond compound decomposition. This is a separate problem from the compound-word dictionary above and would need its own scoring design for multi-position or alternate tokens.

Avoid for now:

- **Using a full English dictionary for compound splitting.** Scanning a general English word list across all lowercase words creates noisy splits whenever a substring happens to be a real word. The explicit dictionary above is bounded and curated; do not extend it into broad lexical decomposition.
- **Preserving originals or catenating split parts by default.** Extra tokens inflate term frequency and field length in BM25F, so this should wait until there is a clear scoring design for multi-position or alternate tokens.
- **Using tokenizer rules to make `planner` emit `plan`.** That is a semantic alias problem, not a token-boundary problem. Broad derivational guessing would make unrelated words collide.

## Relevant Decisions

- [ADR 0005: Build our own match tokenizer](../decisions/0005-build-our-own-match-tokenizer.md)
- [ADR 0006: Build our own compound-word splitter dictionary](../decisions/0006-build-our-own-compound-word-splitter-dictionary.md)

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
