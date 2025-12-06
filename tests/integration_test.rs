use indicatif::ProgressBar;
use std::sync::Arc;
use tempfile::NamedTempFile;
use tokio::sync::Mutex;
use wiremock::matchers::{header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use parallel_downloader::state::{Chunk, DownloadState};
use parallel_downloader::worker::download_chunk;

#[tokio::test]
async fn test_download_single_chunk() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(header("Range", "bytes=0-10"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Hello World"))
        .mount(&mock_server)
        .await;

    let temp_file = NamedTempFile::new().unwrap();
    let output_path = temp_file.path().to_str().unwrap().to_string();
    let state_path = format!("{}.state", output_path);

    let state = Arc::new(Mutex::new(DownloadState {
        url: mock_server.uri(),
        chunks: vec![Chunk {
            start: 0,
            end: 10,
            completed: false,
        }],
    }));

    let chunk = Chunk {
        start: 0,
        end: 10,
        completed: false,
    };
    let pb = ProgressBar::hidden(); // Invisible progress bar for testing

    let result = download_chunk(
        0,
        chunk,
        output_path.clone(),
        pb,
        state.clone(),
        state_path.clone(),
        None, // No rate limiter
    )
    .await;

    assert!(result.is_ok());

    let content = tokio::fs::read_to_string(&output_path).await.unwrap();
    assert_eq!(content, "Hello World");

    let locked_state = state.lock().await;
    assert!(locked_state.chunks[0].completed);
}
