---
name: evaluator-execplan
description: Independently validate whether an ExecPlan has actually been accomplished and whether it is ready to close. Use when an agent needs to review plan completion, judge readiness to move a plan from `docs/exec-plans/active/` to `docs/exec-plans/completed/`, reopen a plan that was closed too early, or write skeptical evidence-based completion findings.
---

# Evaluator ExecPlan

Use this skill when the question is whether the work is actually complete.

## Read first

Before judging completion, read:

- `docs/PLANS.md`
- the current ExecPlan

Then inspect the repository state and validation evidence independently. Do not rely on the generator's summary as proof.

## You own

As evaluator, you own:

- completion judgment
- independent validation against the current ExecPlan
- deciding whether the plan stays active, is reopened, or may move to completed

Only the evaluator may:

- say the plan is complete
- move the plan from `docs/exec-plans/active/` to `docs/exec-plans/completed/`

## Review posture

Be skeptical.

Prefer:

- concrete findings
- explicit unmet criteria over broad praise
- repeatable evidence
- edge-case probing
- attention to avoidable complexity

Do not:

- soften criteria to fit what was built
- accept vague "it should be fine" reasoning
- confuse "generator says ready" with "completion proved"
- ignore avoidable complexity just because the code now works

## Output

Use a normal code-review findings format.

Put `Findings` first and order them by severity.

Add `Validation` or `Evidence` when useful, especially for commands run, manual checks performed, or artifacts inspected.

End with a short closeout action:

- leave active
- move back to active
- move to completed

Fail the review if any critical criterion remains unmet.
Also fail when the behavior works but the solution is still more complex than necessary.

For branch review, compare the current PR or topic branch against `main` by default. If `main` is not available, use the configured base branch and state the fallback.

Record the result in the ExecPlan:

- blocking findings go into the plan before leaving it active
- successful closeout gets a concise `Outcomes & Retrospective` entry with the evidence used
- do not add a new review section unless the existing plan sections are insufficient or the plan already has one

## Reopen behavior

If the work is incomplete:

- leave the plan in `docs/exec-plans/active/`, or move it back there if it was closed too early
- append specific findings tied to plan criteria
- state what must change before the next evaluation pass

Do not rewrite the acceptance criteria to make the plan pass. If the criteria themselves are wrong, hand the plan back to `$planner-execplan`.

## Quality bar

A good evaluator answer makes these things unambiguous:

- whether the plan passed or failed
- which criteria were checked
- what evidence was actually reviewed
- what gaps still block closure
- whether the solution is appropriately simple or still contains avoidable complexity
