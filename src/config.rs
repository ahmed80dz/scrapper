use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapingConfig {
    /// Maximum number of concurrent scraping tasks
    pub max_concurrent_tasks: usize,
    
    /// Delay between spawning tasks (milliseconds)
    pub task_delay_ms: u64,
    
    /// Path to input CSV file
    pub input_file: PathBuf,
    
    /// Output directory for scraped files
    pub output_dir: PathBuf,
    
    /// CSS selector for content extraction
    pub selector: String,
    
    /// Number of initial text nodes to skip
    pub skip_text_nodes: usize,
    
    /// Patterns to filter out from extracted text
    pub filter_patterns: Vec<String>,
    
    /// HTTP request timeout (seconds)
    pub request_timeout_secs: u64,
    
    /// User agent string for HTTP requests
    pub user_agent: String,
    
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for ScrapingConfig {
    fn default() -> Self {
        Self {
            // Reduced from 20 to be more respectful to servers
            // Most sites can handle 5-10 concurrent requests comfortably
            max_concurrent_tasks: 8,
            
            // Increased from 100ms to be more server-friendly
            // This gives servers breathing room between requests
            task_delay_ms: 250,
            
            // Keep existing paths - they're reasonable
            input_file: PathBuf::from("./out/links.csv"),
            output_dir: PathBuf::from("./out"),
            
            // More generic selector that works on many sites
            selector: "main, article, .content, .post-content, .entry-content, #content".to_string(),
            
            // Reduced from 5 to 2 - most sites don't need to skip many nodes
            skip_text_nodes: 2,
            
            // More comprehensive filter patterns for common unwanted content
            filter_patterns: vec![
                "window.".to_string(),        // JavaScript
                "document.".to_string(),      // JavaScript
                "function(".to_string(),      // JavaScript functions
                "Advertisement".to_string(),   // Ads
                "Subscribe".to_string(),      // Newsletter prompts
                "Cookie".to_string(),         // Cookie notices
                "Privacy Policy".to_string(), // Legal links
                "Terms of Service".to_string(), // Legal links
                "Sign up".to_string(),        // Registration prompts
                "Log in".to_string(),         // Login prompts
            ],
            
            // Increased from 30s - some content-heavy pages need more time
            request_timeout_secs: 45,
            
            // More realistic user agent that's less likely to be blocked
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
            
            // Keep verbose false for clean output by default
            verbose: false,
        }
    }
}

impl ScrapingConfig {
    /// Load configuration from a TOML file
    pub async fn from_file<P: Into<PathBuf>>(path: P) -> Result<Self> {
        let path = path.into();
        let contents = fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read config file: {:?}", path))?;
        
        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;
        
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from command line arguments, with optional config file override
    pub async fn from_args() -> Result<Self> {
        use clap::Parser;
        
        let args = Args::parse();
        
        // Start with default config
        let mut config = if let Some(config_path) = &args.config {
            Self::from_file(config_path).await.unwrap_or_else(|e| {
                eprintln!("Warning: Failed to load config file: {}", e);
                eprintln!("Using default configuration");
                Self::default()
            })
        } else {
            Self::default()
        };

        // Override with command line arguments
        if let Some(input) = args.input {
            config.input_file = input;
        }
        if let Some(output) = args.output {
            config.output_dir = output;
        }
        if let Some(selector) = args.selector {
            config.selector = selector;
        }
        if let Some(concurrent) = args.concurrent {
            config.max_concurrent_tasks = concurrent;
        }
        if let Some(delay) = args.delay {
            config.task_delay_ms = delay;
        }
        if args.verbose {
            config.verbose = true;
        }

        config.validate()?;
        Ok(config)
    }

    /// Save current configuration to a TOML file
    pub async fn save_to_file<P: Into<PathBuf>>(&self, path: P) -> Result<()> {
        let path = path.into();
        let toml_string = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        fs::write(&path, toml_string)
            .await
            .with_context(|| format!("Failed to write config file: {:?}", path))?;
        
        Ok(())
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        if self.max_concurrent_tasks == 0 {
            anyhow::bail!("max_concurrent_tasks must be greater than 0");
        }
        
        // Reduced max from 100 to 50 for better server etiquette
        if self.max_concurrent_tasks > 50 {
            anyhow::bail!("max_concurrent_tasks should not exceed 50 to be respectful to servers");
        }
        
        // Add minimum delay validation
        if self.task_delay_ms < 50 {
            anyhow::bail!("task_delay_ms should be at least 50ms to avoid overwhelming servers");
        }
        
        if self.selector.trim().is_empty() {
            anyhow::bail!("selector cannot be empty");
        }
        
        if self.request_timeout_secs == 0 {
            anyhow::bail!("request_timeout_secs must be greater than 0");
        }
        
        // Add reasonable timeout limits
        if self.request_timeout_secs > 300 {
            anyhow::bail!("request_timeout_secs should not exceed 300 seconds (5 minutes)");
        }

        // Validate file paths exist for input
        if !self.input_file.exists() {
            eprintln!("⚠️  Warning: Input file {:?} does not exist", self.input_file);
        }

        Ok(())
    }

    /// Create a sample configuration file
    pub async fn create_sample_config<P: Into<PathBuf>>(path: P) -> Result<()> {
        let config = Self::default();
        config.save_to_file(path).await
    }
}

#[derive(clap::Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Input CSV file path
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output directory
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// CSS selector for content extraction
    #[arg(short, long)]
    selector: Option<String>,

    /// Maximum concurrent tasks
    #[arg(long)]
    concurrent: Option<usize>,

    /// Delay between tasks (milliseconds)
    #[arg(long)]
    delay: Option<u64>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Generate sample configuration file
    #[arg(long)]
    generate_config: Option<PathBuf>,
}

pub async fn handle_config_generation() -> Result<bool> {
    use clap::Parser;
    
    let args = Args::parse();
    
    if let Some(config_path) = args.generate_config {
        ScrapingConfig::create_sample_config(&config_path).await?;
        println!("✅ Sample configuration created at: {:?}", config_path);
        println!("💡 Edit the file and run with: cargo run -- --config {:?}", config_path);
        return Ok(true); // Indicates we should exit after generating config
    }
    
    Ok(false)
}
