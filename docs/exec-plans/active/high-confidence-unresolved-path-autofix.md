# Add High-Confidence Unresolved Path Autofix

Save this in-progress ExecPlan at `docs/exec-plans/active/high-confidence-unresolved-path-autofix.md`. Move it into the completed ExecPlan directory when the work is complete.

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds. Maintain this document in accordance with `docs/PLANS.md`.

## Purpose / Big Picture

Users can already see when a repository-local reference is broken, but today `docgarden fix` refuses to help even when the intended target can be discovered safely by a simple search of the repository tree. After this change, `docgarden fix` will repair a narrow, high-confidence subset of `unresolved-local-path` errors automatically. The first visible example is a broken reference such as `[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)` in `README.md` when the only matching file in the repository is `ARCHITECTURE.md` at the root. A user should be able to run `docgarden fix README.md`, see the path rewritten to the unique matching file, and then rerun `docgarden lint README.md` with no remaining unresolved-path error for that reference.

The scope of this plan is intentionally small. The autofix must trigger only when a simple repository search yields exactly one credible target. If the search is ambiguous or finds nothing, `docgarden` must continue reporting a normal non-fixable `unresolved-local-path` error.

## Progress

- [x] (2026-03-13 00:00Z) Authored the initial ExecPlan in `docs/exec-plans/active/high-confidence-unresolved-path-autofix.md`.
- [ ] Add failing tests that reproduce a uniquely repairable unresolved local path in both backtick and Markdown-link forms.
- [ ] Implement search-based candidate discovery and a strict confidence policy for fixable unresolved local paths.
- [ ] Wire the new high-confidence repair path into the `fix` subcommand without regressing current style-fix behavior or unrelated formatting preservation.
- [ ] Validate the implementation with the full Rust verification stack and record the outcomes in this plan.

## Surprises & Discoveries

- Observation: the old autofix path used to rewrite whole Markdown files by serializing the full abstract syntax tree, which caused unrelated formatting churn. The repository now applies targeted source-span edits instead.
  Evidence: `tests/cli.rs` contains `fix_preserves_unrelated_readme_formatting`, which proves unrelated text stays byte-stable during autofix.

- Observation: Markdown links must be linted as one unit. Descending into a link label and linting an inline-code child separately caused duplicate unresolved-path diagnostics.
  Evidence: `tests/path_behavior.rs` contains `ignored_style_rule_in_readme_still_lints_backticked_link_as_one_link`, which was added to lock in link-as-unit behavior.

## Decision Log

- Decision: Limit v1 of this feature to exact repository-tree search with a single unique match and do not attempt fuzzy search or heuristic renames.
  Rationale: The request is specifically about errors that can be resolved with a simple search of the repository tree. Exact unique matching preserves the tool’s deterministic and safe autofix contract.
  Date/Author: 2026-03-13 / Codex

- Decision: Keep the diagnostic rule identifier as `unresolved-local-path` and make only a high-confidence subset fixable.
  Rationale: The user-visible problem is still an unresolved repository-local reference. Introducing a second rule would complicate the CLI contract and JSON output before the repair policy is proven.
  Date/Author: 2026-03-13 / Codex

- Decision: Search should begin from the reference’s basename, then render the repaired destination relative to the current file using the existing path-rendering helpers.
  Rationale: A broken reference that mentions a nonexistent nested path often intends a file with the same basename elsewhere in the repository. Reusing existing destination rendering keeps rewritten links and backticks consistent with current path normalization rules.
  Date/Author: 2026-03-13 / Codex

## Outcomes & Retrospective

No implementation work has been completed yet. The expected outcome is that a uniquely discoverable broken path becomes fixable under `docgarden fix`, while ambiguous unresolved paths remain normal errors with no rewrite.

## Context and Orientation

`docgarden` is a Rust command-line linter in `src/` that parses Markdown files, classifies repository-local references, reports diagnostics, and applies deterministic autofixes. The core lint traversal lives in `src/lint/mod.rs`. Reference classification, path resolution, label extraction, and relative-path rendering live in `src/lint/references.rs`. Configuration loading for `docgarden.toml`, including per-file ignores and style settings, lives in `src/config.rs`. Human-readable and JSON diagnostics use the shared `Diagnostic` structure defined in `src/diagnostics.rs`.

