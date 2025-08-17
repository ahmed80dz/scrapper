use crate::error::{ScrapperError, ScrapperResult};

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

    /// Validate the chapter record
    pub fn validate(&self) -> ScrapperResult<()> {
        if self.url.is_empty() {
            return Err(ScrapperError::validation("url", "URL cannot be empty"));
        }

        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(ScrapperError::validation(
                "url",
                format!(
                    "Invalid URL format: '{}'. URLs must start with http:// or https://",
                    self.url
                ),
            ));
        }

        if self.chapter_number.is_empty() {
            return Err(ScrapperError::validation(
                "chapter_number",
                "Chapter number cannot be empty",
            ));
        }

        // Check for potentially problematic characters in chapter number
        if self
            .chapter_number
            .contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '.')
        {
            return Err(ScrapperError::validation(
                "chapter_number",
                format!(
                    "Chapter number '{}' contains invalid characters. Only alphanumeric, underscore, hyphen, and dot are allowed.",
                    self.chapter_number
                ),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct ScrapingStats {
    pub total: usize,
    pub existing: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub recoverable_errors: usize,
    pub permanent_errors: usize,
}

impl ScrapingStats {
    pub fn records_to_process(&self) -> usize {
        self.total - self.existing
    }

    pub fn increment_success(&mut self) {
        self.success_count += 1;
    }

    pub fn increment_recoverable_error(&mut self) {
        self.error_count += 1;
        self.recoverable_errors += 1;
    }

    pub fn increment_permanent_error(&mut self) {
        self.error_count += 1;
        self.permanent_errors += 1;
    }

    pub fn success_rate(&self) -> f64 {
        let total_processed = self.success_count + self.error_count;
        if total_processed == 0 {
            0.0
        } else {
            (self.success_count as f64 / total_processed as f64) * 100.0
        }
    }

    pub fn error_rate(&self) -> f64 {
        let total_processed = self.success_count + self.error_count;
        if total_processed == 0 {
            0.0
        } else {
            (self.error_count as f64 / total_processed as f64) * 100.0
        }
    }

    pub fn recoverable_error_rate(&self) -> f64 {
        if self.error_count == 0 {
            0.0
        } else {
            (self.recoverable_errors as f64 / self.error_count as f64) * 100.0
        }
    }

    pub fn completion_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            ((self.success_count + self.existing) as f64 / self.total as f64) * 100.0
        }
    }

    /// Get a summary report of the scraping statistics
    pub fn summary_report(&self) -> String {
        format!(
            "Scraping Summary:
  📊 Total Records: {}
  📁 Already Existing: {}
  ✅ Successful: {}
  ❌ Errors: {}
    └── 🔄 Recoverable: {}
    └── ❌ Permanent: {}
  📈 Success Rate: {:.1}%
  📉 Error Rate: {:.1}%
  🎯 Completion Rate: {:.1}%",
            self.total,
            self.existing,
            self.success_count,
            self.error_count,
            self.recoverable_errors,
            self.permanent_errors,
            self.success_rate(),
            self.error_rate(),
            self.completion_rate()
        )
    }

    /// Get recommendations based on the statistics
    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        if self.error_rate() > 20.0 {
            recommendations.push("High error rate detected. Consider reducing concurrent tasks or increasing delays.".to_string());
        }

        if self.recoverable_error_rate() > 50.0 && self.recoverable_errors > 0 {
            recommendations.push("Many recoverable errors (rate limits, timeouts). Increase delays between requests.".to_string());
        }

        if self.permanent_errors > self.recoverable_errors && self.permanent_errors > 5 {
            recommendations
                .push("Many permanent errors detected. Check URLs and CSS selectors.".to_string());
        }

        if self.success_count == 0 && self.error_count > 0 {
            recommendations.push("No successful scrapes. Check your configuration, network connection, and target URLs.".to_string());
        }

        if self.total > 1000 && self.success_rate() < 95.0 {
            recommendations.push(
                "Large scraping job with errors. Consider implementing retry logic.".to_string(),
            );
        }

        recommendations
    }
}

// Re-export the config type for convenience
pub use crate::config::ScrapingConfig as Config;
