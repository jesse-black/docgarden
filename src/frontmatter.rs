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
        if let YamlValue::Scalar(s) = self.get(key)? {
            Some(s.chars().count())
        } else {
            None
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

    while let Some(&(line_num, line)) = lines.get(i) {
        i += 1;

        // Skip empty lines and comment lines.
        let stripped = line.trim();
        if is_blank_or_comment(stripped) {
            continue;
        }

        // Top-level lines must have no leading whitespace.
        if line_indent(line) > 0 {
            return Err(line_num);
        }

        // Reject unsupported top-level constructs: sequences, multi-doc markers.
        if has_unsupported_top_level_prefix(stripped) {
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
        } else if let Some(chomp) = FoldedScalarMarker::parse(value_str) {
            let (val, consumed) = parse_folded_scalar(lines, i, line_num, chomp)?;
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
        for &(_, cl) in lines.iter().skip(start) {
            if is_blank_or_comment(cl.trim()) {
                continue;
            }
            let indent = line_indent(cl);
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

    for &(cln, cl) in lines.iter().skip(start) {
        if is_blank_or_comment(cl.trim()) {
            consumed += 1;
            continue;
        }
        let this_indent = line_indent(cl);
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
    let Some((_, first_line)) = child_lines.first() else {
        return Err(parent_line);
    };
    let first_trimmed = first_line.trim_start();
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
        if is_blank_or_comment(stripped) {
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
        if is_blank_or_comment(ct) {
            continue;
        }
        let actual_indent = line_indent(line);
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

struct FoldedScalarMarker;

impl FoldedScalarMarker {
    fn parse(value: &str) -> Option<Self> {
        match value {
            ">" | ">-" | ">+" => Some(Self),
            _ => None,
        }
    }
}

/// Parse a folded block scalar (`>`, `>-`, or `>+`) into a scalar string.
///
/// All accepted folded markers are intentionally normalized to stripped
/// semantics for docgarden frontmatter, so `>`, `>-`, and `>+` produce the
/// same scalar value.
fn parse_folded_scalar(
    lines: &[(usize, &str)],
    start: usize,
    parent_line: usize,
    _marker: FoldedScalarMarker,
) -> Result<(YamlValue, usize), usize> {
    let child_indent = folded_scalar_indent(lines, start).ok_or(parent_line)?;
    if child_indent == 0 {
        return Err(parent_line);
    }

    let mut content = Vec::new();
    let mut consumed = 0;

    for &(line_num, line) in lines.iter().skip(start) {
        if line.trim().is_empty() {
            if !content.is_empty() {
                content.push(None);
            }
            consumed += 1;
            continue;
        }

        let indent = line_indent(line);
        if indent < child_indent {
            break;
        }
        if indent > child_indent {
            return Err(line_num);
        }

        content.push(Some(line[child_indent..].trim().to_string()));
        consumed += 1;
    }

    Ok((YamlValue::Scalar(fold_scalar_content(&content)), consumed))
}

fn folded_scalar_indent(lines: &[(usize, &str)], start: usize) -> Option<usize> {
    lines.iter().skip(start).find_map(|&(_, line)| {
        if line.trim().is_empty() {
            None
        } else {
            Some(line_indent(line))
        }
    })
}

fn fold_scalar_content(lines: &[Option<String>]) -> String {
    let trailing_blank_lines = lines.iter().rev().take_while(|line| line.is_none()).count();
    let content_end = lines.len() - trailing_blank_lines;

    let mut output = String::new();
    let mut paragraph = Vec::new();

    for line in lines.iter().take(content_end) {
        match line {
            Some(text) => paragraph.push(text.as_str()),
            None => {
                flush_folded_paragraph(&mut output, &mut paragraph);
                output.push('\n');
            }
        }
    }

    flush_folded_paragraph(&mut output, &mut paragraph);
    output
}

fn flush_folded_paragraph(output: &mut String, paragraph: &mut Vec<&str>) {
    if paragraph.is_empty() {
        return;
    }
    output.push_str(&paragraph.join(" "));
    paragraph.clear();
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
    key.chars()
        .any(|ch| matches!(ch, '{' | '}' | '[' | ']' | '&' | '*' | '!'))
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
    if has_unsupported_scalar_prefix(value) {
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

fn is_blank_or_comment(value: &str) -> bool {
    value.is_empty() || value.starts_with('#')
}

fn line_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn has_unsupported_top_level_prefix(value: &str) -> bool {
    matches!(value, "-" | "...") || value.starts_with("- ")
}

fn has_unsupported_scalar_prefix(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|ch| matches!(ch, '{' | '[' | '&' | '*' | '!' | '|' | '>'))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid(src: &str) -> ParsedFrontmatter {
        let result = parse_from_str(src);
        if let FrontmatterParseResult::Valid(fm) = result {
            fm
        } else {
            panic!("expected Valid, got {result:?}");
        }
    }

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
        let fm = assert_valid(src);
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

    #[test]
    fn parse_quoted_string_strips_quotes() {
        let src = "---\ndescription: \"A quoted description.\"\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("description"),
            Some(&YamlValue::Scalar("A quoted description.".to_string()))
        );
    }

    #[test]
    fn parse_boolean_and_integer_scalars() {
        let src = "---\npublished: true\ncount: 42\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("published"),
            Some(&YamlValue::Scalar("true".to_string()))
        );
        assert_eq!(fm.get("count"), Some(&YamlValue::Scalar("42".to_string())));
    }

    #[test]
    fn parse_date_scalar() {
        let src = "---\nretrieved: 2026-04-01\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("retrieved"),
            Some(&YamlValue::Scalar("2026-04-01".to_string()))
        );
    }

    #[test]
    fn parse_sequence_value() {
        let src = "---\ntags:\n  - rust\n  - docs\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("tags"),
            Some(&YamlValue::Sequence(vec![
                YamlValue::Scalar("rust".to_string()),
                YamlValue::Scalar("docs".to_string()),
            ]))
        );
    }

    #[test]
    fn parse_nested_mapping() {
        let src = "---\nmetadata:\n  version: 1.0\n  status: draft\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("metadata"),
            Some(&YamlValue::Mapping(vec![
                ("version".to_string(), YamlValue::Scalar("1.0".to_string())),
                ("status".to_string(), YamlValue::Scalar("draft".to_string())),
            ]))
        );
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
    fn parse_malformed_top_level_markers() {
        for src in ["---\n-\n---\n", "---\n...\n---\n"] {
            assert!(matches!(
                parse_from_str(src),
                FrontmatterParseResult::Malformed { .. }
            ));
        }
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
    fn parse_folded_strip_description_scalar() {
        let src = "---\ndescription: >-\n  Claude Code generated skill\n  description frontmatter.\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("description"),
            Some(&YamlValue::Scalar(
                "Claude Code generated skill description frontmatter.".to_string()
            ))
        );
    }

    #[test]
    fn parse_folded_clip_description_scalar_uses_strip_semantics() {
        let src = "---\ndescription: >\n  Folded clip\n  description.\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("description"),
            Some(&YamlValue::Scalar("Folded clip description.".to_string()))
        );
    }

    #[test]
    fn parse_folded_keep_description_scalar_uses_strip_semantics() {
        let src = "---\ndescription: >+\n  Folded keep\n  description.\n\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("description"),
            Some(&YamlValue::Scalar("Folded keep description.".to_string()))
        );
    }

    #[test]
    fn parse_folded_strip_scalar_preserves_blank_line_as_paragraph_break() {
        let src = "---\ndescription: >-\n  First paragraph.\n\n  Second paragraph.\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("description"),
            Some(&YamlValue::Scalar(
                "First paragraph.\nSecond paragraph.".to_string()
            ))
        );
    }

    #[test]
    fn parse_folded_strip_scalar_skips_blank_lines_before_content() {
        let src = "---\ndescription: >-\n\n  First line\n  second line\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("description"),
            Some(&YamlValue::Scalar("First line second line".to_string()))
        );
    }

    #[test]
    fn parse_folded_strip_scalar_stops_at_next_top_level_field() {
        let src = "---\ndescription: >-\n  First line\n\nname: Guide\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("description"),
            Some(&YamlValue::Scalar("First line".to_string()))
        );
        assert_eq!(
            fm.get("name"),
            Some(&YamlValue::Scalar("Guide".to_string()))
        );
    }

    #[test]
    fn parse_malformed_folded_strip_scalar_without_content() {
        let src = "---\ndescription: >-\n---\n";
        assert!(matches!(
            parse_from_str(src),
            FrontmatterParseResult::Malformed { line: 2 }
        ));
    }

    #[test]
    fn parse_malformed_folded_strip_scalar_without_indented_content() {
        let src = "---\ndescription: >-\nname: Guide\n---\n";
        assert!(matches!(
            parse_from_str(src),
            FrontmatterParseResult::Malformed { line: 2 }
        ));
    }

    #[test]
    fn parse_malformed_folded_strip_scalar_with_deeper_indent() {
        let src = "---\ndescription: >-\n  First line\n    deeper line\n---\n";
        assert!(matches!(
            parse_from_str(src),
            FrontmatterParseResult::Malformed { line: 4 }
        ));
    }

    #[test]
    fn parse_later_triple_dash_is_body_content() {
        // Only the first --- ... --- block is frontmatter; later --- is body.
        let src = "---\ntitle: Hello\n---\n\n# Body\n\n---\nThis is a thematic break.\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("title"),
            Some(&YamlValue::Scalar("Hello".to_string()))
        );
    }

    #[test]
    fn parse_empty_frontmatter_block_is_valid() {
        let src = "---\n---\n# Body\n";
        let fm = assert_valid(src);
        assert!(fm.fields.is_empty());
    }

    #[test]
    fn parse_blank_value_at_end_of_block() {
        // key: with nothing following it and no more lines before closing ---
        let src = "---\ntitle: Hello\npublished:\n---\n";
        let fm = assert_valid(src);
        assert_eq!(
            fm.get("title"),
            Some(&YamlValue::Scalar("Hello".to_string()))
        );
        assert_eq!(fm.get("published"), Some(&YamlValue::Scalar(String::new())));
    }

    #[test]
    fn parse_blank_value_followed_by_another_key() {
        // key: with no value, followed immediately by another top-level key
        let src = "---\npublished:\nauthor: Alice\n---\n";
        let fm = assert_valid(src);
        assert_eq!(fm.get("published"), Some(&YamlValue::Scalar(String::new())));
        assert_eq!(
            fm.get("author"),
            Some(&YamlValue::Scalar("Alice".to_string()))
        );
    }

    #[test]
    fn scalar_char_count_measures_unicode_correctly() {
        let src = "---\ndescription: héllo\n---\n";
        let fm = assert_valid(src);
        // "héllo" is 5 chars (h, é, l, l, o)
        assert_eq!(fm.scalar_char_count("description"), Some(5));
    }
}
