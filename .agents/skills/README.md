# Skills

Repository-local skills live under `.agents/skills/<skill-name>/`.
These skills should follow the Agent Skills spec and keep repository conventions lightweight and portable.

## Current skills
- `architecture-md-authoring` (`.agents/skills/architecture-md-authoring/SKILL.md`)
  Common request aliases: `architecture md skill`, `architecture.md skill`, `rewrite architecture.md`, `author architecture doc`
- `agents-md-authoring` (`.agents/skills/agents-md-authoring/SKILL.md`)
  Common request aliases: `agents md skill`, `agents.md skill`, `rewrite agents.md`, `author agents guide`
- `dependabot-updates` (`.agents/skills/dependabot-updates/SKILL.md`)
  Common request aliases: `dependabot skill`, `dependabot`, `dependency bump skill`

## Registering a skill
1. Add a `` `SKILL.md` `` at `.agents/skills/<skill-name>/SKILL.md`.
2. Follow the Agent Skills spec in `` `SKILL.md` ``, especially the required frontmatter fields such as `name` and `description`.
3. Add or update a short pointer in top-level `AGENTS.md` so the repository advertises `.agents/skills/` when needed.
4. Document likely user-facing aliases so requests such as "load the dependabot skill" can be mapped to the correct local skill even when the folder name differs.
5. If a specific runtime benefits from extra metadata files, treat them as optional runtime integrations rather than part of the repository's core skill contract.