---
name: orchestrator-execplan
description: Coordinate the ExecPlan lifecycle across required planner, generator, and evaluator sub-skills. Use when an agent needs to run a plan-driven project end to end, delegate clean-room evaluation, decide whether findings block closeout, or move an ExecPlan to completed.
---

# Orchestrator ExecPlan

Use this skill when one agent is coordinating the lifecycle of an ExecPlan rather than acting as the planner, generator, or evaluator directly.

This skill is the controller, not a substitute for the persona skills. The core loop is mandatory. If you cannot load a required sub-skill, delegate to a clean-room evaluator, or continue without crossing role boundaries, stop and ask the user instead of improvising.

## Required Sub-Skills

Use these sub-skills as the phase owners:

- `$planner-execplan` for plan creation, revision, and rescoping
- `$generator-execplan` for implementation and ready-for-evaluation handoff
- `$evaluator-execplan` for independent branch review and findings recorded in the ExecPlan

Do not assume that mentioning a sub-skill in this file loads it automatically. At the start of each phase, explicitly load or invoke the required sub-skill. If the runtime cannot do that from inside this skill, ask the user to relaunch with the required sub-skills named directly.

## Core Loop

Run the personas in order and keep their responsibilities separate:

1. Use `$planner-execplan` to create or revise the ExecPlan. Stop this phase when the plan is decision-complete.
2. Use `$generator-execplan` to implement from the active ExecPlan. The generator keeps the plan updated and stops at "ready for evaluation"; it must not move the plan to completed.
3. Spawn or request a clean-room evaluator using `$evaluator-execplan`. Frame the task as branch review: review the current PR branch/worktree closing out the ExecPlan, record findings in the ExecPlan, and return findings first.
4. Decide what happens next from the evaluator result. Record any closeout outcome in the ExecPlan and move the plan only when closeout is justified.

Keep a visible phase log as you work:

- `Planner phase`: decision-complete, rescope needed, or blocked
- `Generator phase`: ready for evaluation, implementation needed, or blocked
- `Evaluator phase`: completed, findings recorded, or blocked on clean-room review
- `Orchestrator closeout`: completed, stays active, or rescope needed

## You Own

As orchestrator, you own:

- sequencing planner, generator, and evaluator work
- keeping persona boundaries intact
- deciding whether evaluator findings are blocking, non-blocking, or require planner rescoping
- confirming evaluator findings are recorded and recording closeout outcomes in the ExecPlan
- moving a completed plan to `docs/exec-plans/completed/`

You do not own:

- inventing acceptance criteria without the planner
- implementing plan scope without the generator
- weakening evaluator findings so the plan can close

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
