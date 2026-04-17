mod cli;
mod config;
mod defaults;
mod diagnostics;
mod discover;
pub(crate) mod frontmatter;
mod lint;
mod matching;
mod paths;
mod root;
mod score;

pub use cli::run;
