---
description: "Implement the remaining `Adopt` items from `docs/design-docs/analyzer.md` — CamelCase splitting, internal apostrophe preservation with trailing possessive removal, and a small explicit compound-word splitter dictionary — through the shared `analyze_token` entry point used by scoring and highlighting."
---

# Implement analyzer adopt list (CamelCase, apostrophes, compound-word dictionary)

## Goal

The shared `docgarden match` analyzer adopts three new rules from [docs/design-docs/analyzer.md](../../design-docs/analyzer.md):

- CamelCase boundary splitting (`ExecPlan` → `Exec`, `Plan`; `XMLParser` → `XML`, `Parser`)
- Internal apostrophe preservation with trailing English possessive removal (`O'Reilly` and `you're` stay one token; `Jim's` → `Jim`; `O'Reilly's` → `O'Reilly`)
- An explicit compound-word splitter dictionary applied symmetrically at index and query time, with initial entry `execplan` → `exec`, `plan`

Splitting and per-token analysis remain consolidated behind one analyzer entry point so corpus stats, BM25F scoring, explain-mode coverage, and matched-term highlighting observe the same token stream.

## Scope

- In:
  - Extract the analyzer into a new `src/analyzer.rs` module, matching the seam already drawn in `docs/design-docs/analyzer.md`. Move `STOPWORDS`, `ENGLISH_STEMMER`, `is_stopword`, `english_stemmer`, `analyze_token`, `normalize_text`, `normalize_path`, the shared splitter helper, the new CamelCase helper, the possessive strip, and `COMPOUND_DICTIONARY` out of `src/score.rs` and into `src/analyzer.rs`. Move the analyzer-only unit tests with them. `src/score.rs` keeps `Candidate`, `CombinedFieldStats`, `Field`, `ScoredHit`, `score`, BM25 constants, and the field boosts, importing analyzer functions it needs. Add `mod analyzer;` and update imports in `src/main.rs`/`src/lib.rs` and in `src/matching.rs` (which already imports `analyze_token` and `normalize_text` from `score`).
  - One unified splitter for text and paths. Today the two predicates already split on the same set of chars (`/`, `_`, `-`, `.` are all `is_ascii_punctuation()`), so once `'` is removed from both, the only path-specific work is stripping a trailing `.md`. Collapse `normalize_text` and `normalize_path` so they share a single splitter helper; `normalize_path` becomes a thin wrapper that strips `.md` and delegates. The redundant upfront `to_lowercase()` in `normalize_path` is removed (`analyze_token` already lowercases). Mirror the same consolidation in `src/matching.rs`: collapse `is_separator` text/path branches into one predicate, and remove `FieldRenderMode` if its only remaining purpose is choosing between two now-identical separator sets.
  - CamelCase splitter step splitting each post-punctuation chunk at lowercase→uppercase boundaries and at uppercase→uppercase→lowercase boundaries (Lucene-style acronym rule). Digits are not boundaries.
  - Trailing English possessives stripped inside `analyze_token` after lowercasing, before stopword filtering. Both singular `'s` and plural `s'` are removed, so `Jim's` → `jim` and `dogs'` → `dogs` (then Porter2-stemmed alongside the singular form).
  - `static COMPOUND_DICTIONARY: &[(&str, &[&str])]` in `src/analyzer.rs`, applied inside `analyze_token` after possessive strip and stopword filter, before stemming. Initial entry: `("execplan", &["exec", "plan"])`. Splits replace the original token; both halves are stemmed.
  - `analyze_token` return type broadened so it can yield zero, one, or many analyzed tokens (e.g. `Vec<String>` or a `SmallVec`). All call sites in `src/score.rs` and `src/matching.rs::flush_render_token` updated.
  - `src/matching.rs::render_match_field` walks CamelCase and apostrophe boundaries identically to the splitter so highlighting follows the same edges. `is_separator` (text mode) no longer treats `'` as a separator. CamelCase boundaries are detected in the walker. Highlight matches when any analyzer-emitted token from a surface subtoken hits `query_terms`.
  - Unit tests in `src/analyzer.rs` for: `normalize_text("ExecPlan")` → `["exec", "plan"]`; `normalize_text("XMLParser")` → `["xml", "parser"]`; `normalize_text("PowerShot")` → `["power", "shot"]`; `normalize_text("Jim's notebook")` → `["jim", "notebook"]`; `normalize_text("dogs' bowls")` → same first stem as `normalize_text("dog bowls")[0]`; `normalize_text("you're")` → one token (capture and assert exact stem); `normalize_text("O'Reilly")` → one token; `normalize_text("O'Reilly's")` → same single token as `O'Reilly`; `normalize_text("execplan")` → `["exec", "plan"]`; `normalize_path("docs/planner-execplan.md")` → includes `exec` and `plan`; non-splits: `BM25F`, `v1`, `f32`, `R2D2`, `SD500`, `ADR0004` remain a single analyzed token each.
  - Unit tests in `src/matching.rs` for: query `plan` highlights `Plan` (not the whole `ExecPlan`) inside an `ExecPlan` surface form; query `exec` highlights `Exec`; query `jim` highlights `Jim` in `Jim's`; query `the` does not highlight `the` in `there's`; the lowercase compound `execplan` is highlighted as one bolded run for query `plan` (the whole surface token expands to a stem set containing `plan`).
  - End-to-end tests in `tests/cli.rs`:
    - `cargo run -- match plan` returns a fixture whose only routing signal is `ExecPlan` in the name field. Add a minimal fixture under `tests/discovery-repo/docs/` only if none exists.
    - `cargo run -- match exec` returns a controlled fixture whose only `exec` signal is the lowercase compound `execplan`. Add a minimal fixture such as `tests/discovery-repo/docs/execplan.md` with neutral non-`exec` frontmatter if no existing fixture isolates this case; do not rely on `planner-execplan.md`, `generator-execplan.md`, or `evaluator-execplan.md`, which all contain the same lowercase compound in their names/paths.
    - With `--explain` and forced color, the `Plan` half of `ExecPlan` is wrapped in the highlight escape and `Exec` is not, when the query is `plan`.
  - Re-run the five Suggested Evaluation queries from ExecPlan 0010 and the queries recorded in ExecPlan 0017 against `tests/discovery-repo/` and the live repo. Record top three scores and top-vs-second gap per query under `Discoveries` and compare against the 0017 baseline so any regression is visible.
  - Update [docs/design-docs/analyzer.md](../../design-docs/analyzer.md):
    - Rewrite the "Analyzer Order" section so the documented chain matches the new pipeline: punctuation/path split → CamelCase split → per-token analyze (lowercase → strip trailing `'s` → stopword filter → compound dictionary expansion → Porter2 stem).
    - Note that `analyze_token` may emit multiple tokens after compound expansion.
    - Move the three adopted items (CamelCase, apostrophe handling, compound dictionary) out of the "Adopt" list and into the shipped description. Leave "Consider later" intact, and keep "Avoid" except for updating the compound-word avoidance language to match ADR 0006's "full English dictionary" framing. The "single analyzer entry point" invariant stays in the shipped description.
  - Run `cargo run -- lint <changed-md-files> --color never` after any docs changes.

