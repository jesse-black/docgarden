## We made repository knowledge the system of record

Context management is one of the biggest challenges in making agents effective at large and complex tasks. One of the earliest lessons we learned was simple: **give Codex a map, not a 1,000-page instruction manual.**

We tried the “one big [`AGENTS.md`⁠](https://agents.md/)” approach. It failed in predictable ways:

- **Context is a scarce resource.** A giant instruction file crowds out the task, the code, and the relevant docs—so the agent either misses key constraints or starts optimizing for the wrong ones.
- **Too much guidance becomes** **_non-guidance_****.** When everything is “important,” nothing is. Agents end up pattern-matching locally instead of navigating intentionally.
- **It rots instantly.** A monolithic manual turns into a graveyard of stale rules. Agents can’t tell what’s still true, humans stop maintaining it, and the file quietly becomes an attractive nuisance.
- **It’s hard to verify.** A single blob doesn’t lend itself to mechanical checks (coverage, freshness, ownership, cross-links), so drift is inevitable.

So instead of treating `AGENTS.md` as the encyclopedia, we treat it as **the table of contents**.

The repository’s knowledge base lives in a structured `docs/` directory treated as the system of record. A short `AGENTS.md` (roughly 100 lines) is injected into context and serves primarily as a map, with pointers to deeper sources of truth elsewhere.

```
AGENTS.md
ARCHITECTURE.md
docs/
├── design-docs/
│   ├── index.md
│   ├── core-beliefs.md
│   └── ...
├── exec-plans/
│   ├── active/
│   ├── completed/
│   └── tech-debt-tracker.md
├── generated/
│   └── db-schema.md
├── product-specs/
│   ├── index.md
│   ├── new-user-onboarding.md
│   └── ...
├── references/
│   ├── design-system-reference-llms.txt
│   ├── nixpacks-llms.txt
│   ├── uv-llms.txt
│   └── ...
├── DESIGN.md
├── FRONTEND.md
├── PLANS.md
├── PRODUCT_SENSE.md
├── QUALITY_SCORE.md
├── RELIABILITY.md
└── SECURITY.md
```
In-repository knowledge store layout.

Design documentation is catalogued and indexed, including verification status and a set of core beliefs that define agent-first operating principles. [Architecture documentation⁠](architecture-md.md) provides a top-level map of domains and package layering. A quality document grades each product domain and architectural layer, tracking gaps over time.

Plans are treated as first-class artifacts. Ephemeral lightweight plans are used for small changes, while complex work is captured in [execution plans⁠](using-plans-md.md) with progress and decision logs that are checked into the repository. Active plans, completed plans, and known technical debt are all versioned and co-located, allowing agents to operate without relying on external context.

This enables **progressive disclosure**: agents start with a small, stable entry point and are taught where to look next, rather than being overwhelmed up front.

We enforce this mechanically. Dedicated linters and CI jobs validate that the knowledge base is up to date, cross-linked, and structured correctly. A recurring “doc-gardening” agent scans for stale or obsolete documentation that does not reflect the real code behavior and opens fix-up pull requests.
