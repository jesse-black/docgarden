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

The current scorer is a hand-rolled approximation of BM25F with ad-hoc constants that have no principled grounding. The recommended approach is to replace the scoring model with BM25F, then add stopword filtering on top. Phrase bonuses remain optional and should be deferred until BM25F is in place and dogfooding shows a remaining gap.

### 1. Replace The Scoring Model With BM25F

BM25F is the standard field-weighted lexical ranking formula. It directly encodes coverage, rare-term emphasis, and field weighting — the three problems the current ad-hoc scorer patches around separately.

### Field Model

The BM25F field model uses three fields:

| Field | Source | Notes |
|---|---|---|
| `name` | frontmatter `name` if present, else filename stem | primary identity signal |
| `path_prefix` | directory portion of path, excluding filename | contextual/location signal |
| `description` | frontmatter `description` | secondary signal |

`name` carries the highest boost. `path_prefix` carries a lower boost than the current unified `path` weight because directory segments are weaker identity signals than the document name. `description` is unchanged.

The reason `name` falls back to the filename stem rather than treating them as separate fields is that many documents do not have a frontmatter `name`, so the filename is the next-best identity source. For skills the inverse is true: every skill file is named `SKILL.md`, so the filename carries no useful signal and the frontmatter `name` is the actual identifier. Merging them into one field with frontmatter taking priority handles both cases cleanly.

The current unified `path` field is split because filename stem and directory prefix are semantically different signals. A query term matching the filename stem is strong evidence of identity; matching a directory segment is weaker context. Giving them the same weight obscures this difference.

### Formula

```
score(d, q) = Σ_t  IDF(t) × TF_combined(t, d)

TF_combined(t, d) = Σ_f  boost_f × tf_f(t, d)
                         ────────────────────────────────────────────────
                         k1 × (1 − b_f + b_f × |d_f| / avgdl_f) + tf_f(t, d)

IDF(t) = ln((N − df(t) + 0.5) / (df(t) + 0.5) + 1)
```

Where:

- `t` ranges over query terms
- `f` ranges over fields (`name`, `path_prefix`, `description`)
- `boost_f` replaces the current field weights
- `k1` controls term frequency saturation (standard starting value: `1.2`)
- `b_f` controls length normalization per field (standard starting value: `0.75`; `b=0` may be appropriate for `name` since skill names are short and length normalization adds noise)
- `tf_f(t, d)` is term frequency of `t` in field `f` for document `d`

Since the current scorer already uses binary presence (0 or 1) per field rather than raw term frequency, `tf_f` simplifies to 0 or 1 and `TF_combined` becomes:

```
TF_combined(t, d) = Σ_f  boost_f / (k1 × (1 − b_f + b_f × |d_f| / avgdl_f) + 1)
                         (for fields where t is present)
```

How BM25F eliminates the current pain points:

- **Coverage** — documents matching more query terms receive the sum of more IDF contributions; no explicit bonus needed
- **Rare-term emphasis** — the IDF component already weights rare terms higher than the current clamped approximation
- **Substring/prefix tiers** — BM25F operates on exact tokens; the current `10 / 4 / 1` tier constants and their interactions go away

Implementation changes in `src/score.rs` and `src/matching.rs`:

- extract `name_field` from each candidate: frontmatter `name` if present, else filename stem
- extract `path_prefix_field` as the directory portion of the repo-relative path
- replace the per-term tier calculation with the BM25F `TF_combined` formula
- replace the current `raw_idf` with the standard BM25 IDF formula
- replace the hard-coded field weights with named `boost_f` constants
- remove the match-tier constants (`exact = 10`, `prefix = 4`, `substring = 1`)
- keep the existing phrase bonus for now; it sits on top of the base score and is independent

The `ScoredHit` struct and sort order do not need to change structurally. Score magnitudes will change so the color band thresholds in `src/cli.rs` will need recalibration after dogfooding.

### 2. Stopword Filtering

