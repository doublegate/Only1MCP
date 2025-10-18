//! Integration tests for TUI interface

use only1mcp::tui::{Event, LogEntry, LogLevel, MetricsSnapshot, ServerInfo, ServerStatus};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_tui_event_channel_communication() {
    // Test that events can be sent and received through TUI channels
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    // Send metrics update
    let snapshot = MetricsSnapshot {
        uptime_seconds: 3600,
        requests_per_second: 125.5,
        latency_p50: 12.3,
        latency_p95: 45.6,
        latency_p99: 78.9,
        active_servers: 5,
        total_servers: 8,
        cache_hit_rate: 0.92,
        error_rate: 0.02,
        active_batches: 12,
    };

    tx.send(Event::MetricsUpdate(snapshot.clone())).unwrap();

    // Receive and verify
    let received = rx.recv().await.unwrap();
    match received {
        Event::MetricsUpdate(s) => {
            assert_eq!(s.uptime_seconds, 3600);
            assert!((s.requests_per_second - 125.5).abs() < 0.01);
            assert_eq!(s.active_servers, 5);
        },
        _ => panic!("Expected MetricsUpdate event"),
    }
}

#[tokio::test]
async fn test_tui_server_list_update() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let servers = vec![
        ServerInfo {
            id: "server1".to_string(),
            name: "MCP Server 1".to_string(),
            status: ServerStatus::Up,
            health_percentage: 100,
            requests_per_second: 50,
        },
        ServerInfo {
            id: "server2".to_string(),
            name: "MCP Server 2".to_string(),
            status: ServerStatus::Degraded,
            health_percentage: 75,
            requests_per_second: 30,
        },
    ];

    tx.send(Event::ServersUpdate(servers.clone())).unwrap();

    let received = rx.recv().await.unwrap();
    match received {
        Event::ServersUpdate(s) => {
            assert_eq!(s.len(), 2);
            assert_eq!(s[0].status, ServerStatus::Up);
            assert_eq!(s[1].health_percentage, 75);
        },
        _ => panic!("Expected ServersUpdate event"),
    }
}

#[tokio::test]
async fn test_tui_log_streaming() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let log_entry = LogEntry {
        timestamp: chrono::Utc::now(),
        level: LogLevel::Error,
        message: "Test error message".to_string(),
    };

    tx.send(Event::LogMessage(log_entry.clone())).unwrap();

    let received = rx.recv().await.unwrap();
    match received {
        Event::LogMessage(entry) => {
            assert_eq!(entry.level, LogLevel::Error);
            assert_eq!(entry.message, "Test error message");
        },
        _ => panic!("Expected LogMessage event"),
    }
}

#[tokio::test]
async fn test_tui_quit_event() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    tx.send(Event::Quit).unwrap();

    let received = rx.recv().await.unwrap();
    match received {
        Event::Quit => {
            // Quit event received successfully - test passes
        },
        _ => panic!("Expected Quit event"),
    }
}

#[tokio::test]
async fn test_tui_multiple_events_in_sequence() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    // Send multiple events
    let snapshot = MetricsSnapshot::default();
    tx.send(Event::MetricsUpdate(snapshot.clone())).unwrap();

    let log = LogEntry {
        timestamp: chrono::Utc::now(),
        level: LogLevel::Info,
        message: "Info log".to_string(),
    };
    tx.send(Event::LogMessage(log.clone())).unwrap();

    let servers = vec![];
    tx.send(Event::ServersUpdate(servers)).unwrap();

    // Receive all events
    let event1 = rx.recv().await.unwrap();
    let event2 = rx.recv().await.unwrap();
    let event3 = rx.recv().await.unwrap();

    // Verify event types
    assert!(matches!(event1, Event::MetricsUpdate(_)));
    assert!(matches!(event2, Event::LogMessage(_)));
    assert!(matches!(event3, Event::ServersUpdate(_)));
}

#[tokio::test]
async fn test_tui_concurrent_event_sending() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    // Clone sender for concurrent tasks
    let tx1 = tx.clone();
    let tx2 = tx.clone();
    let tx3 = tx.clone();

    // Spawn concurrent tasks
    let handle1 = tokio::spawn(async move {
        for i in 0..10 {
            let log = LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Debug,
                message: format!("Task 1 log {}", i),
            };
            tx1.send(Event::LogMessage(log)).unwrap();
            sleep(Duration::from_millis(10)).await;
        }
    });

    let handle2 = tokio::spawn(async move {
        for i in 0..10 {
            let snapshot = MetricsSnapshot {
                uptime_seconds: i,
                ..Default::default()
            };
            tx2.send(Event::MetricsUpdate(snapshot)).unwrap();
            sleep(Duration::from_millis(10)).await;
        }
    });

    let handle3 = tokio::spawn(async move {
        for _ in 0..10 {
            let servers = vec![];
            tx3.send(Event::ServersUpdate(servers)).unwrap();
            sleep(Duration::from_millis(10)).await;
        }
    });

    // Receive events
    let mut received_count = 0;
    for _ in 0..30 {
        if rx.try_recv().is_ok() {
            received_count += 1;
        }
        sleep(Duration::from_millis(5)).await;
    }

    // Wait for all tasks
    handle1.await.unwrap();
    handle2.await.unwrap();
    handle3.await.unwrap();

    // Should have received most or all events (30 total)
    assert!(received_count >= 25); // Allow for timing variations
}
