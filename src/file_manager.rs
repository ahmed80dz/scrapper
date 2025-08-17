use crate::error::{ScrapperError, ScrapperResult};

#[derive(Debug, Default)]
pub struct FileManagerStats {
    pub total_files: usize,
    pub empty_files: usize,
    pub small_files: usize,
    pub total_size: u64,
}

impl FileManagerStats {
    pub fn valid_files(&self) -> usize {
        self.total_files - self.empty_files
    }

    pub fn average_file_size(&self) -> f64 {
        if self.valid_files() == 0 {
            0.0
        } else {
            self.total_size as f64 / self.valid_files() as f64
        }
    }
}

#[derive(Debug, Default)]
pub struct CleanupStats {
    pub total_checked: usize,
    pub removed_empty: usize,
    pub removed_small: usize,
}

impl CleanupStats {
    pub fn total_removed(&self) -> usize {
        self.removed_empty + self.removed_small
    }
}
use crate::types::ChapterRecord;
use std::path::{Path, PathBuf};
use tokio::fs;

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
        let path = self.get_chapter_path(record);
        path.exists() && self.is_file_valid(&path)
    }

    pub fn get_chapter_path(&self, record: &ChapterRecord) -> PathBuf {
        self.output_dir.join(record.file_name())
    }

    pub async fn ensure_output_dir_exists(&self) -> ScrapperResult<()> {
        if !self.output_dir.exists() {
            fs::create_dir_all(&self.output_dir).await.map_err(|e| {
                ScrapperError::file_system(
                    format!("Failed to create output directory: {e}"),
                    Some(self.output_dir.clone()),
                )
            })?;
        }
        Ok(())
    }

    pub fn output_dir(&self) -> &Path {
        &self.output_dir
    }

    /// Check if a file exists and has content (not empty)
    fn is_file_valid(&self, path: &Path) -> bool {
        if let Ok(metadata) = std::fs::metadata(path) {
            metadata.is_file() && metadata.len() > 0
        } else {
            false
        }
    }

    /// Get information about existing files in the output directory
    pub async fn get_existing_files_info(&self) -> ScrapperResult<FileManagerStats> {
        let mut stats = FileManagerStats::default();

        if !self.output_dir.exists() {
            return Ok(stats);
        }

        let mut entries = fs::read_dir(&self.output_dir).await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to read output directory: {e}"),
                Some(self.output_dir.clone()),
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to read directory entry: {e}"),
                Some(self.output_dir.clone()),
            )
        })? {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("chapter_") && file_name.ends_with(".txt") {
                    let metadata = entry.metadata().await.map_err(|e| {
                        ScrapperError::file_system(
                            format!("Failed to read file metadata: {e}"),
                            Some(path.clone()),
                        )
                    })?;

                    stats.total_files += 1;
                    stats.total_size += metadata.len();

                    if metadata.len() == 0 {
                        stats.empty_files += 1;
                    }

                    if metadata.len() < 100 {
                        stats.small_files += 1;
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Clean up empty or invalid chapter files
    pub async fn cleanup_invalid_files(&self) -> ScrapperResult<CleanupStats> {
        let mut stats = CleanupStats::default();

        if !self.output_dir.exists() {
            return Ok(stats);
        }

        let mut entries = fs::read_dir(&self.output_dir).await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to read output directory for cleanup: {e}"),
                Some(self.output_dir.clone()),
            )
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to read directory entry during cleanup: {e}"),
                Some(self.output_dir.clone()),
            )
        })? {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("chapter_") && file_name.ends_with(".txt") {
                    let metadata = entry.metadata().await.map_err(|e| {
                        ScrapperError::file_system(
                            format!("Failed to read file metadata during cleanup: {e}"),
                            Some(path.clone()),
                        )
                    })?;

                    stats.total_checked += 1;

                    // Remove empty files
                    if metadata.len() == 0 {
                        fs::remove_file(&path).await.map_err(|e| {
                            ScrapperError::file_system(
                                format!("Failed to remove empty file: {e}"),
                                Some(path.clone()),
                            )
                        })?;
                        stats.removed_empty += 1;
                    }
                    // Optionally remove very small files (likely failed scrapes)
                    else if metadata.len() < 50 {
                        // Check if content looks like an error message
                        if let Ok(content) = fs::read_to_string(&path).await {
                            if content.trim().is_empty() || content.len() < 50 {
                                fs::remove_file(&path).await.map_err(|e| {
                                    ScrapperError::file_system(
                                        format!("Failed to remove small invalid file: {e}"),
                                        Some(path.clone()),
                                    )
                                })?;
                                stats.removed_small += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Validate that the output directory is writable
    pub async fn validate_output_dir(&self) -> ScrapperResult<()> {
        // Ensure directory exists
        self.ensure_output_dir_exists().await?;

        // Test if we can write to the directory
        let test_file = self.output_dir.join(".test_write_permission");

        match fs::write(&test_file, "test").await {
            Ok(_) => {
                // Clean up test file
                if let Err(e) = fs::remove_file(&test_file).await {
                    eprintln!("Warning: Failed to clean up test file: {e}");
                }
                Ok(())
            }
            Err(e) => Err(ScrapperError::file_system(
                format!("Output directory is not writable: {e}"),
                Some(self.output_dir.clone()),
            )),
        }
    }
}
