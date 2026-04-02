---
name: import-reference
description: Retrieve an article or documentation page from a URL and save it into `docs/references/` with the repository's standard reference front matter. Use when a user asks to capture, import, archive, summarize, or store externally sourced material from the web in the repository knowledge base.
---

# Import Reference

Store externally sourced material in `docs/references/` as a concise local Markdown reference with clear provenance.

## Workflow

### 1. Fetch the source

Open the URL and read enough of the source to capture the title, author if available, license if clearly stated, and the substance needed for a faithful local reference.

Prefer the canonical article or documentation page over mirrors, excerpts, or aggregator pages.

### 2. Create the front matter

Use this front matter shape:

    ---
    title: <source title>
    source: <canonical URL>
    description: <brief explanation of why this reference matters in this repository>
    retrieved: <YYYY-MM-DD>
    last_reviewed: <YYYY-MM-DD>
    author: <author or publishing organization if known>
    license: <license if clearly stated and relevant>
    ---

Required fields:

- `title`
- `source`
- `description`
- `retrieved`
- `last_reviewed`

Optional fields:

- `author`
- `license`

Set `retrieved` and `last_reviewed` to the date the source was accessed for this ingest unless the user asks for a different convention.

### 3. Capture the content

After the front matter, capture the external content into Markdown with minimal transformation.

The goal is to preserve the source, not to rewrite it into a new repo-authored document.

Prefer:

1. the source title as the front matter `title`
2. the canonical URL as `source`
3. the extracted article or page body as Markdown below the front matter

Preserve headings, lists, and paragraphs where practical. Remove obvious navigation chrome, cookie notices, footers, and unrelated page furniture when you can do so confidently.

Do not add a summary section, `## Key Points`, `## Notes`, or other repo-authored interpretation unless the user explicitly asks for it. Only add the front matter and the minimally cleaned captured content.

### 4. Choose the filename

Save the file under `docs/references/` with a short kebab-case name derived from the source title.

Good examples:

    docs/references/codex-observability-stack.md
    docs/references/agent-skills-specification.md
    docs/references/copilot-custom-instructions.md

Avoid dates in filenames unless they are necessary to distinguish versions of the same source.

### 5. Finish cleanly

Run `docgarden` on the new file before finishing so repository-local path references and formatting issues are caught locally.

## Quality Bar

A good ingested reference should:

- preserve clear provenance back to one canonical external source
- keep the body source-derived rather than agent-authored
- be short enough for progressive disclosure
- avoid pretending to be first-party repository policy
- remove obvious non-content page noise when that can be done confidently
- remain useful even if the external source later changes

## Output Template

    ---
    title: Example Title
    source: https://example.com/article
    description: Explains why the external source is relevant to this repository.
    retrieved: 2026-04-02
    last_reviewed: 2026-04-02
    author: Example Author
    ---

    # Example Title

    Captured article or documentation body in Markdown, cleaned up only enough to remove obvious page chrome and preserve the readable content.
