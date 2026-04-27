use std::path::PathBuf;

use anyhow::Result;

use crate::config::Config;
use crate::documents::{load_document_metadata_for_paths, render_metadata_row};

pub(crate) fn print_list_rows(
    config: &Config,
    files: Vec<PathBuf>,
    explicit_file_targets: &[PathBuf],
) -> Result<()> {
    let documents = load_document_metadata_for_paths(config, &files)?;
    for (path, document) in files.iter().zip(documents) {
        if document.description.is_none() && !explicit_file_targets.contains(path) {
            continue;
        }
        println!("{}", render_metadata_row(&document));
    }
    Ok(())
}
