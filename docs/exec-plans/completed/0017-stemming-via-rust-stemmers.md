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

- [x] Add `rust-stemmers = "1.2"` to `[dependencies]` in `Cargo.toml`; run `cargo build` to refresh `Cargo.lock`
- [x] In `src/score.rs`, add a `OnceLock<rust_stemmers::Stemmer>` initialized to `Stemmer::create(Algorithm::English)`; add `pub(crate) fn analyze_token(token: &str) -> Option<String>` as the single per-token entry point: lowercase → empty filter → stopword filter → Porter2 stem, returning `None` on empty or stopword input
- [x] Refactor `tokenize` into a thin splitter that calls `analyze_token` via `filter_map` over each chunk, so `normalize_text` and `normalize_path` keep their signatures and `CombinedFieldStats::build`, `score()`, and `execute_match` start observing stemmed tokens transparently
- [x] Add `src/score.rs` unit tests: `normalize_text("plans reviews")` returns `["plan", "review"]`; `normalize_path("docs/the-active-plans/scoring-guide.md")` stems each path segment; the single-token analyzer returns `None` for `""` and for stopwords (`"the"`, `"is"`); analyzer lowercases mixed case (`"Reviews"` → `Some("review")`); ordering is preserved
- [x] Update `flush_render_token` in `src/matching.rs` to call `analyze_token` on the surface token and check the result against `query_terms`; remove the ad-hoc `to_lowercase` + raw `contains` path so highlighting cannot drift from scoring
- [x] Add `src/matching.rs` unit tests: a surface token whose stem matches a query term is wrapped with the highlight ANSI sequence; a stopword token never highlights
- [x] Add `tests/cli.rs` end-to-end test that a singular query (e.g., `plan`) returns at least one document whose only surface form is the plural variant; assert top result path and (with `--explain` and forced color) that the plural surface form is wrapped in the highlight escape sequence. If the existing fixtures do not already exercise this, add one minimal fixture under `tests/discovery-repo/docs/`
- [x] Re-run the five Suggested Evaluation queries from ExecPlan 0010 against the discovery fixture and the live repo (`cargo run -- match …` for each); record top three scores and the top-vs-second gap per query under `Discoveries`
- [x] If routing-separation `top >= second * 1.5` floors no longer hold in `match_routes_review_queries_to_expected_execplan_docs`, `match_routes_plan_authoring_and_implementation_queries`, or `match_routes_scoring_query_to_scoring_guide`, document the regression in `Discoveries` and update only the specific failing factor; do not relax assertions wholesale
- [x] Do not add per-fixture band-color assertions for stemming tests. Score bands (`--explain` red/yellow/green) are visual-only output for human inspection and are not part of the match contract. The existing `match_explain_colorizes_scores_by_relative_and_coverage_bands` test exercises the band feature with a varied fixture; if stemming shifts its scores enough that all three bands no longer appear, adjust the fixture inputs to restore band coverage rather than asserting any specific band per row
- [x] Update `docs/design-docs/scoring.md`: rewrite the "Proposed Future Direction 1: Stemming" section as shipped behavior; document the analyzer order (lowercase → split → stopword → Porter2 stem), the shared `analyze_token` entry point, the highlighting alignment, and the `rust-stemmers` dependency; cite ADRs 0003 and 0004; keep the remaining future directions (cutoff, phrase/proximity) intact
- [x] Update `docs/design-docs/match-and-list.md` analyzer-contract description to mention stemming alongside stopword filtering, only where it is already described
- [x] Run `cargo run -- lint <changed-md-files> --color never` for any docs changed in the previous two steps

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

