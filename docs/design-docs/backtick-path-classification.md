# Backtick Path Classification

## Purpose

This document defines how `docgarden` should interpret backticked text when deciding whether it is a repository-local file or directory reference, ambiguous inline code, or ordinary code text that should not trigger path diagnostics. The goal is to keep path linting useful during dogfooding without turning example-heavy documentation into false-positive noise.

## Scope

This policy applies to backticked text in Markdown prose, such as `` `docs/PLANS.md` `` or `` `./src` ``. It does not define Markdown link destination handling, code block handling, or external URL detection except where those systems affect the boundary between "real local path" and "ambiguous inline code".

Inline backticks and Markdown links should be treated as separate lint paths in the implementation. They are not two presentations of the same problem:

- inline backticks may denote either repository paths or ordinary code fragments
- Markdown links are explicit link destinations and should be checked as links

That separation matters because filtering that is reasonable for backticks can be incorrect for links. For example, a glob-shaped token like `` `docs/**/*.md` `` is often a pattern example in inline code and should not become a path diagnostic by default, while `[docs glob](docs/**/*.md)` is still a broken local link target and should be reported as such.

## Why Backticks Need Their Own Policy

Backticks are overloaded in Markdown. Authors use them for repository paths, command names, config keys, module identifiers, crate names, globs, and examples copied from other repositories. A linter that treats every slash-containing backtick as a filesystem path will generate too many false positives to be trusted. A linter that treats almost nothing as a path will miss real broken references.

The classifier therefore needs a narrow definition of "strong path signal" for backticks. The user-written syntax matters. We should not normalize away the very markers that tell us the text is meant to be a path.

Markdown links do not have that ambiguity. Once an author writes a local Markdown link destination, the tool should evaluate whether the destination is a valid local link target under link rules rather than reusing the backtick classifier’s ambiguity heuristics.

## Core Classification Rule

A backticked token should be treated as a definite local path only when it carries a strong path signal. In v1, the strong path signals are:

- a leading `/`, such as `` `/docs/PLANS.md` ``
- a known file extension, such as `` `src/main.rs` ``
- a trailing slash, such as `` `docs/` ``
- a leading `./`, such as `` `./README.md` ``
- a leading `../`, such as `` `../shared/spec.md` ``

If a backticked token does not have one of those signals, it should not become a hard `unresolved-local-path` error by default.

The classifier should also reject a small v1 set of obviously non-path backtick forms before attempting path resolution. In v1 that rejection set includes:

- whitespace
- `//` anywhere in the token
- glob metacharacters such as `*`, `?`, `[`, and `{`
- code punctuation such as `(`, `)`, `<`, `>`, `"`, and `'`
- `:`, because `docgarden` is aimed at workspace-relative sandboxes and portable Git repositories rather than drive-letter path syntax

## Directory References

Trailing slashes on directory references are meaningful and should be preserved. They help both humans and the classifier understand that the author is naming a directory rather than an abstract label.

Examples that should be treated as valid directory references:

    `docs/`
    `src/`
    `docs/exec-plans/active/`

These should not be rewritten to forms without the trailing slash simply for normalization.

These references are still classified as directory-like paths, but a missing trailing-slash directory does not become a hard `unresolved-local-path` error. This keeps lifecycle-dependent directories such as execution-plan staging areas from requiring `.gitkeep` placeholders purely for lint hygiene.

## Relative Markers

Leading `/`, `./`, and `../` are also meaningful syntax, not noise. They help disambiguate paths from ordinary inline code and should be preserved by default.

Examples that should remain valid path references:

    `/docs/PLANS.md`
    `./README.md`
    `./docs/`
    `../shared/guide.md`

The classifier should use the written form to decide whether the token is path-shaped. Resolution can use a normalized internal representation after classification, but the displayed and stored text should not be rewritten just because normalization is possible.

## Bare Slash-Separated Tokens

Bare slash-separated tokens without an extension, trailing slash, or relative marker are too ambiguous to treat as definite filesystem paths in backticks.

Examples:

    `crates/parser`
    `libs/core`
    `docs/generated`

These forms often represent conceptual crate names, module names, example paths from another repository, or shorthand labels. In v1 they should not become path diagnostics by default. Repositories that want extra review signal may opt into reporting them as `ambiguous-inline-code`, but that check should be off by default because it is too noisy for dogfooding and example-heavy docs.

If an author wants these to be treated as real local paths, they should add a stronger signal:

    `crates/parser/`
    `./crates/parser`
    `docs/generated/`

## Classification Versus Resolution

The implementation should keep three concepts separate:

1. Classification form: the exact text the author wrote, used to decide whether the token is path-shaped.
2. Resolution form: an internal normalized form used to check whether the referenced target exists.
3. Display form: the user-visible form preserved in diagnostics and autofixes unless repository policy explicitly requires a rewrite.

This separation prevents the classifier from erasing evidence before it makes the path-versus-code decision.

## Display Preservation Policy

The linter should preserve the author-written display form for valid backticked paths. A resolvable path should not trigger a separate style diagnostic merely because it contains:

- a trailing slash on an existing directory
- a leading `./`
- a leading `../`

Those forms are valid path syntax and strong signals for classification. Internal normalization is useful for resolution, but it should not become a user-visible lint rule unless a repository explicitly opts into such a policy in the future.

## Boundary With Markdown Links

Backticks and Markdown links should be implemented as separate lint paths.

Backticks need ambiguity handling because they may denote either repository paths or ordinary code fragments. Markdown links do not have that ambiguity. Once an author writes a local Markdown link destination, the tool should validate it as a link target rather than reuse the backtick classifier’s heuristics.

This separation affects policy:

- backticks preserve strong path markers such as `/`, `./`, `../`, and trailing `/`
- backticks may ignore glob-shaped examples like `` `docs/**/*.md` ``
- Markdown links follow VS Code Markdown navigation semantics
- Markdown links remain enforceable link destinations, so `[docs glob](docs/**/*.md)` is still a broken local link target

For Markdown links, `docgarden` should follow the same basic resolution model that VS Code uses in Markdown editors:

- a destination starting with `/` is workspace-root-relative
- a destination not starting with `/` is relative to the current document directory

That rule should be called out explicitly in future link-focused design docs as well. It gives authors a familiar mental model and aligns lint behavior with Ctrl+click navigation in common editor workflows.

## Examples

These forms should classify as definite local paths in backticks:

    `docs/PLANS.md`
    `./README.md`
    `../shared/spec.md`
    `docs/`
    `src/bin/`

These forms should classify as ambiguous inline code by default:

    `crates/parser`
    `docs/generated`
    `config/local`

These forms should not be classified as local paths:

    `cargo fmt`
    `local-reference-style`
    `RuleId::AmbiguousInlineCode`

## Implications For Dogfooding

This policy reduces false positives in repository knowledge docs, execution plans, and architecture discussions where slash-separated identifiers often appear as examples or conceptual names. It also gives authors a simple way to force path intent when they want linting to apply: add an extension, a trailing slash, or an explicit relative marker.

## Open Questions

- Whether opt-in `ambiguous-inline-code` should eventually gain narrower heuristics so it becomes useful enough for broader default use.
- Whether repositories should eventually get a narrow allowlist for additional strong path signals in backticks.
- Whether directory references should prefer trailing slash in examples and docs, or merely accept it.
