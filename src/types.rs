use std::path::PathBuf;

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

#[derive(Debug, Clone)]
pub struct Config {
    pub max_concurrent_tasks: usize,
    pub task_delay_ms: u64,
    pub input_file: PathBuf,
    pub output_dir: PathBuf,
    pub selector: String,
    pub skip_text_nodes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 20,
            task_delay_ms: 100,
            input_file: PathBuf::from("./out/links.csv"),
            output_dir: PathBuf::from("./out"),
            selector: ".content-inner".to_string(),
            skip_text_nodes: 5,
        }
    }
}
