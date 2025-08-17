use crate::error::{ScrapperError, ScrapperResult};
use crate::types::{ChapterRecord, Config};
use indicatif::ProgressBar;
use scraper::{Html, Selector};
use std::path::Path;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct ContentExtractor {
    selector: String,
    skip_nodes: usize,
    filter_patterns: Vec<String>,
}

impl ContentExtractor {
    pub fn new(config: &Config) -> ScrapperResult<Self> {
        // Validate selector by attempting to parse it
        let _ = Selector::parse(&config.selector).map_err(|e| {
            ScrapperError::validation(
                "selector",
                format!("Invalid CSS selector '{}': {:?}", config.selector, e),
            )
        })?;

        Ok(Self {
            selector: config.selector.clone(),
            skip_nodes: config.skip_text_nodes,
            filter_patterns: config.filter_patterns.clone(),
        })
    }

    pub fn extract_content(&self, html: &str, url: &str) -> ScrapperResult<String> {
        if html.is_empty() {
            return Err(ScrapperError::content_extraction(
                url,
                "HTML content is empty",
            ));
        }

        let document = Html::parse_document(html);

        // Try each selector in the list (separated by commas)
        let selectors: Vec<&str> = self.selector.split(',').map(|s| s.trim()).collect();
        let mut element = None;

        for selector_str in selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(found_element) = document.select(&selector).next() {
                    element = Some(found_element);
                    break;
                }
            }
        }

        let element = element.ok_or_else(|| {
            ScrapperError::content_extraction(
                url,
                format!(
                    "No element found matching any of the selectors: {}",
                    self.selector
                ),
            )
        })?;

        let mut content = String::new();
        let text_nodes: Vec<_> = element.text().collect();

        if text_nodes.is_empty() {
            return Err(ScrapperError::content_extraction(
                url,
                "No text content found in the selected element",
            ));
        }

        for (i, text_node) in text_nodes.iter().enumerate() {
            // Skip initial text nodes as specified
            if i < self.skip_nodes {
                continue;
            }

            let text = text_node.trim();

            // Skip empty text nodes
            if text.is_empty() {
                continue;
            }

            // Filter out unwanted content
            if self.should_filter_text(text) {
                continue;
            }

            content.push_str(text);
            content.push('\n');
        }

        if content.trim().is_empty() {
            return Err(ScrapperError::content_extraction(
                url,
                format!(
                    "No valid content found after filtering and processing. Skipped {} text nodes, applied {} filters.",
                    self.skip_nodes,
                    self.filter_patterns.len()
                ),
            ));
        }

        // Basic content quality check
        if content.len() < 100 {
            return Err(ScrapperError::content_extraction(
                url,
                format!(
                    "Extracted content is too short ({} characters). This might indicate a parsing error.",
                    content.len()
                ),
            ));
        }

        Ok(content)
    }

    fn should_filter_text(&self, text: &str) -> bool {
        self.filter_patterns
            .iter()
            .any(|pattern| text.contains(pattern))
    }
}

pub struct WebScraper {
    client: reqwest::Client,
    extractor: ContentExtractor,
    config: Config,
}

impl WebScraper {
    pub fn new(config: &Config) -> ScrapperResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .user_agent(&config.user_agent)
            .build()
            .map_err(|e| ScrapperError::config(format!("Failed to create HTTP client: {e}")))?;

        let extractor = ContentExtractor::new(config)?;

        Ok(Self {
            client,
            extractor,
            config: config.clone(),
        })
    }

    pub async fn scrape_chapter(
        &self,
        record: &ChapterRecord,
        output_dir: &Path,
        stats_pb: Option<&ProgressBar>,
    ) -> ScrapperResult<()> {
        let chapter_name = &record.chapter_number;
        let url = &record.url;

        if let Some(pb) = stats_pb {
            pb.println(format!("🔄 Starting chapter {chapter_name}: {url}"));
        }

        // Validate URL format before making request
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ScrapperError::validation(
                "url",
                format!("Invalid URL format: '{url}'. URLs must start with http:// or https://",),
            ));
        }

        // Fetch the web page with detailed error handling
        let response = match self.client.get(url).send().await {
            Ok(response) => response,
            Err(e) => {
                // Check for specific error types
                if e.is_timeout() {
                    return Err(ScrapperError::http(
                        url,
                        None,
                        format!(
                            "Request timeout after {} seconds",
                            self.config.request_timeout_secs
                        ),
                    ));
                } else if e.is_connect() {
                    return Err(ScrapperError::http(
                        url,
                        None,
                        "Connection failed - check your internet connection and the URL",
                    ));
                } else {
                    return Err(ScrapperError::http(
                        url,
                        e.status().map(|s| s.as_u16()),
                        format!("Request failed: {e}"),
                    ));
                }
            }
        };

        // Check HTTP status
        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(ScrapperError::http(
                url,
                Some(status_code),
                format!(
                    "HTTP {} - {}",
                    status_code,
                    error_body.chars().take(200).collect::<String>()
                ),
            ));
        }

        // Read response body
        let html = response.text().await.map_err(|e| {
            ScrapperError::web_scraping(url, format!("Failed to read response body: {e}"))
        })?;

        if self.config.verbose {
            if let Some(pb) = stats_pb {
                pb.println(format!(
                    "📄 Parsing content from {} ({} bytes)",
                    url,
                    html.len()
                ));
            }
        }

        // Extract content from HTML
        let content = self.extractor.extract_content(&html, url)?;

        // Save to file
        let file_path = output_dir.join(record.file_name());
        self.save_content(&file_path, &content).await?;

        if let Some(pb) = stats_pb {
            pb.println(format!(
                "✅ Completed chapter {} ({} bytes)",
                chapter_name,
                content.len()
            ));
        }

        Ok(())
    }

    async fn save_content(&self, file_path: &Path, content: &str) -> ScrapperResult<()> {
        let mut file = File::create(file_path).await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to create file: {e}"),
                Some(file_path.to_path_buf()),
            )
        })?;

        file.write_all(content.as_bytes()).await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to write content to file: {e}"),
                Some(file_path.to_path_buf()),
            )
        })?;

        // Ensure data is written to disk
        file.sync_all().await.map_err(|e| {
            ScrapperError::file_system(
                format!("Failed to sync file to disk: {e}"),
                Some(file_path.to_path_buf()),
            )
        })?;

        Ok(())
    }
}
