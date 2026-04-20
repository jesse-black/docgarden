---
description: "ExecPlan for shipping the `docgarden match` subcommand, including metadata scoring, CLI wiring, discovery fixtures, and follow-up validation."
---

# Implement `docgarden match` subcommand

## Goal

- `docgarden match <QUERY>` (alias `m`) prints a stable, ranked list of Markdown files whose frontmatter/path matches the query, using lint's existing traversal/ignore rules.
- Output format (text only): `score | path | name | description` per line; `--path-only/-p` prints just paths; `--limit N` / `-n N` truncates after ranking.
- Help text explains output columns, the score range, and how score colors map to low/medium/high matches.

## Scope

- **In**:
  - New `Match(MatchArgs)` variant in `src/cli.rs` with `visible_alias = "m"`.
  - `MatchArgs`: positional `query: Vec<String>` (1+ tokens, joined with single spaces before tokenization), `--config`, `--no-gitignore`, `--color`, `--limit/-n`, `--path-only/-p`. No positional targets — the design doc shows only `<QUERY>`, so the command always runs against the full repo-root discovery set determined by config and `--no-gitignore`.
  - Human-readable default output colors the `score` column when color is enabled: green for high-confidence matches, yellow for medium-confidence matches, red for low-confidence matches. The exact numeric thresholds should be documented in `match --help` and covered by tests so the behavior stays stable.
  - Reuse `discover_markdown_files_for_targets` (called with `targets = [repository_root]`), `infer_repository_root`, `Config::load`, `repository_relative_path` unchanged.
  - Promote existing frontmatter parser out of `src/lint/rules/frontmatter.rs` into a top-level `src/frontmatter.rs` so both lint and match can use it without `lint` becoming a dependency of `match`.
  - New scoring module `src/score.rs` (tiered weighted lexical + clamped corpus-local IDF, hand-rolled, no fuzzy tier) with inline unit tests. Deliberately a top-level module, not nested under matching, so a future lint rule can reuse the tokenizer / IDF / tier helpers to flag frontmatter descriptions that aren't sufficiently distinct from other docs in the corpus.
  - New orchestration module `src/matching.rs` (file read → frontmatter extract → tokenize candidates → build IDF table → score → sort → truncate → emit).
  - Integration tests in `tests/cli.rs` covering help output, ranking order, tie-break, `--limit`, `--path-only`, empty-result behavior, and `--no-gitignore`.
  - A dedicated `tests/test-repos/discovery/` fixture whose frontmatter `name`/`description` exercise the scoring tiers, IDF clamping (a term common across all docs should still contribute), and the `.gitignore` exclusion path.
- **Out**:
  - `docgarden list` / `ls` (future plan; share helpers but ship `match` first).
  - `docgarden skills match` / `skills list` — deferred until a configured `skills` scope exists.
  - JSON output surface (design doc explicitly excludes it for v1).
  - `--explain` field-level breakdown.
  - Full-text body matching.
  - Indexing, caching, embeddings, stemming.
  - Changes to the `Skill` / `Init` reserved stubs.

## Relevant Areas

- `src/cli.rs` — add `Match(MatchArgs)`, `struct MatchArgs`, dispatch in `run()`; `ColorChoice` already reusable.
- `src/lib.rs` — declare new `mod frontmatter;`, `mod matching;`, `mod score;`.
- `src/lint/rules/frontmatter.rs` — move `YamlValue`, `ParsedFrontmatter`, `FrontmatterParseResult`, `parse_from_str`, and the internal `parse_yaml_block` helpers into new top-level `src/frontmatter.rs`; replace with `pub(crate) use crate::frontmatter::*;` so `lint/rules/frontmatter.rs` (the rule evaluator) keeps its existing imports working. Do not modify lint-specific types (`FrontmatterRuleContext`, `evaluate_frontmatter_rules`).
- `src/discover.rs` — no changes; call `discover_markdown_files_for_targets` as-is.
- `src/root.rs`, `src/paths.rs`, `src/config.rs` — used as-is.
- `tests/cli.rs` — pattern from existing `--help` and discovery tests (lines 11-24) is the template for new match tests.
- `tests/common/mod.rs` — `fixture_repo(name)` is the existing helper for copying fixtures into tempdirs.
- `docs/design-docs/frontmatter-driven-discovery-commands.md` — source of truth for output shape, flag names, and scoring direction.
- `Cargo.toml` — no new runtime deps. IDF is hand-rolled on `HashMap` / `HashSet` using only `std` and `f32`.

