---
description: "Working design draft for `docgarden match` scoring, including the shipped v1 model, known ranking gaps, and candidate tuning directions for better routing separation."
---

# Match Scoring

## Purpose

This document is a working design draft for the `docgarden match` scorer.

It has three jobs:

- record the scoring model that is currently shipped
- explain the ranking gaps observed during dogfooding
- outline plausible implementation directions before the scorer changes again

The intent is routing quality, not search-theory completeness.

## Current State

The shipped scorer lives in `src/score.rs` and `src/matching.rs`.

Today it:

- matches only `name`, `description`, and repository-relative `path`
- lowercases and tokenizes query and candidate fields
- computes corpus-local IDF with clamping
- scores each query term independently across `name`, `path`, and `description`
- adds a phrase bonus only for multi-term contiguous matches
- sorts by score, then matched query-term count, then best matched field, then path

### Current Weighting

- field weights:
  - `name = 3`
  - `path = 2`
  - `description = 1`
- match tiers:
  - exact token = `10`
  - prefix = `4`
  - substring = `1`
- IDF:
  - `raw_idf = ln((N + 1) / (df + 1)) + 1`
  - clamped to `[0.5, 1.8]`
- phrase bonus:
  - `name` contiguous phrase = `+25`
  - path basename contiguous phrase = `+25`
  - `description` contiguous phrase = `+10`

### Consequences Of The Current Model

- A one-word query that hits only `description` tops out at `18` because `10 * 1 * 1.8 = 18`.
- Queries are additive: a document can rank highly by matching many secondary terms even if it misses the most diagnostic term.
- Common words such as `the` or `active` still contribute unless corpus-local IDF suppresses them enough.
- Substring credit on `name` and `path` lets terms such as `plan` pick up score from `planner` and `execplan`.

## Current Pain Points

The main observed issue is weak separation for routing-style queries.

Examples from this repo:

- `cargo run -- match review`
  - `evaluator-execplan` ties at `18` with other documents that also mention `review` in `description`
  - this is mathematically expected under the current `description`-only ceiling
- `cargo run -- match review against the active plan`
  - `evaluator-execplan` ranks first, which is good
  - `planner-execplan` remains too close because it matches `active`, `plan`, and related substrings even though it does not contain `review`

This means the current scorer is useful, but not yet opinionated enough for high-confidence routing.

## Design Goals For Tuning

Any scoring revision should keep these properties:

- deterministic results for a fixed repo state
- mechanical, explainable ranking
- no external services or heavy dependencies
- low implementation complexity relative to the current code
- stable enough behavior that help text and tests can describe it

The next scorer should improve separation without turning `docgarden match` into fuzzy full-text search.

## Proposed Directions

The current scorer is a hand-rolled approximation of BM25F with ad-hoc constants that have no principled grounding. The recommended approach is to replace the scoring model with BM25F together with analyzer-level stopword filtering, shipped as a single change — the two are coupled through the corpus statistics (`pseudo_df`, `avgdl`, `combined_length`), which must be built over the same token stream the scorer sees. The existing full-query phrase bonus is dropped in this change; bigram/phrase bonuses can be reconsidered later if dogfooding shows a remaining gap.

Lucene's `CombinedFieldQuery` and `BM25Similarity` are the source of truth for the algorithm. Where a choice is under-specified by IR literature, match Lucene's behavior rather than inventing a variant.

### 1. Replace The Scoring Model With BM25F

BM25F is the standard field-weighted lexical ranking formula. It directly encodes coverage, rare-term emphasis, and field weighting — the three problems the current ad-hoc scorer patches around separately.

#### Field Model

The BM25F field model uses three fields:

| Field | Source | Notes |
|---|---|---|
| `name` | frontmatter `name` if present, else filename stem | primary identity signal |
| `path_prefix` | directory portion of path, excluding filename | contextual/location signal |
| `description` | frontmatter `description` | secondary signal |

`name` carries the highest boost. `path_prefix` carries a lower boost than the current unified `path` weight because directory segments are weaker identity signals than the document name. `description` is unchanged.

