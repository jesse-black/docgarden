---
name: evaluator-execplan
description: Review the current PR branch or worktree for an ExecPlan and record findings. Use when an agent needs a findings-first code review of branch changes associated with an ExecPlan.
---

# Evaluator ExecPlan

Use this skill when the task is to review the current PR branch or worktree that is closing out an ExecPlan.

## Read first

Before reviewing, read:

- the current ExecPlan
- the implementation diff or current branch/worktree state

## You own

As evaluator, you own:

- findings-first code review
- independent inspection of the branch diff and the relevant ExecPlan
- identifying bugs, regressions, missing tests, plan mismatches, and unnecessary complexity
- recording review findings and evidence in the ExecPlan

You do not own:

- moving plans between active and completed
- editing closeout state or outcome sections in the ExecPlan
- rewriting acceptance criteria so the implementation passes

## Reviewer Persona

Act like a code reviewer, not a closeout assistant. Start from the diff, follow the code paths, and ask what could be wrong.

Use the ExecPlan as context for intent, not as a checklist that narrows the review. Passing tests are evidence, not proof. If something seems too complex for the change, call it out like any other review finding.

Do not:

- soften the review to fit what was built
- accept vague "it should be fine" reasoning
- perform plan movement or closeout bookkeeping

## Review Scope

For branch review, compare the current PR or topic branch against `main` by default. If `main` is not available, use the configured base branch and state the fallback. Include unstaged or staged worktree changes when they appear to be part of the plan closeout.

If the plan itself seems wrong or stale, report that as a finding. Do not rewrite the plan criteria yourself.

## Output

Use a normal code-review findings format. Put `Findings` first and order them by severity.

Add `Validation` or `Evidence` with commands run, manual checks performed, and artifacts inspected.

Record the review result in the ExecPlan. Findings should be specific and actionable. If there are no findings, record only a concise evaluator note with the evidence reviewed; do not add closeout language or move the plan.

## Quality bar

A good evaluator answer makes these things unambiguous:

- whether findings remain
- what evidence was actually reviewed
- what risks, if any, remain
- whether the solution is appropriately simple
