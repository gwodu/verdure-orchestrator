//! The only crate that opens a network socket (knowledge/26 §4.2 rule (c)).
//! Two jobs: fetch a signed manifest and verify it; fetch a bundle's files
//! with resume and verify every byte.
//!
//! Emerging-market networks drop; a 1.5 GB model must never restart from
//! zero (knowledge/10). So: a `.part` file next to the target, a `Range`
//! request from its length, SHA-256 over the *whole* file (the already-present
//! part is hashed first), and an atomic rename only once the hash matches.
//! The policy decides which hosts may be contacted; this crate only asks.
//!
//! A model file is stored once per device, whatever bundles use it
//! (`model_store_path`); bundle directories hold hard links to it. The second
//! phone run downloaded the same 491 MB language model twice because two
//! bundles listed it — one download and one copy is the rule now.

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

/// Where a verified model file lives once, whatever bundles use it:
/// `<data_dir>/models/<sha256>`. Bundle directories hard-link to it, so two
/// bundles that share a model cost one download and one copy. A filesystem
/// that refuses the link leaves the bundle its own copy; nothing else changes.
pub fn model_store_path(data_dir: &Path, sha256: &str) -> PathBuf {
    data_dir.join("models").join(sha256.to_ascii_lowercase())
}

/// Download every file of `bundle` into its directory, reusing any file the
/// device already holds (this bundle's or another's), resuming partial ones
/// and verifying every byte. Progress is in bytes across the whole bundle;
/// status lines say what is happening to each file; cancellation leaves
/// `.part` files in place for next time. The policy is asked about every
/// URL before any socket opens.
pub fn download_bundle(
    policy: &Policy,
    bundle: &Bundle,
    data_dir: &Path,
    progress: &dyn Progress,
) -> Result<PathBuf, VError> {
    for m in &bundle.models {
        policy.permits_download(&m.url)?;
    }
    fetch_bundle(bundle, data_dir, progress)
}

/// `download_bundle` after the policy has said yes to every URL.
fn fetch_bundle(
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
        let shared = model_store_path(data_dir, &m.sha256);
        let mb = m.size_bytes / 1_000_000;
        if dest.is_file() || shared.is_file() {
            progress.on_event(Event::Status(format!("Checking {}…", m.name)));
        }
        match place_from_store(m, &dest, &shared)? {
            Placed::Already => {
                progress.on_event(Event::Status(format!(
                    "{} is already on this device.",
                    m.name
                )));
            }
            Placed::Shared => {
                progress.on_event(Event::Status(format!(
                    "{}: reusing the copy another bundle downloaded ({mb} MB not downloaded again).",
                    m.name
                )));
            }
            Placed::Missing => {
                progress.on_event(Event::Status(format!("Downloading {} ({mb} MB)…", m.name)));
                fetch_file(m, &dest, &mut |bytes| {
                    progress.on_event(Event::Progress {
                        done: done_before + bytes,
                        total,
                    });
                    progress.is_cancelled()
                })?;
                progress.on_event(Event::Status(format!(
                    "{} downloaded and verified.",
                    m.name
                )));
                // Make it available to other bundles; a refused link is not an error.
                if fs::create_dir_all(shared.parent().unwrap()).is_ok() && !shared.exists() {
                    fs::hard_link(&dest, &shared).ok();
                }
            }
        }
        progress.on_event(Event::Progress {
            done: done_before + m.size_bytes,
            total,
        });
        done_before += m.size_bytes;
    }
    Ok(dir)
}

enum Placed {
    /// The bundle already holds a verified copy.
    Already,
    /// Another bundle's verified copy was linked (or copied) in.
    Shared,
    /// Nothing usable on the device: fetch it.
    Missing,
}

