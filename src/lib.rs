mod cli;
mod config;
mod defaults;
mod diagnostics;
mod discover;
mod documents;
pub(crate) mod frontmatter;
mod lint;
mod listing;
mod matching;
mod paths;
mod root;
mod scopes;
mod score;

pub use cli::run;
