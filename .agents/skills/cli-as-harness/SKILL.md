---
name: cli-as-harness
description: Author CLI surfaces as agent-facing documentation, especially `--help` and actionable error messages. Use with any language or parser design when adding or changing a CLI interface, improving help text or validation errors, moving command reference out of AGENTS.md / CLAUDE.md / a README / a skill and into CLI surfaces, or making errors suggest the corrected command line or tool call.
---

# Authoring CLI surfaces as harness

In agentic engineering the **harness** is everything around the model that shapes what it can do
and know — the agent loop, tool wiring, assembled context, and instructions. Most harness
instruction is *pushed*: baked into a system prompt, an AGENTS.md, or a tool description, all
static and centralized. CLI surfaces are underused local harness: `--help` carries the command
reference, and validation errors carry the recovery path when a command is malformed. Treating a
CLI as harness means deliberately authoring both for an agent reader — not just a human — so the
installed tool explains its current interface and how to recover from common invalid calls.

The organizing rule: **top-level `--help` owns what the tool is for and when to use it;
subcommand `--help` owns command-local semantics; error messages own local recovery from invalid
command use; a skill or AGENTS.md owns cross-tool workflow, policy, and anything that needs the
bigger picture to apply safely.** Most staleness and duplication in agent docs comes from copying
a flag list into AGENTS.md, where it rots the moment the CLI changes.

## Prefer declarative parsers

For new CLIs, prefer the ecosystem's standard declarative parser so behavior, help, and parse
errors share a source of truth. For an existing hand-rolled parser, improve its current surfaces
in place unless migration is in scope. Consider migration when duplicated definitions or
scattered validation repeatedly cause drift; do not expand a narrow help or error task solely to
replace the parser.

## Authoring `--help`

If you own the CLI, put the canonical, agent-facing instruction **in the parser's command
definitions**. Use the project's existing parser framework and conventions; the principle applies
equally to Python `argparse` or Click, Rust clap, Node.js Commander, Go Cobra, and other CLI
parsers. Map these semantic roles to the framework's own API:

| Semantic role | Put in the parser or command definition |
|---------------|-----------------------------------------|
| Root purpose | Summary and long description explaining what the tool does and when to use it |
| Option contract | Help text, value shape, default, allowed values, and whether it is required |
| Subcommand contract | Command-local description, options, examples, and gotchas |
| Extended guidance | Framework-supported footer, after-help, or equivalent verbatim section |

Prefer framework-generated usage, option, and default output over hand-maintained copies. Keep
examples and longer guidance beside the command definition, or in the framework's help hooks, and
preserve intentional line breaks with the framework's own formatter.

Then make instruction files carry a **pointer, not a copy**: ``Run `tool <cmd> --help` for the
authoritative reference.``

### What to put in `--help` (beyond the bare flags)

A flag list is the floor, not the ceiling. The high-value additions for an agent reader:

- **Examples** — concrete, copy-pasteable invocations. Subcommand help usually wants examples,
  not recipes: one realistic command line teaches more than a paragraph.
  > `Example:  tool deploy --env staging --dry-run    # preview without applying`
- **Recipes** — short task flows for top-level `tool --help`, so the agent sees the intended
  *order* for common end-to-end work inside that tool. Keep them brief and point to the exact
  subcommands; do not try to replace subcommand help.
  > `Recipe (log then review):  tool add ... && tool list --due`
- **Isolated guidance** — advice correct on its own, since each help page must stand without
  surrounding narrative. Good: "`--output FILE` writes to `FILE` instead of stdout."
- **Gotchas** — constraints that produce confusing errors if missed. Put each rule next to the
  flag, then name it in the matching error and suggest the correction.
- **Tool when/why** — the root `tool --help` should say when to reach for the tool at all, and host
  the recipes that span its own subcommands.

**The isolation test** decides what qualifies: *would this instruction still be correct and
actionable for an agent that read only this `--help` page and nothing else?* If yes, it belongs in
`--help`. If it only makes sense as one step of a larger cross-tool workflow or repo policy, it
belongs in a skill or AGENTS.md instead.

## Authoring error messages

Error messages are part of the harness because agents often see them at the exact moment they need
to recover. Keep them short, specific, and corrective:

- State what failed using the same flag, value, and subcommand names shown in `--help`.
- When the fix is unambiguous, include a runnable correction. Render dynamic values safely for the
  target interface: shell-quote shell values, use `--` before positional filenames when supported,
  or prefer a structured tool call or argument list.
- For validation constraints, name the accepted values or required pairings directly in the error.
- If an error teaches a rule that should have been known before execution, add the same rule to the
  relevant `--help` page so the command is self-documenting before and after failure.
- Limit recovery text to the immediate correction; omit unrelated workflow and rationale.

For example, a POSIX-shell recovery command for files `file 1` and `file2` is:

```bash
git add -N -- 'file 1' file2
```

Use the framework's native parse errors for syntax it already understands, such as unknown options,
missing values, and invalid enumerated values. Add application errors for semantic constraints the
framework cannot express. Keep terminology and usage shape consistent across both paths.

## Implementation workflow

1. Identify the parser or command-definition source and help-customization hooks before
   editing, and put each fact at the narrowest definition that owns it.
2. Generate root and affected subcommand help from the executable and inspect the rendered output,
   including wrapping and preserved examples.
3. Exercise representative invalid calls; verify each error names the bad input and, when the fix is
   unambiguous, gives a runnable correction. Cover both paths with tests in the project's style,
   asserting durable contract text over incidental spacing.
4. Remove duplicated command reference from instruction files, replacing it with a help pointer.

## What to keep out of CLI surfaces

The isolation test draws the line. Anything that only makes sense with cross-tool workflow, repo
policy, the *rationale* behind a convention, or multi-tool orchestration in view belongs in a skill
or AGENTS.md — not in `--help` or an error, where it can't be applied safely from the page alone.
Conversely, don't leave flag enumerations stranded in AGENTS.md, where they rot the moment the CLI
changes; those belong in the parser. Each surface does what it's structurally good at.
