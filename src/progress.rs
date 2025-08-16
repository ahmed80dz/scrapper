use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::time::Duration;

use crate::types::ScrapingStats;

pub struct ProgressManager {
    main_pb: ProgressBar,
    stats_pb: ProgressBar,
    active_pb: ProgressBar,
}

impl ProgressManager {
    pub fn new(total_records: u64) -> Result<Self> {
        let multi_progress = MultiProgress::new();

        // Main progress bar
        let main_pb = multi_progress.add(ProgressBar::new(total_records));
        main_pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
                )?
                .progress_chars("#>-"),
        );
        main_pb.set_message("Processing chapters");

        // Stats progress bar for showing current activity
        let stats_pb = multi_progress.add(ProgressBar::new_spinner());
        stats_pb.set_style(ProgressStyle::default_spinner().template("{spinner:.blue} {msg}")?);
        stats_pb.enable_steady_tick(Duration::from_millis(100));

        // Active tasks counter
        let active_pb = multi_progress.add(ProgressBar::new_spinner());
        active_pb.set_style(ProgressStyle::default_spinner().template("🔄 Active: {msg}")?);
        active_pb.enable_steady_tick(Duration::from_millis(200));

        Ok(Self {
            main_pb,
            stats_pb,
            active_pb,
        })
    }

    pub fn increment_progress(&self) {
        self.main_pb.inc(1);
    }

    pub fn update_active_tasks(&self, active_count: usize) {
        self.active_pb
            .set_message(format!("{} tasks", active_count));
    }

    pub fn update_stats_with_queue(&self, stats: &ScrapingStats, queue_size: usize) {
        self.stats_pb.set_message(format!(
            "✅ {} success, ❌ {} errors, 📥 {} queued",
            stats.success_count, stats.error_count, queue_size
        ));
    }

    pub fn update_stats_with_remaining(&self, stats: &ScrapingStats, remaining: usize) {
        self.stats_pb.set_message(format!(
            "✅ {} success, ❌ {} errors, 📥 {} remaining",
            stats.success_count, stats.error_count, remaining
        ));
    }

    pub fn log_error(&self, error: &anyhow::Error) {
        self.stats_pb.println(format!("❌ Error: {}", error));
    }

    pub fn log_panic(&self, error: &tokio::task::JoinError) {
        self.stats_pb
            .println(format!("❌ Task panicked: {}", error));
    }

    pub fn log_skip(&self, file_name: &str) {
        self.stats_pb
            .println(format!("⏭️ Skipping existing file: {}", file_name));
    }

    pub fn finish(&self, stats: &ScrapingStats) {
        self.main_pb
            .finish_with_message("✨ All chapters processed!");

        self.stats_pb.finish_with_message(format!(
            "Final: ✅ {} success, ❌ {} errors",
            stats.success_count, stats.error_count
        ));

        self.active_pb.finish_and_clear();

        println!(
            "\n🎉 Scraping completed! {} successful, {} errors",
            stats.success_count, stats.error_count
        );
    }

    pub fn get_stats_pb(&self) -> ProgressBar {
        self.stats_pb.clone()
    }
}
