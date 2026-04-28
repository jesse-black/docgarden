---
description: "Follow-up tasks and cleanup items; read when looking for deferred implementation work, small backlog items, or candidate topics to promote into a future plan."
---

# TODO

This file tracks follow-up tasks and cleanups that came up during planning but are not currently part of an active exec plan.

## Modularize `src/config.rs`

`src/config.rs` holds three distinct concerns that have grown together: TOML deserialization structs, rule-lowering logic (`lower_rules`, `rule_entry_matches`, per-family lowering branches), and per-path policy query methods on `Config`. It is navigable now but will become unwieldy as rule families are added.

**Trigger:** when a second rule family (e.g. headings, links) adds a lowering block and the file grows another ~80 lines, or when a new contributor has visible trouble orienting in the file. At that point extract into `` `src/config/parse.rs` `` (structs + deserialization), `` `src/config/lower.rs` `` (lowering + `rule_entry_matches`), and a thin `` `src/config/mod.rs` `` public surface with `Config` and the query methods. The frontmatter-specific structs and lowering can move to `` `src/config/frontmatter.rs` `` at the same time.