- Out:
  - Letter-number boundary splitting (`SD500`, `ADR0004`, `RFC9110`) — explicitly deferred in the design doc.
  - Unicode word-boundary splitting / `unicode-segmentation` adoption — design-doc "Consider later".
  - Semantic alias layer (`planner` → `plan`) — design-doc "Consider later".
  - Full-English-dictionary or broad lexical compound splitting beyond the explicit dictionary — design-doc "Avoid".
  - Preserving original tokens or catenating split parts — design-doc "Avoid".
  - URL/email-aware tokenization.
  - Tokenizer-level max-token-length policy.
  - Changes to BM25F constants (`k1`, `b`, field boosts) or to the field set.
  - Score-band assertions in any new CLI tests (per the closing-out note in ExecPlan 0017).

## Relevant Areas

- `src/analyzer.rs` (new) — destination module for the splitter wrappers (`normalize_text`, `normalize_path`, shared splitter helper), per-token analyzer (`analyze_token`), stopword set, stemmer, CamelCase splitter helper, possessive strip, and compound dictionary.
- `src/score.rs` — keeps `Candidate`, `CombinedFieldStats`, `Field`, `ScoredHit`, `score`, BM25 constants, field boosts; imports analyzer functions it needs.
- `src/main.rs` / `src/lib.rs` — declare `mod analyzer;` and re-export only what other modules need.
- `src/matching.rs` — imports analyzer functions from `analyzer` instead of `score`; `render_match_field`, `flush_render_token`, and the consolidated `is_separator` mirror the new split rules so highlighting matches scoring.
- `tests/discovery-repo/docs/` — existing `planner-execplan.md`, `generator-execplan.md`, `evaluator-execplan.md` already cover the lowercase compound case via paths/names. Add fixtures only if no existing doc exercises CamelCase or apostrophe routing.
- `tests/cli.rs` — end-to-end coverage and the suggested-evaluation re-runs.
- [docs/design-docs/analyzer.md](../../design-docs/analyzer.md) — analyzer order documentation; promote adopted items to shipped state.
- `Cargo.toml` — no new dependency expected (rule-based; no Unicode segmentation library).

