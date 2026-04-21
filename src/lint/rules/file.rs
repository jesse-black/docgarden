use std::sync::OnceLock;

use anyhow::{Result, anyhow};
use tiktoken_rs::{CoreBPE, o200k_base};

use crate::lint::reporting::DiagnosticPayload;

use super::super::{FilePolicy, Finding};

pub(crate) struct FileRuleContext<'a> {
    pub(crate) policy: FilePolicy,
    pub(crate) file: &'a str,
    pub(crate) source: &'a str,
}

pub(crate) fn evaluate_file_rules<'a>(context: &FileRuleContext<'a>) -> Result<Vec<Finding<'a>>> {
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

    match TOKENIZER.get_or_init(o200k_base).as_ref() {
        Ok(tokenizer) => Ok(tokenizer),
        Err(error) => Err(anyhow!("{error}")),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        OnceLock,
        atomic::{AtomicUsize, Ordering},
    };

    use super::{count_tokens, tokenizer};

    #[test]
    fn tokenizer_is_cached_across_calls() {
        let first = tokenizer().unwrap() as *const _;
        let second = tokenizer().unwrap() as *const _;

        assert_eq!(first, second);
        assert!(count_tokens("hello world").unwrap() > 0);
    }

    #[test]
    fn once_lock_initializes_once_and_reuses_cached_value() {
        let cache = OnceLock::new();
        let init_calls = AtomicUsize::new(0);

        let first = cache
            .get_or_init(|| {
                init_calls.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(41)
            })
            .as_ref()
            .unwrap();
        let second = cache.get().unwrap().as_ref().unwrap();

        assert_eq!(*first, 41);
        assert_eq!(*second, 41);
        assert_eq!(init_calls.load(Ordering::SeqCst), 1);
    }
}
