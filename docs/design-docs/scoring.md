---
description: "Working design draft for `docgarden match` scoring, including the shipped BM25F model, stopword handling, and candidate future tuning directions such as stemming."
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
- uses Lucene-shaped combined-field BM25F with `k1 = 1.2` and `b = 0.75`
- applies field boosts of `name = 3.0`, `path_prefix = 1.0`, and `description = 1.0`
- lowercases and tokenizes query and candidate fields symmetrically
- filters English stopwords inside the shared normalization path used for both corpus statistics and query parsing
- sorts by raw score, then matched query-term count, then best matched field, then path

### BM25F Field Model

The shipped BM25F field model uses three fields:

| Field | Source | Notes |
|---|---|---|
| `name` | frontmatter `name` if present, else filename stem | primary identity signal |
| `path_prefix` | directory portion of path, excluding filename | contextual/location signal |
| `description` | frontmatter `description` | secondary signal |

`name` carries the highest boost. `path_prefix` is intentionally weaker than `name` because directory segments are usually context rather than identity.

The reason `name` falls back to the filename stem rather than treating them as separate fields is that many documents do not have a frontmatter `name`, so the filename is the next-best identity source. For skills the inverse is true: every skill file is named `SKILL.md`, so the filename carries no useful signal and the frontmatter `name` is the actual identifier. Merging them into one field with frontmatter taking priority handles both cases cleanly.

### Lucene-Derived Scoring Shape

The intended implementation follows Lucene's [`CombinedFieldQuery`](https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/sandbox/src/java/org/apache/lucene/search/CombinedFieldQuery.java), [`MultiNormsLeafSimScorer`](https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/sandbox/src/java/org/apache/lucene/search/MultiNormsLeafSimScorer.java), and [`BM25Similarity`](https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/core/src/java/org/apache/lucene/search/similarities/BM25Similarity.java) shape:

    // Treat multiple fields as one synthetic weighted field.
    for each query term t:
        pseudo_df(t) = max(df_f(t) for each field f)

    pseudo_doc_count = max(doc_count_f for each field f)
    pseudo_sum_total_term_freq = sum(boost_f * sum_total_term_freq_f for each field f)

    for each document d:
        combined_freq(t, d) = sum(boost_f * tf_f(t, d) for each field f)
        combined_length(d) = sum(boost_f * len_f(d) for each field f)

        idf(t) = ln(1 + (pseudo_doc_count - pseudo_df(t) + 0.5) / (pseudo_df(t) + 0.5))
        avgdl = pseudo_sum_total_term_freq / pseudo_doc_count
        score contribution for t =
            idf(t) * ((k1 + 1) * combined_freq(t, d))
                     / (combined_freq(t, d) + k1 * (1 - b + b * combined_length(d) / avgdl))

This is the validation target to mirror: combine weighted field statistics, term frequency, and field length first, then apply one BM25 scorer over the synthetic field. It is not equivalent to running BM25 independently per field and summing the results.

### Stopword Filtering

`docgarden` now uses analyzer-style stopword filtering:

- filter stopwords inside `normalize_text` and `normalize_path` so every caller receives already-filtered tokens
- apply the same filtering at index time and query time
- compute BM25F term statistics and field lengths from the filtered token streams
- reject stopword-only queries as invalid

This keeps corpus statistics, query parsing, and displayed explain metrics aligned around one shared token stream.

## Current Gaps

The shipped scorer is much closer to the intended routing behavior, but a few limitations remain:

- no stemming, so `plan` does not match `plans` and `review` does not match `reviews`
- no phrase or proximity evidence beyond unigram coverage
- explain-mode colors still need calibration over more real-world query sets
- default highlighting is still exact-token based, so pluralization and related morphology can look inconsistent to a human even when the result ordering is reasonable

## Design Goals For Future Tuning

Any scoring revision should keep these properties:

- deterministic results for a fixed repo state
- mechanical, explainable ranking
- no external services or heavy dependencies
- low implementation complexity relative to the current code
- stable enough behavior that help text and tests can describe it

The scorer should improve separation and recall without turning `docgarden match` into fuzzy full-text search.

## Proposed Future Directions

Lucene's `CombinedFieldQuery` and `BM25Similarity` remain the source of truth for the shipped lexical ranking model. Future work should build on that base rather than reopening the pre-BM25F tiered scorer.

### 1. Evaluate Document-Union Pseudo Statistics

The shipped scorer intentionally follows Lucene's combined-field shape, including these corpus-level approximations:

- `pseudo_df(term)` uses the maximum per-field document frequency
- `pseudo_doc_count` uses the maximum per-field document count

That behavior is consistent with the linked Lucene implementation and should not be treated as a correctness bug in the current scorer.

A plausible future experiment is to replace those max-based pseudo statistics with document-union statistics that treat a term as present when it appears in any scoring field of a document:

- `pseudo_df(term) = number of documents where the term appears in any scoring field`
- `pseudo_doc_count = total candidate documents`

The motivation would be repository-routing quality, not fidelity to Lucene. In small curated corpora, union-style statistics may reduce cases where a term that appears in different fields across different documents is treated as rarer than it feels to a human reader.

If this is explored, it should be treated as a deliberate scoring-model change:

- compare ranking quality against the current Lucene-shaped baseline on real repo queries
- update `docs/design-docs/frontmatter-driven-discovery-commands.md` if the ranking contract changes materially
- document clearly that `docgarden` is no longer mirroring Lucene's pseudo-statistics approximation exactly

