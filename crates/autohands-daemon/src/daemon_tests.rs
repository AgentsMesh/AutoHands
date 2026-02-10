//! Tests for daemon types and state management.

use super::*;
use crate::daemon_status::DaemonStatus;

#[test]
fn test_daemon_state_conversion() {
    assert_eq!(DaemonStateValue::from(0), DaemonStateValue::Stopped);
    assert_eq!(DaemonStateValue::from(1), DaemonStateValue::Starting);
    assert_eq!(DaemonStateValue::from(2), DaemonStateValue::Running);
    assert_eq!(DaemonStateValue::from(3), DaemonStateValue::ShuttingDown);
    assert_eq!(DaemonStateValue::from(4), DaemonStateValue::Restarting);
    assert_eq!(DaemonStateValue::from(99), DaemonStateValue::Stopped);
}

#[test]
fn test_daemon_new() {
    let config = DaemonConfig::default();
    let daemon = Daemon::new(config).unwrap();
    assert_eq!(daemon.state(), DaemonState::Stopped);
    assert!(!daemon.is_running());
}

#[test]
fn test_daemon_invalid_config() {
    let mut config = DaemonConfig::default();
    config.restart_window_secs = 0;
    let result = Daemon::new(config);
    assert!(result.is_err());
}

#[test]
fn test_restart_tracker() {
    let config = DaemonConfig {
        max_restarts: 3,
        restart_window_secs: 60,
        ..Default::default()
    };
    let mut tracker = RestartTracker::new(&config);

    // First 3 restarts should be OK
    assert!(!tracker.record_restart());
    assert!(!tracker.record_restart());
    assert!(!tracker.record_restart());
    assert_eq!(tracker.count(), 3);

    // 4th restart should exceed limit
    assert!(tracker.record_restart());
}

#[tokio::test]
async fn test_daemon_status() {
    let config = DaemonConfig::default();
    let daemon = Daemon::new(config).unwrap();
    let status = daemon.status().await;

    assert_eq!(status.state, DaemonState::Stopped);
    assert!(status.pid.is_none());
}

#[test]
fn test_daemon_status_display() {
    let status = DaemonStatus {
        state: DaemonState::Running,
        pid: Some(12345),
        health_checks: 100,
        health_failures: 5,
    };

    let display = status.to_string();
    assert!(display.contains("running"));
    assert!(display.contains("12345"));
    assert!(display.contains("95/100"));
}
