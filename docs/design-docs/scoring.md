---
description: "Working design draft for `docgarden match` scoring, including the shipped BM25F model, analyzer pipeline, stopword handling, stemming, and candidate future tuning directions."
---

# Match Scoring

## Purpose

This document is a working design draft for the `docgarden match` scorer.

It has three jobs:

- record the scoring model that is currently shipped
- capture the remaining tuning gaps observed during dogfooding
- outline plausible future directions before the scorer changes again

The intent is routing quality, not search-theory completeness.

## Current State

The shipped scorer lives in `src/score.rs` and `src/matching.rs`.

Today it:

- scores over `name`, `path_prefix`, and `description`
- uses BM25F with Lucene-derived combined-field term-frequency and length accounting, `k1 = 1.2`, and `b = 0.75`
- applies field boosts of `name = 3.0`, `path_prefix = 1.0`, and `description = 1.0`
- lowercases, tokenizes, filters English stopwords, and stems query and candidate fields symmetrically
- computes IDF from document-level collection statistics: `df(term)` is the number of candidates where the term appears in any scoring field, and `N` is the total candidate count
- sorts by raw score, then matched query-term count, then best matched field, then path
- limits default `docgarden match` output to the top 5 ranked results unless `--limit` / `-n` is supplied

This intentionally follows BM25F document-level IDF semantics rather than Lucene's `CombinedFieldQuery` per-field-max approximation. Lucene remains the implementation model for combining weighted field term frequencies and lengths into one synthetic field before applying the BM25 scorer.

### BM25F Field Model

The shipped BM25F field model uses three fields:

| Field | Source | Notes |
|---|---|---|
| `name` | frontmatter `name` if present, else filename stem | primary identity signal |
| `path_prefix` | directory portion of path, excluding filename | contextual/location signal |
| `description` | frontmatter `description` | secondary signal |

`name` carries the highest boost. `path_prefix` is intentionally weaker than `name` because directory segments are usually context rather than identity.

The reason `name` falls back to the filename stem rather than treating them as separate fields is that many documents do not have a frontmatter `name`, so the filename is the next-best identity source. For skills the inverse is true: every skill file is named `SKILL.md`, so the filename carries no useful signal and the frontmatter `name` is the actual identifier. Merging them into one field with frontmatter taking priority handles both cases cleanly.

### Current Lucene-Derived Scoring Shape

The current implementation follows Lucene's [`CombinedFieldQuery`](https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/sandbox/src/java/org/apache/lucene/search/CombinedFieldQuery.java), [`MultiNormsLeafSimScorer`](https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/sandbox/src/java/org/apache/lucene/search/MultiNormsLeafSimScorer.java), and [`BM25Similarity`](https://lucene.apache.org/core/9_9_1/core/org/apache/lucene/search/similarities/BM25Similarity.html) for weighted field-frequency and length normalization, but uses BM25F document-level collection statistics for IDF:

    N = number of candidate documents

    for each document d:
        document_terms(d) = set(tokens_f(d) for each scoring field f)
        for each term t in document_terms(d):
            df(t) += 1

    combined_sum_total_term_freq = sum(boost_f * sum_total_term_freq_f for each field f)
    avgdl = combined_sum_total_term_freq / N

    for each document d:
        combined_freq(t, d) = sum(boost_f * tf_f(t, d) for each field f)
        combined_length(d) = sum(boost_f * len_f(d) for each field f)

        idf(t) = ln(1 + (N - df(t) + 0.5) / (df(t) + 0.5))
        score contribution for t =
            idf(t) * ((k1 + 1) * combined_freq(t, d))
                     / (combined_freq(t, d) + k1 * (1 - b + b * combined_length(d) / avgdl))

Candidates with no tokens in any scoring field still count toward `N`. Functionally empty routed documents are an authoring and lint concern, not an IDF collection-size exclusion.

This describes the shipped scorer shape, not the durable scoring-model source of truth. [ADR 0002](../decisions/0002-use-bm25f-as-the-scoring-model.md) records BM25F as the model that owns field weighting, term-frequency saturation, and document-level IDF semantics. Lucene remains useful implementation history for combining weighted field statistics, term frequency, and field length before applying one BM25 scorer over the synthetic field, but `docgarden` no longer follows Lucene's max-based pseudo-statistics for IDF. The model is not equivalent to running BM25 independently per field and summing the results.

