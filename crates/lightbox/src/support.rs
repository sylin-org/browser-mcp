// SPDX-License-Identifier: LicenseRef-Ghostlight-Commercial
//! Harness support (ADR-0056): a self-cleaning temp root, org-signing helpers, and a localhost
//! bundle server so scenarios exercise the REAL managed:// code (including the ureq/rustls fetch)
//! without touching a fixed admin location or the network.

use std::io::{BufRead as _, Read as _, Write as _};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use ghostlight_core::governance::crypto::admin as crypto_admin;
use ghostlight_core::governance::manifest::bundle;

static UNIQUE: AtomicU64 = AtomicU64::new(0);

/// A temp directory that removes itself on drop.
pub struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    pub fn new(tag: &str) -> anyhow::Result<Self> {
        let n = UNIQUE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("lightbox-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A minimal schema-3 manifest value naming the org and one all-hosts read grant.
pub fn manifest(name: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": 3,
        "name": name,
        "version": "1",
        "grants": [],
    })
}

/// Sign a policy bundle over `manifest` at `seq` with the Ed25519 `seed` (evaluation-grade key).
pub fn sign(seed: &[u8; 32], seq: u64, manifest: serde_json::Value) -> Vec<u8> {
    bundle::sign_bundle(seed, None, seq, manifest, None)
}

/// The org's Ed25519 public key as lowercase hex, for a `managed.json` bootstrap.
pub fn pubkey_hex(seed: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in crypto_admin::ed_public(seed) {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Write a `managed.json` bootstrap at `path` pointing at `source`, trusting the `seed`'s public key.
pub fn write_bootstrap(path: &Path, source: &str, seed: &[u8; 32]) -> anyhow::Result<()> {
    let json = serde_json::json!({
        "source": source,
        "pubkey_ed25519": pubkey_hex(seed),
    });
    std::fs::write(path, serde_json::to_vec_pretty(&json)?)?;
    Ok(())
}

struct ServerState {
    bytes: Vec<u8>,
    etag: String,
    version: u64,
}

/// A localhost HTTP server that serves a policy bundle (with ETag / 304 support) until dropped. The
/// served bundle can be swapped mid-run ([`BundleServer::set_bundle`]) for the poll-update scenario.
pub struct BundleServer {
    addr: SocketAddr,
    state: Arc<Mutex<ServerState>>,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl BundleServer {
    pub fn start(bytes: Vec<u8>) -> anyhow::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let state = Arc::new(Mutex::new(ServerState {
            bytes,
            etag: "\"v1\"".to_string(),
            version: 1,
        }));
        let shutdown = Arc::new(AtomicBool::new(false));
        let (st, sd) = (state.clone(), shutdown.clone());
        let handle = std::thread::spawn(move || serve_loop(listener, st, sd));
        Ok(Self {
            addr,
            state,
            shutdown,
            handle: Some(handle),
        })
    }

    /// The URL a `managed.json` `source` should use.
    pub fn url(&self) -> String {
        format!("http://{}/policy.bundle", self.addr)
    }

    /// Swap the served bundle (bumps the ETag), simulating the org publishing a new policy.
    pub fn set_bundle(&self, bytes: Vec<u8>) {
        let mut s = self.state.lock().unwrap();
        s.version += 1;
        s.etag = format!("\"v{}\"", s.version);
        s.bytes = bytes;
    }
}

impl Drop for BundleServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        // Unblock the accept() so the loop can observe the shutdown flag and exit.
        let _ = TcpStream::connect(self.addr);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn serve_loop(listener: TcpListener, state: Arc<Mutex<ServerState>>, shutdown: Arc<AtomicBool>) {
    for stream in listener.incoming() {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        let Ok(mut stream) = stream else { break };
        let mut reader = std::io::BufReader::new(match stream.try_clone() {
            Ok(s) => s,
            Err(_) => continue,
        });
        let mut if_none_match: Option<String> = None;
        let mut line = String::new();
        loop {
            line.clear();
            if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                break;
            }
            if line.to_ascii_lowercase().starts_with("if-none-match:") {
                if let Some(idx) = line.find(':') {
                    if_none_match = Some(line[idx + 1..].trim().to_string());
                }
            }
        }
        let s = state.lock().unwrap();
        let not_modified = if_none_match.as_deref() == Some(s.etag.as_str());
        let response = if not_modified {
            format!(
                "HTTP/1.1 304 Not Modified\r\nETag: {}\r\nConnection: close\r\n\r\n",
                s.etag
            )
        } else {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: {}\r\nConnection: close\r\n\r\n",
                s.bytes.len(),
                s.etag
            )
        };
        let _ = stream.write_all(response.as_bytes());
        if !not_modified {
            let _ = stream.write_all(&s.bytes);
        }
        let _ = stream.flush();
        // Drain any remaining request body so the client sees a clean close.
        let mut sink = [0u8; 256];
        let _ = stream.read(&mut sink);
    }
}