## Resolved decisions

- **Scoring model**: tiered weighted lexical (exact / prefix / substring) + clamped corpus-local IDF. No fuzzy tier in v1. Hand-rolled; no scoring crate.
- **No positional targets** on `match`. Design doc never shows them; the command always runs against the full repo-root discovery set, which keeps the IDF corpus deterministic per config.
- **Query shape**: `Vec<String>` joined by a single space before tokenization. `docgarden match frontmatter discovery commands` works without quoting.
- **Empty / non-matching queries**: drop zero-score candidates from output; no matches → exit 0, no stdout lines.
- **Parser relocation**: promote pure parser types out of `src/lint/rules/frontmatter.rs` into a top-level `src/frontmatter.rs`.
- **Score display**: non-negative integer; help text describes it as "higher means a closer match; ordering is the contract, not the absolute value." No hard upper bound advertised.
- **Score color semantics**: when `--color` resolves to enabled and `--path-only` is not set, color only the `score` column using red/yellow/green bands for low/medium/high scores. Keep `--color never` fully uncolored and document the thresholds in help text.
- **Fixture strategy**: add a dedicated `tests/discovery-repo/` fixture so ranking assertions are isolated from lint fixtures.

## Open Questions

- IDF clamp bounds are proposed at `[0.5, 1.8]`. Revisit once unit tests over small (N≈5), medium (N≈50), and a synthetic large (N≈500) corpus are in place; adjust if a boundary feels wrong in practice.
- Whether to emit a stderr note on empty results or stay silent — decide during implementation; default silent.
- None yet

## v1 Scoring model

- **Normalize** both query and each candidate field:
  - lowercase
  - split on whitespace and ASCII punctuation
  - for `path`, additionally split on `/`, `_`, `-`, `.` and drop the `.md` extension token
  - collapse empty tokens
  - keep original string for display and for phrase-bonus substring check
- **Corpus-local IDF** (built once per invocation, after discovery):
  - `N` = number of discovered Markdown files
  - for each normalized token `t` appearing in any candidate field, `df(t)` = number of documents containing `t` in at least one scoring field
  - `raw_idf(t) = log((N + 1) / (df(t) + 1)) + 1` (additive smoothing avoids zero and negative values)
  - `idf(t) = clamp(raw_idf(t), 0.5, 1.8)` — lower bound keeps common terms contributing; upper bound stops single-occurrence terms from dominating tiny corpora
  - tokens in the query that never appear in the corpus get `idf = 1.0` (they still score via the substring tier on the original field text)
- **Per-field weights**: `name = 3`, `path = 2`, `description = 1`.
- **Per-(query-term × field) match-tier score** — take the best tier for each pair; do not sum tiers within one pair:
  - exact token match → 10
  - token-prefix match (field token starts with query term, query length ≥ 2) → 4
  - substring match on normalized field text → 1
  - otherwise → 0
- **Per-(query-term × field) contribution** = `tier_score × field_weight × idf(term)`.
- **Phrase bonus**: if a multi-term normalized query appears contiguously in `name` or in the `path` basename, add 25; in `description`, add 10. Applied once per field, not per term. Phrase bonus does not use IDF — it's a flat bump for literal-substring matches.
- **Final integer score** = round(sum of contributions) + phrase bonus.
- **Zero-score candidates are dropped** from results.
- **Tie-break** (strictly deterministic):
  1. more distinct query terms matched in any field
  2. hit in `name` before `path` before `description`
  3. lexicographic repo-relative path (ascending)
- **No fuzzy tier and no stemming in v1.** The substring tier already catches most near-miss shapes; if dogfooding shows typo-tolerance gaps, the design doc explicitly allows adding a fuzzy tier as the weakest match tier in a later revision.

IDF is safe here because the corpus is fully determined by `Config::load` + `discover_markdown_files_for_targets(repository_root)`. Without positional targets, the same invocation in the same repo state always produces the same IDF table, so score ordering stays stable across runs.

## Steps