### Analyzer Pipeline

`docgarden` uses one shared analyzer pipeline for query terms, candidate fields, corpus statistics, scoring, explain-mode coverage, and matched-term highlighting.

The analyzer order is:

1. lowercase
2. split on text or path separators
3. remove English stopwords
4. apply Snowball English (Porter2) stemming through `rust-stemmers`

`src/score.rs` keeps the per-token contract in `analyze_token`, with `normalize_text` and `normalize_path` acting as splitter wrappers around that shared function. The shipped stopword list contains unstemmed surface forms, so stopword filtering intentionally happens before stemming, matching the analyzer order accepted in [ADR 0003](../decisions/0003-use-stemming-for-match-tokens.md) and the Snowball implementation choice in [ADR 0004](../decisions/0004-use-snowball-english-via-rust-stemmers.md).

This means:

- every scoring caller receives already-filtered and already-stemmed tokens
- index-time and query-time analysis use the same token stream
- BM25F term statistics and field lengths are computed from analyzed tokens
- reject stopword-only queries as invalid
- matched-term highlighting analyzes each displayed surface token and highlights it when its stem matches an analyzed query term

This keeps corpus statistics, query parsing, and displayed explain metrics aligned around one shared token stream.

## Current Gaps

The shipped scorer is much closer to the intended routing behavior, but a few limitations remain:

- no phrase or proximity evidence beyond unigram coverage
- explain-mode colors still need calibration over more real-world query sets

## Design Goals For Future Tuning

Any scoring revision should keep these properties:

- deterministic results for a fixed repo state
- mechanical, explainable ranking
- no external services or heavy dependencies
- low implementation complexity relative to the current code
- stable enough behavior that help text and tests can describe it

The scorer should improve separation and recall without turning `docgarden match` into fuzzy full-text search.

## Proposed Future Directions

ADR 0002 records BM25F as the source of truth for the lexical ranking model. Future work should close remaining gaps in normalization, cutoff behavior, and phrase evidence rather than reopening the pre-BM25F tiered scorer.

### 1. Relevancy Cutoff After Ranking Tuning

The next result-shaping step should be a relevancy cutoff, so `docgarden match` can hide weak tail results even when they are inside the default top 5.

Now that stemming has shipped, cutoff tuning should use the post-stemming score distribution, term coverage, and apparent quality gap between useful and weak matches.

Potential cutoff calculations to evaluate:

- relative score floor, such as keeping results above `top_score * 0.20` or `top_score * 0.30`
- query coverage floor, such as requiring at least one informative matched term and requiring a higher fraction for multi-term queries
- hybrid floor, such as keeping results that satisfy either a strong relative score or strong matched-term coverage
- rank-gap cutoff, such as stopping when the score drop from one result to the next crosses a calibrated ratio
- normalized confidence band, using explain-mode's `relative` percentage as the user-facing version of the internal cutoff

The cutoff should remain deterministic and explainable. If a result is hidden by relevance rather than by `--limit`, explain mode should make that behavior inspectable, either by exposing the cutoff value or by offering a later flag that shows filtered tail results for debugging.

### 2. Partial Phrase And Proximity Bonuses (Deferred)

BM25F is a bag-of-words model and does not capture phrase evidence. Bigram bonuses or limited proximity evidence could still add signal on top of BM25F for routing queries where term co-occurrence matters.

This remains lower priority because BM25F plus stemming resolves many of the motivating routing queries. Phrase-aware bonuses should be added only if dogfooding after stemming or other normalization improvements still shows a remaining gap.

If implemented:

- build query bigrams from filtered informative terms
- keep bonuses small relative to the BM25F baseline
- validate against routing queries such as `review against the active plan`

## What Implementation Would Look Like

- `src/score.rs`
  - keep BM25F, stopword filtering, and stemming in the shared analyzer baseline
- `src/matching.rs`
  - keep explain-mode and highlighting aligned with the same normalized-token pipeline the scorer uses
- `src/cli.rs`
  - keep help text aligned with any future scoring or explain-mode contract changes
