---
description: "Decision to adopt MADR 4.0.0 as the format for `docgarden` repository decision records; read when adding a new ADR, picking a template, or asking why `docgarden` standardizes on MADR rather than another ADR convention."
---

# Use MADR 4.0.0 for decision records

## Context and Problem Statement

We want to record decisions made in this project — architectural, operational, or otherwise — so the rationale survives independent of any single living document or contributor's memory.

A structured decision-record format helps authors write compact records that can also be parsed, indexed, scaffolded, and reviewed by repository tooling.

Which format and structure should these records follow?

## Considered Options

- [MADR](https://adr.github.io/madr/) 4.0.0 — Markdown Architectural Decision Records
- [Michael Nygard's template](http://thinkrelevance.com/blog/2011/11/15/documenting-architecture-decisions) — the original ADR convention
- [Sustainable Architectural Decisions](https://www.infoq.com/articles/sustainable-architectural-design-decisions) — Y-Statements
- Other templates listed at <https://github.com/joelparkerhenderson/architecture-decision-record>
- Formless — no conventions for file format and structure

## Decision Outcome

Chosen option: "MADR 4.0.0", because:

- MADR is structured enough to support parsing, indexing, scaffolding, and review, while still leaving room for repository-local conventions.
- MADR supports both compact and expanded records, so authors can start from a small required core and add optional rationale only when the decision needs it.
- Using the format in this repository keeps local ADR conventions grounded in real authoring and review workflows.
- MADR is part of the broader ADR ecosystem rather than a private `docgarden` convention.
- The format is lean and matches `docgarden`'s preference for compact, low-ceremony documentation.

