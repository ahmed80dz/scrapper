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
git clone https://github.com/ahmed80dz/scrapper.git
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

```bash
cargo run
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

You can modify the following constants in `src/main.rs`:

- `MAX_CONCURRENT_TASKS`: Maximum number of simultaneous scraping tasks (default: 20)
- Rate limiting delay: Currently set to 100ms between task spawns

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

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

[Add contribution guidelines here]