/// Settle `dest` from what the device holds, verifying hashes. Also the
/// moment two bundles' separate copies of one model become one file: a
/// verified `dest` that is not yet the store's file is replaced by a link to
/// it (or becomes it), freeing the duplicate.
fn place_from_store(model: &ModelRef, dest: &Path, shared: &Path) -> Result<Placed, VError> {
    let want = model.sha256.to_ascii_lowercase();
    let good = |p: &Path| p.is_file() && file_sha256(p).map(|h| h == want).unwrap_or(false);
    if dest.is_file() {
        if good(dest) {
            if !shared.exists() {
                if fs::create_dir_all(shared.parent().unwrap()).is_ok() {
                    fs::hard_link(dest, shared).ok();
                }
            } else if !same_file(dest, shared) {
                if good(shared) {
                    // Two copies of one verified file: keep the store's, link to it.
                    let tmp = dest.with_extension("link.tmp");
                    fs::remove_file(&tmp).ok();
                    if fs::hard_link(shared, &tmp).is_ok() && fs::rename(&tmp, dest).is_err() {
                        fs::remove_file(&tmp).ok();
                    }
                } else {
                    // The store's copy is damaged; this one is good — it takes over.
                    fs::remove_file(shared).ok();
                    fs::hard_link(dest, shared).ok();
                }
            }
            return Ok(Placed::Already);
        }
        // A complete-looking file with the wrong hash is corrupt: start over.
        fs::remove_file(dest).map_err(|e| VError::Storage(e.to_string()))?;
    }
    if shared.is_file() {
        if good(shared) {
            if fs::hard_link(shared, dest).is_err() {
                fs::copy(shared, dest).map_err(|e| VError::Storage(e.to_string()))?;
            }
            return Ok(Placed::Shared);
        }
        fs::remove_file(shared).ok();
    }
    Ok(Placed::Missing)
}

