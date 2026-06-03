use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Result of a single latency probe.
#[derive(Debug, Clone, Copy)]
pub enum LatencyStatus {
    /// Host reachable, round-trip in milliseconds.
    Reachable { ms: u64 },
    /// TCP connect failed or timed out.
    Unreachable,
    /// No latency check applicable (shell connections).
    Local,
    /// Not yet checked.
    Unknown,
}

/// Shared latency cache keyed by "host:port" strings.
/// Background threads write, draw reads.
pub type LatencyCache = Arc<Mutex<HashMap<String, LatencyStatus>>>;

/// Perform a TCP connect to measure latency.
/// Timeout is 3 seconds. Called from spawned threads.
pub fn probe(host: &str, port: u16) -> LatencyStatus {
    let target = format!("{host}:{port}");
    let start = Instant::now();

    let addr = match std::net::ToSocketAddrs::to_socket_addrs(&target) {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => return LatencyStatus::Unreachable,
        },
        Err(_) => return LatencyStatus::Unreachable,
    };

    match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
        Ok(_) => {
            let ms = start.elapsed().as_millis() as u64;
            LatencyStatus::Reachable { ms }
        }
        Err(_) => LatencyStatus::Unreachable,
    }
}
