//! The only crate that opens a network socket (knowledge/26 §4.2 rule (c)).
//! Two jobs: fetch a signed manifest and verify it; fetch a bundle's files
//! with resume and verify every byte.
//!
//! Emerging-market networks drop; a 1.5 GB model must never restart from
//! zero (knowledge/10). So: a `.part` file next to the target, a `Range`
//! request from its length, SHA-256 over the *whole* file (the already-present
//! part is hashed first), and an atomic rename only once the hash matches.
//! The policy decides which hosts may be contacted; this crate only asks.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use verdure_core::{signing, Bundle, Event, Manifest, ModelRef, Policy, Progress, VError};

const CHUNK: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(60);

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(20))
        .timeout_read(TIMEOUT)
        .timeout_write(TIMEOUT)
        .user_agent("verdure-sdk/0.1")
        .build()
}

fn net_err(e: ureq::Error) -> VError {
    match e {
        ureq::Error::Status(code, _) => VError::Network(format!("the server answered {code}")),
        ureq::Error::Transport(t) => VError::Network(t.to_string()),
    }
}

/// Fetch `manifest_url` and its detached signature at `manifest_url + ".sig"`,
/// verify with `public_key_b64`, parse. A manifest that fails its signature is
/// never parsed — the bytes are untrusted until then.
pub fn fetch_manifest(
    policy: &Policy,
    manifest_url: &str,
    public_key_b64: &str,
) -> Result<Manifest, VError> {
    policy.permits_download(manifest_url)?;
    let agent = agent();
    let body = get_bytes(&agent, manifest_url, 4 * 1024 * 1024)?;
    let sig = get_bytes(&agent, &format!("{manifest_url}.sig"), 4096)?;
    verify_manifest_bytes(
        &body,
        std::str::from_utf8(&sig).unwrap_or(""),
        public_key_b64,
    )
}

/// The signature check and parse, separated so it is testable without a socket.
pub fn verify_manifest_bytes(
    body: &[u8],
    signature_b64: &str,
    public_key_b64: &str,
) -> Result<Manifest, VError> {
    signing::verify(public_key_b64, body, signature_b64.trim()).map_err(|e| VError::Integrity {
        expected: "a manifest signed by the configured key".into(),
        actual: e,
    })?;
    Manifest::parse(body)
}

fn get_bytes(agent: &ureq::Agent, url: &str, cap: usize) -> Result<Vec<u8>, VError> {
    let resp = agent.get(url).call().map_err(net_err)?;
    let mut buf = Vec::new();
    resp.into_reader()
        .take(cap as u64 + 1)
        .read_to_end(&mut buf)
        .map_err(|e| VError::Network(e.to_string()))?;
    if buf.len() > cap {
        return Err(VError::Network(format!(
            "{url} is larger than the {cap}-byte limit"
        )));
    }
    Ok(buf)
}

/// Where a bundle's files live: `<data_dir>/bundles/<bundle id>/`.
pub fn bundle_dir(data_dir: &Path, bundle_id: &str) -> PathBuf {
    data_dir.join("bundles").join(bundle_id)
}

/// Download every file of `bundle` into `bundle_dir`, resuming partial files
/// and skipping verified complete ones. Progress is in bytes across the
/// whole bundle; cancellation leaves `.part` files in place for next time.
pub fn download_bundle(
    policy: &Policy,
    bundle: &Bundle,
    data_dir: &Path,
    progress: &dyn Progress,
) -> Result<PathBuf, VError> {
    let dir = bundle_dir(data_dir, &bundle.id);
    fs::create_dir_all(&dir)
        .map_err(|e| VError::Storage(format!("couldn't create {}: {e}", dir.display())))?;
    let total: u64 = bundle.download_bytes();
    let mut done_before: u64 = 0;
    for m in &bundle.models {
        let dest = dir.join(&m.file);
        download_model(policy, m, &dest, &mut |bytes| {
            progress.on_event(Event::Progress {
                done: done_before + bytes,
                total,
            });
            progress.is_cancelled()
        })?;
        done_before += m.size_bytes;
    }
    progress.on_event(Event::Progress { done: total, total });
    Ok(dir)
}

