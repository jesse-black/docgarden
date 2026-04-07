---
name: orchestrator-execplan
description: Coordinate the full ExecPlan lifecycle across planner, generator, and evaluator personas. Use when an agent needs to run a plan-driven project end to end, spawn or delegate clean-room evaluation, decide whether evaluator findings block closeout, record final evaluator results, or move an ExecPlan from `docs/exec-plans/active/` to `docs/exec-plans/completed/`.
---

# Orchestrator ExecPlan

Use this skill when one agent is coordinating the lifecycle of an ExecPlan rather than acting as the planner, generator, or evaluator directly.

## Core Loop

Run the personas in order and keep their responsibilities separate:

1. Use `$planner-execplan` to create or revise the ExecPlan. Stop this phase when the plan is decision-complete.
2. Use `$generator-execplan` to implement from the active ExecPlan. The generator keeps the plan updated and stops at "ready for evaluation"; it must not move the plan to completed.
3. Spawn or request a clean-room evaluator using `$evaluator-execplan`. Frame the task as branch review: review the current PR branch/worktree closing out the ExecPlan, record findings in the ExecPlan, and return findings first.
4. Decide what happens next from the evaluator result. Record any closeout outcome in the ExecPlan and move the plan only when closeout is justified.

## You Own

As orchestrator, you own:

- sequencing planner, generator, and evaluator work
- keeping persona boundaries intact
- deciding whether evaluator findings are blocking, non-blocking, or require planner rescoping
- recording evaluator findings or outcomes in the ExecPlan
- moving a completed plan to `docs/exec-plans/completed/`

You do not own:

- inventing acceptance criteria without the planner
- implementing plan scope without the generator
- weakening evaluator findings so the plan can close

## Evaluator Prompt

Use a clean-room prompt that asks for review, not closeout. Adapt this shape:

```text
Use $evaluator-execplan to review the current PR branch closing out <plan path>.

Inspect the diff and run whatever validation you need. Record findings and evidence in the ExecPlan, then return a findings-first review summary with commands run and artifacts inspected. Do not move the plan or edit closeout state.
```

Prefer giving the evaluator raw context: the ExecPlan path, branch/base branch, relevant validation outputs, and any generator handoff note. Do not provide the expected answer or your own suspected bug unless the human explicitly asks for a targeted review.

## Closeout Rules

If the evaluator returns blocking findings:

- keep the plan active
- confirm the findings are recorded in the ExecPlan
- route implementation fixes back through `$generator-execplan`, or route changed success criteria through `$planner-execplan`

If the evaluator returns no blocking findings:

- add a concise `Outcomes & Retrospective` entry with the evidence reviewed
- move the plan from `docs/exec-plans/active/` to `docs/exec-plans/completed/`
- update stale path references caused by the move when they are intended as live guidance rather than historical notes

If findings are real but intentionally accepted as non-blocking, record the human or orchestrator decision explicitly before closeout.

## Quality Bar

A good orchestrator run makes these things clear from the plan and final summary:

- which persona did what
- what evidence the evaluator actually reviewed
- which findings blocked or did not block closeout
- why the plan moved to completed or stayed active
