use crate::diagnostics::{Diagnostic, Severity};

pub(crate) struct DiagnosticPayload<'a> {
    pub(crate) file: &'a str,
    pub(crate) position: Option<&'a markdown::unist::Position>,
    pub(crate) rule: &'a str,
    pub(crate) message: String,
    pub(crate) fixable: bool,
    pub(crate) severity: Severity,
}

pub(crate) fn push_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    ignored_rules: &std::collections::BTreeSet<String>,
    payload: DiagnosticPayload<'_>,
) {
    if ignored_rules.contains(payload.rule) {
        return;
    }
    let (line, column) = payload
        .position
        .map(|value| (value.start.line, value.start.column))
        .unwrap_or((1, 1));
    diagnostics.push(Diagnostic {
        file: payload.file.to_string(),
        line,
        column,
        severity: payload.severity,
        rule: payload.rule.to_string(),
        message: payload.message,
        fixable: payload.fixable,
    });
}
