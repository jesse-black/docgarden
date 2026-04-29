---
description: "Add Snowball English (Porter2) stemming via the `rust-stemmers` crate to the shared `docgarden match` analyzer so query, corpus statistics, scoring, and matched-term highlighting observe the same stemmed tokens."
---

# Apply Snowball English (Porter2) stemming to `docgarden match` tokens

## Goal

`docgarden match` analyzes query and corpus tokens through one shared pipeline that lowercases, splits on punctuation/path separators, removes English stopwords, and applies Snowball English (Porter2) stemming via the `rust-stemmers` crate. The same analyzed tokens drive `CombinedFieldStats`, `score()`, the `execute_match` query, and `render_match_field` highlighting, so morphological variants such as `plan`/`plans` and `review`/`reviews` are matched and highlighted symmetrically.

## Scope

- In:
  - `rust-stemmers = "1.2"` dependency in `Cargo.toml`
  - One shared `Stemmer` instance for `Algorithm::English` held in a `OnceLock<rust_stemmers::Stemmer>` in `src/score.rs` next to the existing stopword set (`Stemmer` is `Sync` in current versions, so `OnceLock` is the right primitive; `Stemmer::create` is cheap but not free, so initialize once)
  - Crate-private `pub(crate) fn analyze_token(&str) -> Option<String>` in `src/score.rs` as the **single source of truth** for per-token analysis: lowercase → empty filter → stopword filter → Porter2 stem, returning `None` when the token is empty or a stopword
  - `tokenize` becomes a thin splitter that calls `analyze_token` via `filter_map` over each chunk; `normalize_text` and `normalize_path` keep their signatures and start producing stemmed tokens transparently (analyzer order matches Lucene `EnglishAnalyzer` — the shipped stopword list is unstemmed surface forms)
  - `flush_render_token` in `src/matching.rs` calls `analyze_token` on the surface token and highlights when the result matches an analyzed `query_terms` entry (`plans` highlights when the query is `plan`); the ad-hoc `to_lowercase` + raw `contains` path is removed so highlighting cannot drift from scoring
  - Stopword-only-query rejection in `execute_match` keeps working post-analysis with stemming on
  - `src/score.rs` unit tests for: `normalize_text` stems (`plans`→`plan`, `reviews`→`review`, `analyzed`→`analyz`), `normalize_path` stems on path-segmented input, single-token analyzer returns `None` for empty and stopword input, stopword filtering still applies before stemming, deterministic ordering, empty input is unaffected
  - `src/matching.rs` unit tests for: a token whose stem matches a query term is highlighted; a stopword surface token is not highlighted
  - `tests/cli.rs` end-to-end tests for: a singular query returns documents whose only surface form is the plural variant, and (with `--explain` and forced color) the plural surface form is wrapped in the highlight ANSI sequence
  - Re-run the five Suggested Evaluation queries from ExecPlan 0010 (`review`, `review against the active plan`, `implement from the active plan`, `revise the active plan`, `docgarden match scoring`) against `tests/discovery-repo/` and the live repo; record the new top scores and gaps under `Discoveries`
  - Update `docs/design-docs/scoring.md` to promote stemming from "Proposed Future Direction 1" to shipped behavior, document the analyzer order (lowercase → split → stopword → Porter2 stem), and cite ADRs 0003 and 0004
  - Update `docs/design-docs/match-and-list.md` analyzer-contract description to mention stemming alongside stopword filtering, only where it is already described
- Out:
  - Lemmatization, dictionary stemmers, or any non-Snowball algorithm
  - Multi-language stemming or runtime algorithm selection
  - Relevancy cutoff (Future Direction 2 in `docs/design-docs/scoring.md`)
  - Phrase or proximity bonuses (Future Direction 3)
  - JSON output, `--explain` schema changes, new CLI flags
  - `docgarden lint` rule changes
  - Changes to BM25F constants (`k1`, `b`, field boosts) or to the field set

## Relevant Areas

- `Cargo.toml` — add `rust-stemmers` dependency
- `src/score.rs` — add `OnceLock<rust_stemmers::Stemmer>`; stem inside the shared `tokenize` helper after stopword filtering; add a single-token analyzer helper used by highlighting; existing `normalize_text`/`normalize_path`/`CombinedFieldStats::build`/`score` keep their signatures and start producing stemmed tokens transparently
- `src/matching.rs` — `flush_render_token` analyzes the surface token (lowercase + stopword + stem) and matches against the already-stemmed `query_terms` `Vec<String>` from `normalize_text`
- `tests/discovery-repo/docs/` — confirm existing evaluator/planner/generator fixtures still cover routing-separation queries; add a small fixture only if a new morphological case is needed (e.g., a doc whose only surface form is `plans` for a `plan` query)
- `tests/cli.rs` — add stemming end-to-end tests; recheck score-literal assertions and exact-color band assertions; rerun Suggested Evaluation queries
- `docs/design-docs/scoring.md` — promote stemming to shipped; remove or rewrite "Proposed Future Direction 1: Stemming" so it no longer reads as deferred
- `docs/design-docs/match-and-list.md` — small analyzer-contract update only where stopword filtering is already described
- `docs/decisions/0003-use-stemming-for-match-tokens.md`, `docs/decisions/0004-use-snowball-english-via-rust-stemmers.md` — source ADRs; do not modify

