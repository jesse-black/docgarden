---
description: "ADR (Architectural Decision Record) authoring contract for `docgarden`; read when adding a new decision record, picking a template, or asking how to supersede an existing decision."
---

# Decision Records

This document defines how to author Architectural Decision Records ("ADRs") in `docgarden`. An ADR is a compact, durable record of one decision: the problem, the options, and the chosen outcome.

The format is [MADR 4.0.0](https://adr.github.io/madr/) (per [ADR 0001](decisions/0001-use-madr-for-decision-records.md)). This document records local authoring rules on top of MADR.

## Locations

Store ADRs in:
- `docs/decisions/`

Name ADR files `NNNN-<short-title>.md`, where `NNNN` is the next zero-padded sequence number and `<short-title>` is a concise kebab-case slug.

To print the next sequence number:

```sh
ls docs/decisions/ | grep -E '^[0-9]{4}-' | sort | tail -1 | cut -d- -f1 | awk '{printf "%04d\n", $1+1}'
```

## Rules

- One decision per record. Keep scope narrow; if the body covers two decisions, split it.
- Prefer bullets over prose in `Considered Options` and `Decision Outcome`.
- State the decision as an accepted outcome, not as a roadmap, migration, implementation note, or explanation of how the repository reached this point.
- Explain durable forces and trade-offs. If a sentence only makes sense before, during, or after a particular implementation, move it to a plan, design doc, issue, or review note.
- Link only to stable sources that are part of the durable rationale, such as prior ADRs, external papers, standards, specifications, or permanent issue discussions.
- Do not link to active ExecPlans, living design docs, transient task plans, or implementation files from accepted ADRs.
- Do not pin operational details such as template variants, required frontmatter, lint configuration, file paths outside the ADR directory, validation commands, or code locations.
- End the body at the last `Decision Outcome` rationale. Do not append a closing paragraph that disclaims scope, pre-empts hypothetical future swaps, or restates what the ADR does not commit to.

## Template

```md
---
description: "{one-line summary of the decision and its scope; specific enough that `docgarden match` can route to this ADR from a relevant query}"
---

# {short title — represents the solved problem and solution}

## Context and Problem Statement

{Describe the context and problem statement, e.g., in free form using two to three sentences or in the form of an illustrative story. You may want to articulate the problem in the form of a question and add links to collaboration boards or issue trackers.}

## Considered Options

- {title of option 1}
- {title of option 2}
- {title of option 3}

## Decision Outcome

Chosen option: "{title of option 1}", because {justification — e.g. only option that meets a knock-out criterion, resolves a force, or satisfies the most decision drivers}.
```

This is the [MADR 4.0.0 bare-minimal template](https://github.com/adr/madr/blob/main/template/adr-template-bare-minimal.md) plus the `description:` frontmatter required by `docgarden`. Optional MADR sections may be added only when they clarify the decision-specific rationale.

## Section Guidance

- `description` (frontmatter): one-line summary that helps `docgarden match` route to this ADR. Specific enough to disambiguate from other decisions; not a restatement of the title.
- `Title`: short noun phrase describing the chosen direction, not the problem.
- `Context and Problem Statement`: state the enduring problem or force that required a decision. Do not snapshot current functionality, implementation history, or roadmap intent.
- `Considered Options`: list the real alternatives. Include an existing behavior as an option when retaining it was seriously considered.
- `Decision Outcome`: name the chosen option and the durable reasons. Reasons should explain the trade-off, not implementation timing or repository layout.

## Review Test

Before accepting an ADR, read it as if the implementation is already complete and the author is no longer available. The ADR should still explain the decision without needing the reader to know the branch state, plan state, or implementation history.

## Supersession Rules

ADRs become immutable once committed. Do not edit the body of a committed ADR to change the decision. Uncommitted drafts may still be revised, renamed, or renumbered freely; the supersession workflow only applies after the ADR lands in git.

To replace a decision:
- write a new ADR that records the new context and outcome;
- in the new ADR, link to the ADR(s) it supersedes;
- in the old ADR, append a single line at the top of the body: `> Superseded by [ADR NNNN](NNNN-<title>.md).`

Acceptable in-place edits to existing ADRs:
- frontmatter updates that improve discovery without changing the decision;
- typo and link fixes;
- adding the supersession line described above;
- correcting factual errors that do not change the decision (note the correction inline).

## Principle

The ADR records the durable why. Operational details that will evolve belong in the relevant planning, product, architecture, or design document.