- [x] Create `src/frontmatter.rs`; move `YamlValue`, `ParsedFrontmatter`, `FrontmatterParseResult`, `parse_from_str`, and the private `parse_yaml_block` helpers from `src/lint/rules/frontmatter.rs`; keep existing unit tests co-located with the moved code.
- [x] Add `mod frontmatter;` to `src/lib.rs`; in `src/lint/rules/frontmatter.rs`, replace the moved definitions with `pub(crate) use crate::frontmatter::{YamlValue, ParsedFrontmatter, FrontmatterParseResult, parse_from_str};` and verify lint still compiles and its inline tests still pass.
- [x] Create `src/score.rs` implementing the scorer. Public surface:
  - `struct Candidate<'a> { name: Option<&'a str>, description: Option<&'a str>, repo_relative_path: &'a str }`
  - `struct IdfTable { /* token -> clamped weight */ }` with `pub fn build(candidates: &[Candidate]) -> IdfTable` and `pub fn weight(&self, token: &str) -> f32` (returns 1.0 for unknown tokens).
  - `struct ScoredHit { score: i32, matched_terms: u32, first_field_hit: Option<Field> }`
  - `pub fn score(query_terms: &[String], candidate: &Candidate, idf: &IdfTable) -> ScoredHit`
  - normalization helpers (`normalize_text`, `normalize_path`) exposed `pub(crate)` for reuse by `matching` and tests.
  - Inline `#[cfg(test)] mod tests` covers: exact-over-prefix-over-substring ordering; field-weight priority (name beats description); phrase bonus; tie-break determinism; zero-score drop condition; empty-query handling; IDF raises score for rare terms and lowers it for ubiquitous ones; IDF clamp prevents runaway scores on 1-of-5 rarity and prevents zero contribution on 5-of-5 ubiquity; tests can pass a hand-built `IdfTable` (or one built via `IdfTable::build`) to isolate tier vs. IDF behavior.
- [x] Create `src/matching.rs` with `pub fn execute_match(args: MatchArgs) -> Result<()>`:
  - resolve `config_path`, `no_gitignore`, `color` the same way `execute_lint` does (factor the shared prelude if clean; otherwise duplicate for now and refactor in a follow-up)
  - infer repository root via `infer_repository_root(&[cwd], config_path, markers)`; pass `&[repository_root]` to `discover_markdown_files_for_targets`
  - join `args.query` with single spaces, then tokenize via the `score` normalizer — reject empty queries (after normalization) with a clear error
  - for each discovered file: `fs::read_to_string`, `frontmatter::parse_from_str`, extract `name` and `description` as scalar strings when present; collect into `Vec<(PathBuf, Candidate)>` with `repo_relative_path` via `crate::paths::repository_relative_path`
  - build `IdfTable` from the full candidate set; score every candidate; drop zero-score rows
  - sort by `(score desc, tie-break rules)`, apply `--limit` after sort
  - emit either `path-only` lines or `score | path | name | description` lines; for fields containing literal `|`, replace with `\|` (document in `--help`)
- [x] Add `Match(MatchArgs)` to `enum Command` in `src/cli.rs` with `visible_alias = "m"` and a clap `#[command(about = ..., long_about = ...)]` attribute. `about` is the one-line summary shown on `docgarden --help`; `long_about` (rendered on `docgarden match --help`) must document the output columns, the `--path-only` format, and describe the score as ordering-first ("higher means closer; ordering is the contract, not the absolute value"). Add `struct MatchArgs` with: `query: Vec<String>` (positional, `num_args = 1..`, required, `#[arg(help = "Query terms; joined with spaces before tokenization")]`), `config: Option<PathBuf>`, `no_gitignore: bool`, `color: ColorChoice`, `limit: Option<usize>` (`-n`, `--limit`), `path_only: bool` (`-p`, `--path-only`). No positional `targets`. Wire dispatch in `run()` to `matching::execute_match`.
- [x] Update the `match` CLI/help and rendering path so `--color` is honored consistently: when enabled, color the score column green/yellow/red for high/medium/low score bands, leave `--path-only` uncolored, and document the thresholds in `match --help`.
- [x] Add integration tests in `tests/cli.rs`:
  - `match --help` mentions `name`, `description`, `path`, `score`, `--limit`, `--path-only`, and the `m` alias
  - multi-token query (`docgarden match frontmatter discovery`) is accepted without quoting
  - a ranking test using the `discovery` fixture where `name` frontmatter match beats `description` match beats pure path match for the same query
  - IDF end-to-end check: in a fixture where one term is in every doc and another term is in only one, the single-occurrence term pulls its doc to the top even if the other term matches more files
  - `--limit 2` truncates to two results and preserves order
  - `--path-only` emits one repo-relative path per line, no score/name/description columns
  - `--no-gitignore` exposes a matched file inside a `.gitignore`-excluded dir
  - alias `docgarden m <QUERY>` behaves identically to `docgarden match <QUERY>`
  - no-matches case exits 0 and emits no lines (or a documented "no matches" stderr line — pick one during implementation)
  - `--color always` colors low/medium/high scores red/yellow/green in human-readable output, `--color never` disables those escapes, and `--path-only` never emits colored output
