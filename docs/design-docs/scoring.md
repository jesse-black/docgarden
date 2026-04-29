---
description: "Working design draft for `docgarden match` scoring, including the shipped BM25F ranking model, field weighting, corpus statistics, and candidate future tuning directions."
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
- uses BM25F with `k1 = 1.2` and `b = 0.75`, matching Lucene `BM25Similarity` defaults
- applies field boosts of `name = 3.0`, `path_prefix = 1.0`, and `description = 1.0`
- receives analyzed tokens from the shared analyzer chain described in [`analyzer.md`](analyzer.md)
- computes IDF from document-level collection statistics: `df(term)` is the number of candidates where the term appears in any scoring field, and `N` is the total candidate count
- sorts by raw score, then matched query-term count, then best matched field, then path
- limits default `docgarden match` output to the top 5 ranked results unless `--limit` / `-n` is supplied

This intentionally follows BM25F document-level IDF semantics rather than Lucene's `CombinedFieldQuery` per-field-max approximation. Lucene is historical context and the source of the current BM25 defaults, not the scoring model.

### BM25F Field Model

The shipped BM25F field model uses three fields:

| Field | Source | Notes |
|---|---|---|
| `name` | frontmatter `name` if present, else filename stem | primary identity signal |
| `path_prefix` | directory portion of path, excluding filename | contextual/location signal |
| `description` | frontmatter `description` | secondary signal |

`name` carries the highest boost. `path_prefix` is intentionally weaker than `name` because directory segments are usually context rather than identity.

The reason `name` falls back to the filename stem rather than treating them as separate fields is that many documents do not have a frontmatter `name`, so the filename is the next-best identity source. For skills the inverse is true: every skill file is named `SKILL.md`, so the filename carries no useful signal and the frontmatter `name` is the actual identifier. Merging them into one field with frontmatter taking priority handles both cases cleanly.

### Current Scoring Formula

The current implementation combines weighted field term frequency and weighted field length before applying one BM25-style score contribution per query term:

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

[ADR 0002](../decisions/0002-use-bm25f-as-the-scoring-model.md) is the source of truth for the model choice: BM25F owns field weighting, term-frequency saturation, and document-level IDF semantics. The model is not equivalent to running BM25 independently per field and summing the results.

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

## Related Work

### BM25F

The recommended scoring model. BM25F extends BM25 with per-field weighting and is the standard for multi-field lexical ranking. Coverage, rare-term emphasis, and field preference are all encoded in the formula rather than approximated with ad-hoc constants.

References:

- Robertson, Zaragoza, Taylor, "Simple BM25 Extension to Multiple Weighted Fields" (CIKM 2004): https://dl.acm.org/doi/10.1145/1031171.1031181
- Robertson, Zaragoza, "The Probabilistic Relevance Framework: BM25 and Beyond" (2009): https://www.nowpublishers.com/article/Details/INR-019
- BM25F in Lucene discussion: https://opensourceconnections.com/blog/2016/10/19/bm25f-in-lucene/
- Lucene `BM25Similarity` docs: https://lucene.apache.org/core/9_9_1/core/org/apache/lucene/search/similarities/BM25Similarity.html
- Lucene `BM25Similarity` source: https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/core/src/java/org/apache/lucene/search/similarities/BM25Similarity.java

### Phrase And Proximity Evidence

IR literature has long explored adding phrase or proximity evidence on top of unigram lexical ranking. Relevant if dogfooding after BM25F shows phrase-aware bonuses are still needed.

Reference:

- Term proximity for BM25-style retrieval: https://www.sciencedirect.com/science/article/abs/pii/S0020025511001356

## Open Questions

- What are the right BM25F `k1` and `b` values for this corpus? Lucene's `BM25Similarity` defaults are `k1 = 1.2`, `b = 0.75` and are the right starting point. The combined-field length is dominated by short `name` and `path_prefix` content, so a lower `b` may reduce length penalty for documents with longer descriptions; more dogfooding will confirm whether the defaults hold.
- Should explain-mode color bands remain the current relative-plus-coverage rule, or become more dynamic after more real-world query sets are sampled?

## Suggested Evaluation

Any tuning pass should re-check at least these queries:

- `cargo run -- match review`
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`

The bar is not only “top result is correct.” The bar is also stronger separation between the intended top result and the nearby false positives.