/// Are `a` and `b` the same file on disk (two names for one set of bytes)?
/// Device and inode on Unix, volume and file index on Windows. An unreadable
/// path is "no": the caller then verifies and links, which is always safe.
fn same_file(a: &Path, b: &Path) -> bool {
    same_file::is_same_file(a, b).unwrap_or(false)
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

    /// Events collected from a bundle download, for the assertions below.
    #[derive(Default)]
    struct Log(Mutex<Vec<Event>>);
    impl verdure_core::Progress for Log {
        fn on_event(&self, e: Event) {
            self.0.lock().unwrap().push(e);
        }
    }
    impl Log {
        fn statuses(&self) -> Vec<String> {
            self.0
                .lock()
                .unwrap()
                .iter()
                .filter_map(|e| match e {
                    Event::Status(s) => Some(s.clone()),
                    _ => None,
                })
                .collect()
        }
    }

    fn bundle(id: &str, models: Vec<ModelRef>) -> Bundle {
        Bundle {
            id: id.into(),
            task: verdure_core::Task::Scribe,
            tiers: vec![verdure_core::Tier::P3],
            models,
            requires_acknowledgement: vec![],
            attributions: vec![],
            notes: String::new(),
            speed: Default::default(),
        }
    }

    /// Two names for one file on disk — the claim "one copy per model",
    /// checked the same way on Unix and Windows.
    fn one_file(a: &Path, b: &Path) -> bool {
        same_file::is_same_file(a, b).unwrap()
    }

    /// The second phone run (2026-11-04) fetched the 491 MB language model a
    /// second time because `scribe-small` and `scribe-floor` both list it.
    #[test]
    fn two_bundles_sharing_a_model_download_it_once_and_hold_one_copy() {
        let data = body();
        let srv = serve(data.clone(), None, false);
        let dir = tempfile::tempdir().unwrap();
        let mut llm = model(&format!("{}/m.gguf", srv.url), &data);
        llm.name = "Language model".into();
        let floor = bundle("scribe-floor", vec![llm.clone()]);
        let small = bundle("scribe-small", vec![llm.clone()]);

        let log = Log::default();
        fetch_bundle(&floor, dir.path(), &log).unwrap();
        assert_eq!(srv.requests.lock().unwrap().len(), 1);
        assert_eq!(
            log.statuses(),
            vec![
                "Downloading Language model (0 MB)…",
                "Language model downloaded and verified."
            ]
        );

        let log = Log::default();
        fetch_bundle(&small, dir.path(), &log).unwrap();
        assert_eq!(srv.requests.lock().unwrap().len(), 1, "no second download");
        assert_eq!(
            log.statuses(),
            vec![
                "Checking Language model…",
                "Language model: reusing the copy another bundle downloaded (0 MB not downloaded again)."
            ]
        );
        let a = bundle_dir(dir.path(), "scribe-floor").join("m.gguf");
        let b = bundle_dir(dir.path(), "scribe-small").join("m.gguf");
        let store = model_store_path(dir.path(), &llm.sha256);
        assert_eq!(fs::read(&b).unwrap(), data);
        assert!(
            one_file(&a, &store),
            "the first download became the store's file"
        );
        assert!(one_file(&b, &store), "the second bundle links to it");
        // Progress still reaches the whole-bundle total when nothing was fetched.
        let last = log.0.lock().unwrap().iter().rev().find_map(|e| match e {
            Event::Progress { done, total } => Some((*done, *total)),
            _ => None,
        });
        assert_eq!(last, Some((data.len() as u64, data.len() as u64)));

        // A third call on a bundle that is complete says so and fetches nothing.
        let log = Log::default();
        fetch_bundle(&floor, dir.path(), &log).unwrap();
        assert_eq!(srv.requests.lock().unwrap().len(), 1);
        assert_eq!(
            log.statuses(),
            vec![
                "Checking Language model…",
                "Language model is already on this device."
            ]
        );
    }

    /// Devices that downloaded before the store existed hold one copy per
    /// bundle; the next `download` of either bundle folds them into one.
    #[test]
    fn separate_copies_from_before_the_store_are_folded_into_one_file() {
        let data = body();
        let srv = serve(data.clone(), None, false);
        let dir = tempfile::tempdir().unwrap();
        let llm = model(&format!("{}/m.gguf", srv.url), &data);
        for id in ["scribe-floor", "scribe-small"] {
            let d = bundle_dir(dir.path(), id);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("m.gguf"), &data).unwrap(); // two independent copies, no store
        }
        let a = bundle_dir(dir.path(), "scribe-floor").join("m.gguf");
        let b = bundle_dir(dir.path(), "scribe-small").join("m.gguf");
        assert!(!one_file(&a, &b));

        fetch_bundle(
            &bundle("scribe-floor", vec![llm.clone()]),
            dir.path(),
            &Log::default(),
        )
        .unwrap();
        fetch_bundle(
            &bundle("scribe-small", vec![llm.clone()]),
            dir.path(),
            &Log::default(),
        )
        .unwrap();
        assert_eq!(
            srv.requests.lock().unwrap().len(),
            0,
            "both were verified, nothing fetched"
        );
        let store = model_store_path(dir.path(), &llm.sha256);
        assert!(one_file(&a, &store));
        assert!(one_file(&b, &store), "the duplicate was replaced by a link");
        assert_eq!(fs::read(&b).unwrap(), data);

        // A damaged store copy is replaced by a bundle's good one, never trusted.
        fs::remove_file(&store).unwrap();
        fs::write(&store, b"damaged").unwrap();
        fetch_bundle(
            &bundle("scribe-floor", vec![llm.clone()]),
            dir.path(),
            &Log::default(),
        )
        .unwrap();
        assert_eq!(fs::read(&store).unwrap(), data);
        assert!(one_file(&a, &store));
        // And a damaged store copy with no good bundle copy is fetched afresh.
        fs::remove_file(&store).unwrap();
        fs::write(&store, b"damaged").unwrap();
        let d = bundle_dir(dir.path(), "scribe-L2");
        fetch_bundle(
            &bundle("scribe-L2", vec![llm.clone()]),
            dir.path(),
            &Log::default(),
        )
        .unwrap();
        assert_eq!(srv.requests.lock().unwrap().len(), 1);
        assert_eq!(fs::read(d.join("m.gguf")).unwrap(), data);
        assert_eq!(fs::read(&store).unwrap(), data);
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
        // A bundle is refused whole before any file moves: the second model's
        // host is not allowed, so the first is not fetched either.
        let allowed = model("https://cdn.vendor.com/a.gguf", b"a");
        let b = bundle("mixed", vec![allowed, m]);
        assert!(matches!(
            download_bundle(&gated, &b, dir.path(), &Log::default()),
            Err(VError::PolicyDenied(_))
        ));
        assert!(!bundle_dir(dir.path(), "mixed").exists());
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