BM25F's IDF suppresses common terms, but in a small corpus IDF cannot fully separate universal function words from moderately common content words. `docgarden` should use analyzer-style stopword filtering:

- apply stopword filtering in the analyzer, not as an afterthought in scoring
- apply it symmetrically at index time and query time for normal bag-of-words retrieval
- use standard language stopword lists as a starting point, then customize conservatively

Concrete design:

- filter stopwords during candidate field normalization as well as query normalization
- compute BM25F term statistics and field lengths from the filtered token streams so scoring and normalization stay aligned
- use a Lucene-derived English stopword list
- keep the stopwords in `src/data/stopwords_en.txt`, one term per line
- load the file at compile time with `include_str!`
- if every query term is filtered out, fall back to the unfiltered query so single-word queries like `the` still behave mechanically instead of erroring

Implementation:

- add `src/data/stopwords_en.txt`
- parse `include_str!("data/stopwords_en.txt")` into a static stopword set in `src/score.rs`
- add `fn is_stopword(term: &str) -> bool`
- add `fn informative_query_terms(query_terms: &[String]) -> Vec<String>`
- score against the filtered query when it is non-empty

### 3. Partial Phrase And Proximity Bonuses (Deferred)

BM25F is a bag-of-words model and does not capture phrase evidence. The current full-query phrase bonus is too strict for routing queries like `review against the active plan`, and bigram bonuses could add signal on top of BM25F.

This is deferred because BM25F's coverage and rare-term handling are likely to resolve the motivating examples without it. For the `review against the active plan` case, `evaluator-execplan` matches `review` (high IDF) plus `active` and `plan`, while `planner-execplan` misses `review` entirely — BM25F's IDF sum already separates them. Bigram bonuses should be added only if dogfooding after BM25F shows a remaining gap.

If implemented:

- build query bigrams from filtered informative terms (requires stopword filtering from direction 2)
- for each field, add a smaller bonus than the existing full-query phrase bonus:
  - `name`/basename bigram: `+8`
  - `description` bigram: `+4`

## Recommended Order

1. Replace the scoring model with BM25F
2. Add stopword filtering
3. Recalibrate score-color band thresholds
4. Optionally add bigram bonuses if dogfooding shows a remaining gap

## What Implementation Would Look Like

- `src/score.rs`
  - replace the per-term tier and IDF calculation with BM25F
  - add stopword utilities and `informative_query_terms`
  - remove match-tier constants
- `src/matching.rs`
  - no major structural changes expected
- `src/cli.rs`
  - recalibrate the score-color band thresholds after dogfooding
- `tests/cli.rs`
  - add routing-separation tests for evaluator vs planner vs generator queries
  - add tests that stopwords do not dominate rankings
  - add a test for the stopword-only query fallback (`the` still returns results)
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

### Phrase And Proximity Evidence

IR literature has long explored adding phrase or proximity evidence on top of unigram lexical ranking. Relevant if dogfooding after BM25F shows bigram bonuses are still needed.

Reference:

- Term proximity for BM25-style retrieval: https://www.sciencedirect.com/science/article/abs/pii/S0020025511001356

### Stopword Handling

Search systems routinely treat stopwords specially because low-signal function words can distort lexical ranking in small corpora where IDF alone cannot suppress them.

Reference:

- Lucene `EnglishAnalyzer` docs: https://lucene.apache.org/core/10_2_1/analysis/common/org/apache/lucene/analysis/en/EnglishAnalyzer.html

## Open Questions

- What are the right BM25F `k1` and `b_f` values for this corpus? The `name` field likely wants `b=0` (no length normalization) since skill names are short and uniform. The `path_prefix` and `description` fields may benefit from `b=0.75` but need dogfooding to confirm.
- Should score-color band thresholds be recalibrated after BM25F changes the score distribution, or derived dynamically from the result set?

## Suggested Evaluation

Any tuning pass should re-check at least these queries:

- `cargo run -- match review`
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`

The bar is not only “top result is correct.” The bar is also stronger separation between the intended top result and the nearby false positives.
