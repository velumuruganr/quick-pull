//! Progress observers used by workers to report progress.
//!
//! Implementations of `ProgressObserver` are used by the workers to
//! update either a console-based progress bar or the daemon's in-memory
//! job tracking structure.
use crate::daemon::ActiveJobData;
use indicatif::ProgressBar;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Trait implemented by progress observers used while downloading.
///
/// Implementors receive small updates (increments), finalization
/// notifications and textual messages to display to the user or UI.
pub trait ProgressObserver: Send + Sync {
    /// Increment the observer by `delta` bytes.
    fn inc(&self, delta: u64);
    /// Called when the observed work is finished.
    fn finish(&self);
    /// Send a short human-readable message to the observer/UI.
    fn message(&self, message: String);
}

/// A console-based observer that updates an `indicatif::ProgressBar`.
pub struct ConsoleObserver {
    pub pb: ProgressBar,
}

impl ProgressObserver for ConsoleObserver {
    fn inc(&self, delta: u64) {
        self.pb.inc(delta);
    }

    fn finish(&self) {
        self.pb.finish_with_message("Done!");
    }

    fn message(&self, message: String) {
        self.pb.set_message(message);
    }
}

/// An observer used when running as a daemon; updates the shared job state.
pub struct DaemonObserver {
    pub job_data: Arc<ActiveJobData>,
}

impl ProgressObserver for DaemonObserver {
    fn inc(&self, delta: u64) {
        self.job_data
            .downloaded_bytes
            .fetch_add(delta, Ordering::Relaxed);
    }

    fn finish(&self) {
        self.message("Done".into());
    }

    fn message(&self, message: String) {
        let job_ref = self.job_data.clone();
        tokio::spawn(async move {
            let mut state = job_ref.state.lock().await;
            *state = message;
        });
    }
}
