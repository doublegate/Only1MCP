//! Signal handling for graceful shutdown
//!
//! Provides asynchronous signal handling for SIGTERM and SIGINT to enable
//! graceful shutdown of the server.

use tokio::sync::broadcast;
use tracing::{error, info};

/// Setup signal handlers for graceful shutdown
///
/// Returns a broadcast sender that will send a shutdown signal when SIGTERM or SIGINT is received.
/// The server should subscribe to this channel and initiate shutdown when a signal is received.
///
/// # Example
/// ```rust,no_run
/// use only1mcp::daemon::signals::setup_signal_handlers;
///
/// #[tokio::main]
/// async fn main() {
///     let (shutdown_tx, mut shutdown_rx) = setup_signal_handlers();
///
///     // Server runs until shutdown signal received
///     shutdown_rx.recv().await;
///     println!("Shutting down...");
/// }
/// ```
#[cfg(unix)]
pub fn setup_signal_handlers() -> (broadcast::Sender<()>, broadcast::Receiver<()>) {
    use tokio::signal::unix::{signal, SignalKind};

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let shutdown_tx_clone = shutdown_tx.clone();

    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to setup SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to setup SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
            }
        }

        if let Err(e) = shutdown_tx_clone.send(()) {
            error!("Failed to send shutdown signal: {}", e);
        }
    });

    (shutdown_tx, shutdown_rx)
}

/// Setup signal handlers for graceful shutdown (Windows version)
///
/// Windows doesn't support SIGTERM, so we only handle Ctrl+C.
#[cfg(windows)]
pub fn setup_signal_handlers() -> (broadcast::Sender<()>, broadcast::Receiver<()>) {
    use tokio::signal;

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let shutdown_tx_clone = shutdown_tx.clone();

    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to listen for Ctrl+C: {}", e);
            return;
        }

        info!("Received Ctrl+C, initiating graceful shutdown");

        if let Err(e) = shutdown_tx_clone.send(()) {
            error!("Failed to send shutdown signal: {}", e);
        }
    });

    (shutdown_tx, shutdown_rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_signal_handler_creation() {
        let (_tx, _rx) = setup_signal_handlers();
        // If we get here without panicking, the signal handler was set up successfully
    }
}
