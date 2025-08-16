use anyhow::{Context, Result};
use indicatif::ProgressBar;
use scraper::{Html, Selector};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::types::{ChapterRecord, Config};

pub struct ContentExtractor {
    selector: String,
    skip_nodes: usize,
    filter_patterns: Vec<String>,
}

impl ContentExtractor {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            selector: config.selector.clone(),
            skip_nodes: config.skip_text_nodes,
            filter_patterns: vec!["window.pubfuturetag".to_string()],
        })
    }

    pub fn extract_content(&self, html: &str) -> Result<String> {
        let document = Html::parse_document(html);
        let selector = Selector::parse(&self.selector).map_err(|e| {
            anyhow::anyhow!("Failed to parse CSS selector '{}': {:?}", self.selector, e)
        })?;

        let element = document
            .select(&selector)
            .next()
            .context("No element found matching the selector")?;

        let mut content = String::new();

        for (i, text_node) in element.text().enumerate() {
            // Skip initial text nodes as specified
            if i < self.skip_nodes {
                continue;
            }

            // Filter out unwanted content
            if self.should_filter_text(text_node) {
                continue;
            }

            content.push_str(text_node);
            content.push('\n');
        }

        Ok(content)
    }

    fn should_filter_text(&self, text: &str) -> bool {
        self.filter_patterns
            .iter()
            .any(|pattern| text.starts_with(pattern))
    }
}

pub struct WebScraper {
    client: reqwest::Client,
    extractor: ContentExtractor,
}

impl WebScraper {
    pub fn new(config: &Config) -> Result<Self> {
        let client = reqwest::Client::new();
        let extractor = ContentExtractor::new(config)?;

        Ok(Self { client, extractor })
    }

    pub async fn scrape_chapter(
        &self,
        record: &ChapterRecord,
        output_dir: &Path,
        stats_pb: Option<&ProgressBar>,
    ) -> Result<()> {
        let chapter_name = &record.chapter_number;

        if let Some(pb) = stats_pb {
            pb.println(format!(
                "🔄 Starting chapter {}: {}",
                chapter_name, record.url
            ));
        }

        // Fetch the web page
        let response = self
            .client
            .get(&record.url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch URL: {}", record.url))?;

        let html = response
            .text()
            .await
            .context("Failed to read response body")?;

        if let Some(pb) = stats_pb {
            pb.println(format!("📄 Parsing content from {}", record.url));
        }

        // Extract content from HTML
        let content = self
            .extractor
            .extract_content(&html)
            .with_context(|| format!("Failed to extract content from {}", record.url))?;

        // Save to file
        let file_path = output_dir.join(&record.file_name());
        self.save_content(&file_path, &content).await?;

        if let Some(pb) = stats_pb {
            pb.println(format!("✅ Completed chapter {}", chapter_name));
        }

        Ok(())
    }

    async fn save_content(&self, file_path: &Path, content: &str) -> Result<()> {
        let mut file = File::create(file_path)
            .await
            .with_context(|| format!("Failed to create file: {:?}", file_path))?;

        file.write_all(content.as_bytes())
            .await
            .context("Failed to write content to file")?;

        Ok(())
    }
}
