use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub enum Command {
    Add { url: String },
    Status,
    Shutdown,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok(String),
    StatusList(Vec<JobStatus>),
    Err(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobStatus {
    pub id: usize,
    pub filename: String,
    pub progress_percent: u64,
    pub state: String,
}
