use crate::error::{ScrapperError, ScrapperResult};
use crate::types::{ChapterRecord, ScrapingStats};
use csv_async::AsyncReader;
use std::path::Path;
use tokio::fs::File;
use tokio_stream::StreamExt;

pub struct CsvReader {
    file_path: std::path::PathBuf,
}

impl CsvReader {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
        }
    }

    pub async fn read_records(&self) -> ScrapperResult<Vec<ChapterRecord>> {
        let file = File::open(&self.file_path).await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to open CSV file: {e}"),
                Some(self.file_path.clone()),
            )
        })?;

        let mut reader = AsyncReader::from_reader(file);
        let mut records = reader.records();
        let mut chapter_records = Vec::new();
        let mut line_number = 1; // Track line number for better error reporting

        while let Some(record) = records.next().await {
            let record = record.map_err(|e| {
                ScrapperError::csv(format!(
                    "Failed to read CSV record at line {line_number}: {e}"
                ))
            })?;

            let url = record
                .get(0)
                .ok_or_else(|| {
                    ScrapperError::csv(format!("Missing URL column in CSV at line {line_number}"))
                })?
                .trim()
                .to_string();

            let chapter_number = record
                .get(1)
                .ok_or_else(|| {
                    ScrapperError::csv(format!(
                        "Missing chapter number column in CSV at line {line_number}"
                    ))
                })?
                .trim()
                .to_string();

            // Validate URL format
            if url.is_empty() {
                return Err(ScrapperError::csv(format!(
                    "Empty URL at line {line_number}"
                )));
            }

            // Basic URL validation
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ScrapperError::csv(format!(
                    "Invalid URL format at line {line_number}: '{url}'. URLs must start with http:// or https://"
                )));
            }

            // Validate chapter number
            if chapter_number.is_empty() {
                return Err(ScrapperError::csv(format!(
                    "Empty chapter number at line {line_number}"
                )));
            }

            chapter_records.push(ChapterRecord::new(url, chapter_number));
            line_number += 1;
        }

        if chapter_records.is_empty() {
            return Err(ScrapperError::csv(
                "CSV file contains no valid records. Ensure the file has 'url,chapter_number' format.",
            ));
        }

        Ok(chapter_records)
    }

    pub async fn count_records_and_existing<P: AsRef<Path>>(
        &self,
        output_dir: P,
    ) -> ScrapperResult<ScrapingStats> {
        let file = File::open(&self.file_path).await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to open CSV file for counting: {e}"),
                Some(self.file_path.clone()),
            )
        })?;

        let mut reader = AsyncReader::from_reader(file);
        let mut records = reader.records();
        let mut stats = ScrapingStats::default();
        let mut line_number = 1;

        while let Some(record) = records.next().await {
            let record = record.map_err(|e| {
                ScrapperError::csv(format!(
                    "Failed to read CSV record while counting at line {line_number}: {e}"
                ))
            })?;

            stats.total += 1;

            if let Some(chapter_number) = record.get(1) {
                let chapter_number = chapter_number.trim();
                if !chapter_number.is_empty() {
                    let file_path = output_dir
                        .as_ref()
                        .join(format!("chapter_{chapter_number}.txt"));

                    if file_path.exists() {
                        stats.existing += 1;
                    }
                }
            }

            line_number += 1;
        }

        Ok(stats)
    }

    /// Validate CSV file format without fully parsing it
    pub async fn validate_format(&self) -> ScrapperResult<()> {
        let file = File::open(&self.file_path).await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to open CSV file for validation: {e}"),
                Some(self.file_path.clone()),
            )
        })?;

        let mut reader = AsyncReader::from_reader(file);

        // Check if we can read at least one record
        if let Some(record) = reader.records().next().await {
            let record = record
                .map_err(|e| ScrapperError::csv(format!("CSV format validation failed: {e}")))?;

            // Check if we have at least 2 columns
            if record.len() < 2 {
                return Err(ScrapperError::csv(format!(
                    "CSV must have at least 2 columns (url, chapter_number), found {} columns",
                    record.len()
                )));
            }

            // Check if columns are not empty
            let url = record.get(0).unwrap_or("").trim();
            let chapter = record.get(1).unwrap_or("").trim();

            if url.is_empty() {
                return Err(ScrapperError::csv("First column (URL) cannot be empty"));
            }

            if chapter.is_empty() {
                return Err(ScrapperError::csv(
                    "Second column (chapter_number) cannot be empty",
                ));
            }
        } else {
            return Err(ScrapperError::csv(
                "CSV file is empty or contains no valid records",
            ));
        }

        Ok(())
    }

    /// Get basic statistics about the CSV file
    pub async fn get_stats(&self) -> ScrapperResult<CsvStats> {
        let file = File::open(&self.file_path).await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to open CSV file for stats: {e}"),
                Some(self.file_path.clone()),
            )
        })?;

        let mut reader = AsyncReader::from_reader(file);
        let mut records = reader.records();
        let mut stats = CsvStats::default();

        while let Some(record) = records.next().await {
            match record {
                Ok(record) => {
                    stats.total_rows += 1;
                    if record.len() >= 2 {
                        stats.valid_rows += 1;
                    } else {
                        stats.invalid_rows += 1;
                    }
                }
                Err(_) => {
                    stats.invalid_rows += 1;
                }
            }
        }

        Ok(stats)
    }
}

#[derive(Debug, Default)]
pub struct CsvStats {
    pub total_rows: usize,
    pub valid_rows: usize,
    pub invalid_rows: usize,
}

impl CsvStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_rows == 0 {
            0.0
        } else {
            (self.valid_rows as f64 / self.total_rows as f64) * 100.0
        }
    }
}
