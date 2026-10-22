//! `verdure` — the SDK's developer tool. No engines; everything here runs on
//! any machine in milliseconds: read the device, mint and check licence
//! tokens, sign and check manifests, render a schema's grammar, and dry-run
//! `recommend` against a manifest.
//!
//! Secrets: `license keygen` writes the signing key to a file OUTSIDE the
//! repo (default `~/.verdure/issuer.key`, owner-only) and prints only the
//! public half. The public half goes into `crates/verdure-ffi/keys/issuer.pub`.

use std::fs;
use std::path::PathBuf;

use verdure_core::signing::KeyPair;
use verdure_core::{Issuer, LicenseClaims, Manifest, Task, Tier, Verifier};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("verdure: {e}");
            2
        }
    };
    std::process::exit(code);
}

const USAGE: &str = "usage:
  verdure capabilities [--data-dir DIR]
  verdure license keygen [--out FILE]              (default ~/.verdure/issuer.key)
  verdure license issue --key FILE --vendor ID [--seats N] [--days N] [--bundle ID]... [--ack NAME]...
  verdure license verify --public KEY TOKEN
  verdure manifest check FILE
  verdure manifest sign --key FILE FILE             (writes FILE.sig)
  verdure manifest verify --public KEY FILE
  verdure recommend --manifest FILE --task scribe|tutor|chw_forms [--tier L2] [--ram MB] [--disk MB]
  verdure grammar SCHEMA.json";

