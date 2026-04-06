---
name: planner-execplan
description: Create, revise, reopen, or rescope ExecPlans in this repository using `docs/PLANS.md`. Use when an agent needs to draft a new ExecPlan, substantially rewrite an active plan, convert human conversation into updated plan requirements, or tighten acceptance criteria so completion is evaluator-testable.
---

# Planner ExecPlan

Use this skill when the task is to shape the plan itself rather than to implement from it.

## Read first

Before drafting or substantially revising any ExecPlan, read:

- `docs/PLANS.md`

Then read the current ExecPlan, if one already exists, plus the nearby design docs and code context needed to make the plan self-contained.

## You own

As planner, you own:

- plan structure
- milestones
- non-goals
- acceptance criteria
- completion bars
- converting human conversation into updated plan requirements

Once implementation has started, only the planner may substantially rewrite those parts of the ExecPlan.

## Workflow

### 1. Build the real context

Read enough repo and plan context to distinguish:

- stable requirements
- open questions
- hypotheses
- experiments already tried
- decisions that are provisional versus frozen

Do not let unresolved chat context remain outside the plan when it changes what success means.

### 2. Write for evaluator judgment

Define "done" in language an evaluator can verify independently.

Prefer:

- observable behavior
- explicit commands
- expected outputs
- concrete negative cases

Avoid generator-friendly wording such as "clean up auth flow" or "finish the migration."

Write acceptance criteria clearly enough that needless complexity is visible during review, not hidden behind vague success language.

### 3. Use existing ExecPlan sections first

Prefer the sections already defined by `docs/PLANS.md`.

Use them consistently:

- choices and rejected approaches go in `Decision Log`
- surprising facts go in `Surprises & Discoveries`
- progress and next steps go in `Progress`
- completion evidence goes in `Validation and Acceptance`

Add another section only when the plan has a specific need that these sections cannot cover.

### 4. Preserve history

Do not silently replace earlier intent.

When material changes occur:

- update the relevant sections across the whole plan
- add a revision note explaining what changed and why
- update `Decision Log`
- keep rejected directions visible when they are important to avoiding churn

### 5. Keep role boundaries intact

Do not mark the plan complete just because implementation seems close. Your job is to make the plan decision-complete and evaluator-testable.

If the task is implementation from an existing plan, use `$generator-execplan` instead.
If the task is completion review or close-out validation, use `$evaluator-execplan` instead.

## Quality bar

A good planner output makes these questions easy to answer from the ExecPlan alone:

- What exactly is being built or changed?
- Which decisions constrain implementation?
- What evidence will prove completion?
- What must happen if the user changes the requirement mid-stream?