This is worth evaluating before more speculative scoring additions if dogfooding shows corpus-statistics shape matters more than token normalization.

### 2. Stemming With A Shared Normalization Path

The next plausible routing improvement is stemming, and current repo dogfooding suggests it has more upside than phrase/proximity bonuses.

Motivating examples:

- `plan` should likely match `plans`
- `review` should likely match `reviews`
- `exec` should likely match `execution`
- highlighting should not suggest a weaker match story than the scorer actually used

Current evidence from repo queries:

- the canonical routing queries such as `review against the active plan` and `implement from the active plan` already have strong BM25F separation
- the more obvious remaining mismatch is morphological, such as `exec plan` ranking `ExecPlan` documents very close to or above documents that contain `Execution plan`
- that pattern suggests normalization is a better next lever than phrase-aware bonuses

If stemming is added, it should use one shared code path as the source of truth for:

- corpus statistics
- query parsing
- scoring
- matched-term highlighting
- explain-mode coverage metrics

The repository should avoid a split where scoring uses stemming but display highlighting still uses only exact token matches, or vice versa.

Candidate evaluation direction:

- evaluate the `rust-stemmers` crate as the likely lightweight implementation option
- verify whether its stemming behavior is conservative enough for repository-routing queries
- keep stopword filtering and stemming in the same normalization pipeline so all consumers observe the same post-analysis tokens
- dogfood carefully against this repo before treating stemming as the new default, because overly aggressive stemming could reduce routing precision in a small curated corpus

Implementation shape if accepted:

- keep one shared normalization pipeline in `src/score.rs`
- expose enough normalized-token information for `src/matching.rs` highlighting to follow the same analyzed terms
- add tests showing that ranking and highlighting stay aligned for stemmed forms such as `plan` / `plans`

### 3. Partial Phrase And Proximity Bonuses (Deferred)

BM25F is a bag-of-words model and does not capture phrase evidence. Bigram bonuses or limited proximity evidence could still add signal on top of BM25F for routing queries where term co-occurrence matters.

This is now a lower-priority direction than stemming because BM25F already resolves many of the motivating phrase-shaped routing queries well. Phrase-aware bonuses should be added only if dogfooding after stemming or other normalization improvements still shows a remaining gap.

If implemented:

- build query bigrams from filtered informative terms
- keep bonuses small relative to the BM25F baseline
- validate against routing queries such as `review against the active plan`

## Recommended Order

1. Recalibrate explain-mode color thresholds
2. Evaluate whether document-union pseudo statistics outperform the current Lucene-shaped max-based approximation on real routing queries
3. Evaluate stemming with a single shared normalization path if recall and highlighting consistency still need improvement
4. Optionally add bigram or proximity bonuses if dogfooding still shows a remaining gap after stemming

## What Implementation Would Look Like

- `src/score.rs`
  - keep BM25F and stopword filtering as the shared baseline
  - if stemming is added later, implement it in the shared normalization path rather than in a display-only helper
- `src/matching.rs`
  - keep explain-mode and highlighting aligned with the same normalized-token pipeline the scorer uses
- `src/cli.rs`
  - recalibrate explain-mode color thresholds after dogfooding
- `tests/cli.rs`
  - keep routing-separation tests for evaluator vs planner vs generator queries
  - keep tests that stopwords do not dominate rankings
  - if stemming lands, add tests showing highlighting and scoring agree on stemmed query/document forms
- `docs/design-docs/frontmatter-driven-discovery-commands.md`
  - update if future scoring or highlighting semantics change materially
- `docs/exec-plans/active/*.md`
  - if this becomes active work, capture the accepted tuning scope in an ExecPlan rather than only in this design draft

## Related Work

### BM25F

The recommended scoring model. BM25F extends BM25 with per-field weighting and is the standard for multi-field lexical ranking. Coverage, rare-term emphasis, and field preference are all encoded in the formula rather than approximated with ad-hoc constants.

References:

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

Stemming is a plausible next step if `docgarden` needs better recall across singular/plural and other closely related word forms without adopting fuzzier matching.

Candidate library to evaluate:

- `rust-stemmers`

## Open Questions

- What are the right BM25F `k1` and `b` values for this corpus? Lucene's `BM25Similarity` defaults are `k1 = 1.2`, `b = 0.75` and are the right starting point. The combined-field length is dominated by short `name` and `path_prefix` content, so a lower `b` may reduce length penalty for documents with longer descriptions; dogfooding will confirm whether the defaults hold.
- Should explain-mode color bands be recalibrated after BM25F changes the score distribution, or derived dynamically from the result set?
- Should stemming be adopted for both scoring and highlighting, or is exact-token behavior still the better fit for repository-routing precision?

## Explain And Display

The default `docgarden match` output should prioritize routing clarity over score inspection:

- default output shows `path | name | description`
- matched informative query terms may be highlighted when styled output is enabled
- raw BM25F score should be hidden from the default view

`docgarden match --explain` is the place to surface score diagnostics:

- print a header row followed by `score | relative | coverage | path | name | description`
- `score` is the raw BM25F score rendered with two decimal places
- `relative` is the result's percentage of the top score in that result set
- `coverage` is `matched_terms/query_terms` after stopword filtering
- any color bands should apply only in explain mode and should use a hybrid relative-plus-coverage rule rather than fixed absolute raw-score thresholds

## Suggested Evaluation

Any tuning pass should re-check at least these queries:

- `cargo run -- match review`
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`

The bar is not only “top result is correct.” The bar is also stronger separation between the intended top result and the nearby false positives.
