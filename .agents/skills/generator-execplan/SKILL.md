---
name: generator-execplan
description: Implement from an active ExecPlan while keeping the plan up to date as a living handoff artifact. Use when an agent needs to continue plan-driven work, leave strict breadcrumbs in the ExecPlan, capture requirement changes from human conversation, log rejected experiments and surprises, or prepare work for evaluator review without declaring it complete.
---

# Generator ExecPlan

Use this skill when the plan already exists and the task is to execute from it without losing the thread.

## Read first

Before implementing, read:

- `docs/PLANS.md`
- the active ExecPlan you are executing

Before retrying any line of work, scan:

- `Progress`
- `Decision Log`
- `Surprises & Discoveries`

## You own

As generator, you own:

- implementation from the current ExecPlan
- keeping the plan current while you work
- leaving enough breadcrumbs for a fresh contributor to resume
- simplifying the solution after feedback changes the approach

You do not own:

- changing acceptance criteria
- redefining milestones or completion bars
- moving a plan to `completed/`

## Mandatory breadcrumbs

At every meaningful stopping point, update the ExecPlan with:

- `Progress` timestamps and explicit next steps
- any requirement changes learned from human conversation
- rejected approaches when they explain a decision
- surprises with concrete evidence
- any mismatch between the plan and reality as a proposed planner delta

Treat the ExecPlan as the handoff artifact, not as a summary to fill in later.

## Implementation rules

Follow these rules strictly:

- If retrying a previously rejected path, state what new evidence justifies reopening it.
- If a human request changes scope mid-stream, append a requirement delta before more coding continues.
- If reality diverges from the plan in a way that changes success criteria, stop and hand the plan back to `$planner-execplan`.
- When feedback changes the approach, revisit the whole solution shape.
- Remove or collapse complexity that is no longer needed.

## Ready for evaluation

Your strongest close-out claim is "ready for evaluation."

When you believe the work is ready:

- point to the relevant acceptance criteria
- point to the evidence the evaluator should inspect
- call out any known risks or incomplete edges
- say whether the latest round simplified the solution or only added another fix

Do not mark the plan complete and do not move it to `docs/exec-plans/completed/`.

## Quality bar

A good generator leaves behind an ExecPlan that lets a fresh contributor answer:

- What was tried?
- What failed?
- What changed in the requirements?
- What exactly still needs evaluator judgment?
