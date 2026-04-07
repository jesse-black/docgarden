# ExecPlan Persona Contracts

This document defines the shared operating contract for the four repo-local ExecPlan skills:

- `planner-execplan`
- `generator-execplan`
- `evaluator-execplan`
- `orchestrator-execplan`

Use this document together with `docs/PLANS.md`. `docs/PLANS.md` defines what an ExecPlan must contain. This document defines who may change which parts of a plan, what evidence must be captured while work is in flight, and how closeout responsibility is separated from review.

## Purpose

These persona rules define how long-running ExecPlan work is handed from one agent to the next. A fresh agent should be able to read the plan, understand what has been agreed, continue from the current state, and hand the result to an independent evaluator without relying on chat history.

`docs/PLANS.md` defines the ExecPlan as the shared artifact. This document defines how the roles use it: the planner turns intent into testable requirements, the generator implements against those requirements and keeps the plan current, the evaluator reviews the result skeptically, and the orchestrator records the result and closes the plan only when justified.

The governing principle is role separation. A plan is more reliable when the same persona is not simultaneously inventing scope, implementing the work, reviewing its own work, and deciding whether to close the artifact.

Keep the process no more complex than the work requires. Preserve role separation where it improves handoff or judgment, and remove process that is no longer load-bearing.

## Role Contract

The planner owns the shape of the plan: scope, milestones, non-goals, acceptance criteria, and completion bars. The planner also turns material user conversations into updated requirements inside the ExecPlan.

The generator owns implementation from the active plan and the in-flight record: progress, discoveries, requirement deltas, rejected approaches, and proposed plan changes when reality diverges from the plan.

The evaluator owns independent review findings. The evaluator reviews the branch skeptically, uses the current plan as context for intent, records findings and evidence in the ExecPlan, and reports findings without moving or closing the plan.

The orchestrator owns lifecycle coordination and closeout. The orchestrator sequences planner, generator, and evaluator work; decides whether evaluator findings block closeout, require generator fixes, or require planner rescoping; records closeout outcomes in the plan; and moves a plan to `docs/exec-plans/completed/` when closeout is justified.

The orchestrator does not replace the other personas. It uses `planner-execplan`, `generator-execplan`, and `evaluator-execplan` as required sub-skills for their phases. If the runtime cannot load those sub-skills or cannot run a clean-room evaluator when one is required, the orchestrator should stop and ask for the missing capability rather than silently self-reviewing or closing the plan.

## Rewrite and Change-Control Rules

Once implementation has started, only the planner may substantially rewrite:

- milestones
- acceptance criteria
- non-goals
- completion bars
- plan structure that changes how completion will be judged

The generator may still append:

- requirement deltas for planner review
- discoveries and surprises
- progress notes
- rejected approaches when they explain a decision or discovery
- proposed follow-up changes when the current plan no longer matches reality

If the generator concludes the work cannot be completed without changing success criteria, it must stop implementation and hand the plan back to the planner rather than quietly redefining "done."

## Automation and Context Isolation

This contract is about responsibility, not runtime shape. A single agent may follow the roles manually, or separate agents may handle separate roles when the runtime supports it.

The ExecPlan is the handoff artifact. If a role works in a separate context, accepted requirements, progress, discoveries, and evaluation evidence must be written into the plan rather than left in chat history. The evaluator records review findings and evidence directly; the orchestrator records final closeout outcomes.

The evaluator should usually work from a cleaner context than the generator. When using `orchestrator-execplan`, the evaluator phase should use a fresh context or subagent when the runtime supports it. Give the evaluator the current ExecPlan, repository state or diff, validation commands and outputs, and any generator handoff note.

## Handoff and Sections

Before implementation starts, the ExecPlan should make the intended outcome, out-of-scope work, completion evidence, and implementation constraints clear from the plan itself.

Use the living-document sections from `docs/PLANS.md` before adding plan-specific sections:

- rejected approaches belong in `Decision Log` when they explain a choice
- surprising facts belong in `Surprises & Discoveries`
- progress and next steps belong in `Progress`
- completion evidence belongs in `Validation and Acceptance`, evaluator findings, and orchestrator closeout notes

When the generator believes the work is ready, it should leave a short handoff note in the plan with the relevant acceptance criteria, evidence to inspect, and any known risks. That handoff is a request for judgment, not a declaration of success.

## Completion Rubric

The orchestrator may move a plan to `docs/exec-plans/completed/` only when all of these are true:

1. The acceptance criteria still match the latest user-approved requirements.
2. The repository state and observed behavior satisfy those criteria.
3. The solution is no more complex than necessary.
4. The validation evidence is concrete enough to repeat.
5. The plan records the important progress, decisions, surprises, and unresolved gaps.
6. Independent evaluator review found no blocking findings, or a human explicitly accepted remaining findings as non-blocking.

If the evaluator records findings, the plan stays active or is moved back to active unless the orchestrator or human explicitly classifies the findings as non-blocking.

## Standard Evaluator Output

Use the evaluator's natural code-review instincts. Put findings first and order them by severity. The evaluator records and reports review findings; it does not perform closeout mutation.

Use these parts when they are useful:

- `Findings`: concrete issues, ordered by severity
- `Validation` or `Evidence`: commands run, manual checks performed, or artifacts inspected

For branch review, the default scope is the current PR or topic branch compared against `main`. If `main` is not available, compare against the configured base branch or state the fallback used.

## Standard Orchestrator Closeout

The orchestrator turns evaluator output into a lifecycle action. When the plan closes, add a concise `Outcomes & Retrospective` entry with the evidence used and move the plan to `docs/exec-plans/completed/`. Do not add a new review section unless the existing ExecPlan sections are insufficient or the plan already has an appropriate findings section.