- `tests/cli.rs`
  - keep routing-separation tests for evaluator vs planner vs generator queries
  - keep tests that stopwords do not dominate rankings
  - keep tests showing highlighting and scoring agree on stemmed query/document forms
- `docs/design-docs/match-and-list.md`
  - update if future scoring or highlighting semantics change materially
- `docs/exec-plans/active/*.md`
  - if this becomes active work, capture the accepted tuning scope in an ExecPlan rather than only in this design draft

## Related Work

### BM25F

The recommended scoring model. BM25F extends BM25 with per-field weighting and is the standard for multi-field lexical ranking. Coverage, rare-term emphasis, and field preference are all encoded in the formula rather than approximated with ad-hoc constants.

References:

- Robertson, Zaragoza, Taylor, "Simple BM25 Extension to Multiple Weighted Fields" (CIKM 2004): https://dl.acm.org/doi/10.1145/1031171.1031181
- Robertson, Zaragoza, "The Probabilistic Relevance Framework: BM25 and Beyond" (2009): https://www.nowpublishers.com/article/Details/INR-019
- BM25F in Lucene discussion: https://opensourceconnections.com/blog/2016/10/19/bm25f-in-lucene/
- Lucene `BM25Similarity` docs: https://lucene.apache.org/core/9_9_1/core/org/apache/lucene/search/similarities/BM25Similarity.html
- Lucene `CombinedFieldQuery` source: https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/sandbox/src/java/org/apache/lucene/search/CombinedFieldQuery.java
- Lucene `MultiNormsLeafSimScorer` source: https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/sandbox/src/java/org/apache/lucene/search/MultiNormsLeafSimScorer.java
- Lucene `BM25Similarity` source: https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/core/src/java/org/apache/lucene/search/similarities/BM25Similarity.java

### Phrase And Proximity Evidence

IR literature has long explored adding phrase or proximity evidence on top of unigram lexical ranking. Relevant if dogfooding after BM25F shows phrase-aware bonuses are still needed.

Reference:

- Term proximity for BM25-style retrieval: https://www.sciencedirect.com/science/article/abs/pii/S0020025511001356

### Stopword Handling

Search systems routinely treat stopwords specially because low-signal function words can distort lexical ranking in small corpora where IDF alone cannot suppress them.

Reference:

- Lucene `EnglishAnalyzer` docs: https://lucene.apache.org/core/10_2_1/analysis/common/org/apache/lucene/analysis/en/EnglishAnalyzer.html

### Stemming

Stemming improves recall across singular/plural and other closely related word forms without adopting fuzzier matching.

Chosen implementation:

- `rust-stemmers` with `Algorithm::English`, which implements Snowball English / Porter2

## Open Questions

- What are the right BM25F `k1` and `b` values for this corpus? Lucene's `BM25Similarity` defaults are `k1 = 1.2`, `b = 0.75` and are the right starting point. The combined-field length is dominated by short `name` and `path_prefix` content, so a lower `b` may reduce length penalty for documents with longer descriptions; more dogfooding will confirm whether the defaults hold.
- Should explain-mode color bands remain the current relative-plus-coverage rule, or become more dynamic after more real-world query sets are sampled?

## Explain And Display

The default `docgarden match` output should prioritize routing clarity over score inspection:

- default output shows `path | name | description`
- default output is capped at the top 5 ranked results unless `--limit` / `-n` is supplied
- matched informative query terms may be highlighted when styled output is enabled
- raw BM25F score should be hidden from the default view

`docgarden match --explain` is the place to surface score diagnostics:

- print a header row followed by `score | relative | coverage | path | name | description`
- `score` is the raw BM25F score rendered with two decimal places
- `relative` is the result's percentage of the top score in that result set
- `coverage` is `matched_terms/query_terms` after stopword filtering and stemming
- any color bands should apply only in explain mode and should use a hybrid relative-plus-coverage rule rather than fixed absolute raw-score thresholds

## Suggested Evaluation

Any tuning pass should re-check at least these queries:

- `cargo run -- match review`
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`

The bar is not only “top result is correct.” The bar is also stronger separation between the intended top result and the nearby false positives.
