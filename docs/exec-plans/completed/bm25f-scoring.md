---
description: "Rewrite `docgarden match` scorer as Lucene-style combined-field BM25F with analyzer stopword filtering, f32 scores, and recalibrated color bands."
---

# Rewrite `docgarden match` scorer as combined-field BM25F

## Goal

`docgarden match` ranks candidates with a Lucene-style combined-field BM25F scorer (shape of `CombinedFieldQuery` + `MultiNormsLeafSimScorer` + `BM25Similarity`) over three fields (`name`, `path_prefix`, `description`), with analyzer-level English stopword filtering applied inside `normalize_text`/`normalize_path`, `f32` scores end-to-end, and color bands recalibrated to the new distribution.

## Scope

- In:
  - `src/data/stopwords_en.txt` (Lucene `EnglishAnalyzer` list) loaded via `include_str!`
  - Stopword filtering inside `normalize_text` and `normalize_path`; symmetric at index and query time
  - `Candidate` gains `path_prefix` (directory portion, no filename, no extension; empty string at repo root)
  - Replace `IdfTable` + tier scorer with `CombinedFieldStats` (per-field `df`, per-field `doc_count`, per-field `sum_total_term_freq`, derived `avgdl`) and a combined-field BM25F `score()` with `k1 = 1.2`, `b = 0.75`, boosts `name = 3.0`, `path_prefix = 1.0`, `description = 1.0`
  - Remove match-tier constants (`10 / 4 / 1`) and the full-query phrase bonus
  - `ScoredHit.score`, `MatchResult.score`, `render_score`, `score_band`, and `tests/cli.rs` parsers move from `i32` to `f32`; sort uses `f32::total_cmp`
  - Stopword-only query returns a user-facing error distinct from the empty-query error
  - Routing-separation integration tests (evaluator / planner / generator) added under `tests/discovery-repo/docs/`
  - Add `[[rules]] path = "**/SKILL.md"` with `required = ["name"]` to `docgarden.toml`
  - Recalibrate `score_band` thresholds after dogfooding and update `match --help` text
