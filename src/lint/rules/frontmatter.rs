use anyhow::Result;

use crate::config::Config;
use crate::diagnostics::Severity;
use crate::lint::Finding;
use crate::lint::reporting::DiagnosticPayload;

// ---------------------------------------------------------------------------
// YAML value representation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum YamlValue {
    Scalar(String),
    Sequence(Vec<YamlValue>),
    Mapping(Vec<(String, YamlValue)>),
}

// ---------------------------------------------------------------------------
// Parsed frontmatter
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedFrontmatter {
    /// Top-level fields in document order.
    pub fields: Vec<(String, YamlValue)>,
}

impl ParsedFrontmatter {
    pub fn get(&self, key: &str) -> Option<&YamlValue> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn has_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Returns the char count of a top-level scalar field, if present.
    pub fn scalar_char_count(&self, key: &str) -> Option<usize> {
        match self.get(key)? {
            YamlValue::Scalar(s) => Some(s.chars().count()),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Parse result
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum FrontmatterParseResult {
    /// File does not begin with a `---` frontmatter block.
    None,
    /// A valid frontmatter block was parsed.
    Valid(ParsedFrontmatter),
    /// A leading `---` was found but the block is malformed or uses
    /// unsupported YAML constructs.  The line number (1-indexed) points
    /// to the first problematic line.
    Malformed { line: usize },
}

// ---------------------------------------------------------------------------
// Public entry point: parse from a full document string
// ---------------------------------------------------------------------------

/// Parse YAML frontmatter from a full document string.
///
/// Frontmatter is present only when:
/// - The document begins with a line that is exactly `---`.
/// - A closing line that is exactly `---` appears before any body content.
///
/// Any later `---` in the body is ordinary Markdown content.
pub fn parse_from_str(source: &str) -> FrontmatterParseResult {
    let mut lines = source.lines().enumerate().peekable();

    // First line must be exactly "---".
    match lines.next() {
        Some((_, "---")) => {}
        _ => return FrontmatterParseResult::None,
    }

    let mut content: Vec<(usize, &str)> = Vec::new();
    let mut found_close = false;

    for (idx, line) in lines {
        let line_num = idx + 1; // 1-indexed
        if line == "---" {
            found_close = true;
            break;
        }
        content.push((line_num, line));
    }

    if !found_close {
        // Unclosed frontmatter block.
        let error_line = content.last().map(|(n, _)| *n).unwrap_or(1);
        return FrontmatterParseResult::Malformed { line: error_line };
    }

    match parse_yaml_block(&content) {
        Ok(fields) => FrontmatterParseResult::Valid(ParsedFrontmatter { fields }),
        Err(line) => FrontmatterParseResult::Malformed { line },
    }
}

// ---------------------------------------------------------------------------
// Internal YAML block parser
// ---------------------------------------------------------------------------

/// Parse the lines between the two `---` delimiters.
///
/// Returns the top-level fields, or the 1-indexed line number of the first
/// parse error.
fn parse_yaml_block(lines: &[(usize, &str)]) -> Result<Vec<(String, YamlValue)>, usize> {
    let mut fields: Vec<(String, YamlValue)> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let (line_num, line) = lines[i];
        i += 1;

        // Skip empty lines and comment lines.
        let trimmed = line.trim_end();
        let stripped = trimmed.trim();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        // Top-level lines must have no leading whitespace.
        let leading = line.len() - line.trim_start().len();
        if leading > 0 {
            return Err(line_num);
        }

        // Reject unsupported top-level constructs: sequences, multi-doc markers.
        if stripped.starts_with("- ") || stripped == "-" {
            return Err(line_num);
        }
        if stripped == "..." {
            return Err(line_num);
        }

        // Parse `key: value` or `key:`.
        let (key, value_str) = split_key_value(stripped).ok_or(line_num)?;
        check_mapping_key(key, &fields, line_num)?;

        let value = if value_str.is_empty() {
            // Block value: child lines follow.
            let (val, consumed) = parse_block_value(lines, i, line_num)?;
            i += consumed;
            val
        } else {
            // Inline scalar.
            parse_inline_scalar(value_str).ok_or(line_num)?
        };

        fields.push((key.to_string(), value));
    }

    Ok(fields)
}

/// Parse a block value that follows a bare `key:` line.
///
/// Returns `(value, lines_consumed)`.
fn parse_block_value(
    lines: &[(usize, &str)],
    start: usize,
    parent_line: usize,
) -> Result<(YamlValue, usize), usize> {
    // Find the first non-empty, non-comment child line to determine indent.
    let child_indent = {
        let mut ci = None;
        for j in start..lines.len() {
            let (_, cl) = lines[j];
            let ct = cl.trim();
            if ct.is_empty() || ct.starts_with('#') {
                continue;
            }
            let indent = cl.len() - cl.trim_start().len();
            if indent == 0 {
                // Next top-level line reached before any indented children: blank value.
                return Ok((YamlValue::Scalar(String::new()), 0));
            }
            ci = Some(indent);
            break;
        }
        ci
    };

    let Some(child_indent) = child_indent else {
        // End of block with no children: blank value.
        return Ok((YamlValue::Scalar(String::new()), 0));
    };

    // Collect child lines at exactly child_indent level.
    let mut child_lines: Vec<(usize, &str)> = Vec::new();
    let mut consumed = 0;

    for j in start..lines.len() {
        let (cln, cl) = lines[j];
        let ct = cl.trim();
        if ct.is_empty() || ct.starts_with('#') {
            consumed += 1;
            continue;
        }
        let this_indent = cl.len() - cl.trim_start().len();
        if this_indent < child_indent {
            break; // back to parent scope
        }
        if this_indent > child_indent {
            return Err(cln); // unexpected deeper indent
        }
        child_lines.push((cln, cl));
        consumed += 1;
    }

    if child_lines.is_empty() {
        return Err(parent_line);
    }

    // Determine whether this is a sequence or a nested mapping.
    let first_trimmed = child_lines[0].1.trim_start();
    let value = if first_trimmed.starts_with("- ") || first_trimmed == "-" {
        parse_sequence(&child_lines)?
    } else {
        let sub_fields = parse_nested_mapping(&child_lines, child_indent)?;
        YamlValue::Mapping(sub_fields)
    };

    Ok((value, consumed))
}

/// Parse sequence items from child lines.
fn parse_sequence(lines: &[(usize, &str)]) -> Result<YamlValue, usize> {
    let mut items = Vec::new();
    for (line_num, line) in lines {
        let stripped = line.trim_start();
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        if !stripped.starts_with("- ") {
            return Err(*line_num);
        }
        let item_str = stripped[2..].trim();
        if item_str.is_empty() {
            return Err(*line_num);
        }
        let item = parse_inline_scalar(item_str).ok_or(*line_num)?;
        items.push(item);
    }
    Ok(YamlValue::Sequence(items))
}

/// Parse a single-level nested mapping from child lines at the given indent.
fn parse_nested_mapping(
    lines: &[(usize, &str)],
    indent: usize,
) -> Result<Vec<(String, YamlValue)>, usize> {
    let mut fields: Vec<(String, YamlValue)> = Vec::new();
    for (line_num, line) in lines {
        let ct = line.trim();
        if ct.is_empty() || ct.starts_with('#') {
            continue;
        }
        let actual_indent = line.len() - line.trim_start().len();
        if actual_indent != indent {
            return Err(*line_num);
        }
        let (key, value_str) = split_key_value(ct).ok_or(*line_num)?;
        check_mapping_key(key, &fields, *line_num)?;
        // Nested-nested block values are not supported in v1.
        if value_str.is_empty() {
            return Err(*line_num);
        }
        let value = parse_inline_scalar(value_str).ok_or(*line_num)?;
        fields.push((key.to_string(), value));
    }
    Ok(fields)
}

// ---------------------------------------------------------------------------
// Scalar parsing helpers
// ---------------------------------------------------------------------------

/// Split a `key: value` line.  The key is the part before the first `:`,
/// the value is the trimmed part after it (may be empty).
fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let colon = line.find(':')?;
    let key = &line[..colon];
    let rest = line[colon + 1..].trim();
    Some((key, rest))
}

/// Return `true` if the key contains characters that indicate unsupported
/// YAML constructs (anchors, aliases, tags, flow collections).
fn has_unsupported_key_chars(key: &str) -> bool {
    key.contains('{')
        || key.contains('}')
        || key.contains('[')
        || key.contains(']')
        || key.contains('&')
        || key.contains('*')
        || key.contains('!')
}

/// Validate a mapping key against the accumulated fields for duplicates.
///
/// Returns `Err(line_num)` if the key is empty, contains unsupported
/// characters, or duplicates a key already in `fields`.
fn check_mapping_key(
    key: &str,
    fields: &[(String, YamlValue)],
    line_num: usize,
) -> Result<(), usize> {
    if key.is_empty() {
        return Err(line_num);
    }
    if has_unsupported_key_chars(key) {
        return Err(line_num);
    }
    if fields.iter().any(|(k, _)| k == key) {
        return Err(line_num); // duplicate key
    }
    Ok(())
}

/// Parse an inline scalar value.  Returns `None` for unsupported constructs.
fn parse_inline_scalar(value: &str) -> Option<YamlValue> {
    // Reject flow-style collections, anchors, aliases, tags, block scalars.
    if value.starts_with('{')
        || value.starts_with('[')
        || value.starts_with('&')
        || value.starts_with('*')
        || value.starts_with('!')
        || value.starts_with('|')
        || value.starts_with('>')
    {
        return None;
    }

    // Strip double-quoted strings.
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        return Some(YamlValue::Scalar(value[1..value.len() - 1].to_string()));
    }
    // Strip single-quoted strings.
    if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        return Some(YamlValue::Scalar(value[1..value.len() - 1].to_string()));
    }

    Some(YamlValue::Scalar(value.to_string()))
}