/// One file: resume, verify, rename. `tick(bytes_of_this_file)` returns true
/// to cancel. The policy is consulted before any socket opens.
pub fn download_model(
    policy: &Policy,
    model: &ModelRef,
    dest: &Path,
    tick: &mut dyn FnMut(u64) -> bool,
) -> Result<(), VError> {
    policy.permits_download(&model.url)?;
    fetch_file(model, dest, tick)
}

/// The resume/verify/rename machinery, after the policy has said yes.
fn fetch_file(
    model: &ModelRef,
    dest: &Path,
    tick: &mut dyn FnMut(u64) -> bool,
) -> Result<(), VError> {
    if dest.is_file() {
        if file_sha256(dest)? == model.sha256.to_ascii_lowercase() {
            tick(model.size_bytes);
            return Ok(());
        }
        // A complete-looking file with the wrong hash is corrupt: start over.
        fs::remove_file(dest).map_err(|e| VError::Storage(e.to_string()))?;
    }
    let part = dest.with_extension(format!(
        "{}.part",
        dest.extension().and_then(|e| e.to_str()).unwrap_or("bin")
    ));
    let mut have = part.metadata().map(|m| m.len()).unwrap_or(0);
    if have > model.size_bytes {
        fs::remove_file(&part).map_err(|e| VError::Storage(e.to_string()))?;
        have = 0;
    }
    let mut hasher = Sha256::new();
    if have > 0 {
        let mut f = File::open(&part).map_err(|e| VError::Storage(e.to_string()))?;
        hash_into(&mut hasher, &mut f)?;
    }
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part)
        .map_err(|e| VError::Storage(e.to_string()))?;

    if have < model.size_bytes {
        let agent = agent();
        let mut req = agent.get(&model.url);
        if have > 0 {
            req = req.set("Range", &format!("bytes={have}-"));
        }
        let resp = req.call().map_err(net_err)?;
        if have > 0 && resp.status() != 206 {
            // The server ignored the range: it is sending the whole file. Start the
            // hash and the part file over rather than appending a duplicate.
            drop(out);
            out = File::create(&part).map_err(|e| VError::Storage(e.to_string()))?;
            hasher = Sha256::new();
            have = 0;
        }
        let mut reader = resp.into_reader();
        let mut buf = vec![0u8; CHUNK];
        loop {
            let n = reader.read(&mut buf).map_err(|e| {
                VError::Network(format!(
                    "the connection dropped at {have} of {} bytes; resume to continue ({e})",
                    model.size_bytes
                ))
            })?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])
                .map_err(|e| VError::Storage(e.to_string()))?;
            hasher.update(&buf[..n]);
            have += n as u64;
            if have > model.size_bytes {
                return Err(VError::Integrity {
                    expected: format!("{} bytes", model.size_bytes),
                    actual: format!("more than {have} bytes"),
                });
            }
            if tick(have) {
                out.flush().ok();
                return Err(VError::Cancelled);
            }
        }
        out.flush().map_err(|e| VError::Storage(e.to_string()))?;
    }
    drop(out);

    if have != model.size_bytes {
        return Err(VError::Network(format!(
            "the connection ended at {have} of {} bytes; resume to continue",
            model.size_bytes
        )));
    }
    let actual = hex(&hasher.finalize());
    if actual != model.sha256.to_ascii_lowercase() {
        fs::remove_file(&part).ok();
        return Err(VError::Integrity {
            expected: model.sha256.to_ascii_lowercase(),
            actual,
        });
    }
    fs::rename(&part, dest).map_err(|e| VError::Storage(e.to_string()))?;
    Ok(())
}

/// Hex SHA-256 of a file on disk.
pub fn file_sha256(path: &Path) -> Result<String, VError> {
    let mut f = File::open(path).map_err(|e| VError::Storage(e.to_string()))?;
    let mut hasher = Sha256::new();
    hash_into(&mut hasher, &mut f)?;
    Ok(hex(&hasher.finalize()))
}