- Focused unit and CLI checks pass post-stemming: `cargo test --lib score`, `cargo test --lib matching`, `cargo test --test cli match_singular_query_matches_and_highlights_plural_surface_form`, `cargo test --test cli match_routes`, and `cargo test --test cli match`.
- Final validation passed with `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, validation `cargo run -- match ...` commands from this plan, `cargo run -- lint docs/exec-plans/active/0017-stemming-via-rust-stemmers.md docs/design-docs/scoring.md docs/design-docs/match-and-list.md --color never`, and `cargo xtask validate`.
- Discovery fixture evaluation (`cargo run --manifest-path /workspaces/dglint/Cargo.toml -- match --explain --limit 3 ...` from `tests/discovery-repo/`):
  - `review`: `docs/evaluator-execplan.md` 2.13; no second result.
  - `review against the active plan`: `docs/evaluator-execplan.md` 6.24, `docs/exec-plans/active/current.md` 2.61, `docs/generator-execplan.md` 2.47; top/second gap 2.39.
  - `implement from the active plan`: `docs/generator-execplan.md` 6.15, `docs/exec-plans/active/current.md` 2.61, `docs/planner-execplan.md` 2.47; top/second gap 2.36.
  - `revise the active plan`: `docs/planner-execplan.md` 4.55, `docs/exec-plans/active/current.md` 2.61, `docs/generator-execplan.md` 2.47; top/second gap 1.74.
  - `docgarden match scoring`: `docs/scoring-guide.md` 2.27, `docs/discovery-overview.md` 1.32, `docs/common-word.md` 1.03; top/second gap 1.72.
- Live repo evaluation (`cargo run -- match --explain --limit 3 ...` from `/workspaces/dglint`):
  - `review`: `docs/REVIEWING.md` 3.87, `.agents/skills/evaluator-execplan/SKILL.md` 2.92, `docs/TESTING.md` 2.83; top/second gap 1.33.
  - `review against the active plan`: `.agents/skills/evaluator-execplan/SKILL.md` 9.89, `docs/REVIEWING.md` 4.65, `.agents/skills/generator-execplan/SKILL.md` 4.08; top/second gap 2.13.
  - `implement from the active plan`: `.agents/skills/generator-execplan/SKILL.md` 9.18, `docs/exec-plans/completed/0016-document-union-pseudo-statistics.md` 4.57, `.agents/skills/planner-execplan/SKILL.md` 3.33; top/second gap 2.01.
  - `revise the active plan`: `.agents/skills/planner-execplan/SKILL.md` 5.38, `.agents/skills/generator-execplan/SKILL.md` 4.08, `docs/PLANS.md` 3.44; top/second gap 1.32.
  - `docgarden match scoring`: `docs/design-docs/scoring.md` 5.12, `docs/exec-plans/completed/0010-bm25f-scoring.md` 4.92, `docs/decisions/0002-use-bm25f-as-the-scoring-model.md` 4.85; top/second gap 1.04.

## Review

- [x] **ADR 0001 was edited after being committed.** exception to ADR immutability made in this case as the deleted line was violating the last rule of `DECISIONS.md` and deleting non-decision text does not change the decision therefore there is nothing to supersede.
- [x] **Out-of-scope policy edits to `docs/DECISIONS.md`.** This is edited outside of the normal plan workflow and is accepted.
- [x] **`docs/design-docs/match-and-list.md` adds new analyzer-contract description rather than augmenting an existing one.** Plan step says: "Update `docs/design-docs/match-and-list.md` analyzer-contract description to mention stemming alongside stopword filtering, **only where it is already described**." `git show main:docs/design-docs/match-and-list.md | grep -i stopword` returns nothing — the original file had no analyzer-contract description in this section. The implementation adds two new bullets (one for analyzer contract, one for highlighting alignment) under "Relationship To Scoring", which conflicts with the plan's "scoring/normalization belong in `docs/design-docs/scoring.md`" final bullet right above. Either drop the two new bullets and let `scoring.md` own the analyzer description, or update the plan step to authorize introducing a new analyzer mention here. Resolved by dropping the analyzer and highlighting bullets from `docs/design-docs/match-and-list.md`; `docs/design-docs/scoring.md` remains the owner for analyzer behavior and highlighting alignment. ([docs/design-docs/match-and-list.md](../../design-docs/match-and-list.md))
- [x] **Plan step 12 is no longer satisfied by the final state of `docs/design-docs/scoring.md`.** The step required scoring.md to (a) document the analyzer order (lowercase → split → stopword → Porter2 stem), (b) name the shared `analyze_token` entry point, (c) describe the highlighting alignment, (d) mention the `rust-stemmers` dependency, and (e) cite ADRs 0003 and 0004. Commit `f17d79a` delivered all five. The out-of-plan trim in commit `12388aa` removed all five and replaced them with a single line: "receives analyzed tokens from the shared tokenizer described in `tokenizer.md`". The new `tokenizer.md` does not pick any of them up — it covers only the splitter, names `analyze_token` without describing what it does, does not document the analyzer order, does not describe highlighting alignment, does not cite ADR 0003 or 0004, and references `rust-stemmers` only as an example punctuation-boundary identifier in the "Recommended Direction" section of [tokenizer.md](../../design-docs/analyzer.md). Net effect: stemming as shipped behavior is no longer explained anywhere in `docs/design-docs/`, and `cargo run -- match stemming` no longer routes to `scoring.md` because its frontmatter description was also trimmed of "stemming" and "stopword handling". Either restore the analyzer documentation (in `tokenizer.md` if that is the new owner, or back in `scoring.md`) and the ADR citations, or revise plan step 12 to acknowledge the new partition and confirm what `tokenizer.md` is now expected to own. Resolved by adding an "Analyzer Order" section to [tokenizer.md](../../design-docs/analyzer.md) that documents the four-step chain, names `analyze_token` and the splitter wrappers, describes highlighting alignment for plural surface forms, names the `rust-stemmers` dependency, and cites ADRs 0003 and 0004; this honors the partition recorded in `match-and-list.md` ("tokenization and analyzer changes belong in `docs/design-docs/tokenizer.md`").
- [x] **CLI test asserts a non-meaningful display invariant.** In `match_singular_query_matches_and_highlights_plural_surface_form` ([tests/cli.rs](../../../tests/cli.rs)), the assertion `assert!(!rows[0].name.contains("Plan "));` guards against a stem leaking into displayed surface text. The render pipeline only emits surface tokens; stems are internal scoring state and have no path to the displayed `name` column. The assertion does not protect against any reachable regression, and the trailing space in `"Plan "` makes the intent unclear. Replace with a positive `name` assertion (e.g., `assert_eq!(rows[0].name, "Release Plans")`) or drop it. Resolved by dropping the assertion.
