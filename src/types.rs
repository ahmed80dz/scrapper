#[derive(Debug, Clone)]
pub struct ChapterRecord {
    pub url: String,
    pub chapter_number: String,
}

impl ChapterRecord {
    pub fn new(url: String, chapter_number: String) -> Self {
        Self {
            url,
            chapter_number,
        }
    }

    pub fn file_name(&self) -> String {
        format!("chapter_{}.txt", self.chapter_number)
    }
}

#[derive(Debug, Default)]
pub struct ScrapingStats {
    pub total: usize,
    pub existing: usize,
    pub success_count: usize,
    pub error_count: usize,
}

impl ScrapingStats {
    pub fn records_to_process(&self) -> usize {
        self.total - self.existing
    }

    pub fn increment_success(&mut self) {
        self.success_count += 1;
    }

    pub fn increment_error(&mut self) {
        self.error_count += 1;
    }
}

// Re-export the config type for convenience
pub use crate::config::ScrapingConfig as Config;
