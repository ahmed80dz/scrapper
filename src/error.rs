use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the scrapper application
#[derive(Error, Debug)]
pub enum ScrapperError {
    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("CSV processing error: {message}")]
    Csv { message: String },

    #[error("File system error: {message}")]
    FileSystem {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("Web scraping error for URL '{url}': {message}")]
    WebScraping { url: String, message: String },

    #[error("Content extraction error for URL '{url}': {message}")]
    ContentExtraction { url: String, message: String },

    #[error("HTTP request error for URL '{url}': {message}")]
    Http {
        url: String,
        status: Option<u16>,
        message: String,
    },

    #[error("Task execution error: {message}")]
    TaskExecution { message: String },

    #[error("Progress tracking error: {message}")]
    Progress { message: String },

    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },

    #[error("IO error: {message}")]
    Io {
        message: String,
        path: Option<PathBuf>,
    },
}

impl ScrapperError {
    /// Create a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a CSV processing error
    pub fn csv<S: Into<String>>(message: S) -> Self {
        Self::Csv {
            message: message.into(),
        }
    }

    /// Create a file system error with optional path
    pub fn file_system<S: Into<String>>(message: S, path: Option<PathBuf>) -> Self {
        Self::FileSystem {
            message: message.into(),
            path,
        }
    }

    /// Create a web scraping error
    pub fn web_scraping<U: Into<String>, S: Into<String>>(url: U, message: S) -> Self {
        Self::WebScraping {
            url: url.into(),
            message: message.into(),
        }
    }

    /// Create a content extraction error
    pub fn content_extraction<U: Into<String>, S: Into<String>>(url: U, message: S) -> Self {
        Self::ContentExtraction {
            url: url.into(),
            message: message.into(),
        }
    }

    /// Create an HTTP error with optional status code
    pub fn http<U: Into<String>, S: Into<String>>(url: U, status: Option<u16>, message: S) -> Self {
        Self::Http {
            url: url.into(),
            status,
            message: message.into(),
        }
    }

    /// Create a task execution error
    pub fn task_execution<S: Into<String>>(message: S) -> Self {
        Self::TaskExecution {
            message: message.into(),
        }
    }

    /// Create a progress tracking error
    pub fn progress<S: Into<String>>(message: S) -> Self {
        Self::Progress {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation<F: Into<String>, S: Into<String>>(field: F, message: S) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an IO error with optional path
    pub fn io<S: Into<String>>(message: S, path: Option<PathBuf>) -> Self {
        Self::Io {
            message: message.into(),
            path,
        }
    }

    /// Check if the error is recoverable (temporary network issues, etc.)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ScrapperError::Http { status: Some(429), .. } // Rate limited
            | ScrapperError::Http { status: Some(503), .. } // Service unavailable
            | ScrapperError::Http { status: Some(502), .. } // Bad gateway
            | ScrapperError::Http { status: None, .. } // Connection error
        )
    }

    /// Get the URL associated with the error, if any
    pub fn url(&self) -> Option<&str> {
        match self {
            ScrapperError::WebScraping { url, .. }
            | ScrapperError::ContentExtraction { url, .. }
            | ScrapperError::Http { url, .. } => Some(url),
            _ => None,
        }
    }

