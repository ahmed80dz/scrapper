use anyhow::{Context, Result};
use tokio::task::JoinSet;
use tokio::time::{Duration, sleep};

mod config;
mod csv_reader;
mod file_manager;
mod progress;
mod types;
mod web_scraper;

use csv_reader::CsvReader;
use file_manager::FileManager;
use progress::ProgressManager;
use types::{Config, ScrapingStats};
use web_scraper::WebScraper;

struct ScrapperApp {
    config: Config,
    csv_reader: CsvReader,
    file_manager: FileManager,
}

impl ScrapperApp {
    async fn new() -> Result<Self> {
        // Check if we should generate a config file and exit
        if config::handle_config_generation().await? {
            std::process::exit(0);
        }

        // Load configuration from args/file
        let config = Config::from_args()
            .await
            .context("Failed to load configuration")?;

        if config.verbose {
            println!("🔧 Configuration loaded:");
            println!("   Input file: {:?}", config.input_file);
            println!("   Output directory: {:?}", config.output_dir);
            println!("   CSS selector: {}", config.selector);
            println!("   Max concurrent tasks: {}", config.max_concurrent_tasks);
            println!("   Task delay: {}ms", config.task_delay_ms);
            println!("   Request timeout: {}s", config.request_timeout_secs);
            println!();
        }

        let csv_reader = CsvReader::new(&config.input_file);
        let file_manager = FileManager::new(&config.output_dir);

        Ok(Self {
            config,
            csv_reader,
            file_manager,
        })
    }

    async fn run(&self) -> Result<()> {
        // Ensure output directory exists
        self.file_manager.ensure_output_dir_exists().await?;

        // Count total records and existing files
        let initial_stats = self
            .csv_reader
            .count_records_and_existing(self.file_manager.output_dir())
            .await
            .context("Failed to count records and existing files")?;

        let records_to_process = initial_stats.records_to_process();
        if records_to_process == 0 {
            println!("All files already exist. Nothing to process.");
            return Ok(());
        }

        // Initialize progress tracking
        let progress = ProgressManager::new(records_to_process as u64)
            .context("Failed to initialize progress manager")?;

        // Read all records
        let records = self
            .csv_reader
            .read_records()
            .await
            .context("Failed to read CSV records")?;

        // Process records concurrently
        self.process_records(records, initial_stats, &progress)
            .await
    }

    async fn process_records(
        &self,
        records: Vec<types::ChapterRecord>,
        mut stats: ScrapingStats,
        progress: &ProgressManager,
    ) -> Result<()> {
        let mut join_set = JoinSet::new();
        let stats_pb = progress.get_stats_pb();

        for record in records {
            // Skip existing files
            if self.file_manager.chapter_exists(&record) {
                progress.log_skip(&record.file_name());
                continue;
            }

            // Wait if we've reached the maximum concurrent tasks
            if join_set.len() >= self.config.max_concurrent_tasks {
                if let Some(result) = join_set.join_next().await {
                    self.handle_task_result(result, &mut stats, progress);
                }
            }

            // Update progress displays
            progress.update_active_tasks(join_set.len());
            progress.update_stats_with_queue(&stats, join_set.len());

            // Clone data needed for the async task
            let output_dir = self.file_manager.output_dir().to_path_buf();
            let stats_pb_clone = stats_pb.clone();
            let config_clone = self.config.clone();

            join_set.spawn(async move {
                let scraper = WebScraper::new(&config_clone)?;
                scraper
                    .scrape_chapter(&record, &output_dir, Some(&stats_pb_clone))
                    .await
            });

            // Rate limiting delay
            sleep(Duration::from_millis(self.config.task_delay_ms)).await;
        }

        // Wait for all remaining tasks to complete
        while let Some(result) = join_set.join_next().await {
            self.handle_task_result(result, &mut stats, progress);

            // Update progress displays
            progress.update_active_tasks(join_set.len());
            progress.update_stats_with_remaining(&stats, join_set.len());
        }

        // Finish progress display
        progress.finish(&stats);

        Ok(())
    }

    fn handle_task_result(
        &self,
        result: Result<Result<()>, tokio::task::JoinError>,
        stats: &mut ScrapingStats,
        progress: &ProgressManager,
    ) {
        match result {
            Ok(Ok(())) => {
                stats.increment_success();
                progress.increment_progress();
            }
            Ok(Err(e)) => {
                stats.increment_error();
                progress.log_error(&e);
                progress.increment_progress();
            }
            Err(e) => {
                stats.increment_error();
                progress.log_panic(&e);
                progress.increment_progress();
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let app = ScrapperApp::new()
        .await
        .context("Failed to initialize application")?;
    app.run().await.context("Application failed to run")?;
    Ok(())
}

