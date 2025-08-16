# Scrapper

A high-performance, concurrent web scraper built in Rust for extracting chapter content from web pages.

## Features

- **Concurrent Processing**: Handles up to 20 simultaneous scraping tasks
- **Progress Tracking**: Real-time progress bars showing processing status, success/error counts, and active tasks
- **Resume Support**: Automatically skips already downloaded chapters
- **Error Handling**: Robust error handling with detailed logging
- **Rate Limiting**: Built-in delays to be respectful to target servers

## Prerequisites

- Rust (2024 edition)
- A CSV file containing links to scrape

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd scrapper
```

2. Build the project:
```bash
cargo build --release
```

## Usage

### Setup

1. Create an `out` directory in the project root:
```bash
mkdir out
```

2. Prepare your CSV file at `./out/links.csv` with the following format:
```csv
link,chapter_number
https://example.com/chapter1,1
https://example.com/chapter2,2
```

### Running the Scraper

**Basic usage:**
```bash
cargo run
```

**With command-line options:**
```bash
# Custom input and output paths
cargo run -- --input ./data/links.csv --output ./results

# Different CSS selector and concurrency
cargo run -- --selector ".main-content" --concurrent 10

# Enable verbose output
cargo run -- --verbose
```

**Using a configuration file:**
```bash
# Generate sample configuration
cargo run -- --generate-config scrapper.toml

# Run with configuration file
cargo run -- --config scrapper.toml
```

The scraper will:
- Read links from `./out/links.csv`
- Skip any chapters that already exist as `./out/chapter_{number}.txt`
- Process remaining chapters concurrently
- Save content to `./out/chapter_{number}.txt` files

### Output

Each chapter will be saved as a text file in the format:
- `./out/chapter_1.txt`
- `./out/chapter_2.txt`
- etc.

## Configuration

Scrapper supports multiple configuration methods, with command-line arguments taking precedence over configuration files:

### Configuration File (Recommended)

1. **Generate a sample configuration:**
   ```bash
   cargo run -- --generate-config scrapper.toml
   ```

2. **Edit the configuration file** with your preferred settings:
   ```toml
   max_concurrent_tasks = 15
   task_delay_ms = 200
   selector = ".article-content"
   request_timeout_secs = 45
   verbose = true
   ```

3. **Run with the configuration:**
   ```bash
   cargo run -- --config scrapper.toml
   ```

### Command-Line Arguments

```bash
# All available options
cargo run -- --help

# Common examples
cargo run -- --input ./my-links.csv --output ./downloads --concurrent 5
cargo run -- --selector ".main-article" --delay 500 --verbose
```

### Configuration Options

| Option | CLI Flag | Default | Description |
|--------|----------|---------|-------------|
| Input File | `--input` | `./out/links.csv` | Path to CSV file with URLs |
| Output Directory | `--output` | `./out` | Directory for scraped files |
| CSS Selector | `--selector` | `.content-inner` | Element selector for content |
| Max Concurrent | `--concurrent` | `20` | Simultaneous scraping tasks |
| Task Delay | `--delay` | `100` | Milliseconds between tasks |
| Verbose Mode | `--verbose` | `false` | Enable detailed logging |
| Config File | `--config` | None | Path to TOML config file |

### Advanced Configuration

The configuration file supports additional options not available via CLI:

- **`filter_patterns`**: Text patterns to exclude from scraped content
- **`request_timeout_secs`**: HTTP request timeout
- **`user_agent`**: Custom user agent string
- **`skip_text_nodes`**: Number of initial text nodes to skip

## Dependencies

- **reqwest**: HTTP client for web requests
- **scraper**: HTML parsing and CSS selector support
- **tokio**: Async runtime
- **csv-async**: Async CSV reading
- **indicatif**: Progress bars and status indicators
- **anyhow**: Error handling

## Content Extraction

The scraper targets elements with the `.content-inner` CSS selector and:
- Skips the first 5 text nodes
- Filters out JavaScript content (lines starting with "window.pubfuturetag")
- Preserves text content with newlines

## Error Handling

The application provides detailed error reporting including:
- Failed HTTP requests
- File I/O errors
- CSV parsing errors
- Task panics

All errors are logged to the console with descriptive messages.

## Performance

- Processes multiple chapters simultaneously (up to 20 by default)
- Includes rate limiting to avoid overwhelming target servers
- Automatically resumes from where it left off if interrupted

## Output Structure

```
out/
├── links.csv          # Input CSV file
├── chapter_1.txt      # Scraped content
├── chapter_2.txt
└── ...
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

We welcome contributions to improve Scrapper! Here's how you can help:

### Getting Started

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/ahmed80dz/scrapper.git
   cd scrapper
   ```
3. **Create a branch** for your feature or bugfix:
   ```bash
   git checkout -b feature/your-feature-name
   ```

### Development Setup

1. Ensure you have Rust installed (2024 edition)
2. Install dependencies:
   ```bash
   cargo build
   ```
3. Run tests:
   ```bash
   cargo test
   ```
4. Check code formatting:
   ```bash
   cargo fmt --check
   ```

### Making Changes

- **Follow Rust conventions**: Use `cargo fmt` and `cargo clippy`
- **Write tests**: Add tests for new functionality
- **Update documentation**: Keep README and code comments current
- **Be respectful**: Follow responsible scraping practices

### Types of Contributions

- 🐛 **Bug fixes**: Fix issues or improve error handling
- ✨ **Features**: Add new selectors, output formats, or configuration options
- 📚 **Documentation**: Improve README, code comments, or examples
- 🔧 **Performance**: Optimize scraping speed or memory usage
- 🛡️ **Security**: Improve rate limiting or add safety features

### Pull Request Process

1. **Test thoroughly**: Ensure your changes work with various websites
2. **Update documentation**: Reflect changes in README if needed
3. **Commit messages**: Use clear, descriptive commit messages
4. **Submit PR**: Include a clear description of what your changes do

### Code Style

- Use `cargo fmt` for consistent formatting
- Run `cargo clippy` to catch common mistakes
- Keep functions focused and well-documented
- Use meaningful variable names
- Handle errors properly with `anyhow::Result`

### Reporting Issues

When reporting bugs, please include:
- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce the issue
- Expected vs actual behavior
- Sample CSV data (if relevant)

### Ideas for Contributions

- Support for different CSS selectors per site
- Multiple output formats (JSON, XML, etc.)
- Proxy support for scraping
- Better error recovery and retry logic
- Configuration file support
- More detailed logging options

### Questions?

Feel free to open an issue for discussion before starting work on major changes. We're happy to provide guidance and feedback!

Thank you for contributing! 🎉
