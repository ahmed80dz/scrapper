use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use scraper::{Html, Selector};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::task::JoinSet;
use tokio::time::{Duration, sleep};
use tokio_stream::StreamExt;
const MAX_CONCURRENT_TASKS: usize = 20;
#[tokio::main]
async fn main() -> Result<()> {
    let mut rdr = csv_async::AsyncReader::from_reader(
        File::open("./out/links.csv")
            .await
            .context("Failed to open links.csv")?,
    );
    let mut records = rdr.records();
    let mut total = 0;
    let mut existing = 0;

    while let Some(record) = records.next().await {
        let record = record.context("Failed to read CSV record while counting")?;
        total += 1;

        if let Some(chapn) = record.get(1) {
            let file_name = format!("./out/chapter_{chapn}.txt");
            if Path::new(&file_name).exists() {
                existing += 1;
            }
        }
    }
    let records_to_process = total - existing;
    if records_to_process == 0 {
        println!("All files already exist. Nothing to process.");
        return Ok(());
    }
    let multi_progress = MultiProgress::new();
    let main_pb = multi_progress.add(ProgressBar::new(records_to_process as u64));
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
    let mut rdr = csv_async::AsyncReader::from_reader(
        File::open("./out/links.csv")
            .await
            .context("Failed to open links.csv")?,
    );
    let mut records = rdr.records();

    let mut set = JoinSet::new();
    let mut success_count = 0;
    let mut error_count = 0;
    while let Some(record) = records.next().await {
        let record = record.context("Failed to read CSV record")?;
        let chapn = record
            .get(1)
            .context("Missing chapter number column in CSV")?;
        let link = record
            .get(0)
            .context("Missing link column in CSV")?
            .to_string();
        let file_name = format!("./out/chapter_{chapn}.txt");
        if Path::new(&file_name).exists() {
            stats_pb.println(format!("Skipping existing file: {file_name}"));
            continue;
        }
        if set.len() >= MAX_CONCURRENT_TASKS {
            if let Some(result) = set.join_next().await {
                match result {
                    Ok(Ok(_)) => {
                        success_count += 1;
                        main_pb.inc(1);
                    }
                    Ok(Err(e)) => {
                        error_count += 1;
                        stats_pb.println(format!("❌ Error: {e}"));
                        main_pb.inc(1);
                    }
                    Err(e) => {
                        error_count += 1;
                        stats_pb.println(format!("❌ Task panicked: {e}"));
                        main_pb.inc(1);
                    }
                }
            }
        }
        active_pb.set_message(format!("{} tasks", set.len()));
        stats_pb.set_message(format!(
            "✅ {} success, ❌ {} errors, 📥 {} queued",
            success_count,
            error_count,
            set.len()
        ));
        let stats_pb_clone = stats_pb.clone();
        set.spawn(async move { scrape(link, file_name, stats_pb_clone).await });
        sleep(Duration::from_millis(100)).await;
    }
    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(_)) => {
                success_count += 1;
                main_pb.inc(1);
            }
            Ok(Err(e)) => {
                error_count += 1;
                stats_pb.println(format!("❌ Error: {e}"));
                main_pb.inc(1);
            }
            Err(e) => {
                error_count += 1;
                stats_pb.println(format!("❌ Task panicked: {e}"));
                main_pb.inc(1);
            }
        }

        // Update counters
        active_pb.set_message(format!("{} tasks", set.len()));
        stats_pb.set_message(format!(
            "✅ {} success, ❌ {} errors, 📥 {} remaining",
            success_count,
            error_count,
            set.len()
        ));
    }
    main_pb.finish_with_message("✨ All chapters processed!");
    stats_pb.finish_with_message(format!(
        "Final: ✅ {success_count} success, ❌ {error_count} errors"
    ));
    active_pb.finish_and_clear();

    println!("\n🎉 Scraping completed! {success_count} successful, {error_count} errors");
    Ok(())
}

async fn scrape(link: String, file_name: String, stats_pb: ProgressBar) -> Result<()> {
    let chapter_name = file_name
        .strip_prefix("./out/chapter_")
        .and_then(|s| s.strip_suffix(".txt"))
        .unwrap_or("unknown");

    stats_pb.println(format!("🔄 Starting chapter {chapter_name}: {link}"));
    let resp = reqwest::get(&link).await?.text().await?;
    stats_pb.println(format!("parssing {link}"));
    let mut out = String::new();
    {
        let html = Html::parse_document(&resp);
        let selector = Selector::parse(".content-inner").unwrap();
        for node in html.select(&selector).next().unwrap().text().skip(5) {
            if !node.starts_with("window.pubfuturetag") {
                out.push_str(node);
            }
            out.push('\n')
        }
    }

    stats_pb.println(format!("saving file {file_name}"));
    let mut file = File::create(&file_name)
        .await
        .with_context(|| format!("Failed to create file: {file_name}"))?;
    file.write_all(out.as_bytes())
        .await
        .context("Failed to write content to file")?;
    stats_pb.println(format!("✅ Completed chapter {chapter_name}"));
    Ok::<_, anyhow::Error>(())
}