An unresolved local path is currently reported when `src/lint/mod.rs` sees a backticked path or Markdown link destination that classifies as repository-local but does not exist on disk after normalization. These diagnostics are not fixable today. Safe autofix already exists for style-only rewrites such as converting a same-label Markdown link to backticks or converting a backticked path to link form. Those rewrites now operate through source-span edits, not full Markdown reserialization, so any new unresolved-path autofix must use the same edit pipeline.

Integration coverage is spread across `tests/cli.rs`, which verifies CLI-level fix behavior, and `tests/path_behavior.rs`, which verifies classification and resolution edge cases. Small checked-in fixture repositories live under `tests/test-repos/`, but some path behaviors are easier to express with temporary repositories created directly inside tests. This feature will need both kinds of tests: a focused temporary-repo test for unique-match resolution and a CLI-level test that proves `docgarden fix` rewrites the file and a second lint pass succeeds.

The term “high-confidence” in this plan means the repair can be explained mechanically without guessing intent. In concrete terms for v1, the broken reference is eligible for autofix only when searching the repository tree yields exactly one file or directory with the same basename as the unresolved target, after applying the same repository root, include, and exclude context that normal linting already uses. If the basename appears in two or more places, the result is ambiguous and must remain a plain `unresolved-local-path` error.

## Plan of Work

Start with tests. In `tests/path_behavior.rs`, add a failing test that creates a temporary repository with a broken reference and a real target file `ARCHITECTURE.md` at the repository root. Use indented examples like the following inside the test input setup:

    `docs/ARCHITECTURE.md`
    [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)

Run `docgarden` in check-only mode and assert that the unresolved diagnostic is marked fixable only when the unique-match policy applies. Add a companion test where the same basename exists in two places and assert that the diagnostic remains non-fixable and no autofix is offered.

Then add CLI-level proof in `tests/cli.rs`. Use a temporary repository or extend a fixture so `docgarden fix` rewrites the broken reference to the discovered path and preserves unrelated formatting. Follow the repository’s TDD policy: the new test must fail before code changes and pass after. Also add a second lint pass assertion to prove idempotence and confirm the repaired reference no longer produces `unresolved-local-path`.

Implement candidate discovery in `src/lint/references.rs` or a nearby helper module if that keeps responsibilities clearer. The helper should accept the repository root, the referencing file path, and the unresolved candidate. It should search the repository tree for entries with the same basename as the unresolved target. Keep the first implementation simple and deterministic: exact basename match only, no fuzzy match, no extension swapping, no case folding beyond the platform’s own filesystem behavior. Return a structured result such as “no candidate”, “one candidate with repo-relative path”, or “ambiguous candidates”.

After candidate discovery exists, update `src/lint/mod.rs` so both unresolved backtick paths and unresolved Markdown links can ask for a repair candidate before emitting the diagnostic. If there is exactly one candidate, keep the rule id as `unresolved-local-path` but mark the diagnostic `fixable: true`. In fix mode, compute the replacement text using the existing rendering helpers. A backticked unresolved path should be rewritten to a backticked resolved path in backtick-style mode, or to a Markdown link if the current style logic already requires link form. An unresolved Markdown link should be rewritten by updating only its destination for meaningful-label links, and by preserving the current link-vs-backtick policy for same-label links.

Be strict about safety. Do not autofix a broken link label to a different label just because the discovered destination has a different filename. Do not autofix when the unresolved reference points at a directory-like path ending with `/` unless the unique discovered match is also a directory and the trailing slash can be preserved canonically. Do not search hidden tool state such as `target/`, `.git/`, or paths already excluded by repository configuration if normal linting would not treat them as repository knowledge targets.

Finally, update any user-facing text or plan artifacts needed to explain the new behavior. If the diagnostic message changes when a repair is available, keep it concise and deterministic. If no user-facing text changes are needed, record that decision in `Decision Log` instead of silently omitting it.

## Concrete Steps

Work from the repository root.

1. Add the failing tests first.

    cargo test unique_unresolved_path

Expected result before the implementation: the new test fails because `unresolved-local-path` is still non-fixable or `docgarden fix` does not rewrite the broken reference.

2. Implement the search helper and wire it into unresolved-path handling.

    cargo fmt
    cargo check

Expected result: the project builds, and the new tests are the only remaining source of truth for whether the behavior is correct.

3. Run the focused tests while iterating.

    cargo test unique_unresolved_path -- --nocapture
    cargo test fix_rewrites_uniquely_resolved_unresolved_path -- --nocapture

