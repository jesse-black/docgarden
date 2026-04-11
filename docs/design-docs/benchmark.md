---
description: "Working design draft for a benchmark that measures whether repository knowledge systems and `docgarden` reduce tokens needed for agent task success; read when planning evaluation methodology, task sources, or token-efficiency metrics."
---

# Benchmark

## Purpose

This document is a working design draft for a benchmark that measures whether repository knowledge systems and `docgarden` reduce the input tokens an agent needs to complete real repository tasks.

The benchmark should not begin from one fixed external benchmark assumption. Instead, it should define the evaluation shape first and then determine which task sources are suitable for that purpose. [SWE-bench Verified](https://www.swebench.com/verified.html) is one candidate task source under consideration, but it is not yet the committed foundation of the benchmark.

## Core Benchmark Question

For the same repository task, same user-visible goal, same model, and same agent scaffold:

- how many input tokens are required in a raw repository
- how many input tokens are required in a repository with a structured knowledge base
- how many input tokens are required in a repository with a structured knowledge base plus `docgarden`

The desired headline metric is `tokens_to_success`.

Additional supporting metrics should include:

- task success rate
- success rate under fixed token budgets
- files opened
- estimated file tokens loaded
- wall-clock time to success

## Why Group Tasks By Repository

The expensive part of this benchmark is not sourcing tasks. It is constructing a realistic repository knowledge system and then tuning repository policy so `docgarden` can reinforce it mechanically.

For that reason, the benchmark should prefer selecting multiple tasks from the same repository rather than spreading early experiments across many unrelated repositories.

This design has several advantages:

- one repository-knowledge design investment can be reused across multiple tasks
- the benchmark can measure whether the knowledge investment compounds across tasks
- per-repository benchmark stories are easier to explain than one-off cherry-picked tasks
- repository-specific knowledge structures can be made more realistic and less toy-like

The current direction should be:

- pick a small set of benchmark candidate repositories
- select multiple task instances per repository
- build one benchmark recipe per repository family
- run the same issue set against all repository variants

## Task Selection Strategy

The first benchmark version should be selective rather than exhaustive.

Ideal task clusters are repositories where:

- multiple benchmark-worthy tasks exist for the same repo
- repository navigation is non-trivial
- architectural or product context plausibly helps the agent
- documentation can be added without making the task unrealistic
- objective grading is available or can be built credibly

Tasks should be avoided or deprioritized when:

- the issue is extremely local and requires almost no repo understanding
- the task is dominated by framework trivia or one-file bug fixing
- a realistic knowledge base would not materially change navigation or planning

The benchmark should be honest about this curation. The point is not to claim that every software issue benefits equally from a repository knowledge system. The point is to measure the class of tasks where agent legibility should matter.

## Candidate Task Sources

The benchmark should remain open to multiple task sources while the benchmark-design work is still exploratory.

Plausible task sources include:

- existing public benchmark datasets with repository snapshots and objective grading
- repository-specific benchmark suites created for this project
- hybrid benchmark sets that combine external tasks with repo-legibility-first tasks

### SWE-bench Verified As A Candidate Source

SWE-bench Verified is an especially attractive candidate source because it already provides several expensive benchmark ingredients:

- real repository snapshots
- real issue descriptions
- objective success criteria via hidden tests
- a public, recognizable benchmark source

Relevant official sources:

- [SWE-bench Verified overview](https://www.swebench.com/verified.html)
- [SWE-bench GitHub repository and evaluation harness](https://github.com/SWE-bench/SWE-bench)
- [SWE-bench dataset guide](https://www.swebench.com/SWE-bench/guides/datasets/)
- [SWE-bench Verified dataset on Hugging Face](https://huggingface.co/datasets/SWE-bench/SWE-bench_Verified)

SWE-bench Verified is still only one candidate source. The benchmark should evaluate whether it is suitable for measuring the impact of repository knowledge systems rather than assuming that suitability in advance.

## SWE-bench Verified Suitability Questions

The benchmark should explicitly investigate whether SWE-bench Verified structurally understates the value of repository knowledge systems for agent workflows.

The most important concern is that SWE-bench Verified states that human annotators reviewed each instance to ensure the task is solvable with the available information. That is a strength for correctness benchmarking, but it may also select for tasks where enough relevant context is already front-loaded into the issue description that the repository itself has less opportunity to serve as the agent's map.

This is currently a hypothesis, not a settled conclusion.

The benchmark-design work should evaluate questions such as:

- How much architectural or navigational information is commonly present in SWE-bench Verified issue descriptions?
- Does that front-loaded issue context reduce the measurable effect size of repository knowledge systems?
- Are some SWE-bench repositories or task families still good fits because the agent must do substantial repo discovery anyway?
- Should SWE-bench-derived tasks be a secondary validation track rather than the primary benchmark track?

The benchmark should not treat this concern as a reason to reject SWE-bench Verified immediately. It should treat it as a dataset-suitability question to investigate empirically.

## Repository Variants

Each selected task should be evaluated against three repository states.

### Variant 1: Raw Repo

This is the benchmark repository at the task's `base_commit` with no benchmark-added knowledge layer.

Small mechanical setup files that are required to run the task harness are acceptable, but the repository should otherwise remain close to the original benchmark state.

### Variant 2: Knowledge Repo

This variant adds a realistic repository knowledge system intended to improve progressive discovery.

Typical additions may include:

- a short `AGENTS.md` that acts as a map rather than an encyclopedia
- an `ARCHITECTURE.md` that gives a stable code map
- a structured `docs/` tree with focused documents
- optional front matter for discovery and routing
- clear documentation boundaries between source-derived and repo-authored docs

This design direction is aligned with the repository-knowledge and LLM-wiki references already captured in:

- `docs/references/harness-engineering.md`
- `docs/references/llm-wiki.md`
- `docs/references/agent-skills-specification.md`

The benchmark should treat the knowledge repo as the "agent-legible repository" condition even before `docgarden` is introduced.

### Variant 3: Knowledge Repo + `docgarden`

This variant starts from the knowledge repo and adds `docgarden` plus repository policy configured to reinforce the knowledge system mechanically.

Depending on the implementation stage of `docgarden`, this may include:

- local-reference linting and autofix
- document-family-aware rule targeting
- front matter validation
- context-budget checks
- discovery commands and metadata-driven matching
- generated or templated agent guidance

This variant should not merely install a binary. It should embody the claim that the repository knowledge system is mechanically maintained and therefore more trustworthy for agents.

## Handling Multiple Base Commits From One Repository

Multiple candidate benchmark tasks from the same repository may point to different `base_commit` values.

The benchmark should therefore avoid a single hand-maintained knowledge branch that drifts away from the task snapshot. The stronger direction is:

- define a repeatable knowledge-base generation recipe for a repository
- materialize that recipe separately for each task's `base_commit`
- apply the same policy recipe for the `docgarden` variant

This keeps the experiment closer to the benchmark snapshot while still amortizing the expensive intellectual work of designing the repository knowledge system.

In other words, the reusable artifact should be the recipe, not one mutable branch snapshot.

## Harness Strategy

The benchmark should, where possible, reuse existing task definitions and success grading, but it should not depend on a stock agent loop.

One candidate ecosystem is SWE-bench, which supplies benchmark tasks, datasets, and harness infrastructure:

- [SWE-bench GitHub repository](https://github.com/SWE-bench/SWE-bench)
- [SWE-bench evaluation docs](https://www.swebench.com/SWE-bench/)

The recommended benchmark architecture is:

1. Load a task instance from the selected benchmark source.
2. Materialize the selected repository variant for that task.
3. Run an instrumented Codex-based agent on the issue description inside that repository.
4. Submit the resulting patch to the benchmark's grading flow.
5. Record both task outcome and benchmark telemetry.

This keeps benchmark correctness grounded in an objective task harness while allowing full control over agent behavior and instrumentation.

## Codex SDK As The Agent Runtime

The benchmark should use the Codex SDK as the agent runtime boundary because it gives the benchmark runner a clean place to wrap agent execution and collect telemetry.

Relevant OpenAI documentation:

- [Codex docs](https://platform.openai.com/docs/codex)
- [Codex SDK README](https://github.com/openai/codex/blob/main/sdk/typescript/README.md)
- [Codex SDK announcement](https://openai.com/index/codex-now-generally-available/)
- [Code generation guide mentioning Codex in CI/CD and the Responses API](https://developers.openai.com/api/docs/guides/code-generation)
- [Responses API reference](https://developers.openai.com/api/reference/responses)
- [Prompt caching guide](https://platform.openai.com/docs/guides/prompt-caching)

The benchmark should treat the Codex SDK as the execution substrate, not as the benchmark itself.

## Instrumentation Points For Codex SDK

The benchmark runner should add logging wrappers around the Codex SDK and the repo-facing tool layer.

### Run Boundary

Wrap `new Codex(...)` and `startThread()` or `resumeThread()` to log:

- run identifier
- task identifier
- repository identifier
- repository variant
- benchmark commit or recipe version
- model name
- working directory
- wall-clock start and end times

### Turn Boundary

Wrap `thread.run(...)` for simple turn-level logging and `thread.runStreamed(...)` for event-level logging.

The current SDK README explicitly documents `runStreamed()` as the mechanism for observing intermediate events and shows a `turn.completed` event that carries usage data. The benchmark should use streamed runs by default so it can capture:

- turn completion
- usage for that turn
- intermediate tool activity
- file change notifications

### Tool Boundary

Any repo-facing tools exposed to Codex should be wrapped so the benchmark can log:

- tool name
- arguments
- duration
- whether the tool read files
- whether the tool changed files
- result size

This is essential for measuring progressive discovery rather than only API spend.

### File-Read Boundary

Any direct file-read helper used by the agent should log:

- file path
- bytes read
- estimated token count
- whether content was actually surfaced to the model

This supports metrics such as:

- files opened
- estimated retrieval tokens
- irrelevant tokens loaded

### Response Usage Boundary

The benchmark should persist provider-reported usage from the SDK or underlying Responses API for every completed turn, including when available:

- input tokens
- output tokens
- total tokens
- cached input tokens

Provider-reported usage should be treated as the source of truth for API cost measurement. Local token estimation should be used for repository-read analytics and debugging, not as a replacement for API usage records.

## Benchmark Telemetry Schema

The benchmark should write one structured event stream per run and one aggregated summary record per task execution.

The event stream should be rich enough to reconstruct:

- what the agent looked at
- what the agent asked the model
- how much usage each turn consumed
- how the run ended

A minimal summary record should include:

- `task_id`
- `repo`
- `base_commit`
- `repo_variant`
- `model`
- `success`
- `total_input_tokens`
- `total_output_tokens`
- `total_cached_input_tokens`
- `files_opened`
- `estimated_file_tokens`
- `wall_clock_seconds`

## Scoring Direction

The top-line benchmark score should not be a single pass rate.

The strongest public-facing outputs should be:

- median `tokens_to_success`
- success rate under fixed token budgets
- median files opened before success
- per-repository improvement from `raw_repo` to `knowledge_repo`
- per-repository improvement from `knowledge_repo` to `knowledge_repo_plus_docgarden`

This supports claims such as:

- same tasks, fewer input tokens
- same token budget, more tasks solved
- one repository knowledge investment compounds across multiple issues

## Naming And Positioning

This benchmark should be positioned as an agent-legibility benchmark, not as a replacement leaderboard for any existing benchmark.

Good framing:

- focused on repository legibility and progressive discovery
- extended for agent token-efficiency measurement
- optionally built on one or more external benchmark task sources

Bad framing:

- claiming a new official score for an existing benchmark after modifying repository contents
- implying that the benchmark measures frontier model quality in the same way as stock benchmark reporting

## Open Questions

- Which existing benchmark datasets are the best candidates for agent-legibility evaluation?
- Is SWE-bench Verified suitable as a primary benchmark source, a secondary validation source, or neither?
- How should we measure the amount of front-loaded context in issue descriptions when evaluating dataset suitability?
- How much repository-authored documentation can be added before the benchmark stops feeling realistic?
- Which parts of the knowledge repo should be generated mechanically versus curated by hand?
- At what stage of `docgarden` maturity should the `knowledge_repo_plus_docgarden` condition be considered benchmark-ready?
- Should the benchmark report only provider usage, or also publish a deterministic local token estimate for all repository reads using one fixed tokenizer?
