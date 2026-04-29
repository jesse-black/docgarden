---
description: "Add Snowball English (Porter2) stemming via the `rust-stemmers` crate to the shared `docgarden match` analyzer so query, corpus statistics, scoring, and matched-term highlighting observe the same stemmed tokens."
---

# Apply Snowball English (Porter2) stemming to `docgarden match` tokens

## Goal

`docgarden match` analyzes query and corpus tokens through one shared pipeline that lowercases, splits on punctuation/path separators, removes English stopwords, and applies Snowball English (Porter2) stemming via the `rust-stemmers` crate. The same analyzed tokens drive `CombinedFieldStats`, `score()`, the `execute_match` query, and `render_match_field` highlighting, so morphological variants such as `plan`/`plans` and `review`/`reviews` are matched and highlighted symmetrically.

## Scope

- In:
  - `rust-stemmers = "1.2"` dependency in `Cargo.toml`
  - One shared `Stemmer` instance for `Algorithm::English` held in a `OnceLock` in `src/score.rs` next to the existing stopword set
  - Stemming inside the shared `tokenize` helper, applied **after** lowercasing and stopword filtering so `normalize_text` and `normalize_path` return stemmed tokens (matches Lucene `EnglishAnalyzer` ordering — the shipped stopword list is unstemmed surface forms)
  - Crate-private single-token analyzer that exposes the same lowercase + stopword + stem chain for callers that already hold a single token; used by both `tokenize` and `flush_render_token` so highlighting and scoring share one source of truth
  - `flush_render_token` in `src/matching.rs` highlights a surface token whose stem matches an analyzed query term (`plans` highlights when the query is `plan`)
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

- Confirm `rust_stemmers::Stemmer` is `Sync` (it is, in current versions) so a `OnceLock<Stemmer>` is safe; if not, fall back to `thread_local!` — `Stemmer::create(Algorithm::English)` is cheap but not free.
- The current `explain_score_band` rule is relative + coverage (`relative >= 0.75 && coverage >= 0.75` → high; etc.), so band cutoffs are not absolute and should generally survive any score shift from stemming. Verify this against the fixture exact-color assertions in `tests/cli.rs` and only adjust an assertion if the relative/coverage thresholds genuinely flip a fixture row.
- `tests/cli.rs` routing-separation `top >= second * 1.5` floors may shift slightly; hold the assertion shape and only relax a specific factor if dogfooding shows a real regression rather than a noise-level change.
- Whether to expose the single-token analyzer as `pub(crate) fn analyze_token(&str) -> Option<String>` or to refactor `tokenize` to return an iterator that maps through the same per-token function. Either is acceptable as long as `tokenize`, `normalize_text`/`normalize_path`, and the highlighter share one definition of "analyzed token."

## Steps

- [ ] Add `rust-stemmers = "1.2"` to `[dependencies]` in `Cargo.toml`; run `cargo build` to refresh `Cargo.lock`
- [ ] In `src/score.rs`, add a `OnceLock<rust_stemmers::Stemmer>` initialized to `Stemmer::create(Algorithm::English)`; add a crate-private single-token analyzer that lowercases, returns `None` for empty or stopword tokens, and otherwise returns the Porter2 stem as `String`
- [ ] Refactor the existing `tokenize` helper so the per-chunk pipeline is `lowercase → empty filter → stopword filter → stem`, keeping `normalize_text` and `normalize_path` signatures unchanged so `CombinedFieldStats::build`, `score()`, and `execute_match` start observing stemmed tokens transparently
- [ ] Add `src/score.rs` unit tests: `normalize_text("plans reviews")` returns `["plan", "review"]`; `normalize_path("docs/the-active-plans/scoring-guide.md")` stems each path segment; the single-token analyzer returns `None` for `""` and for stopwords (`"the"`, `"is"`); analyzer lowercases mixed case (`"Reviews"` → `Some("review")`); ordering is preserved
- [ ] Update `flush_render_token` in `src/matching.rs` to analyze the surface token via the same single-token analyzer and check the stemmed result against `query_terms`; remove the ad-hoc `to_lowercase` + raw `contains` path so highlighting cannot drift from scoring
- [ ] Add `src/matching.rs` unit tests: a surface token whose stem matches a query term is wrapped with the highlight ANSI sequence; a stopword token never highlights
- [ ] Add `tests/cli.rs` end-to-end test that a singular query (e.g., `plan`) returns at least one document whose only surface form is the plural variant; assert top result path and (with `--explain` and forced color) that the plural surface form is wrapped in the highlight escape sequence. If the existing fixtures do not already exercise this, add one minimal fixture under `tests/discovery-repo/docs/`
- [ ] Re-run the five Suggested Evaluation queries from ExecPlan 0010 against the discovery fixture and the live repo (`cargo run -- match …` for each); record top three scores and the top-vs-second gap per query under `Discoveries`
- [ ] If routing-separation `top >= second * 1.5` floors no longer hold in `match_routes_review_queries_to_expected_execplan_docs`, `match_routes_plan_authoring_and_implementation_queries`, or `match_routes_scoring_query_to_scoring_guide`, document the regression in `Discoveries` and update only the specific failing factor; do not relax assertions wholesale
- [ ] If any fixture exact-color assertion in `tests/cli.rs` flips because the relative/coverage band changed, document the new expected band in `Discoveries` and update that assertion; do not soften the test
- [ ] Update `docs/design-docs/scoring.md`: rewrite the "Proposed Future Direction 1: Stemming" section as shipped behavior; document the analyzer order (lowercase → split → stopword → Porter2 stem), the shared single-token analyzer, the highlighting alignment, and the `rust-stemmers` dependency; cite ADRs 0003 and 0004; keep the remaining future directions (cutoff, phrase/proximity) intact
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
