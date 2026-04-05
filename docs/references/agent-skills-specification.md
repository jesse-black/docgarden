---
title: Specification - Agent Skills
source: https://agentskills.io/specification
description: Agent Skills specification covering skill directory layout, `SKILL.md` front matter, optional directories, progressive disclosure, and validation guidance.
retrieved: 2026-04-02
last_reviewed: 2026-04-02
author: Agent Skills
---

# Specification

The complete format specification for Agent Skills.

## Directory structure

A skill is a directory containing, at minimum, a `SKILL.md` file:

    skill-name/
    ├── SKILL.md          # Required: metadata + instructions
    ├── scripts/          # Optional: executable code
    ├── references/       # Optional: documentation
    ├── assets/           # Optional: templates, resources
    └── ...               # Any additional files or directories

## `SKILL.md` format

The `SKILL.md` file contains YAML front matter followed by Markdown content.

### Front matter

| Field | Required | Constraints |
| --- | --- | --- |
| `name` | Yes | Maximum 64 characters. Lowercase letters, numbers, and hyphens only. Must not start or end with a hyphen. |
| `description` | Yes | Maximum 1024 characters. Non-empty. Describes what the skill does and when to use it. |
| `license` | No | License name or reference to a bundled license file. |
| `compatibility` | No | Maximum 500 characters. Indicates environment requirements such as intended product, system packages, or network access. |
| `metadata` | No | Arbitrary key-value mapping for additional metadata. |
| `allowed-tools` | No | Space-delimited list of pre-approved tools the skill may use. Experimental. |

Minimal example:

    ---
    name: skill-name
    description: A description of what this skill does and when to use it.
    ---

Example with optional fields:

    ---
    name: pdf-processing
    description: Extract PDF text, fill forms, merge files. Use when handling PDFs.
    license: Apache-2.0
    metadata:
      author: example-org
      version: "1.0"
    ---

### `name` field

The required `name` field:

- Must be 1-64 characters.
- May contain lowercase ASCII letters, numbers, and hyphens.
- Must not start or end with a hyphen.
- Must not contain consecutive hyphens.
- Must match the parent directory name.

Valid examples:

    name: pdf-processing
    name: data-analysis
    name: code-review

Invalid examples:

    name: PDF-Processing
    name: -pdf
    name: pdf--processing

### `description` field

The required `description` field:

- Must be 1-1024 characters.
- Should describe both what the skill does and when to use it.
- Should include keywords that help agents identify relevant tasks.

Good example:

    description: Extracts text and tables from PDF files, fills PDF forms, and merges multiple PDFs. Use when working with PDF documents or when the user mentions PDFs, forms, or document extraction.

Poor example:

    description: Helps with PDFs.

### `license` field

The optional `license` field:

- Specifies the license applied to the skill.
- The spec recommends a short value such as a license name or bundled license filename.

Example:

    license: Proprietary. LICENSE.txt has complete terms

### `compatibility` field

The optional `compatibility` field:

- Must be 1-500 characters if provided.
- Should be included only when the skill has specific environment requirements.
- Can describe intended product, required system packages, or network access.

Examples:

    compatibility: Designed for Claude Code (or similar products)
    compatibility: Requires git, docker, jq, and access to the internet
    compatibility: Requires Python 3.14+ and uv

Most skills do not need the `compatibility` field.

### `metadata` field

The optional `metadata` field:

- Is a mapping from string keys to string values.
- Lets clients store properties that are not part of the base Agent Skills spec.
- Should use reasonably unique key names to avoid conflicts.

Example:

    metadata:
      author: example-org
      version: "1.0"

### `allowed-tools` field

The optional `allowed-tools` field:

- Is a space-delimited list of tools that are pre-approved to run.
- Is marked experimental, and support may vary between agent implementations.

Example:

    allowed-tools: Bash(git:*) Bash(jq:*) Read

### Body content

The Markdown body after the front matter contains the skill instructions. The spec does not impose a format on the body. Recommended content includes:

- step-by-step instructions
- examples of inputs and outputs
- common edge cases

The page notes that the full `SKILL.md` body is loaded when a skill activates, so longer content should be split into referenced files when appropriate.

## Optional directories

### `scripts/`

Contains executable code that agents can run. The page recommends scripts that:

- are self-contained or clearly document dependencies
- include helpful error messages
- handle edge cases gracefully

Supported languages depend on the client implementation. Common examples include Python, Bash, and JavaScript.

### `references/`

Contains documentation agents can load on demand, such as:

- `REFERENCE.md`
- `FORMS.md`
- domain-specific files like `finance.md` or `legal.md`

The page recommends keeping individual reference files focused so agents load less context at a time.

### `assets/`

Contains static resources such as:

- templates
- images
- data files

## Progressive disclosure

Skills should be structured for efficient context use:

1. Metadata: the `name` and `description` fields are loaded at startup for all skills.
2. Instructions: the main `SKILL.md` body is loaded when the skill is activated.
3. Resources: files in `scripts/`, `references/`, or `assets/` are loaded only when needed.

The page recommends keeping the main `SKILL.md` under 500 lines and moving detailed material into separate files.

## File references

When referencing other files in a skill, use relative paths from the skill root.

Examples:

    See [the reference guide](references/REFERENCE.md) for details.

    Run the extraction script:
    scripts/extract.py

The page recommends keeping file references one level deep from `SKILL.md` and avoiding deeply nested reference chains.

## Validation

The page recommends the `skills-ref` reference library for validation:

    skills-ref validate ./my-skill

The validator checks that `SKILL.md` front matter is valid and follows the specification's naming conventions.
