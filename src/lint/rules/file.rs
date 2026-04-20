use std::sync::OnceLock;

use anyhow::{Result, anyhow};
use tiktoken_rs::{CoreBPE, o200k_base};

use crate::config::Config;
use crate::lint::reporting::DiagnosticPayload;

use super::super::{FilePolicy, Finding};

pub(crate) struct FileRuleContext<'a> {
    pub(crate) config: &'a Config,
    pub(crate) policy: FilePolicy,
    pub(crate) file: &'a str,
    pub(crate) source: &'a str,
}

pub(crate) fn evaluate_file_rules<'a>(context: &FileRuleContext<'a>) -> Result<Vec<Finding<'a>>> {
    let _ = context.config;
    let mut findings = Vec::new();

    if let Some(limit) = context.policy.max_tokens {
        let observed = count_tokens(context.source)?;
        if observed > limit.limit {
            findings.push(Finding {
                payload: DiagnosticPayload {
                    file: context.file,
                    position: None,
                    rule: "max_tokens",
                    message: format!(
                        "File has {observed} tokens, which exceeds configured max_tokens = {}.",
                        limit.limit
                    ),
                    fixable: false,
                    severity: limit.severity.into(),
                },
                edit: None,
            });
        }
    }

    if let Some(limit) = context.policy.max_lines {
        let observed = count_lines(context.source);
        if observed > limit.limit {
            findings.push(Finding {
                payload: DiagnosticPayload {
                    file: context.file,
                    position: None,
                    rule: "max_lines",
                    message: format!(
                        "File has {observed} lines, which exceeds configured max_lines = {}.",
                        limit.limit
                    ),
                    fixable: false,
                    severity: limit.severity.into(),
                },
                edit: None,
            });
        }
    }

    Ok(findings)
}

fn count_tokens(source: &str) -> Result<usize> {
    let tokenizer = tokenizer()?;
    Ok(tokenizer.encode_ordinary(source).len())
}

fn count_lines(source: &str) -> usize {
    source.lines().count()
}

fn tokenizer() -> Result<&'static CoreBPE> {
    static TOKENIZER: OnceLock<Result<CoreBPE, String>> = OnceLock::new();

    match TOKENIZER.get_or_init(|| o200k_base().map_err(|error| error.to_string())) {
        Ok(tokenizer) => Ok(tokenizer),
        Err(error) => Err(anyhow!(error.clone())),
    }
}
