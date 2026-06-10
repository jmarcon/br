//! Client side of the daemon IPC: used by `br open` to hand off a request to
//! an already-running daemon, avoiding cold-start cost.

use crate::{OpenRequest, PORT};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Attempts to send `request` to a running daemon. Returns `Ok(true)` if the
/// daemon accepted and handled the request, `Ok(false)`/`Err` if no daemon is
/// reachable (the caller should fall back to handling it locally).
pub fn try_dispatch(request: &OpenRequest) -> std::io::Result<bool> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{PORT}").parse().unwrap(),
        Duration::from_millis(200),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(request.encode().as_bytes())?;
    stream.flush()?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response.trim() == "OK")
}

/// Returns whether a daemon appears to be listening on the IPC port.
pub fn is_running() -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{PORT}").parse().unwrap(),
        Duration::from_millis(100),
    )
    .is_ok()
}
