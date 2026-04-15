use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
    pub rule: String,
    pub message: String,
    pub fixable: bool,
}

#[derive(Debug, Default)]
pub struct FixSummary {
    pub fixable_count: usize,
    pub fixable_rules: BTreeSet<String>,
}

impl FixSummary {
    pub fn record(&mut self, diagnostic: &Diagnostic) {
        if diagnostic.fixable {
            self.fixable_count += 1;
            self.fixable_rules.insert(diagnostic.rule.clone());
        }
    }
}

pub struct PatternMatcher {
    matcher: Gitignore,
}

impl PatternMatcher {
    pub fn new(patterns: &[String]) -> Result<Self> {
        let mut builder = GitignoreBuilder::new("/");
        for pattern in patterns {
            builder
                .add_line(None, pattern)
                .with_context(|| format!("invalid pattern {pattern}"))?;
        }
        let matcher = builder.build()?;
        Ok(Self { matcher })
    }

    pub fn is_match(&self, path: &str, is_dir: bool) -> bool {
        self.matcher.matched(path, is_dir).is_ignore()
    }
}

pub fn ignored_rules_for_path(
    root: &Path,
    rules: &BTreeMap<String, Vec<String>>,
    relative_path: &str,
) -> Result<BTreeSet<String>> {
    let mut ignored = BTreeSet::new();
    for (pattern, entries) in rules {
        let mut builder = GitignoreBuilder::new(root);
        builder
            .add_line(None, pattern)
            .with_context(|| format!("invalid ignored-rule pattern {pattern}"))?;
        let matcher = builder.build()?;
        if matcher
            .matched_path_or_any_parents(Path::new(relative_path), false)
            .is_ignore()
        {
            for entry in entries {
                ignored.insert(entry.clone());
            }
        }
    }
    Ok(ignored)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use super::{Diagnostic, FixSummary, PatternMatcher, Severity, ignored_rules_for_path};

    #[test]
    fn fix_summary_counts_only_fixable_diagnostics() {
        let mut summary = FixSummary::default();
        summary.record(&Diagnostic {
            file: "docs/guide.md".to_string(),
            line: 1,
            column: 1,
            severity: Severity::Error,
            rule: "prefer-links-for-local-paths".to_string(),
            message: "rewrite".to_string(),
            fixable: true,
        });
        summary.record(&Diagnostic {
            file: "docs/guide.md".to_string(),
            line: 2,
            column: 1,
            severity: Severity::Warning,
            rule: "ambiguous-inline-code".to_string(),
            message: "warn".to_string(),
            fixable: false,
        });

        assert_eq!(summary.fixable_count, 1);
        assert!(
            summary
                .fixable_rules
                .contains("prefer-links-for-local-paths")
        );
        assert!(!summary.fixable_rules.contains("ambiguous-inline-code"));
    }

    #[test]
    fn pattern_matcher_uses_gitignore_style_matching() {
        let matcher =
            PatternMatcher::new(&["docs/**/*.md".to_string(), "README.md".to_string()]).unwrap();

        assert!(matcher.is_match("docs/guide/intro.md", false));
        assert!(matcher.is_match("README.md", false));
        assert!(!matcher.is_match("src/main.rs", false));
    }

    #[test]
    fn ignored_rules_merge_entries_for_matching_path() {
        let mut rules = BTreeMap::new();
        rules.insert(
            "docs/**/*.md".to_string(),
            vec![
                "ambiguous-inline-code".to_string(),
                "prefer-links-for-local-paths".to_string(),
            ],
        );
        rules.insert(
            "docs/guide.md".to_string(),
            vec!["unresolved-local-path".to_string()],
        );

        let ignored =
            ignored_rules_for_path(Path::new("/tmp/repo"), &rules, "docs/guide.md").unwrap();

        assert_eq!(ignored.len(), 3);
        assert!(ignored.contains("ambiguous-inline-code"));
        assert!(ignored.contains("prefer-links-for-local-paths"));
        assert!(ignored.contains("unresolved-local-path"));
    }

    #[test]
    fn ignored_rules_skip_non_matching_patterns() {
        let mut rules = BTreeMap::new();
        rules.insert(
            "docs/**/*.md".to_string(),
            vec!["unresolved-local-path".to_string()],
        );

        let ignored = ignored_rules_for_path(Path::new("/tmp/repo"), &rules, "README.md").unwrap();

        assert!(ignored.is_empty());
    }

    #[test]
    fn ignored_rules_match_descendants_of_directory_patterns() {
        let mut rules = BTreeMap::new();
        rules.insert(
            "docs/references".to_string(),
            vec!["unresolved-local-path".to_string()],
        );

        let ignored =
            ignored_rules_for_path(Path::new("/tmp/repo"), &rules, "docs/references/source.md")
                .unwrap();

        assert!(ignored.contains("unresolved-local-path"));
    }

    #[test]
    fn ignored_rules_report_invalid_patterns() {
        let mut rules = BTreeMap::new();
        rules.insert(
            "{docs,README.md".to_string(),
            vec!["unresolved-local-path".to_string()],
        );

        let error = ignored_rules_for_path(Path::new("/tmp/repo"), &rules, "README.md")
            .unwrap_err()
            .to_string();

        assert!(error.contains("invalid ignored-rule pattern {docs,README.md"));
    }
}