fn run(args: &[String]) -> Result<(), String> {
    let mut a = Args::new(args);
    match (
        a.next_positional().as_deref(),
        a.next_positional().as_deref(),
    ) {
        (Some("capabilities"), None) => {
            let dir = a
                .opt("--data-dir")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir);
            let profile = verdure_hw::probe(&dir);
            let caps = verdure_core::Capabilities::from_profile(profile, vec![]);
            println!("{}", serde_json::to_string_pretty(&caps).unwrap());
        }
        (Some("license"), Some("keygen")) => {
            let out = a
                .opt("--out")
                .map(PathBuf::from)
                .unwrap_or_else(default_key_path);
            if out.exists() {
                return Err(format!(
                    "{} exists; refusing to overwrite a signing key",
                    out.display()
                ));
            }
            let kp = KeyPair::generate()?;
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            fs::write(&out, kp.secret_b64()).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&out, fs::Permissions::from_mode(0o600))
                    .map_err(|e| e.to_string())?;
            }
            eprintln!(
                "secret key written to {} (keep it out of the repo)",
                out.display()
            );
            println!("{}", kp.public_b64());
        }
        (Some("license"), Some("issue")) => {
            let key = read_key(&a.req("--key")?)?;
            let vendor = a.req("--vendor")?;
            let seats: u32 = a
                .opt("--seats")
                .map(|s| {
                    s.parse()
                        .map_err(|_| "--seats must be a number".to_string())
                })
                .transpose()?
                .unwrap_or(0);
            let days: u64 = a
                .opt("--days")
                .map(|s| s.parse().map_err(|_| "--days must be a number".to_string()))
                .transpose()?
                .unwrap_or(0);
            let mut bundles = a.multi("--bundle");
            if bundles.is_empty() {
                bundles.push("*".into());
            }
            let now = verdure_core::unix_now();
            let claims = LicenseClaims {
                vendor_id: vendor,
                seats,
                issued_at: now,
                expires_at: if days == 0 { 0 } else { now + days * 86_400 },
                bundles,
                acknowledgements: a.multi("--ack"),
            };
            println!("{}", Issuer::new(key).issue(&claims));
        }
        (Some("license"), Some("verify")) => {
            let public = a.req("--public")?;
            let token = a.next_positional().ok_or("a TOKEN is required")?;
            let claims = Verifier::new(&public)
                .verify(&token, verdure_core::unix_now())
                .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&claims).unwrap());
        }
        (Some("manifest"), Some("check")) => {
            let path = a.next_positional().ok_or("a manifest FILE is required")?;
            let m = Manifest::parse(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            for b in &m.bundles {
                let tiers: Vec<String> = b.tiers.iter().map(|t| t.to_string()).collect();
                println!(
                    "{}  task={}  tiers={}  download={} MB  needs {} MB of memory  {} model(s)",
                    b.id,
                    b.task,
                    tiers.join(","),
                    b.download_bytes() / 1_048_576,
                    b.required_mb(),
                    b.models.len()
                );
            }
        }
        (Some("manifest"), Some("sign")) => {
            let key = read_key(&a.req("--key")?)?;
            let path = a.next_positional().ok_or("a manifest FILE is required")?;
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            Manifest::parse(&bytes).map_err(|e| format!("refusing to sign: {e}"))?;
            let sig = key.sign(&bytes);
            let sig_path = format!("{path}.sig");
            fs::write(&sig_path, &sig).map_err(|e| e.to_string())?;
            eprintln!("signature written to {sig_path}");
            println!("{}", key.public_b64());
        }
        (Some("manifest"), Some("verify")) => {
            let public = a.req("--public")?;
            let path = a.next_positional().ok_or("a manifest FILE is required")?;
            let bytes = fs::read(&path).map_err(|e| e.to_string())?;
            let sig = fs::read_to_string(format!("{path}.sig"))
                .map_err(|e| format!("{path}.sig: {e}"))?;
            let m = verdure_download::verify_manifest_bytes(&bytes, &sig, &public)
                .map_err(|e| e.to_string())?;
            println!(
                "ok: {} bundle(s), issued_at {}",
                m.bundles.len(),
                m.issued_at
            );
        }
        (Some("recommend"), None) => {
            let path = a.req("--manifest")?;
            let task = match a.req("--task")?.as_str() {
                "scribe" => Task::Scribe,
                "tutor" => Task::Tutor,
                "chw_forms" | "chw" => Task::ChwForms,
                other => return Err(format!("unknown task {other}")),
            };
            let m = Manifest::parse(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let profile = verdure_hw::probe(&std::env::temp_dir());
            let tier: Tier = match a.opt("--tier") {
                Some(t) => t.parse()?,
                None => profile.tier(),
            };
            let ram = a
                .opt("--ram")
                .map(|s| s.parse::<u64>().map_err(|_| "--ram must be MB".to_string()))
                .transpose()?
                .unwrap_or(profile.available_ram_mb);
            let disk = a
                .opt("--disk")
                .map(|s| {
                    s.parse::<u64>()
                        .map_err(|_| "--disk must be MB".to_string())
                })
                .transpose()?
                .unwrap_or(profile.free_disk_mb);
            let licence = LicenseClaims {
                vendor_id: "dry-run".into(),
                seats: 0,
                issued_at: 0,
                expires_at: 0,
                bundles: vec!["*".into()],
                acknowledgements: vec!["llama-community".into(), "gemma-terms".into()],
            };
            match verdure_core::registry::resolve(&m, task, tier, ram, disk, &licence) {
                Ok(r) => {
                    println!("{}", serde_json::to_string_pretty(&r.bundle).unwrap());
                    if let Some(from) = r.downgraded_from {
                        println!("downgraded from {from}");
                    }
                    for s in r.skipped {
                        println!("skipped: {s}");
                    }
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        (Some("grammar"), Some(path)) => {
            let schema: serde_json::Value =
                serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let g = verdure_grammar::to_gbnf(&schema).map_err(|e| e.to_string())?;
            print!("{}", g.text);
            for w in g.warnings {
                eprintln!("note: {w}");
            }
        }
        _ => return Err(USAGE.to_string()),
    }
    Ok(())
}

fn default_key_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".verdure").join("issuer.key")
}

fn read_key(path: &str) -> Result<KeyPair, String> {
    let s = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    KeyPair::from_secret_b64(&s)
}

/// Tiny argument reader: `--flag value` options anywhere, positionals in order.
struct Args {
    items: Vec<String>,
    pos: usize,
}

impl Args {
    fn new(items: &[String]) -> Self {
        Args {
            items: items.to_vec(),
            pos: 0,
        }
    }
    fn next_positional(&mut self) -> Option<String> {
        while self.pos < self.items.len() {
            let i = self.pos;
            self.pos += 1;
            if self.items[i].starts_with("--") {
                self.pos += 1; // skip its value
                continue;
            }
            return Some(self.items[i].clone());
        }
        None
    }
    fn opt(&self, flag: &str) -> Option<String> {
        self.items
            .iter()
            .position(|x| x == flag)
            .and_then(|i| self.items.get(i + 1).cloned())
    }
    fn req(&self, flag: &str) -> Result<String, String> {
        self.opt(flag)
            .ok_or_else(|| format!("{flag} is required\n{USAGE}"))
    }
    fn multi(&self, flag: &str) -> Vec<String> {
        self.items
            .windows(2)
            .filter(|w| w[0] == flag)
            .map(|w| w[1].clone())
            .collect()
    }
}
