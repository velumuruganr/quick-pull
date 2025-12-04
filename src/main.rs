use anyhow::{Result, anyhow, Context};
use clap::Parser;
use futures_util::future::join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::io::{AsyncWriteExt, AsyncSeekExt};
use std::io::SeekFrom;
use reqwest::header::{CONTENT_LENGTH, RANGE};

#[derive(Debug, Clone, Copy)]
struct Chunk {
    start: u64,
    end: u64,
}

/// A fast, concurrent file downloader built in Rust.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The URL of the file to download
    #[arg(short, long)]
    url: String,

    /// The output file name
    #[arg(short, long)]
    output: Option<String>,

    /// Number of threads to use
    #[arg(short = 't', long, default_value_t = 4)]
    threads: u8,
}

fn calculate_chunks(total_size: u64, num_threads: u64) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let chunk_size = total_size / num_threads;

    for i in 0..num_threads {
        let start = i * chunk_size;

        let end = if i == num_threads - 1 {
            total_size - 1
        } else {
            (start + chunk_size) - 1
        };

        chunks.push(Chunk { start, end })
    }

    chunks
}

async fn get_file_size(url: &str) -> Result<u64> {
    let client = reqwest::Client::new();

    let response = client.head(url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Request failed. Status Code: {}",
            response.status()
        ));
    }

    let headers = response.headers();
    let content_length = headers
        .get(CONTENT_LENGTH)
        .ok_or(anyhow!("Content Length not found in response header."))?
        .to_str()?
        .parse::<u64>()?;

    Ok(content_length)
}

async fn download_chunk(url: String, chunk: Chunk, output_file: String, pb: ProgressBar) -> Result<()> {
    let client = reqwest::Client::new();
    let range_header = format!("bytes={}-{}", chunk.start, chunk.end);
    
    let mut response = client.get(&url).header(RANGE, range_header).send().await?;

    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .open(&output_file)
        .await
        .context("Failed to open file")?;

    file.seek(SeekFrom::Start(chunk.start)).await?;

    while let Some(response_bytes) = response.chunk().await? {
        pb.inc(response_bytes.len() as u64);
        file.write_all(&response_bytes).await?;
    }

    pb.finish_with_message("Done!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("Starting download for: {}", args.url);

    let file_size: u64 = get_file_size(&args.url).await?;
    println!("File Size: {}", file_size);

    let output_filename = args.output.unwrap_or_else(|| "output.bin".to_string());

    let file = tokio::fs::File::create(&output_filename).await?;
    file.set_len(file_size).await?;

    let chunks = calculate_chunks(file_size, args.threads as u64);

    let multi_progress = MultiProgress::new();

    let style = ProgressStyle::with_template(
        "{msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes/total_bytes} ({eta})"
    ).unwrap().progress_chars("=>-");

    let mut tasks = Vec::new();

    for (i, chunk) in chunks.into_iter().enumerate() {
        let url = args.url.clone();
        let filename = output_filename.clone();

        let pb = multi_progress.add(ProgressBar::new(chunk.end - chunk.start + 1));
        pb.set_style(style.clone());
        pb.set_message(format!("Thread: {}", i+1));
        let task = tokio::spawn(async move {
            download_chunk(url, chunk, filename, pb).await
        });

        tasks.push(task);
    }
    println!("Downloading...");

    let results = join_all(tasks).await;

    for result in results {
        result??;
    }

    println!("Download completed.");
    Ok(())
}