Expected result after implementation: the focused tests pass, showing a uniquely discoverable unresolved path is repaired and ambiguous cases are left alone.

4. Run the full verification stack required by this repository.

    cargo fmt
    cargo check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test
    cargo llvm-cov --summary-only

Expected result: every command succeeds and coverage stays at or above 80 percent for the repository.

5. Dogfood the behavior against this repository’s `README.md` example if the broken path still exists.

    cargo run -- fix README.md --color never
    cargo run -- lint README.md --color never

Expected result: the first command rewrites the uniquely discoverable broken path if the repository still contains that example, and the second command no longer reports the repaired unresolved-path error.

6. Lint the updated documentation before finishing.

    cargo run -- docs/exec-plans/active/high-confidence-unresolved-path-autofix.md --color never

Expected result: no documentation-path or style-policy errors for the ExecPlan itself.

## Validation and Acceptance

Acceptance is behavioral and must be proven in three layers.

First, a unit-level or focused integration test must show that a broken repository-local reference with exactly one basename match elsewhere in the repository becomes fixable. The test should assert the diagnostic still uses `unresolved-local-path`, but now includes the `fixable` marker in human-readable output or `fixable: true` in JSON.

Second, a CLI autofix test must prove end-to-end rewriting. After running `docgarden fix <target>`, the target Markdown file should contain the discovered resolved path, unrelated formatting must remain unchanged, and a second `docgarden lint <target>` invocation should succeed.

Third, an ambiguity test must prove the safety boundary. If the repository contains more than one basename match for the unresolved target, `docgarden` must not autofix. The diagnostic must remain an error, must not be marked fixable, and check-only mode must not advertise a repair for that specific case.

When dogfooding on this repository, a broken reference such as `[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)` in `README.md` should repair to either `[ARCHITECTURE.md](ARCHITECTURE.md)` or a style-policy-equivalent representation that resolves correctly from `README.md`. The repaired output must match the configured style and current link-label preservation rules.

## Idempotence and Recovery

Every step in this plan should be safe to repeat. The tests create temporary repositories and should leave no persistent state outside `target/`. `cargo fmt` and `cargo check` are naturally repeatable. `docgarden fix` must remain idempotent: once the unresolved path is repaired, rerunning the same command should produce no additional edits.

If a search-based autofix begins rewriting the wrong target during implementation, stop and tighten the confidence policy rather than broadening test allowances. The safe fallback is always to leave the diagnostic non-fixable. If a work-in-progress implementation corrupts a checked-in fixture, restore only the intended fixture content manually and rerun the focused tests before proceeding.

## Artifacts and Notes

Expected human-readable diagnostic shape when a unique repair is available in check-only mode:

    README.md:100:3  error  unresolved-local-path  fixable
    Local repository link `[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)` does not resolve within the repository.

    1 fixable violation found.
    Fixable rules in this run: unresolved-local-path
    Run `docgarden fix README.md` to apply fixes.

Expected ambiguity example:

    README.md:12:5  error  unresolved-local-path
    Local repository path `docs/ARCHITECTURE.md` does not resolve within the repository.

Expected repaired content example for a same-directory file:

    Before: * [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
    After:  * [ARCHITECTURE.md](ARCHITECTURE.md)

## Interfaces and Dependencies

Keep the implementation within the current Rust crate. Use the existing `markdown` parser and current lint traversal in `src/lint/mod.rs`; do not introduce a second Markdown parser. Reuse helper functions in `src/lint/references.rs` such as `resolve_candidate`, `render_repo_relative`, and `render_link_destination` wherever possible so the new repair path stays consistent with existing path normalization and relative rendering rules.

By the end of this work, the crate should have a helper with a stable, testable surface that can be called from unresolved-path handling. One acceptable shape is:

    pub(crate) enum RepairCandidate {
        None,
        Unique(ResolvedReference),
        Ambiguous(Vec<ResolvedReference>),
    }

    pub(crate) fn search_repair_candidate(
        repository_root: &Path,
        file: &str,
        candidate: &CandidateReference,
        kind: ReferenceKind,
    ) -> RepairCandidate

The exact function name may change, but the implementation must expose enough structured information for `src/lint/mod.rs` to decide whether the diagnostic is fixable and what replacement text to render. Any helper that walks the repository tree must document how it honors repository exclusions and why that behavior is safe.

Revision note: Created this plan to scope a new autofixable subset of `unresolved-local-path` for cases where a simple repository-tree search yields exactly one safe repair candidate.