fn hash_into(hasher: &mut Sha256, f: &mut File) -> Result<(), VError> {
    f.seek(SeekFrom::Start(0))
        .map_err(|e| VError::Storage(e.to_string()))?;
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = f
            .read(&mut buf)
            .map_err(|e| VError::Storage(e.to_string()))?;
        if n == 0 {
            return Ok(());
        }
        hasher.update(&buf[..n]);
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use verdure_core::{ModelKind, NetworkPolicy};

    /// A one-file HTTP server on loopback that honours `Range`, can be told to
    /// cut the connection after N bytes, and counts requests. Plain `http://`
    /// is what it speaks, so the tests use a policy that allows it (the
    /// production default is https-only — asserted separately).
    struct Server {
        url: String,
        requests: Arc<Mutex<Vec<String>>>,
    }

    fn serve(body: Vec<u8>, cut_after: Option<usize>, ignore_range: bool) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let reqs = requests.clone();
        let body = Arc::new(body);
        std::thread::spawn(move || {
            let cut = Arc::new(Mutex::new(cut_after));
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).ok();
                let mut range: Option<u64> = None;
                loop {
                    let mut h = String::new();
                    if reader.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" {
                        break;
                    }
                    if let Some(v) = h.strip_prefix("Range: bytes=") {
                        range = v.trim().trim_end_matches('-').parse().ok();
                    }
                }
                reqs.lock().unwrap().push(line.trim().to_string());
                let mut s = stream;
                let (status, start) = match range {
                    Some(r) if !ignore_range => (
                        format!(
                            "206 Partial Content\r\nContent-Range: bytes {r}-{}/{}",
                            body.len() - 1,
                            body.len()
                        ),
                        r as usize,
                    ),
                    _ => ("200 OK".to_string(), 0),
                };
                let mut payload = &body[start..];
                let mut this_cut = cut.lock().unwrap();
                if let Some(n) = *this_cut {
                    payload = &payload[..n.min(payload.len())];
                    *this_cut = None; // only the first response is cut
                }
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len() - start
                );
                s.write_all(head.as_bytes()).ok();
                s.write_all(payload).ok();
                // Shutdown without sending the rest: the client sees EOF early.
                s.shutdown(std::net::Shutdown::Both).ok();
            }
        });
        Server {
            url: format!("http://127.0.0.1:{port}"),
            requests,
        }
    }

    fn body() -> Vec<u8> {
        (0..700_000u32).map(|i| (i % 251) as u8).collect()
    }

    fn model(url: &str, body: &[u8]) -> ModelRef {
        ModelRef {
            id: "m".into(),
            kind: ModelKind::Llm,
            name: "m".into(),
            file: "m.gguf".into(),
            url: url.into(),
            sha256: hex(&Sha256::digest(body)),
            size_bytes: body.len() as u64,
            engine: "llama".into(),
            prompt_style: None,
            kv_kb_per_token: 0,
            trained_ctx: 0,
            licence: "mit".into(),
        }
    }

    /// The loopback server speaks plain http, which the production policy
    /// refuses (asserted in `the_policy_is_consulted_before_any_socket_opens`),
    /// so the transfer tests call the machinery below the policy line.
    fn download_model_http(
        model: &ModelRef,
        dest: &Path,
        tick: &mut dyn FnMut(u64) -> bool,
    ) -> Result<(), VError> {
        fetch_file(model, dest, tick)
    }

    #[test]
    fn a_whole_file_downloads_verifies_and_lands_atomically() {
        let data = body();
        let srv = serve(data.clone(), None, false);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.gguf");
        let m = model(&format!("{}/m.gguf", srv.url), &data);
        let mut ticks = Vec::new();
        download_model_http(&m, &dest, &mut |b| {
            ticks.push(b);
            false
        })
        .unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
        assert!(
            !dest.with_extension("gguf.part").exists(),
            "no part file left behind"
        );
        assert_eq!(*ticks.last().unwrap(), data.len() as u64);
        assert!(ticks.len() > 1, "progress was reported along the way");
    }

    #[test]
    fn a_cut_connection_resumes_from_the_part_file_with_a_range_request() {
        let data = body();
        let srv = serve(data.clone(), Some(300_000), false);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.gguf");
        let m = model(&format!("{}/m.gguf", srv.url), &data);
        let err = download_model_http(&m, &dest, &mut |_| false).unwrap_err();
        assert!(
            matches!(err, VError::Network(ref s) if s.contains("resume")),
            "{err:?}"
        );
        let part = dest.with_extension("gguf.part");
        assert_eq!(
            part.metadata().unwrap().len(),
            300_000,
            "the first 300 000 bytes are kept"
        );

        download_model_http(&m, &dest, &mut |_| false).unwrap();
        assert_eq!(
            fs::read(&dest).unwrap(),
            data,
            "resumed bytes hash with the kept ones"
        );
        let reqs = srv.requests.lock().unwrap();
        assert_eq!(reqs.len(), 2);
        // The second request asked for the rest, not the whole file.
        assert!(reqs[1].starts_with("GET /m.gguf"));
        assert!(!part.exists());
    }

    #[test]
    fn a_server_that_ignores_range_does_not_corrupt_the_file() {
        let data = body();
        let srv = serve(data.clone(), Some(100_000), true);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.gguf");
        let m = model(&format!("{}/m.gguf", srv.url), &data);
        assert!(download_model_http(&m, &dest, &mut |_| false).is_err());
        download_model_http(&m, &dest, &mut |_| false).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
    }

    #[test]
    fn a_wrong_hash_is_refused_and_a_corrupt_complete_file_is_replaced() {
        let data = body();
        let srv = serve(data.clone(), None, false);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.gguf");
        let mut m = model(&format!("{}/m.gguf", srv.url), &data);
        m.sha256 = "ab".repeat(32);
        let err = download_model_http(&m, &dest, &mut |_| false).unwrap_err();
        assert!(matches!(err, VError::Integrity { .. }), "{err:?}");
        assert!(!dest.exists() && !dest.with_extension("gguf.part").exists());

        // A complete file already on disk with the wrong content is re-fetched.
        fs::write(&dest, b"stale").unwrap();
        let good = model(&format!("{}/m.gguf", srv.url), &data);
        download_model_http(&good, &dest, &mut |_| false).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
        // And a correct one is not fetched again.
        let before = srv.requests.lock().unwrap().len();
        download_model_http(&good, &dest, &mut |_| false).unwrap();
        assert_eq!(
            srv.requests.lock().unwrap().len(),
            before,
            "verified file, no request"
        );
    }

    #[test]
    fn cancellation_keeps_the_part_file_for_next_time() {
        let data = body();
        let srv = serve(data.clone(), None, false);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.gguf");
        let m = model(&format!("{}/m.gguf", srv.url), &data);
        let err = download_model_http(&m, &dest, &mut |b| b > 200_000).unwrap_err();
        assert_eq!(err, VError::Cancelled);
        let part = dest.with_extension("gguf.part");
        assert!(part.metadata().unwrap().len() >= 200_000);
        download_model_http(&m, &dest, &mut |_| false).unwrap();
        assert_eq!(fs::read(&dest).unwrap(), data);
    }

    #[test]
    fn the_policy_is_consulted_before_any_socket_opens() {
        let m = model("https://cdn.example.org/m.gguf", b"x");
        let off = Policy {
            network: NetworkPolicy::None,
            ..Policy::default()
        };
        let dir = tempfile::tempdir().unwrap();
        let err = download_model(&off, &m, &dir.path().join("m.gguf"), &mut |_| false).unwrap_err();
        assert_eq!(err, VError::PolicyDenied("network is off".into()));
        let gated = Policy {
            allowed_hosts: vec!["cdn.vendor.com".into()],
            ..Policy::default()
        };
        assert!(matches!(
            fetch_manifest(&gated, "https://cdn.example.org/manifest.json", "k"),
            Err(VError::PolicyDenied(_))
        ));
        // Plain http is refused by the production default, loopback included.
        let m = model("http://127.0.0.1:1/m.gguf", b"x");
        assert!(matches!(
            download_model(
                &Policy::default(),
                &m,
                &dir.path().join("m.gguf"),
                &mut |_| false
            ),
            Err(VError::PolicyDenied(_))
        ));
    }

    #[test]
    fn manifests_are_trusted_only_with_a_valid_signature() {
        let kp = signing::KeyPair::generate().unwrap();
        let manifest = br#"{"schema":1,"issued_at":1,"bundles":[]}"#;
        let sig = kp.sign(manifest);
        let m = verify_manifest_bytes(manifest, &sig, &kp.public_b64()).unwrap();
        assert_eq!(m.bundles.len(), 0);
        let tampered = br#"{"schema":1,"issued_at":2,"bundles":[]}"#;
        assert!(matches!(
            verify_manifest_bytes(tampered, &sig, &kp.public_b64()),
            Err(VError::Integrity { .. })
        ));
        let other = signing::KeyPair::generate().unwrap();
        assert!(matches!(
            verify_manifest_bytes(manifest, &sig, &other.public_b64()),
            Err(VError::Integrity { .. })
        ));
    }
}
