use anyhow::{Context, Result};
use csv_async::AsyncReader;
use std::path::Path;
use tokio::fs::File;
use tokio_stream::StreamExt;

use crate::types::{ChapterRecord, ScrapingStats};

pub struct CsvReader {
    file_path: std::path::PathBuf,
}

impl CsvReader {
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
        }
    }

    pub async fn read_records(&self) -> Result<Vec<ChapterRecord>> {
        let file = File::open(&self.file_path)
            .await
            .with_context(|| format!("Failed to open CSV file: {:?}", self.file_path))?;

        let mut reader = AsyncReader::from_reader(file);
        let mut records = reader.records();
        let mut chapter_records = Vec::new();

        while let Some(record) = records.next().await {
            let record = record.context("Failed to read CSV record")?;

            let url = record
                .get(0)
                .context("Missing URL column in CSV")?
                .to_string();

            let chapter_number = record
                .get(1)
                .context("Missing chapter number column in CSV")?
                .to_string();

            chapter_records.push(ChapterRecord::new(url, chapter_number));
        }

        Ok(chapter_records)
    }

    pub async fn count_records_and_existing<P: AsRef<Path>>(
        &self,
        output_dir: P,
    ) -> Result<ScrapingStats> {
        let file = File::open(&self.file_path)
            .await
            .with_context(|| format!("Failed to open CSV file: {:?}", self.file_path))?;

        let mut reader = AsyncReader::from_reader(file);
        let mut records = reader.records();
        let mut stats = ScrapingStats::default();

        while let Some(record) = records.next().await {
            let record = record.context("Failed to read CSV record while counting")?;
            stats.total += 1;

            if let Some(chapter_number) = record.get(1) {
                let file_path = output_dir
                    .as_ref()
                    .join(format!("chapter_{}.txt", chapter_number));

                if file_path.exists() {
                    stats.existing += 1;
                }
            }
        }

        Ok(stats)
    }
}
