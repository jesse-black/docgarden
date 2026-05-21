---
name: evaluator-execplan
description: Review the current branch, PR, or worktree against the plan or ExecPlan and record findings. Use when an agent needs a code review with findings for changes associated with an active plan.
---

# Evaluator ExecPlan

Use this skill when the task is to review the current PR branch or worktree that is closing out an ExecPlan.

## Pass 1: Cold review

Read the diff or changed files. Do not load the ExecPlan, CODESTYLE.md, or TESTING.md yet.

For new methods, exports, or class members, grep for callers before assuming they are used.

Record your observations before moving to Pass 2.

## Pass 2: Context review

Now load:

- `docs/PLANS.md`
- the current ExecPlan

Follow the review authorities listed under Pass 2 in the ExecPlan's Evaluator DoD. If a changed surface is not covered by the plan's DoD, treat the missing coverage as a finding. Then add findings that require plan context:

- Does the implementation match what the plan's completed steps claim was done?
- Do completed review items have matching worktree evidence?
- Does the implementation conflict with or omit a requirement from CODESTYLE.md or TESTING.md that the plan did not mention?
- Are any plan-specified designs themselves violations of CODESTYLE.md? A design element that appears in the ExecPlan spec is not pre-approved.
- Compare new code with nearby code in the same layer: unexplained departures from local style and structure are findings.

Apply each rule at the granularity it requires. If TESTING.md says individual tests must move unless they call private functions, inspect individual tests, not just whether a module contains any test.

## You own

As evaluator, you own:

- independent review of the branch diff and the relevant ExecPlan
- identifying bugs, regressions, missing tests, plan mismatches, plan defects, `CODESTYLE.md` violations, `TESTING.md` violations, and unnecessary complexity
- challenging completed plan items when the worktree evidence does not support them
- recording review findings and evidence in the ExecPlan
- checking off the Evaluator items in the `Definition of Done` section once the review is clean and all findings are addressed

You do not own:

- moving plans between active and completed
- rewriting acceptance criteria so the implementation passes
- taking over implementation or plan closeout responsibilities

## Role boundaries

Use the evaluator when the task is to perform an independent review of the current branch or worktree against the plan.

Act like a reviewer, not a closeout assistant. Record findings in the ExecPlan without softening the review to fit what was built.

## Findings

- **Concise**: state the issue in one sentence and the required action in one sentence. If the finding is too complex to be concise, such as if it requires reopening the plan to rescope or rearchitect, STOP and ask the user what they want to do.
- **Scope**: if the issue is in a file already modified by this plan, require it to be fixed before close.
- **Ambiguity**: if you are unsure whether something is a finding, a required fix, or out of scope, STOP and ask the user before recording.
