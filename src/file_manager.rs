use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::types::ChapterRecord;

pub struct FileManager {
    output_dir: PathBuf,
}

impl FileManager {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    pub fn chapter_exists(&self, record: &ChapterRecord) -> bool {
        self.get_chapter_path(record).exists()
    }

    pub fn get_chapter_path(&self, record: &ChapterRecord) -> PathBuf {
        self.output_dir.join(&record.file_name())
    }

    pub async fn ensure_output_dir_exists(&self) -> Result<()> {
        if !self.output_dir.exists() {
            fs::create_dir_all(&self.output_dir)
                .await
                .with_context(|| {
                    format!("Failed to create output directory: {:?}", self.output_dir)
                })?;
        }
        Ok(())
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }
}