## Open Questions

- None.

## Steps

- [x] Create `src/analyzer.rs` and mechanically move the analyzer surface out of `src/score.rs`: `STOPWORDS`, `ENGLISH_STEMMER`, `is_stopword`, `english_stemmer`, `analyze_token`, `normalize_text`, `normalize_path`, the existing `tokenize` helper, and the analyzer-only unit tests. Adjust `include_str!("data/stopwords_en.txt")` in the new location (the path stays the same — it is relative to the source file and the data directory is at `src/data/`). Add `mod analyzer;` to `src/main.rs` (or `src/lib.rs`, whichever owns the module tree). Update imports in `src/matching.rs` from `crate::score::{analyze_token, normalize_text, …}` to pull analyzer items from `crate::analyzer::…` and scoring items from `crate::score::…`. Run `cargo build` and `cargo test` to confirm the move is behavior-preserving before any new logic lands.
- [x] Unify the splitter pipeline in `src/analyzer.rs`: introduce one shared splitter helper whose separator predicate is `c.is_whitespace() || (c.is_ascii_punctuation() && c != '\'')`. Reduce `normalize_path` to `strip_suffix(".md")` followed by a call to the shared helper (drop the redundant upfront `to_lowercase()` and the dead `matches!(c, '/' | '_' | '-' | '.')` enumeration). `normalize_text` calls the shared helper directly. Add unit tests that `normalize_text("you're")` and `normalize_path("docs/you're.md")` each produce a single analyzed token before any other adopt-list logic lands, so this stage is observable in isolation.
- [x] Mirror the consolidation in `src/matching.rs`: collapse `is_separator` text/path branches into one predicate matching the splitter. If `FieldRenderMode` no longer carries any other distinction, remove it and update the callers in `render_default_row` and `render_explain_row` accordingly; otherwise keep the enum but route both variants through the same separator predicate.
- [x] Add a `split_camel_case` helper in `src/analyzer.rs` returning the boundary positions or subslices for a chunk. Rules: split at lowercase→uppercase, and at uppercase→uppercase→lowercase (so `XMLParser` → `XML`/`Parser`). Digits are not boundaries. Wire it into `tokenize` so each post-punctuation chunk is camel-split before per-token analysis. Unit-test cases: `ExecPlan`, `XMLParser`, `PowerShot`, `BM25F`, `R2D2`, `v1`, `f32`, `SD500`, `ADR0004`.
- [x] Broaden `analyze_token` from `Option<String>` to a multi-token shape (`Vec<String>` or `SmallVec<[String; 2]>`). Update all call sites: the shared splitter in `src/analyzer.rs` and `flush_render_token` in `src/matching.rs`. Empty/stopword input → empty result; normal input → one stem; compound match → multiple stems.
- [x] Inside `analyze_token`, after lowercasing and before the stopword filter, strip a trailing English possessive: try `'s` first, then `s'`. Unit tests: `analyze_token("Jim's")` → one token (`jim`); `analyze_token("O'Reilly's")` produces the same single token as `analyze_token("O'Reilly")`; `analyze_token("dogs'")` produces the same stem as `analyze_token("dog")` (Porter2 collapses `dogs` and `dog`); `analyze_token("you're")` produces one token (capture exact Porter2 output and assert it; the internal apostrophe must not be stripped); `analyze_token("'s")` and `analyze_token("s'")` → empty (each becomes empty after strip and is dropped).
- [x] Add `static COMPOUND_DICTIONARY: &[(&str, &[&str])] = &[("execplan", &["exec", "plan"])];` in `src/analyzer.rs`. Inside `analyze_token`, after possessive strip and stopword filter, look up the lowercased token; on hit, stem each entry of the expansion and return the resulting set; otherwise stem the token itself and return it as a single-element set. Unit tests: `analyze_token("execplan")` → `["exec", "plan"]`; `normalize_path("docs/planner-execplan.md")` includes `exec` and `plan`; entries not in the dictionary are unaffected.
- [x] In `src/matching.rs` (after the splitter and `is_separator` consolidation above):
  - Update `render_match_field` to additionally split surface tokens at CamelCase boundaries before calling `flush_render_token`, so highlighting and analysis agree on token edges.
  - Update `flush_render_token` to consume the multi-token analyzer return type and highlight when any emitted analyzed token hits `query_terms`.
  - Unit tests: query `plan` highlights `Plan` inside `ExecPlan` and not `Exec`; query `exec` highlights `Exec` inside `ExecPlan`; query `jim` highlights `Jim` inside `Jim's`; query `there` does not highlight `there` inside `there's` if Porter2 disagrees (capture and assert); query `plan` highlights the whole lowercase surface `execplan` because compound expansion includes `plan`.
