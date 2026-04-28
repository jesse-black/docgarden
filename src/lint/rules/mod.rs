use crate::config::{Config, EffectiveRulePolicy};

pub(crate) mod file;
pub(crate) mod frontmatter;
pub(crate) mod local_paths;

pub(crate) struct NodeRuleContext<'a> {
    pub(crate) config: &'a Config,
    pub(crate) policy: &'a EffectiveRulePolicy,
    pub(crate) file: &'a str,
}