- [x] Add a dedicated fixture `tests/discovery-repo/` with a minimal `docgarden.toml`, a couple of docs with rich `name`/`description`, one doc with only `name`, one doc with no frontmatter (path-only discoverability), and one doc inside a gitignored subdir. `tests/common/mod.rs::fixture_repo` already loads `tests/<name>`, so no helper changes were needed.
- [x] Fix the remaining score/test mismatch around phrase bonus semantics, then rerun targeted and full tests.
- [x] Run full test + lint suite; iterate on scoring thresholds if dogfooding output against this repo's own docs and skills feels off.
- [x] Update `docs/design-docs/frontmatter-driven-discovery-commands.md` to reflect the v1 divergences: clamped corpus-local IDF in place of uncapped IDF; `match` takes no positional targets; no fuzzy tier in v1. Follow the doc's own working-draft voice.

## Validation

- `cargo build`
- `cargo test` — confirms unit tests (`score`, moved `frontmatter` tests) and all integration tests in `tests/cli.rs` pass.
- `cargo clippy -- -D warnings` if the repo's CI uses clippy (check `.github/workflows/` during implementation).
- `cargo run -- match --help` — manually verify column-order explanation and alias listing.
- `cargo run -- match discovery` from repo root — should rank this design doc highly on name/phrase match and show the skill docs via description matches.
- `cargo run -- m planner --limit 3 --path-only` — should return three skill paths, `planner-execplan` first.
- `cargo run -- match nonsense-xyz` — confirms empty-result handling (either silent exit 0 or a terse stderr note, matching what the step above finalizes).
- `cargo run -- match frontmatter --no-gitignore` vs without — confirms gitignore toggle reaches discovery the same way `lint` does.
- `cargo test match_color_always_and_never_control_only_score_column --test cli -- --nocapture`
- `cargo test match_path_only_never_emits_color_even_when_forced --test cli -- --nocapture`
- `cargo test first_field_hit_ --lib -- --nocapture`

## Discoveries

- `src/frontmatter.rs`, `src/score.rs`, `src/matching.rs`, the `match` CLI wiring, and the integration fixture/tests are already present in the worktree; the handoff interruption happened after most implementation landed.
- The discovery fixture lives at `tests/discovery-repo/`, not `tests/test-repos/discovery/`; this matches `tests/common/mod.rs::fixture_repo`, which copies `tests/<name>`.
- Current failing test indicates the intended phrase bonus semantics are "bonus only for multi-term contiguous queries"; single-term exact matches should rely on normal tier scoring only.
- `cargo run -- match discovery` ranks the design doc first and related discovery docs next, which is a reasonable smoke test for the shipped scorer on the live repo.
- `cargo run -- m planner --limit 3 --path-only` currently returns a single planner skill path, which is expected because only one discovered document in this repo clearly matches that token.
- `cargo run -- lint docs/exec-plans/active/implement-match-subcommand.md docs/design-docs/frontmatter-driven-discovery-commands.md` passes after adding required frontmatter to the active ExecPlan.
- `match --color` now uses fixed score bands documented in help text: `1-24` low/red, `25-59` medium/yellow, `60+` high/green; `--path-only` bypasses color even when forced.
- `first_field_hit` now records the best matched field across the full query, so tie-breaks stay stable even when a lower-priority field matches an earlier query term.

## Review

- [x] `src/score.rs` records `first_field_hit` from the first query term that matches instead of the highest-priority matched field, so equivalent queries can produce different tie-break outcomes depending on term order; update scoring/tests so tie-breaks always prefer `name` before `path` before `description`.
- [x] `src/cli.rs` exposes `match --color`, but the dispatch into `src/matching.rs` drops the parsed value and the command never uses it; either thread the flag through consistently or remove the unsupported option from the `match` surface.