The reason `name` falls back to the filename stem rather than treating them as separate fields is that many documents do not have a frontmatter `name`, so the filename is the next-best identity source. For skills the inverse is true: every skill file is named `SKILL.md`, so the filename carries no useful signal and the frontmatter `name` is the actual identifier. Merging them into one field with frontmatter taking priority handles both cases cleanly.

Under default `docgarden lint` configuration, skills are required to have a frontmatter `name` (per the Agent Skills spec) and non-skill documents are required to have a frontmatter `description`. This means the filename-stem fallback branch is primarily meaningful for non-skill documents that intentionally omit a frontmatter `name`. When frontmatter `name` is present, the filename stem is dropped from scoring entirely — this is an accepted trade-off because in the skills case the filename stem is the uninformative `SKILL.md`, and in the non-skill case the frontmatter author has deliberately chosen a distinct identity over the filename.

The current unified `path` field is split because filename stem and directory prefix are semantically different signals. A query term matching the filename stem is strong evidence of identity; matching a directory segment is weaker context. Giving them the same weight obscures this difference.

#### Lucene-Derived Scoring Shape

The intended implementation should follow Lucene's [`CombinedFieldQuery`](https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/sandbox/src/java/org/apache/lucene/search/CombinedFieldQuery.java), [`MultiNormsLeafSimScorer`](https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/sandbox/src/java/org/apache/lucene/search/MultiNormsLeafSimScorer.java), and [`BM25Similarity`](https://github.com/apache/lucene-solr/blob/branch_8_11/lucene/core/src/java/org/apache/lucene/search/similarities/BM25Similarity.java) shape:

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

How BM25F eliminates the current pain points:

- **Coverage** — documents matching more query terms receive the sum of more IDF contributions; no explicit bonus needed
- **Rare-term emphasis** — the IDF component already weights rare terms higher than the current clamped approximation
- **Substring/prefix tiers** — BM25F operates on exact tokens; the current `10 / 4 / 1` tier constants and their interactions go away

Implementation changes in `src/score.rs` and `src/matching.rs`:

- extract `name_field` from each candidate: frontmatter `name` if present, else filename stem
- extract `path_prefix_field` as the directory portion of the repo-relative path
- replace the per-term tier calculation with the Lucene-style combined-field BM25F scorer
- replace the current `raw_idf` with the standard BM25 IDF formula
- replace the hard-coded field weights with named `boost_f` constants; starting values are `name = 3.0`, `path_prefix = 1.0`, `description = 1.0` — these mirror the relative shape of the current 3/2/1 scheme while acknowledging `path_prefix` is a weaker identity signal than the current unified `path`
- use Lucene's `BM25Similarity` defaults as starting points: `k1 = 1.2`, `b = 0.75`
- remove the match-tier constants (`exact = 10`, `prefix = 4`, `substring = 1`)
- remove the existing full-query phrase bonus entirely; BM25F alone is the baseline, and phrase/bigram evidence is deferred as described below
- change `ScoredHit.score` from `i32` to `f32`; BM25F contributions are floating-point and rounding to integers collapses small-but-meaningful separation (e.g. `3.12` vs `3.48`) — the sort comparator should compare `f32` with a total-ordering wrapper or stable tiebreakers

The sort order does not need to change structurally. Score magnitudes will change so the color band thresholds in `src/cli.rs` will need recalibration after dogfooding.

### 2. Stopword Filtering

BM25F's IDF suppresses common terms, but in a small corpus IDF cannot fully separate universal function words from moderately common content words. `docgarden` should use analyzer-style stopword filtering:

- apply stopword filtering inside the tokenizer itself, not as a post-processing pass
- apply it symmetrically at index time and query time for normal bag-of-words retrieval
- use Lucene's `EnglishAnalyzer` stopword list as a starting point, then customize conservatively

Concrete design:

- filter stopwords inside `normalize_text` and `normalize_path` so every caller (candidate indexing, query parsing, phrase checks, tests) receives already-filtered tokens by construction — this guarantees the index-time / query-time symmetry without additional plumbing
- compute BM25F term statistics and field lengths from the filtered token streams so scoring and normalization stay aligned
- use a Lucene-derived English stopword list
- keep the stopwords in `src/data/stopwords_en.txt`, one term per line
- load the file at compile time with `include_str!`
- if every query term is filtered out, treat the query as invalid and return a user-facing error for a stopword-only query; no fallback to unfiltered matching is needed

