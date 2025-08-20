use tokio::time::{Duration, sleep};

mod config;
mod csv_reader;
mod error;
mod file_manager;
mod progress;
mod task_manager;
mod types;
mod web_scraper;
use csv_reader::CsvReader;
use error::{ScrapperError, ScrapperResult};
use file_manager::FileManager;
use progress::ProgressManager;
use task_manager::TaskManager;
use types::{Config, ScrapingStats};
use web_scraper::WebScraper;

struct ScrapperApp {
    config: Config,
    csv_reader: CsvReader,
    file_manager: FileManager,
}

impl ScrapperApp {
    async fn new() -> ScrapperResult<Self> {
        // Check if we should generate a config file and exit
        if config::handle_config_generation().await? {
            std::process::exit(0);
        }

        // Load configuration from args/file
        let config = Config::from_args().await?;

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

    async fn run(&self) -> ScrapperResult<()> {
        // Validate CSV file format first
        if self.config.verbose {
            println!("🔍 Validating CSV file format...");
        }

        self.csv_reader.validate_format().await?;

        if self.config.verbose {
            let csv_stats = self.csv_reader.get_stats().await?;
            println!("📊 CSV Statistics:");
            println!("   Total rows: {}", csv_stats.total_rows);
            println!("   Valid rows: {}", csv_stats.valid_rows);
            println!("   Invalid rows: {}", csv_stats.invalid_rows);
            println!("   Success rate: {:.1}%", csv_stats.success_rate());
            println!();
        }

        // Ensure output directory exists and is writable
        self.file_manager.validate_output_dir().await?;

        // Optional: Clean up any invalid files from previous runs
        if self.config.verbose {
            println!("🧹 Cleaning up invalid files from previous runs...");
            let cleanup_stats = self.file_manager.cleanup_invalid_files().await?;
            if cleanup_stats.total_removed() > 0 {
                println!("   Removed {} invalid files", cleanup_stats.total_removed());
            }
        }

        // Count total records and existing files
        let initial_stats = self
            .csv_reader
            .count_records_and_existing(self.file_manager.output_dir())
            .await?;

        let records_to_process = initial_stats.records_to_process();
        if records_to_process == 0 {
            println!("✅ All files already exist. Nothing to process.");
            if self.config.verbose {
                println!("{}", initial_stats.summary_report());
            }
            return Ok(());
        }

        println!(
            "📋 Processing {} new chapters ({} already exist)",
            records_to_process, initial_stats.existing
        );

        // Initialize progress tracking
        let progress = ProgressManager::new(records_to_process as u64)?;

        // Read all records
        let records = self.csv_reader.read_records().await?;

        // Validate all records before processing
        if self.config.verbose {
            println!("🔍 Validating {} records...", records.len());
        }

        for (i, record) in records.iter().enumerate() {
            if let Err(e) = record.validate() {
                return Err(ScrapperError::validation(
                    "record",
                    format!("Invalid record at position {}: {}", i + 1, e),
                ));
            }
        }

        // Process records concurrently
        self.process_records(records, initial_stats, &progress)
            .await
    }

    async fn process_records(
        &self,
        records: Vec<types::ChapterRecord>,
        mut stats: ScrapingStats,
        progress: &ProgressManager,
    ) -> ScrapperResult<()> {
        let mut tasks = TaskManager::new(self.config.max_concurrent_tasks);
        let stats_pb = progress.get_stats_pb();

        // Track retry attempts for recoverable errors
        let mut retry_queue: Vec<(types::ChapterRecord, usize)> = Vec::new();
        const MAX_RETRIES: usize = 3;

        for record in records {
            // Skip existing files
            if self.file_manager.chapter_exists(&record) {
                progress.log_skip(&record.file_name());
                continue;
            }

            // Clone data needed for the async task
            if let Some(result) = tasks
                .spawn_or_wait(|| {
                    let output_dir = self.file_manager.output_dir().to_path_buf();
                    let stats_pb_clone = stats_pb.clone();
                    let config_clone = self.config.clone();
                    let record_clone = record.clone();

                    async move {
                        let scraper = WebScraper::new(&config_clone)?;
                        scraper
                            .scrape_chapter(&record_clone, &output_dir, Some(&stats_pb_clone))
                            .await
                    }
                })
                .await
            {
                self.handle_task_result(Ok(result), &mut stats, progress);
            }

            // Update progress displays
            progress.update_active_tasks(tasks.len());
            progress.update_stats_with_queue(&stats, tasks.len());
            sleep(Duration::from_millis(self.config.task_delay_ms)).await;
        }
        // Wait for all remaining tasks to complete
        let remaining_results = tasks.join_all().await;
        for result in remaining_results {
            self.handle_task_result(Ok(result), &mut stats, progress);

            // Update progress displays
            progress.update_active_tasks(tasks.len());
            progress.update_stats_with_remaining(&stats, tasks.len());
        }

        // Process retry queue for recoverable errors
        if !retry_queue.is_empty() && self.config.verbose {
            progress.log_info(&format!(
                "Processing {} items from retry queue...",
                retry_queue.len()
            ));

            while let Some((record, retry_count)) = retry_queue.pop() {
                if retry_count >= MAX_RETRIES {
                    progress.log_warning(&format!(
                        "Max retries exceeded for chapter {}",
                        record.chapter_number
                    ));
                    stats.increment_permanent_error();
                    progress.increment_progress();
                    continue;
                }

                // Exponential backoff for retries
                let delay = Duration::from_millis(
                    self.config.task_delay_ms * (2_u64.pow(retry_count as u32)),
                );
                sleep(delay).await;

                let output_dir = self.file_manager.output_dir().to_path_buf();
                let stats_pb_clone = stats_pb.clone();
                let config_clone = self.config.clone();

                match WebScraper::new(&config_clone) {
                    Ok(scraper) => {
                        match scraper
                            .scrape_chapter(&record, &output_dir, Some(&stats_pb_clone))
                            .await
                        {
                            Ok(_) => {
                                stats.increment_success();
                                progress.increment_progress();
                            }
                            Err(e) if e.is_recoverable() => {
                                retry_queue.push((record, retry_count + 1));
                            }
                            Err(e) => {
                                stats.increment_permanent_error();
                                progress.log_error(&e);
                                progress.increment_progress();
                            }
                        }
                    }
                    Err(e) => {
                        stats.increment_permanent_error();
                        progress.log_error(&e);
                        progress.increment_progress();
                    }
                }
            }
        }

        // Finish progress display
        progress.finish(&stats);

        // Show final recommendations
        let recommendations = stats.get_recommendations();
        if !recommendations.is_empty() {
            println!("\n💡 Recommendations:");
            for rec in recommendations {
                println!("   • {rec}");
            }
        }

        // Show detailed stats if verbose
        if self.config.verbose {
            println!("\n{}", stats.summary_report());

            // Show file system statistics
            let fs_stats = self.file_manager.get_existing_files_info().await?;
            println!("\n📁 File System Statistics:");
            println!("   Total files: {}", fs_stats.total_files);
            println!("   Valid files: {}", fs_stats.valid_files());
            println!("   Empty files: {}", fs_stats.empty_files);
            println!(
                "   Average file size: {:.1} bytes",
                fs_stats.average_file_size()
            );
        }

        // Validate final progress state
        progress.validate_progress_state()?;

        Ok(())
    }

    fn handle_task_result(
        &self,
        result: Result<ScrapperResult<()>, tokio::task::JoinError>,
        stats: &mut ScrapingStats,
        progress: &ProgressManager,
        // retry_queue: &mut Vec<(types::ChapterRecord, usize)>,
    ) {
        match result {
            Ok(Ok(())) => {
                stats.increment_success();
                progress.increment_progress();
            }
            Ok(Err(e)) => {
                if e.is_recoverable() {
                    // Add to retry queue if we have the record info
                    // Note: We'd need to modify the task to return the record on error
                    // For now, just count as recoverable error
                    stats.increment_recoverable_error();
                } else {
                    stats.increment_permanent_error();
                }
                progress.log_error(&e);
                progress.increment_progress();
            }
            Err(e) => {
                let scrapper_error = ScrapperError::task_execution(e.to_string());
                stats.increment_permanent_error();
                progress.log_error(&scrapper_error);
                progress.increment_progress();
            }
        }
    }
}

#[tokio::main]
async fn main() -> ScrapperResult<()> {
    // Set up better panic handling
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();
        eprintln!("💥 Application panicked{location}: {panic_info}");
        eprintln!("This is likely a bug. Please report it with the error details above.");
    }));

    let result = async {
        let app = ScrapperApp::new().await?;
        app.run().await
    }
    .await;
    match result {
        Ok(()) => {
            println!("🎉 Scraping completed successfully!");
            Ok(())
        }
        Err(e) => {
            eprintln!("\n💥 Application failed:");
            eprintln!("   {}", e.user_friendly_message());

            if let Some(url) = e.url() {
                eprintln!("   URL: {url}");
            }

            // Show debug info in verbose mode or for certain error types
            match &e {
                ScrapperError::Config { .. }
                | ScrapperError::Validation { .. }
                | ScrapperError::Csv { .. } => {
                    eprintln!("   Debug: {}", e.debug_info());
                }
                _ => {}
            }

            eprintln!("\n💡 For more help, run with --verbose flag or check the documentation.");
            std::process::exit(1);
        }
    }
}