// ---------------------------------------------------------------------------
// Lint rule evaluation helpers
// ---------------------------------------------------------------------------

fn missing_field_finding<'a>(file: &'a str, field: &str) -> Finding<'a> {
    Finding {
        payload: DiagnosticPayload {
            file,
            position: None,
            rule: "frontmatter-field-missing",
            message: format!("Required frontmatter field `{field}` is missing."),
            fixable: false,
            severity: Severity::Error,
        },
        edit: None,
    }
}

// ---------------------------------------------------------------------------
// Lint rule evaluation
// ---------------------------------------------------------------------------

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
                    rule: "frontmatter-malformed",
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
                if let Some(char_count) = fm.scalar_char_count(field) {
                    if char_count > *max_chars {
                        findings.push(Finding {
                            payload: DiagnosticPayload {
                                file: context.file,
                                position: None,
                                rule: "frontmatter-field-max-chars",
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
    }

    Ok(findings)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Parser tests ---

    #[test]
    fn parse_returns_none_when_no_frontmatter() {
        assert_eq!(
            parse_from_str("# Hello\n\nBody."),
            FrontmatterParseResult::None
        );
        assert_eq!(parse_from_str(""), FrontmatterParseResult::None);
        assert_eq!(parse_from_str("--\n---\n"), FrontmatterParseResult::None);
    }

    #[test]
    fn parse_simple_scalar_fields() {
        let src = "---\ntitle: My Doc\nauthor: Alice\n---\n# Body\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(
                    fm.get("title"),
                    Some(&YamlValue::Scalar("My Doc".to_string()))
                );
                assert_eq!(
                    fm.get("author"),
                    Some(&YamlValue::Scalar("Alice".to_string()))
                );
                assert!(fm.get("missing").is_none());
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_quoted_string_strips_quotes() {
        let src = "---\ndescription: \"A quoted description.\"\n---\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(
                    fm.get("description"),
                    Some(&YamlValue::Scalar("A quoted description.".to_string()))
                );
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_boolean_and_integer_scalars() {
        let src = "---\npublished: true\ncount: 42\n---\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(
                    fm.get("published"),
                    Some(&YamlValue::Scalar("true".to_string()))
                );
                assert_eq!(fm.get("count"), Some(&YamlValue::Scalar("42".to_string())));
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_date_scalar() {
        let src = "---\nretrieved: 2026-04-01\n---\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(
                    fm.get("retrieved"),
                    Some(&YamlValue::Scalar("2026-04-01".to_string()))
                );
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_sequence_value() {
        let src = "---\ntags:\n  - rust\n  - docs\n---\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(
                    fm.get("tags"),
                    Some(&YamlValue::Sequence(vec![
                        YamlValue::Scalar("rust".to_string()),
                        YamlValue::Scalar("docs".to_string()),
                    ]))
                );
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_nested_mapping() {
        let src = "---\nmetadata:\n  version: 1.0\n  status: draft\n---\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(
                    fm.get("metadata"),
                    Some(&YamlValue::Mapping(vec![
                        ("version".to_string(), YamlValue::Scalar("1.0".to_string())),
                        ("status".to_string(), YamlValue::Scalar("draft".to_string())),
                    ]))
                );
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_malformed_unclosed_block() {
        let src = "---\ntitle: Hello\n# no closing ---\n# body starts\n";
        assert!(matches!(
            parse_from_str(src),
            FrontmatterParseResult::Malformed { .. }
        ));
    }

    #[test]
    fn parse_malformed_duplicate_key() {
        let src = "---\ntitle: A\ntitle: B\n---\n";
        assert!(matches!(
            parse_from_str(src),
            FrontmatterParseResult::Malformed { .. }
        ));
    }

    #[test]
    fn parse_malformed_flow_collection() {
        let src = "---\ntags: [a, b]\n---\n";
        assert!(matches!(
            parse_from_str(src),
            FrontmatterParseResult::Malformed { .. }
        ));
    }

    #[test]
    fn parse_malformed_flow_mapping() {
        let src = "---\nmeta: {key: val}\n---\n";
        assert!(matches!(
            parse_from_str(src),
            FrontmatterParseResult::Malformed { .. }
        ));
    }

    #[test]
    fn parse_malformed_block_scalar() {
        let src = "---\ndescription: |\n  Multi-line\n  text here\n---\n";
        assert!(matches!(
            parse_from_str(src),
            FrontmatterParseResult::Malformed { .. }
        ));
    }

    #[test]
    fn parse_later_triple_dash_is_body_content() {
        // Only the first --- ... --- block is frontmatter; later --- is body.
        let src = "---\ntitle: Hello\n---\n\n# Body\n\n---\nThis is a thematic break.\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(
                    fm.get("title"),
                    Some(&YamlValue::Scalar("Hello".to_string()))
                );
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_empty_frontmatter_block_is_valid() {
        let src = "---\n---\n# Body\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert!(fm.fields.is_empty());
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_blank_value_at_end_of_block() {
        // key: with nothing following it and no more lines before closing ---
        let src = "---\ntitle: Hello\npublished:\n---\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(
                    fm.get("title"),
                    Some(&YamlValue::Scalar("Hello".to_string()))
                );
                assert_eq!(fm.get("published"), Some(&YamlValue::Scalar(String::new())));
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn parse_blank_value_followed_by_another_key() {
        // key: with no value, followed immediately by another top-level key
        let src = "---\npublished:\nauthor: Alice\n---\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                assert_eq!(fm.get("published"), Some(&YamlValue::Scalar(String::new())));
                assert_eq!(
                    fm.get("author"),
                    Some(&YamlValue::Scalar("Alice".to_string()))
                );
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }

    #[test]
    fn scalar_char_count_measures_unicode_correctly() {
        let src = "---\ndescription: héllo\n---\n";
        match parse_from_str(src) {
            FrontmatterParseResult::Valid(fm) => {
                // "héllo" is 5 chars (h, é, l, l, o)
                assert_eq!(fm.scalar_char_count("description"), Some(5));
            }
            other => panic!("expected Valid, got {other:?}"),
        }
    }
}
