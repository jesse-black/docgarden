---
description: "Move the `docgarden match` scorer from Lucene's per-field-max approximation to BM25F document-level collection statistics, dogfood the Suggested Evaluation queries, and update `docs/design-docs/scoring.md` to reflect the implemented state."
---

# Document-Level IDF Statistics for BM25F

Implements [ADR 0002 — Use BM25F as the scoring model](../../decisions/0002-use-bm25f-as-the-scoring-model.md).

## Goal

- `CombinedFieldStats` computes IDF using document-level statistics per Robertson, Zaragoza, and Taylor's BM25F definition: `df(term)` is the number of candidates that contain `term` in any scoring field, and `N` is the total number of candidate documents in the scoring collection.
- Routing-separation behavior on the Suggested Evaluation queries is preserved or improved; raw scores and color bands are recalibrated only if dogfooding requires it.
- `docs/design-docs/scoring.md` describes document-level IDF statistics as the shipped behavior, and direction #1 is removed from "Proposed Future Directions".

## Scope

- In:
  - Replace the per-field-max `pseudo_df` aggregation in `src/score.rs::CombinedFieldStats::build` with a per-candidate union pass that records each term once per document if it appears in any scoring field.
  - Replace per-field-max `pseudo_doc_count` with the total number of candidate documents in the scoring collection.
  - Count every candidate document toward `N`, including candidates with no tokens in any scoring field after normalization; functionally empty routed documents are an authoring/lint concern, not an IDF exclusion.
  - Rename implementation fields away from Lucene's `pseudo_*` vocabulary where the values now represent BM25F collection statistics.
  - Keep `pseudo_sum_total_term_freq` and `avgdl` boost-weighted as today; they describe combined-field length, not IDF.
  - Update `src/score.rs` unit tests to assert the document-level IDF semantics directly (a term that appears in different fields across different documents has a `df` equal to the number of documents containing it, not the per-field max).
  - Re-dogfood the existing Suggested Evaluation queries; record actual score distributions under `Discoveries`; add more dogfooding queries only if those runs reveal a coverage gap.
  - Recalibrate `score_band` thresholds and `match --help` text only if the new distribution makes the current relative-plus-coverage bands wrong.
  - Update `docs/design-docs/scoring.md`:
    - Move document-level IDF statistics into "Current State" and "Current Lucene-Derived Scoring Shape" (replace the max-based pseudocode), keeping the explicit note that `docgarden` now follows BM25F document-level IDF rather than Lucene's `CombinedFieldQuery` approximation.
    - Remove the now-shipped direction #1 from "Proposed Future Directions"; renumber the remaining directions.
    - Refresh "Open Questions" if any item is resolved or invalidated by the change.
  - Update `docs/design-docs/match-and-list.md` only if the ranking contract changes materially (e.g. score scale or color-band thresholds shift).
- Out:
  - Stemming, phrase/proximity bonuses, relevancy cutoff (directions #2-#4 in `scoring.md`).
  - Per-field `b_f`, tuning of `k1` / `b`.
  - Changes to field set (`name` / `path_prefix` / `description`), boosts, stopword list, or tokenization.
  - Index caching or persistence of corpus statistics.

## Relevant Areas

- `src/score.rs` — `CombinedFieldStats::build` (per-field-max aggregation lives at the per-field `FieldStats::record` and the `pseudo_df` / `pseudo_doc_count` reduction); `idf` consumes those collection statistics; unit tests under `#[cfg(test)] mod tests`.
- `src/matching.rs` — only relevant if the score scale shifts enough to require updating `score_band` thresholds or the zero-drop heuristic.
- `src/cli.rs` — `match --help` text references the BM25F shape and color bands; update only if calibration changes.
- `tests/cli.rs` — routing-separation tests for the Suggested Evaluation queries; check that new score values still satisfy the asserted ordering and separation floors.
- `docs/design-docs/scoring.md` — current-state + future-directions documentation; this plan is responsible for landing the implemented-state edits.
- `docs/design-docs/match-and-list.md` — command-level ranking contract; touch only if materially affected.

## Open Questions

- None yet

## Steps

- [ ] In `src/score.rs::CombinedFieldStats::build`, collect per-candidate token sets across all scoring fields (after `normalize_text` / `normalize_path` and stopword filtering), then increment `df[term]` once per candidate that contains `term` in the union, and set `doc_count` / `N` to the total number of candidate documents in the scoring collection, including candidates whose scoring fields normalize to zero tokens. Keep sum-total-term-frequency and `avgdl` boost-weighted as today.
- [ ] Remove the per-field `df.max(...)` and `doc_count.max(...)` reductions from `CombinedFieldStats::build`; per-field `FieldStats` may stay only if still needed for sum-total-term-frequency / `avgdl`. Delete `FieldStats` fields that become unused.
- [ ] Rename `CombinedFieldStats` fields and tests from Lucene-style `pseudo_df` / `pseudo_doc_count` to BM25F-oriented names such as `df` / `doc_count`, `document_frequency` / `document_count`, or similarly clear local names.
- [ ] Add `src/score.rs` unit tests:
  - a term appearing in `name` of doc A and `description` of doc B has `df == 2`, not `1` (the previous max-based behavior);
  - a candidate with all fields empty after normalization still contributes to `N`;
  - a single-doc corpus where the term appears in two fields still has `df == 1`;
  - existing assertions (rare-term outranks common term, boosted field outranks weaker field, longer combined length penalty, deterministic ordering, empty `path_prefix` no-panic) continue to hold.
- [ ] Run `cargo test --lib score` and `cargo test --test cli match`; fix any test that asserts exact pre-change scores rather than ordering.
- [ ] Dogfood the Suggested Evaluation queries against this repo and the discovery fixture:
  - `cargo run -- match review`
  - `cargo run -- match review against the active plan`
  - `cargo run -- match implement from the active plan`
  - `cargo run -- match revise the active plan`
  - `cargo run -- match docgarden match scoring`
  - `cargo run -- match the` (must still error)
  Record top-result scores and the gap to the second result under `Discoveries`. Add more queries only if this set exposes a coverage gap.
- [ ] Recalibrate `score_band` thresholds and `match --help` long_about only if the recorded distribution makes the current relative-plus-coverage bands wrong. If unchanged, note that explicitly under `Discoveries`.
- [ ] Update `docs/design-docs/scoring.md`:
  - rewrite "Current Lucene-Derived Scoring Shape" pseudocode so `df` is document-union and `N` is total candidate documents;
  - add a one-paragraph note in "Current State" that `docgarden` follows BM25F document-level IDF and intentionally diverges from Lucene's `CombinedFieldQuery` per-field-max approximation;
  - delete "### 1. Evaluate Document-Level IDF Statistics" from "Proposed Future Directions" and renumber the remaining directions;
  - refresh "Open Questions" if needed.
- [ ] Update `docs/design-docs/match-and-list.md` only if the ranking contract changed materially.
- [ ] Run `cargo run -- lint docs/design-docs/scoring.md docs/design-docs/match-and-list.md docs/exec-plans/active/0016-document-union-pseudo-statistics.md --color never` and address any findings.
- [ ] Run `cargo xtask validate` as the final pre-handoff check.

## Validation

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --lib score`
- `cargo test --test cli match`
- `cargo run -- match review`
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`
- `cargo run -- match the` (expect non-zero exit, stopword-only error on stderr)
- `cargo run -- lint docs/design-docs/scoring.md docs/design-docs/match-and-list.md docs/exec-plans/active/0016-document-union-pseudo-statistics.md --color never`
- `cargo xtask validate`

## Discoveries

- None yet

## Review

- [ ] None yet
