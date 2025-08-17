use crate::error::{ScrapperError, ScrapperResult};
use crate::types::ScrapingStats;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::time::Duration;

pub struct ProgressManager {
    main_pb: ProgressBar,
    stats_pb: ProgressBar,
    active_pb: ProgressBar,
}

impl ProgressManager {
    pub fn new(total_records: u64) -> ScrapperResult<Self> {
        let multi_progress = MultiProgress::new();

        // Main progress bar
        let main_pb = multi_progress.add(ProgressBar::new(total_records));
        main_pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
                )
                .map_err(|e| ScrapperError::progress(
                    format!("Failed to create main progress bar template: {e}")
                ))?
                .progress_chars("#>-"),
        );
        main_pb.set_message("Processing chapters");

        // Stats progress bar for showing current activity
        let stats_pb = multi_progress.add(ProgressBar::new_spinner());
        stats_pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.blue} {msg}")
                .map_err(|e| {
                    ScrapperError::progress(format!(
                        "Failed to create stats progress bar template: {e}"
                    ))
                })?,
        );
        stats_pb.enable_steady_tick(Duration::from_millis(100));

        // Active tasks counter
        let active_pb = multi_progress.add(ProgressBar::new_spinner());
        active_pb.set_style(
            ProgressStyle::default_spinner()
                .template("🔄 Active: {msg}")
                .map_err(|e| {
                    ScrapperError::progress(format!(
                        "Failed to create active tasks progress bar template: {e}"
                    ))
                })?,
        );
        active_pb.enable_steady_tick(Duration::from_millis(200));

        Ok(Self {
            main_pb,
            stats_pb,
            active_pb,
            // multi_progress,
        })
    }

    pub fn increment_progress(&self) {
        self.main_pb.inc(1);
    }

    pub fn update_active_tasks(&self, active_count: usize) {
        self.active_pb.set_message(format!("{active_count} tasks"));
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

    pub fn log_error(&self, error: &ScrapperError) {
        // Use user-friendly message for display
        let message = if error.is_recoverable() {
            format!("⚠️  Recoverable error: {}", error.user_friendly_message())
        } else {
            format!("❌ Error: {}", error.user_friendly_message())
        };

        self.stats_pb.println(message);

        // Log debug info if available
        if let Some(url) = error.url() {
            self.stats_pb.println(format!("   URL: {url}"));
        }
    }

    pub fn log_skip(&self, file_name: &str) {
        self.stats_pb
            .println(format!("⏭️ Skipping existing file: {file_name}"));
    }

    pub fn log_info(&self, message: &str) {
        self.stats_pb.println(format!("ℹ️ {message}",));
    }

    pub fn log_warning(&self, message: &str) {
        self.stats_pb.println(format!("⚠️ {message}"));
    }

    pub fn finish(&self, stats: &ScrapingStats) {
        self.main_pb
            .finish_with_message("✨ All chapters processed!");

        let final_message = if stats.error_count == 0 {
            format!(
                "🎉 Perfect! ✅ {} chapters completed successfully!",
                stats.success_count
            )
        } else {
            format!(
                "Final: ✅ {} success, ❌ {} errors ({}% success rate)",
                stats.success_count,
                stats.error_count,
                stats.success_rate()
            )
        };

        self.stats_pb.finish_with_message(final_message);
        self.active_pb.finish_and_clear();

        // Final summary
        println!("\n📊 Scraping Summary:");
        println!("   ✅ Successful: {}", stats.success_count);
        println!("   ❌ Errors: {}", stats.error_count);
        println!("   📈 Success Rate: {:.1}%", stats.success_rate());

        if stats.error_count > 0 {
            println!("\n💡 Tip: Check the error messages above for specific issues.");
            println!("   Common solutions:");
            println!("   • Increase delays if you're being rate-limited");
            println!("   • Check your internet connection for connection errors");
            println!("   • Verify URLs are correct for 404 errors");
        }
    }

    pub fn get_stats_pb(&self) -> ProgressBar {
        self.stats_pb.clone()
    }

    /// Check if progress tracking is working correctly
    pub fn validate_progress_state(&self) -> ScrapperResult<()> {
        if self.main_pb.is_finished()
            && self.main_pb.position() != self.main_pb.length().unwrap_or(0)
        {
            return Err(ScrapperError::progress(
                "Progress bar finished but position doesn't match total length",
            ));
        }
        Ok(())
    }
}
