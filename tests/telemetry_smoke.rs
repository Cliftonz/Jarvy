use assert_cmd::prelude::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

fn handle_client(
    mut stream: TcpStream,
    traces_seen: &Arc<AtomicBool>,
    logs_seen: &Arc<AtomicBool>,
) {
    // Read headers
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let headers_end;
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => return, // connection closed
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(i) = twoway::find_bytes(&buf, b"\r\n\r\n") {
                    headers_end = i + 4;
                    break;
                }
                if buf.len() > 1024 * 1024 {
                    // 1MB header guard
                    return;
                }
            }
            Err(_) => return,
        }
    }

    // Parse request line and headers
    let headers = match std::str::from_utf8(&buf[..headers_end]) {
        Ok(h) => h,
        Err(_) => return,
    };
    let mut lines = headers.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let _method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    // Mark which endpoint was hit
    if path == "/v1/traces" {
        traces_seen.store(true, Ordering::SeqCst);
    }
    if path == "/v1/logs" {
        logs_seen.store(true, Ordering::SeqCst);
    }

    // Determine content length
    let mut content_len: usize = 0;
    for line in lines {
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            if let Ok(v) = rest.trim().parse::<usize>() {
                content_len = v;
            }
        } else if let Some(rest) = line.strip_prefix("content-length:") {
            if let Ok(v) = rest.trim().parse::<usize>() {
                content_len = v;
            }
        }
    }

    // Read remaining body if any
    let already = buf.len().saturating_sub(headers_end);
    let to_read = content_len.saturating_sub(already);
    let mut remaining = to_read;
    while remaining > 0 {
        let n = match stream.read(&mut tmp) {
            Ok(n) => n,
            Err(_) => 0,
        };
        if n == 0 {
            break;
        }
        remaining = remaining.saturating_sub(n);
    }

    // Respond 200 OK
    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let _ = stream.flush();
}

#[test]
fn telemetry_smoke_error_logs_only() -> Result<(), Box<dyn std::error::Error>> {
    // Try to bind the default compile-time OTLP HTTP port on both IPv4 and IPv6. If both are in use, skip.
    let listener_v4 = TcpListener::bind(("127.0.0.1", 4318)).ok();
    let listener_v6 = TcpListener::bind(("::1", 4318)).ok();
    if listener_v4.is_none() && listener_v6.is_none() {
        eprintln!(
            "SKIP telemetry_smoke: could not bind 127.0.0.1:4318 or [::1]:4318. Is an OTLP collector running?"
        );
        return Ok(());
    }
    if let Some(ref l) = listener_v4 {
        l.set_nonblocking(true)?;
    }
    if let Some(ref l) = listener_v6 {
        l.set_nonblocking(true)?;
    }

    let traces_seen = Arc::new(AtomicBool::new(false));
    let logs_seen = Arc::new(AtomicBool::new(false));
    let traces_seen_srv = Arc::clone(&traces_seen);
    let logs_seen_srv = Arc::clone(&logs_seen);

    // Server thread
    let server = thread::spawn(move || {
        let start = Instant::now();
        loop {
            let mut handled = false;
            if let Some(ref l) = listener_v4 {
                match l.accept() {
                    Ok((stream, _addr)) => {
                        handle_client(stream, &traces_seen_srv, &logs_seen_srv);
                        handled = true;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => {}
                }
            }
            if let Some(ref l) = listener_v6 {
                match l.accept() {
                    Ok((stream, _addr)) => {
                        handle_client(stream, &traces_seen_srv, &logs_seen_srv);
                        handled = true;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => {}
                }
            }
            if !handled {
                if start.elapsed() > Duration::from_secs(10) {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            if logs_seen_srv.load(Ordering::SeqCst) {
                break;
            }
        }
    });

    // Run the CLI with telemetry enabled and smoke trigger
    let mut cmd = Command::cargo_bin("jarvy")?;
    let assert = cmd
        .env("JARVY_TEST_MODE", "1")
        .env("JARVY_TELEMETRY_SMOKE", "1")
        .arg("bootstrap")
        .assert();
    // The CLI should exit successfully
    assert.success();

    // Wait until server observes the logs endpoint or timeout
    let timeout = Instant::now() + Duration::from_secs(10);
    while !logs_seen.load(Ordering::SeqCst) {
        if Instant::now() > timeout {
            break;
        }
        thread::sleep(Duration::from_millis(25));
    }

    // Tear down server thread
    let _ = server.join();

    assert!(
        logs_seen.load(Ordering::SeqCst),
        "no request to /v1/logs observed"
    );

    Ok(())
}

// Minimal, allocation-free substring finder for headers terminator
mod twoway {
    pub fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        haystack.windows(needle.len()).position(|w| w == needle)
    }
}
