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
    static TOKENIZER: OnceLock<Result<CoreBPE>> = OnceLock::new();

    cached_result(&TOKENIZER, o200k_base)
        .as_ref()
        .map_err(|error| anyhow!("{error}"))
}

fn cached_result<T, E>(
    cache: &OnceLock<Result<T, E>>,
    init: impl FnOnce() -> Result<T, E>,
) -> &Result<T, E> {
    cache.get_or_init(init)
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::{cached_result, count_tokens, tokenizer};

    #[test]
    fn tokenizer_is_cached_across_calls() {
        let first = tokenizer().unwrap() as *const _;
        let second = tokenizer().unwrap() as *const _;

        assert_eq!(first, second);
        assert!(count_tokens("hello world").unwrap() > 0);
    }

    #[test]
    fn cached_result_initializes_once_and_reuses_cached_value() {
        let cache = OnceLock::new();

        let first = cached_result(&cache, || Ok::<_, ()>(41)).as_ref().unwrap();
        let second = cached_result(&cache, || Ok::<_, ()>(99)).as_ref().unwrap();

        assert_eq!(*first, 41);
        assert_eq!(*second, 41);
    }
}
