---
name: dependabot-updates
description: Handle Dependabot dependency update work, especially grouped updates, peer-dependency conflicts, lockfile refreshes, and validation for npm or other package-manager changes. Use when an agent needs to review, implement, test, or explain a dependency bump PR; decide whether related packages should be grouped; verify whether plugins, adapters, or companion packages can unblock a failed upgrade; or summarize why an attempted bump could not be made safely.
---

# Dependabot Updates

Follow this workflow for Dependabot PRs and manual dependency bumps.

## Build Context First

Inspect the dependency manifests, lockfiles, Dependabot config, CI failure, and the files that exercise the bumped packages before editing anything.

For ecosystem-coupled packages, identify the full set of related packages rather than treating the proposed bump as isolated. Common examples:

- `eslint` with `@eslint/js`, plugins, parsers, formatters, and config helpers
- framework packages with their adapters or compiler plugins
- SDKs with transport, auth, or generated-client companions

When a bump failed because of peer dependencies or missing companions, check whether a related package upgrade could unblock it before giving up.

## Decide When Grouping Is Needed

Recommend or add Dependabot grouping when one package is routinely upgraded together with tightly coupled companions.

Strong signals that grouping is needed:

- The packages share peer-dependency ranges.
- One package's major version usually requires a matching plugin, preset, adapter, or config package bump.
- A prior Dependabot PR upgraded one package alone and produced an install or CI failure.
- The repo already treats the packages as one toolchain in config or scripts.

For grouped Dependabot changes, update both version and security groups when that matches the repo's existing pattern.

Grouping is not enough when the ecosystem is genuinely blocked. If the latest compatible companion still does not support the target version, say that plainly and recommend an ignore or version pin when appropriate.

## Try To Unblock Before Concluding It Is Blocked

When a bump fails because of peer constraints:

1. Identify the blocking package.
2. Check whether a newer version exists that might unblock the upgrade.
3. Verify the newer version's declared compatibility, not just its existence.
4. If it still does not support the target version, say so explicitly in the response.

Use package-manager metadata or primary package sources to verify compatibility. For npm projects, `npm view <pkg> version peerDependencies --json` is a good first check.

Do not stop at "package X is blocking." Say whether you tried to find a newer unblocking version and what you found.

Example expectation for an ESLint-style conflict:

- Identify that `eslint-plugin-react-hooks` has a newer version available.
- Check whether that newer version accepts the target `eslint` major.
- If it does not, state that the unblock attempt was considered and why it still fails.

## Make The Smallest Safe Change

Prefer the smallest change that restores a valid dependency graph and keeps the PR aligned with the repo's intent.

Examples:

- Add missing companion packages that the upgraded package now expects.
- Bump a plugin or adapter if it truly supports the target version.
- Revert or pin the primary package when the ecosystem is not yet compatible.
- Add or adjust Dependabot grouping or ignore rules to prevent repeat breakage.

Do not claim a package bump was attempted unless you actually checked whether it exists and whether it is compatible.

If the smallest safe fix requires adding a temporary lint/tooling override (for example disabling a newly introduced rule for specific files), also add a concrete follow-up task to `docs/cleanup-backlog.md` describing how to remove that override later.

### `legacy-peer-deps` / `--force` Are Last Resort

Do **not** use `legacy-peer-deps`, `--force`, or equivalent package-manager bypasses as the default fix for peer conflicts.

Before considering any bypass, you must explicitly:

1. Identify the exact blocking package and peer range.
2. Check for a newer compatible version of the blocker and companion packages.
3. Evaluate safer alternatives (tool/plugin upgrade, temporary pin/revert, or Dependabot ignore/grouping adjustment).

Only use a bypass when all safer options are exhausted and you document:

- why each safer option was not viable,
- the precise scope of risk introduced, and
- a follow-up plan to remove the bypass.

When a bypass is unavoidable, prefer the narrowest scope possible (single command or isolated workflow), and avoid repo-wide defaults that affect unrelated installs.

## Mandatory Validation

For Node.js projects, `npm ci` is mandatory after dependency edits. Run it in the project directory that owns the manifest and lockfile.

When dependencies are bumped in a Node.js project, `npm run build` is mandatory if the project defines a build script.

When development tools are bumped, run the relevant tools, not just install/build. At minimum:

- If `eslint` or an ESLint plugin/config/parser is bumped, run the repo's lint command.
- If a test runner or test-related package is bumped, run the relevant test command.
- If a formatter or typechecker is bumped and the repo has dedicated verification commands for them, run those too.

If a required validation command cannot be run, say exactly which command was not run and why.

## Response Requirements

In the final response:

- State the change you made.
- State the validation you ran.
- State any attempted unblock you investigated for coupled packages.
- If the upgrade could not be completed, explain the blocker in concrete terms.

For retrospectives and reviews, prefer wording that makes the investigation visible. Good pattern:

- "Checked whether `<companion-package>` had a newer release that could unblock `<target-package>`."
- "Found `<version>`, but its declared compatibility still only supports `<range>`."
- "Because of that, the upgrade still cannot land safely, so the fix was to `<pin/downgrade/add-ignore>`."
