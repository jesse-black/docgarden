use anyhow::Result;

use crate::config::{Config, Rule};
use crate::diagnostics::Severity;
use crate::frontmatter::{FrontmatterParseResult, parse_from_str};
use crate::lint::Finding;
use crate::lint::reporting::DiagnosticPayload;

fn missing_field_finding<'a>(file: &'a str, field: &str) -> Finding<'a> {
    Finding {
        payload: DiagnosticPayload {
            file,
            position: None,
            rule: Rule::FrontmatterFieldMissing,
            message: format!("Required frontmatter field `{field}` is missing."),
            fixable: false,
            severity: Severity::Error,
        },
        edit: None,
    }
}

pub(crate) struct FrontmatterRuleContext<'a> {
    pub(crate) config: &'a Config,
    pub(crate) file: &'a str,
    pub(crate) source: &'a str,
}

pub(crate) fn evaluate_frontmatter_rules<'a>(
    context: &FrontmatterRuleContext<'a>,
) -> Result<Vec<Finding<'a>>> {
    let policy = context.config.frontmatter_policy_for_path(context.file)?;

    // Nothing to check if no frontmatter policy applies.
    if policy.required.is_empty() && policy.field_max_chars.is_empty() {
        return Ok(Vec::new());
    }

    let mut findings = Vec::new();

    match parse_from_str(context.source) {
        FrontmatterParseResult::None => {
            // No frontmatter present.  Emit missing-field diagnostics for each
            // required field.
            for field in &policy.required {
                findings.push(missing_field_finding(context.file, field));
            }
        }
        FrontmatterParseResult::Malformed { line } => {
            findings.push(Finding {
                payload: DiagnosticPayload {
                    file: context.file,
                    position: None,
                    rule: Rule::FrontmatterMalformed,
                    message: format!(
                        "Frontmatter block is malformed or uses unsupported YAML constructs (line {line})."
                    ),
                    fixable: false,
                    severity: Severity::Error,
                },
                edit: None,
            });
        }
        FrontmatterParseResult::Valid(fm) => {
            // Check required fields.
            for field in &policy.required {
                if !fm.has_key(field) {
                    findings.push(missing_field_finding(context.file, field));
                }
            }
            // Check max_chars for fields that are present.
            for (field, max_chars) in &policy.field_max_chars {
                if let Some(char_count) = fm.scalar_char_count(field)
                    && char_count > *max_chars
                {
                    findings.push(Finding {
                        payload: DiagnosticPayload {
                            file: context.file,
                            position: None,
                            rule: Rule::FrontmatterFieldMaxChars,
                            message: format!(
                                "Frontmatter field `{field}` has {char_count} characters, \
                                 which exceeds configured max_chars = {max_chars}."
                            ),
                            fixable: false,
                            severity: Severity::Error,
                        },
                        edit: None,
                    });
                }
            }
        }
    }

    Ok(findings)
}
