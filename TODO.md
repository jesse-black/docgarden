---
description: "Follow-up tasks and deferred cleanup items for `docgarden`; read when looking for small backlog work that is not part of an active ExecPlan."
---

# TODO

- Extend `cargo xtask sync-skills` to recursively sync support files under each source skill directory, and update `sync-skills --check` tests to catch missing or stale generated support files. The initial redistributable skill only has `SKILL.md`, so this is deferred until source skills need additional files.
- Consider format-aware validation for fragments on non-Markdown repository links, such as OpenAPI JSON Pointers in `openapi.yaml#/components/schemas/Foo` or renderer-specific source-line anchors like `src/lib.rs#L42`; current linting only verifies that the non-Markdown target file exists.