- Out:
  - Bigram / phrase / proximity bonuses (deferred per design doc direction 3)
  - Per-field `b_f` (single `b` per Lucene's combined-field shape)
  - Tuning `k1` / `b` beyond Lucene defaults unless dogfooding forces it
  - JSON output, `--explain`, dynamic per-result-set color bands
  - Stemming, fuzzy matching, index caching
  - `docgarden lint` rule-engine changes other than the `SKILL.md` rule above

## Relevant Areas

- `src/score.rs` — replace `IdfTable`, `match_tier`, phrase-bonus, and `score()` with combined-field BM25F; add stopword set and `is_stopword`; extend `Candidate`; change `ScoredHit.score` to `f32`
- `src/matching.rs` — derive `path_prefix`; change `MatchResult.score` to `f32`; update zero-drop, sort comparator, `render_score`, `score_band`; reject stopword-only query
- `src/data/stopwords_en.txt` — new file, one term per line, Lucene `EnglishAnalyzer` list
- `src/cli.rs` — `match --help` text updated for BM25F and new band thresholds
- `tests/cli.rs` — rewrite score-literal assertions at `:221,:232,:242,:253` and `parse::<i32>` at `:397-401`; add routing and stopword-only tests
- `tests/discovery-repo/docs/` — add evaluator/planner/generator fixture docs
- `docgarden.toml` — add `SKILL.md` frontmatter rule
- `docs/design-docs/frontmatter-driven-discovery-commands.md` — refresh the v1 scoring section

## Open Questions

- Score-band thresholds for the BM25F scale: the current `0-24 / 25-59 / 60+` integer bands are meaningless after the rewrite. Pick values only after running the design doc's Suggested Evaluation queries and record actual distributions under `Discoveries` before wiring them in.
- `path_prefix` for depth-1 documents: `Path::new(rel).parent()` yields `""`; confirm `combined_length` stays positive (other fields' contributions) and `avgdl` never divides by zero even when every candidate has empty `path_prefix`. Add a root-level doc to `tests/discovery-repo/` to exercise this.
- Stopword list version: use Lucene 9.x `EnglishAnalyzer` `ENGLISH_STOP_WORDS_SET` (33 terms).
- Score display format: default `{:.2}`; revisit only if dogfood output looks noisy. Tests parse via `f32::from_str`, so format choice does not change assertion shape.
- Sort tie-break order: keep the existing `matched_terms` → `first_field_hit` → path chain unchanged. Revisit only if dogfooding shows flips.

## Steps

- [x] Add `src/data/stopwords_en.txt` with the Lucene `EnglishAnalyzer` English stopwords, one term per line
- [x] Load the list via `include_str!` into a `OnceLock<HashSet<&'static str>>` in `src/score.rs`; add `pub(crate) fn is_stopword(term: &str) -> bool`
- [x] Apply `is_stopword` inside `normalize_text` and `normalize_path` after lowercasing/splitting; unit tests for stopword-only input, mixed input, path tokenization (`the-active-plan` splits before filtering drops `the`)
- [x] In `src/matching.rs`, reject post-normalization stopword-only queries with a dedicated user-facing error distinct from the empty-query error
- [x] Extend `Candidate` with `path_prefix: &'a str`; construct via `Path::new(rel).parent()` in `src/matching.rs` (empty string at repo root); tokenize at scoring time via existing `normalize_path`
- [x] Replace `IdfTable` in `src/score.rs` with `CombinedFieldStats` (per-field `df`, `doc_count`, `sum_total_term_freq`; precomputed `pseudo_doc_count`, `pseudo_sum_total_term_freq`, per-term `pseudo_df`); single `build(&[Candidate])` entry point
- [x] Rewrite `score()` to implement the combined-field BM25F formula exactly as written in `docs/design-docs/scoring.md:123-131`; `k1` and `b` live as `const` with Lucene source references; return `ScoredHit { score: f32, matched_terms, first_field_hit }`
- [x] Delete `match_tier`, tier constants (`10 / 4 / 1`), and the full-query phrase-bonus block; remove `basename_norm` if unused
- [x] Rewrite `src/score.rs` unit tests: rare-term outranks common term, boosted field outranks weaker field at equal tf/df, longer combined length penalizes at fixed combined_freq, stopword filter affects index and query symmetrically, empty `path_prefix` does not panic, deterministic ordering
- [x] Propagate `f32` through `src/matching.rs`: `MatchResult.score: f32`; zero-drop `hit.score <= 0.0`; sort via `b.score.total_cmp(&a.score).then(...)`; `render_score` and `score_band` take `f32`
- [x] Render scores with `{:.2}`; keep ANSI color wrapping; update `match --help` long_about to say "BM25F, higher is better; ordering is the contract"
- [x] Add evaluator / planner / generator fixture docs under `tests/discovery-repo/docs/` with frontmatter `name` and `description`; do not disturb existing fixtures
- [x] Rewrite affected `tests/cli.rs` assertions to parse the leading column as `f32` and assert ordering relationships; keep one exact-color assertion per band with calibrated values
- [x] Add routing-separation tests for the five Suggested Evaluation queries (`review`, `review against the active plan`, `implement from the active plan`, `revise the active plan`, `docgarden match scoring`); assert top result and a separation floor (top ≥ 1.5 × second, or an equivalent gap fixed during dogfooding)
- [x] Add `tests/cli.rs` test: `docgarden match the` exits non-zero with the stopword-only error on stderr and no stdout
- [x] Dogfood the five Suggested Evaluation queries against this repo; record score distributions under `Discoveries`; pick `low / medium / high` thresholds; update `score_band` and `match --help`
- [x] Add `[[rules]] path = "**/SKILL.md"` with `[rules.frontmatter] required = ["name"]` to `docgarden.toml`; run `cargo run -- lint .` and add `name:` to any flagged skill doc
- [x] Update `docs/design-docs/frontmatter-driven-discovery-commands.md` v1 scoring section: BM25F + stopwords, fields `name / path_prefix / description`, `k1 = 1.2`, `b = 0.75`, boosts `3.0 / 1.0 / 1.0`, `f32` scores, new color bands

## Validation

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo test --lib score`
- `cargo test --test cli match`
- `cargo run -- match review`
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`
- `cargo run -- match the` (expect non-zero exit, stopword-only error on stderr)
- `cargo run -- match --help` (expect updated scoring description and color-band thresholds)
- `cargo run -- lint .` (expect pass after adding frontmatter `name` to any flagged `SKILL.md`)

## Discoveries

- Score-band calibration from dogfooding and fixture checks landed at `low < 1.25`, `medium 1.25-2.49`, `high >= 2.50`; the fixture color checks resolve to `1.01`, `1.63`, and `2.89`.
- Suggested-evaluation query distributions in the discovery fixture:
  - `review` -> `evaluator-execplan` only hit at `1.86`
  - `review against the active plan` -> `evaluator 5.78`, `generator 2.62`, `planner 2.41`
  - `implement from the active plan` -> `generator 6.25`, `planner 2.41`, `evaluator 2.06`
  - `revise the active plan` -> `planner 4.22`, `generator 2.62`, `evaluator 2.06`
  - `docgarden match scoring` -> `scoring-guide 1.74`, `discovery-overview 1.03`, `common-word 0.81`
- Repository dogfooding with the real skills/docs corpus still routes the active-plan queries correctly, with top scores `3.40` (`review`), `11.35` (`review against the active plan`), `12.46` (`implement from the active plan`), and `7.28` (`revise the active plan`); `cargo run -- lint .` passed without requiring additional `SKILL.md` frontmatter edits.

## Review

- [x] 2026-04-20 evaluator review: no actionable findings. The worktree matches the active plan's scoring/model changes, targeted validation passed (`cargo test --lib score`, `cargo test --test cli match`, `cargo run -- match review`, `cargo run -- match review against the active plan`, `cargo run -- match implement from the active plan`, `cargo run -- match revise the active plan`, `cargo run -- match docgarden match scoring`, `cargo run -- match the`, `cargo run -- lint .`), and the recorded dogfooding numbers in `Discoveries` matched the observed command outputs during review.
