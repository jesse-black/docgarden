use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::Result;

use crate::config::Config;
use crate::documents::{load_document_metadata_for_paths, render_metadata_row};

#[derive(Default)]
pub(crate) struct ListRenderOptions {
    pub(crate) allow_undescribed_paths: BTreeSet<PathBuf>,
}

pub(crate) fn print_list_rows(
    config: &Config,
    files: Vec<PathBuf>,
    options: &ListRenderOptions,
) -> Result<()> {
    let documents = load_document_metadata_for_paths(config, &files)?;
    for document in documents {
        if document.description.is_none()
            && !options
                .allow_undescribed_paths
                .contains(&document.absolute_path)
        {
            continue;
        }
        println!("{}", render_metadata_row(&document));
    }
    Ok(())
}
