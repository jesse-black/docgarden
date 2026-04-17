use std::collections::BTreeSet;

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::{Diagnostic, FixSummary, PatternMatcher, Severity};

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
            severity: Severity::Error,
            rule: "unresolved-link-path".to_string(),
            message: "broken link".to_string(),
            fixable: false,
        });

        assert_eq!(summary.fixable_count, 1);
        assert!(
            summary
                .fixable_rules
                .contains("prefer-links-for-local-paths")
        );
        assert!(!summary.fixable_rules.contains("unresolved-link-path"));
    }

    #[test]
    fn pattern_matcher_uses_gitignore_style_matching() {
        let matcher =
            PatternMatcher::new(&["docs/**/*.md".to_string(), "README.md".to_string()]).unwrap();

        assert!(matcher.is_match("docs/guide/intro.md", false));
        assert!(matcher.is_match("README.md", false));
        assert!(!matcher.is_match("src/main.rs", false));
    }
}