    /// Get a user-friendly error message with debugging hints
    pub fn user_friendly_message(&self) -> String {
        match self {
            ScrapperError::Config { message } => {
                format!(
                    "Configuration issue: {message}. Check your config file or command-line arguments."
                )
            }
            ScrapperError::Csv { message } => {
                format!(
                    "CSV file issue: {message}. Ensure your CSV has the correct format with 'url,chapter_number' columns."
                )
            }
            ScrapperError::FileSystem { message, path } => {
                if let Some(path) = path {
                    format!(
                        "File system error at {path:?}: {message}. Check file permissions and disk space."
                    )
                } else {
                    format!("File system error: {message}. Check file permissions and disk space.")
                }
            }
            ScrapperError::WebScraping { url, message } => {
                format!(
                    "Failed to scrape {url}: {message}. The website might be down or blocking requests."
                )
            }
            ScrapperError::ContentExtraction { url, message } => {
                format!(
                    "Couldn't extract content from {url}: {message}. The page structure might have changed."
                )
            }
            ScrapperError::Http {
                url,
                status,
                message,
            } => match status {
                Some(404) => format!("Page not found (404): {url}. Check if the URL is correct."),
                Some(403) => {
                    format!("Access denied (403) for {url}. The site might be blocking scrapers.")
                }
                Some(429) => {
                    format!("Rate limited (429) for {url}. Increase delays between requests.")
                }
                Some(500..=599) => {
                    format!("Server error ({status:?}) for {url}: {message}. Try again later.")
                }
                Some(status) => format!("HTTP error ({status}) for {url}: {message}"),
                None => format!(
                    "Connection error for {url}: {message}. Check your internet connection."
                ),
            },
            ScrapperError::TaskExecution { message } => {
                format!(
                    "Task execution failed: {message}. This might indicate a programming error."
                )
            }
            ScrapperError::Progress { message } => {
                format!(
                    "Progress tracking error: {message}. This doesn't affect scraping functionality."
                )
            }
            ScrapperError::Validation { field, message } => {
                format!("Invalid {field}: {message}. Please check your configuration.")
            }
            ScrapperError::Io { message, path } => {
                if let Some(path) = path {
                    format!("I/O error at {path:?}: {message}. Check file permissions.")
                } else {
                    format!("I/O error: {message}. Check file permissions.")
                }
            }
        }
    }

    /// Get debugging information for developers
    pub fn debug_info(&self) -> String {
        match self {
            ScrapperError::Http {
                url,
                status,
                message,
            } => {
                format!("URL: {url}, Status: {status:?}, Details: {message}")
            }
            ScrapperError::FileSystem { message, path } | ScrapperError::Io { message, path } => {
                format!("Path: {path:?}, Details: {message}")
            }
            ScrapperError::WebScraping { url, message }
            | ScrapperError::ContentExtraction { url, message } => {
                format!("URL: {url}, Details: {message}")
            }
            ScrapperError::Validation { field, message } => {
                format!("Field: {field}, Details: {message}")
            }
            _ => format!("{self}"),
        }
    }
}

/// Result type alias for convenience
pub type ScrapperResult<T> = Result<T, ScrapperError>;

/// Convert from common error types
impl From<std::io::Error> for ScrapperError {
    fn from(err: std::io::Error) -> Self {
        ScrapperError::io(err.to_string(), None)
    }
}

impl From<reqwest::Error> for ScrapperError {
    fn from(err: reqwest::Error) -> Self {
        let url = err.url().map(|u| u.to_string()).unwrap_or_default();
        let status = err.status().map(|s| s.as_u16());
        ScrapperError::http(url, status, err.to_string())
    }
}

impl From<csv_async::Error> for ScrapperError {
    fn from(err: csv_async::Error) -> Self {
        ScrapperError::csv(err.to_string())
    }
}

impl From<toml::de::Error> for ScrapperError {
    fn from(err: toml::de::Error) -> Self {
        ScrapperError::config(format!("TOML parsing error: {err}"))
    }
}

impl From<tokio::task::JoinError> for ScrapperError {
    fn from(err: tokio::task::JoinError) -> Self {
        ScrapperError::task_execution(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let config_err = ScrapperError::config("Invalid timeout");
        assert!(matches!(config_err, ScrapperError::Config { .. }));

        let http_err = ScrapperError::http("https://example.com", Some(404), "Not found");
        assert!(matches!(http_err, ScrapperError::Http { .. }));
        assert_eq!(http_err.url(), Some("https://example.com"));
    }

    #[test]
    fn test_recoverable_errors() {
        let recoverable = ScrapperError::http("https://example.com", Some(429), "Rate limited");
        assert!(recoverable.is_recoverable());

        let non_recoverable = ScrapperError::http("https://example.com", Some(404), "Not found");
        assert!(!non_recoverable.is_recoverable());
    }

    #[test]
    fn test_user_friendly_messages() {
        let err = ScrapperError::http("https://example.com", Some(404), "Not found");
        let message = err.user_friendly_message();
        assert!(message.contains("Page not found"));
        assert!(message.contains("Check if the URL is correct"));
    }
}