- [x] Add `tests/cli.rs` end-to-end tests (no per-row band-color assertions, per ExecPlan 0017's closing-out note):
  - `cargo run -- match plan` returns a fixture whose only routing signal is `ExecPlan` in name. Add a minimal fixture under `tests/discovery-repo/docs/` if none exists.
  - `cargo run -- match exec` returns the controlled lowercase-compound fixture added for this case.
  - With `--explain` and forced color, the `Plan` half of an `ExecPlan` surface token is wrapped in the highlight escape and `Exec` is not, for query `plan`.
- [x] Re-run the five Suggested Evaluation queries from ExecPlan 0010 (`review`, `review against the active plan`, `implement from the active plan`, `revise the active plan`, `docgarden match scoring`) plus any others recorded in ExecPlan 0017 against `tests/discovery-repo/` and the live repo (`cargo run -- match --explain --limit 3 …`). Record top three scores and top-vs-second gap per query under `Discoveries` and compare to the 0017 baseline.
- [x] If routing-separation floors in `match_routes_review_queries_to_expected_execplan_docs`, `match_routes_plan_authoring_and_implementation_queries`, or `match_routes_scoring_query_to_scoring_guide` regress, document the regression under `Discoveries` and update only the specific failing factor; do not relax assertions wholesale.
- [x] Update [docs/design-docs/analyzer.md](../../design-docs/analyzer.md): rewrite "Analyzer Order" to describe the new chain, note that `analyze_token` may emit multiple tokens, move CamelCase, apostrophe handling, and the compound dictionary out of the "Adopt" list and into the shipped description, and keep the compound-word "Avoid" language aligned with ADR 0006's "full English dictionary" framing.
- [x] Run `cargo run -- lint docs/design-docs/analyzer.md docs/exec-plans/active/0018-analyzer-adopt-list.md --color never` plus any other `.md` files touched.
- [ ] Replace matching-owned display splitting with an analyzer-owned span API. Add an analyzer helper such as `analyze_surface_spans(token) -> Vec<AnalyzedSpan<'_>>`, where each span carries the original display slice plus analyzed terms. It should emit `Exec`/`Plan` spans for CamelCase, `Jim` plus a no-terms possessive suffix span for `Jim's`, and one `execplan` span whose terms are `["exec", "plan"]`.
- [ ] Rebase `normalize_text` and `normalize_path` on the span API by splitting on the shared separator, feeding each separator-delimited chunk through the span helper, and flattening span terms. Preserve the existing analyzer order: lowercase, possessive handling, stopword filtering, compound expansion, then stemming.
- [ ] Update `src/matching.rs::flush_render_token` to consume analyzer spans instead of `render_token_parts` and `split_possessive_suffix`; remove the matching-local possessive splitter so display boundaries and analyzed terms live in `src/analyzer.rs`. Highlight a span when any span term is in `query_terms`; continue escaping pipe characters in the rendered surface.
- [ ] Add focused coverage for the span API and rendering handoff: analyzer unit tests for `ExecPlan`, `Jim's`, `execplan`, and stopword suffix/no-term spans; matching unit tests proving the existing highlight cases still pass through analyzer spans.
- [ ] Re-run validation after the span API refactor: `cargo test --lib analyzer`, `cargo test --lib matching`, `cargo test --test cli match`, `cargo run -- lint docs/exec-plans/active/0018-analyzer-adopt-list.md docs/design-docs/analyzer.md --color never`, and the final validation commands listed below if any production code or design docs changed.

## Validation

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --lib analyzer`
- `cargo test --lib score`
- `cargo test --lib matching`
- `cargo test --test cli match`
- `cargo test`
- `cargo run -- match plan`
- `cargo run -- match exec`
- `cargo run -- match ExecPlan`
- `cargo run -- match jim` (against a fixture exercising trailing `'s`)
- `cargo run -- match review`
- `cargo run -- match review against the active plan`
- `cargo run -- match implement from the active plan`
- `cargo run -- match revise the active plan`
- `cargo run -- match docgarden match scoring`
- `cargo run -- match the` (expect non-zero exit, stopword-only error on stderr, no stdout)
- `cargo run -- match --help` (expect description still accurate; update only if the analyzer line now misrepresents tokenization)
- `cargo run -- lint <changed-md-files> --color never`
- `cargo xtask validate` (final pass before handoff)

## Discoveries

- Mechanical analyzer extraction completed with `cargo build`, `cargo test --lib analyzer`, and `cargo test --lib score` passing before behavioral changes.
- Analyzer/matching behavior implementation completed with `cargo test --lib analyzer`, `cargo test --lib matching`, `cargo test --lib score`, and `cargo test --test cli match` passing after adding the adopt-list rules and fixtures.
- Compared with ExecPlan 0017's baseline, discovery fixture top routes are unchanged. Scores increased slightly because the fixture corpus now includes CamelCase, compound, and possessive docs; routing-separation tests still pass without assertion changes.
- Discovery fixture evaluation (`cargo run --manifest-path /workspaces/dglint/Cargo.toml -- match --explain --limit 3 ...` from `tests/discovery-repo/`):
  - `review`: `docs/evaluator-execplan.md` 2.14; no second result.
  - `review against the active plan`: `docs/evaluator-execplan.md` 6.78, `docs/exec-plans/active/current.md` 2.65, `docs/generator-execplan.md` 2.58; top/second gap 2.56.
  - `implement from the active plan`: `docs/generator-execplan.md` 6.33, `docs/exec-plans/active/current.md` 2.65, `docs/planner-execplan.md` 2.58; top/second gap 2.39.
  - `revise the active plan`: `docs/planner-execplan.md` 4.67, `docs/exec-plans/active/current.md` 2.65, `docs/generator-execplan.md` 2.58; top/second gap 1.76.
  - `docgarden match scoring`: `docs/scoring-guide.md` 2.59, `docs/discovery-overview.md` 1.50, `docs/common-word.md` 1.17; top/second gap 1.73.
- Live repo evaluation (`cargo run -- match --explain --limit 3 ...` from `/workspaces/dglint`):
  - `review`: `docs/REVIEWING.md` 4.03, `docs/TESTING.md` 2.96, `.agents/skills/evaluator-execplan/SKILL.md` 2.94; top/second gap 1.36.
  - `review against the active plan`: `.agents/skills/evaluator-execplan/SKILL.md` 10.16, `docs/REVIEWING.md` 4.86, `.agents/skills/generator-execplan/SKILL.md` 4.32; top/second gap 2.09.
  - `implement from the active plan`: `.agents/skills/generator-execplan/SKILL.md` 8.94, `docs/exec-plans/active/0018-analyzer-adopt-list.md` 6.24, `docs/exec-plans/completed/0016-document-union-pseudo-statistics.md` 4.48; top/second gap 1.43. ExecPlan 0017's second result was completed plan 0016 at 4.57; the active plan now ranks second because it intentionally contains the active analyzer implementation terms.
  - `revise the active plan`: `.agents/skills/planner-execplan/SKILL.md` 5.51, `.agents/skills/generator-execplan/SKILL.md` 4.32, `docs/PLANS.md` 3.63; top/second gap 1.28.
  - `docgarden match scoring`: `docs/design-docs/scoring.md` 4.94, `docs/exec-plans/completed/0010-bm25f-scoring.md` 4.76, `docs/decisions/0002-use-bm25f-as-the-scoring-model.md` 4.70; top/second gap 1.04.
- Final validation passed: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --lib analyzer`, `cargo test --lib score`, `cargo test --lib matching`, `cargo test --test cli match`, `cargo test`, all listed `cargo run -- match ...` smoke commands including expected stopword-only failure, changed Markdown lint, and `cargo xtask validate` after `git add -N` made new files visible to diff coverage.

## Review

- [ ] Update `ARCHITECTURE.md` after extracting `src/analyzer.rs`: the code map still says `src/score.rs` owns shared token normalization and stopword filtering, but this branch moved the analyzer chain into `src/analyzer.rs` and reuses it from `src/score.rs` and `src/matching.rs`.
- [ ] Align display highlighting with the analyzer boundary source. `src/matching.rs` currently carries display-only possessive splitting while `src/analyzer.rs` owns possessive analysis, which weakens the "one fact, one place" style goal and could drift when analyzer boundary rules change.
