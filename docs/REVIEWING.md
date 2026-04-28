---
description: "Review routing guide for `docgarden` changes; read when reviewing a branch, PR, worktree, ADR, plan, test, or documentation update."
---

# Reviewing Changes

Use this guide when reviewing repository changes. It adds repository-specific routing and documentation checks on top of the agent's normal review posture.

## Review Routes

Read the relevant source documents for the changed surface:

| Changed surface | Read |
| --- | --- |
| Rust code | [`docs/CODESTYLE.md`](CODESTYLE.md) |
| Tests | [`docs/TESTING.md`](TESTING.md) |
| ExecPlans or plan-driven work | [`docs/PLANS.md`](PLANS.md) |
| ADR additions, supersession, immutability, or decision-record policy | [`docs/DECISIONS.md`](DECISIONS.md) |
| Architecture boundaries or module ownership | [`ARCHITECTURE.md`](../ARCHITECTURE.md) |
| Product scope, users, or positioning | [`docs/PRODUCT.md`](PRODUCT.md) |

## Documentation Freshness

Check whether the reviewed work changes any reader-facing or agent-routing contract:

| Change type | Confirm |
| --- | --- |
| Public command behavior, installation, configuration, or examples changed | [`README.md`](../README.md) reflects the user-facing behavior. |
| Product scope, target user, workflow, shipped capability, or non-goal changed | [`docs/PRODUCT.md`](PRODUCT.md) reflects the product contract. |
| Module ownership, execution flow, system boundary, or architectural invariant changed | [`ARCHITECTURE.md`](../ARCHITECTURE.md) reflects the code shape. |
| Design rationale, policy direction, or future implementation guidance changed | The relevant document under [`docs/design-docs/`](design-docs/) reflects the current design direction. |
| A durable architectural or policy decision was made or reversed | An ADR under [`docs/decisions/`](decisions/) exists or is updated according to [`docs/DECISIONS.md`](DECISIONS.md). |

## Validation Checks

- For Markdown changes, run `cargo run -- lint <changed-files> --color never`.
- For code changes, run targeted tests that cover the changed behavior.
- For bug fixes or review findings, expect a failing test or clear reproduction before the fix.
- Reserve `cargo xtask validate` for final validation before handoff.