## Open Questions

- None.

## Steps

- [ ] Add `rust-stemmers = "1.2"` to `[dependencies]` in `Cargo.toml`; run `cargo build` to refresh `Cargo.lock`
- [ ] In `src/score.rs`, add a `OnceLock<rust_stemmers::Stemmer>` initialized to `Stemmer::create(Algorithm::English)`; add `pub(crate) fn analyze_token(token: &str) -> Option<String>` as the single per-token entry point: lowercase → empty filter → stopword filter → Porter2 stem, returning `None` on empty or stopword input
- [ ] Refactor `tokenize` into a thin splitter that calls `analyze_token` via `filter_map` over each chunk, so `normalize_text` and `normalize_path` keep their signatures and `CombinedFieldStats::build`, `score()`, and `execute_match` start observing stemmed tokens transparently
- [ ] Add `src/score.rs` unit tests: `normalize_text("plans reviews")` returns `["plan", "review"]`; `normalize_path("docs/the-active-plans/scoring-guide.md")` stems each path segment; the single-token analyzer returns `None` for `""` and for stopwords (`"the"`, `"is"`); analyzer lowercases mixed case (`"Reviews"` → `Some("review")`); ordering is preserved
- [ ] Update `flush_render_token` in `src/matching.rs` to call `analyze_token` on the surface token and check the result against `query_terms`; remove the ad-hoc `to_lowercase` + raw `contains` path so highlighting cannot drift from scoring
- [ ] Add `src/matching.rs` unit tests: a surface token whose stem matches a query term is wrapped with the highlight ANSI sequence; a stopword token never highlights
- [ ] Add `tests/cli.rs` end-to-end test that a singular query (e.g., `plan`) returns at least one document whose only surface form is the plural variant; assert top result path and (with `--explain` and forced color) that the plural surface form is wrapped in the highlight escape sequence. If the existing fixtures do not already exercise this, add one minimal fixture under `tests/discovery-repo/docs/`
- [ ] Re-run the five Suggested Evaluation queries from ExecPlan 0010 against the discovery fixture and the live repo (`cargo run -- match …` for each); record top three scores and the top-vs-second gap per query under `Discoveries`
- [ ] If routing-separation `top >= second * 1.5` floors no longer hold in `match_routes_review_queries_to_expected_execplan_docs`, `match_routes_plan_authoring_and_implementation_queries`, or `match_routes_scoring_query_to_scoring_guide`, document the regression in `Discoveries` and update only the specific failing factor; do not relax assertions wholesale
- [ ] Do not add per-fixture band-color assertions for stemming tests. Score bands (`--explain` red/yellow/green) are visual-only output for human inspection and are not part of the match contract. The existing `match_explain_colorizes_scores_by_relative_and_coverage_bands` test exercises the band feature with a varied fixture; if stemming shifts its scores enough that all three bands no longer appear, adjust the fixture inputs to restore band coverage rather than asserting any specific band per row
- [ ] Update `docs/design-docs/scoring.md`: rewrite the "Proposed Future Direction 1: Stemming" section as shipped behavior; document the analyzer order (lowercase → split → stopword → Porter2 stem), the shared `analyze_token` entry point, the highlighting alignment, and the `rust-stemmers` dependency; cite ADRs 0003 and 0004; keep the remaining future directions (cutoff, phrase/proximity) intact
- [ ] Update `docs/design-docs/match-and-list.md` analyzer-contract description to mention stemming alongside stopword filtering, only where it is already described
- [ ] Run `cargo run -- lint <changed-md-files> --color never` for any docs changed in the previous two steps

## Validation

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --lib score`
- `cargo test --lib matching`
- `cargo test --test cli match`
- `cargo test`
- `cargo run -- match plan` (expect at least one doc whose only surface form is `plans`)
- `cargo run -- match review` (expect routing to `evaluator-execplan` to remain)
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`
- `cargo run -- match the` (expect non-zero exit, stopword-only error on stderr, no stdout)
- `cargo run -- match --help` (expect description still accurate; update only if the analyzer line now misrepresents tokenization)
- `cargo run -- lint <changed-md-files> --color never`

## Discoveries

- None yet

## Review

- [ ] None yet
