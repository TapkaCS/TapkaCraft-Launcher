//! Shared test-only infrastructure: a minimal raw-HTTP mock server used
//! wherever a test needs to exercise real `reqwest` network code without
//! reaching an external host - this sandbox's egress policy blocks the
//! real Mojang/Microsoft/Modrinth domains outright, so integration tests
//! for anything that talks to them run against this instead.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Binds a listening socket and returns it together with its address, so a
/// test can bake the address into address-dependent payloads (a JSON body
/// containing download URLs, for instance) *before* the file map those
/// payloads live inside is finalized and served. Spawning a *second* server
/// after adding more files - so URLs baked from the first server's address
/// are served by a different, incomplete one - is exactly the bug this
/// split prevents; see `core::versions::install`'s tests for the case that
/// caught it.
pub async fn bind_mock_server() -> (TcpListener, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    (listener, addr)
}

/// Starts serving `files` (request path -> response body) over an
/// already-bound `listener`, responding 200 for a known path and 404
/// otherwise. Runs until the listener is dropped.
pub fn serve_mock_files(listener: TcpListener, files: HashMap<String, Vec<u8>>) {
    let files = Arc::new(files);
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let files = files.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let Ok(n) = socket.read(&mut buf).await else {
                    return;
                };
                let request = String::from_utf8_lossy(&buf[..n]);
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_string();
                match files.get(&path) {
                    Some(content) => {
                        let header = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            content.len()
                        );
                        let _ = socket.write_all(header.as_bytes()).await;
                        let _ = socket.write_all(content).await;
                    }
                    None => {
                        let _ = socket
                            .write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n")
                            .await;
                    }
                }
            });
        }
    });
}

/// Convenience for the common case with no chicken-and-egg URL problem:
/// binds, serves `files` immediately, and returns the base URL
/// (`http://127.0.0.1:PORT`) to build request URLs from.
pub async fn spawn_mock_server(files: HashMap<String, Vec<u8>>) -> String {
    let (listener, addr) = bind_mock_server().await;
    serve_mock_files(listener, files);
    format!("http://{addr}")
}
