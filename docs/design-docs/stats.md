---
description: "Working design draft for the `docgarden stats` reporting command; read when designing informational file measurements, repository-wide size reporting, or human-readable summaries of line and token budgets."
---

# Stats

## Purpose

This document is a working design draft for the `docgarden stats` reporting command.

The goal is to give maintainers an informational view of measurable document size signals before or alongside budget enforcement. The command should stay narrow, mechanical, and aligned with agent-oriented repository workflows.

## Relationship To Budget Enforcement

`docgarden stats` is the reporting companion to line-count and token-budget enforcement, but it is not itself an enforcement command.

It should report the same two measures used by context-budget rules:

- line count
- token count

It should reuse the same Markdown file discovery path as `docgarden lint` so a target such as `docs/` means the same thing in both commands. It should also use the same tokenizer decision as budget enforcement, currently `o200k_base` through `tiktoken-rs`, so the reported token counts match `max-tokens` diagnostics.

## Command Shape

The working command shape is:

    docgarden stats <targets>

The command should be informational by default. Exceeding a configured limit should not make `docgarden stats` fail; `docgarden lint` remains the enforcement command. A future option such as `--fail-over-budget` may be useful for scripts, but it should not be required for the first version because it would duplicate existing lint behavior.

For directory targets, `stats` should support:

- `-R, --recurse`

With `--recurse`, the command should descend into nested directories under each target. Without it, directory targets should be limited to Markdown files directly within the named directory. The flag only changes target expansion; ignore rules and Markdown-file filtering should still match the shared discovery behavior used elsewhere in `docgarden`.

Recursive output should stay flat. `stats -R` should report one file per row using the normal table format, not per-directory sections, nested tree rendering, or subtotal rows.

## Default Output

The default human output should be a compact table:

    path                                      lines  tokens
    AGENTS.md                                    25     921
    docs/design-docs/line-and-token-limits.md  184    2360

If the loaded configuration supplies `max-lines` or `max-tokens` for a reported file, the command may include those effective limits as additional columns:

    path              lines  max-lines  tokens  max-tokens
    AGENTS.md            25        150     921        1200

The command should stay text-first in v1 and should not add a `--json` output mode just for speculative automation. Repository-gardening workflows are likely to be agent-mediated, and compact human-readable output is a better default fit for that use case. If a concrete non-agent automation case appears later, structured output can be reconsidered then.

## Filters

Filter switches would make the command more useful for exploratory cleanup:

    docgarden stats docs/ --min-tokens 5000
    docgarden stats docs/ --min-lines 300

These filters should include only files whose observed count is at least the supplied lower bound. They are ad hoc reporting thresholds, not configuration rules, so they should not be written back into `docgarden.toml` or treated as diagnostics. If both filters are present, the command should probably use OR semantics so users can ask for "files that exceed either of these size signals" in one pass:

    docgarden stats docs/ --min-tokens 5000 --min-lines 300

If users need AND semantics later, add an explicit option rather than making the default harder to predict.

## Scope

The first implementation should stay narrow. It exists to expose the same measurable document-size signals already used by budget rules, not to become a general document analytics surface.

Other possible measures such as file size or section count are lower priority and should not be assumed yet.