Implementation:

- add `src/data/stopwords_en.txt`
- parse `include_str!("data/stopwords_en.txt")` into a static stopword set (e.g. `OnceLock<HashSet<&'static str>>`) in `src/score.rs`
- add `fn is_stopword(term: &str) -> bool`
- apply the filter inside `normalize_text` and `normalize_path` after lowercasing and token splitting

### 3. Partial Phrase And Proximity Bonuses (Deferred)

BM25F is a bag-of-words model and does not capture phrase evidence. The current full-query phrase bonus is too strict for routing queries like `review against the active plan`, and bigram bonuses could add signal on top of BM25F.

This is deferred because BM25F's coverage and rare-term handling are likely to resolve the motivating examples without it. For the `review against the active plan` case, `evaluator-execplan` matches `review` (high IDF) plus `active` and `plan`, while `planner-execplan` misses `review` entirely — BM25F's IDF sum already separates them. Bigram bonuses should be added only if dogfooding after BM25F shows a remaining gap.

If implemented:

- build query bigrams from filtered informative terms (requires stopword filtering from direction 2)
- for each field, add a smaller bonus than the existing full-query phrase bonus:
  - `name`/basename bigram: `+8`
  - `description` bigram: `+4`

## Recommended Order

1. Replace the scoring model with BM25F *and* add analyzer-level stopword filtering as a single change — they share corpus statistics and splitting them doubles the calibration work
2. Recalibrate score-color band thresholds
3. Optionally add bigram bonuses if dogfooding shows a remaining gap

## What Implementation Would Look Like

- `src/score.rs`
  - replace the per-term tier and IDF calculation with the Lucene-style combined-field BM25F scorer
  - change `ScoredHit.score` from `i32` to `f32`
  - remove match-tier constants and the full-query phrase bonus
  - add `is_stopword` and a static stopword set sourced from `src/data/stopwords_en.txt`
  - filter stopwords inside `normalize_text` and `normalize_path` so every caller sees filtered tokens
- `src/data/stopwords_en.txt`
  - new file, Lucene-derived English stopwords, one term per line
- `src/matching.rs`
  - no major structural changes expected; any `i32` score consumers must accept `f32`
- `src/cli.rs`
  - recalibrate the score-color band thresholds after dogfooding
- `tests/cli.rs`
  - add routing-separation tests for evaluator vs planner vs generator queries
  - add tests that stopwords do not dominate rankings
  - add a test documenting that a stopword-only query (`the`) returns a user-facing error
- `docs/design-docs/frontmatter-driven-discovery-commands.md`
  - update if the shipped v1 scoring description changes materially
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

IR literature has long explored adding phrase or proximity evidence on top of unigram lexical ranking. Relevant if dogfooding after BM25F shows bigram bonuses are still needed.

Reference:

- Term proximity for BM25-style retrieval: https://www.sciencedirect.com/science/article/abs/pii/S0020025511001356

### Stopword Handling

Search systems routinely treat stopwords specially because low-signal function words can distort lexical ranking in small corpora where IDF alone cannot suppress them.

Reference:

- Lucene `EnglishAnalyzer` docs: https://lucene.apache.org/core/10_2_1/analysis/common/org/apache/lucene/analysis/en/EnglishAnalyzer.html

## Open Questions

- What are the right BM25F `k1` and `b` values for this corpus? Lucene's `BM25Similarity` defaults are `k1 = 1.2`, `b = 0.75` and are the right starting point. The combined-field length is dominated by short `name` and `path_prefix` content, so a lower `b` may reduce length penalty for documents with longer descriptions; dogfooding will confirm whether the defaults hold.
- Should score-color band thresholds be recalibrated after BM25F changes the score distribution, or derived dynamically from the result set?

## Suggested Evaluation

Any tuning pass should re-check at least these queries:

- `cargo run -- match review`
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`

The bar is not only “top result is correct.” The bar is also stronger separation between the intended top result and the nearby false positives.
