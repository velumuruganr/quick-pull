use crate::ipc::{Command, JobStatus, Response};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

struct DaemonState {
    jobs: HashMap<usize, JobStatus>,
}

pub async fn start_daemon(port: u16) -> Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    println!("Daemon started on port {}", port);

    let state = Arc::new(Mutex::new(DaemonState {
        jobs: HashMap::new(),
    }));
    let next_id = Arc::new(AtomicUsize::new(1));

    loop {
        let (mut socket, _) = listener.accept().await?;
        let state_ref = state.clone();
        let next_id_ref = next_id.clone();

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            let n = match socket.read(&mut buf).await {
                Ok(0) => return,
                Ok(n) => n,
                Err(_) => return,
            };

            let req_str = String::from_utf8_lossy(&buf[..n]);

            let command: Command = match serde_json::from_str(&req_str) {
                Ok(c) => c,
                Err(e) => {
                    let _ =
                        send_response(&mut socket, Response::Err(format!("Invalid JSON: {}", e)))
                            .await;
                    return;
                }
            };

            match command {
                Command::Shutdown => {
                    let _ =
                        send_response(&mut socket, Response::Ok("Shutting down...".into())).await;
                    std::process::exit(0);
                }
                Command::Status => {
                    let locked = state_ref.lock().await;
                    let list: Vec<JobStatus> = locked.jobs.values().cloned().collect();
                    let _ = send_response(&mut socket, Response::StatusList(list)).await;
                }
                Command::Add { url } => {
                    let id = next_id_ref.fetch_add(1, Ordering::SeqCst);
                    let filename = crate::utils::get_filename_from_url(&url);

                    let job_status = JobStatus {
                        id,
                        filename: filename.clone(),
                        progress_percent: 0,
                        state: "Starting".to_string(),
                    };

                    {
                        let mut locked = state_ref.lock().await;
                        locked.jobs.insert(id, job_status);
                    }

                    let state_clone = state_ref.clone();
                    tokio::spawn(async move {
                        {
                            let mut locked = state_clone.lock().await;
                            if let Some(job) = locked.jobs.get_mut(&id) {
                                job.state = "Done".to_string();
                                job.progress_percent = 100;
                            }
                        }
                    });

                    let _ = send_response(&mut socket, Response::Ok(format!("Added job #{}", id)))
                        .await;
                }
            }
        });
    }
}

async fn send_response(socket: &mut tokio::net::TcpStream, resp: Response) -> Result<()> {
    let json = serde_json::to_string(&resp)?;
    socket.write_all(json.as_bytes()).await?;
    Ok(())
}
